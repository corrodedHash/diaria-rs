# AGENTS.md

Guide for AI agents (and humans) working in this repository. Read this before
making changes.

## Project

`diaria` is an encrypted, plain-text command-line diary. Entries are compressed
(brotli) and encrypted (X448 + XChaCha20Poly1305) at rest under an XDG data
directory. The whole vault can live in a git repo and sync across machines.

The crypto stack (KDF, AEAD, key algorithm, on-disk layout) is **implied by a
single integer format version** in `manifest.toml` — never read from disk as
free-form parameters. See `src/manifest.rs`.

## Commands you must run

CI runs these; they must all pass before any change is merged.

```
cargo fmt --all -- --check        # formatting check (do NOT skip)
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
mise run e2e                      # end-to-end tests (Python/pytest), builds first
```

Shortcuts (`mise.toml`): `mise run check` runs fmt-check + clippy + test.
`mise run e2e` runs the Python e2e suite (needs `uv`, provided via mise).

The toolchain is pinned in `rust-toolchain.toml` (Rust **1.96**, edition 2024).
`rustup toolchain install` picks it up automatically.

## Code conventions

- **No comments unless requested.** Existing code is comment-light; doc
  comments (`///`) explain _why_, never _what_. Match that tone.
- `rustfmt.toml` sets `edition = "2024"`. Run `cargo fmt` before committing.
- Errors use `thiserror` derives with `#[error("...")]` messages that are
  user-facing (they reach the terminal via `Display`, not `Debug`).

## Architecture

### Dependency injection (the central pattern)

Every command is a `struct Command { ... Box<dyn Trait> ... }` constructed via
`Di::xxx()` factory methods. **`src/di.rs` is the only composition root** — the
single place production code chooses concrete implementations. Tests bypass it
and pass mocks straight into `Command::new(...)`.

To add or change a command's dependencies:

1. Add the trait field(s) to the command struct.
2. Add parameters to `Command::new`.
3. Wire concrete impls in the matching `Di::xxx()` in `src/di.rs`.

Never call `std::env`, `std::fs`, or `println!` directly from command logic — go
through the injected trait so tests stay deterministic.

All mocks are `#[mockall::automock]`-generated.

### Command layout

`src/commands/<name>.rs` — one file per CLI subcommand. Each has:

- `pub struct Command { injected deps }`
- `impl Command { pub fn new(...) -> Self; pub fn execute(...) -> Result<...> }`
- A `#[cfg(test)] mod tests` with mock-driven unit tests.

Register a new command in three places:

1. `src/commands.rs` — `mod name;` + `pub use name::Command as CmdName;`
2. `src/di.rs` — `pub fn name() -> CmdName { ... }`
3. `src/main.rs` — add a `Commands::Name` variant to the clap `Subcommand` enum
   and a match arm in `run()`.

### CLI (`src/main.rs`)

clap derive, `#[command(version, about, name = "diaria")]`. The subcommand is
`Option<Commands>` so a bare `diaria` defaults to `Status` rather than erroring.
Keep that default-in-default pattern if the no-arg behavior should stay.

`main()` prints errors via `{e}` (Display) to stderr and returns
`ExitCode::FAILURE`; success returns `ExitCode::SUCCESS`.

### Vault on-disk layout

Resolved via `xdg::BaseDirectories::with_prefix("diaria")` →
`$XDG_DATA_HOME/diaria/`:

```
<base>/key.key         # password-encrypted private key: salt||nonce||ciphertext
<base>/key.pub         # X448 public key (56 bytes)
<base>/key.sym         # symmetric salt (32 bytes)
<base>/manifest.toml   # version = <u32>
<base>/entries/        # *.diaria entry files, optionally a git repo for sync
```

`DiariaMetaRepository` owns key/manifest I/O; `DiariaEntryRepository` owns
entry listing/reading/writing. `DiariaFsRepository` implements both.

### Entry format

`src/entry/mod.rs` handles the envelope: `MAGIC_TAG (b"DIARIA") || version:u8`.
It dispatches on the version byte to a per-version body codec. Adding a format
version = new `versionNN.rs` module + a new `match` arm in `decode()`. Do not
mutate existing version codecs.

`src/entry/version01.rs` is the v1 body: ephemeral X448 keypair → HKDF-SHA256
→ XChaCha20Poly1305, with brotli compression before encryption. The HKDF input
key material is the ephemeral X448 shared secret concatenated with the
per-vault symmetric key (a local secret), with a fixed domain-separation salt.
Body layout: `ephemeral_pub(56) || nonce(24) || ciphertext`.

### Keys

`init` generates an X448 keypair, a 32-byte symmetric key (`key.sym`), and an
Argon2 salt; the private key is encrypted with a key derived from the password
(`src/crypto.rs::derive_key_from_password`). Reading entries needs the
password (to decrypt the private key); **adding** entries only needs the public
key + `key.sym`, so `add` never prompts for a password.

`key.sym` is a local secret folded into the HKDF _input key material_, so 
breaking X448 alone is not enough to recover an entry's AEAD key.
It must not be synced to remotes; only the `entries/` subtree is synced (via
`diaria sync`), which keeps `key.sym` local.

### Security limitations

- **No sender authentication.** The XChaCha20Poly1305 tag proves integrity, not
  authorship. Anyone with `key.pub` + `key.sym` (both unencrypted in the vault
  dir) can author entries that decrypt cleanly and look authentic. This is a
  deliberate trade-off for "add needs no password". The `key.sym` is supposed to stay local,
  which is the only guard against addition of forged entries.
  Documenting, not patching: closing it would require signing entries with the
  long-term private key, which makes `add` need the password.

## Testing

### Unit tests (Rust, `cargo test`)

Live inline in each module under `#[cfg(test)] mod tests`. They construct mocks
and assert on what `execute` did. Patterns to follow:

- Set up `Mock*` expectations, then call `Command::new(...).execute(...)`.
- Use `.withf(...)` on `MockUserOutput::expect_print` to assert on printed
  content without coupling to exact formatting.
- Leave a mock with **no expectations** to assert "this must never be called"
  (mockall fails any unexpected call).
- Committed binary testdata lives in `src/commands/testdata/` (keys, a
  manifest, and an encrypted entry) and is loaded via `include_bytes!`.
  Password for that testdata is `"test"`.

### End-to-end tests (Python/pytest, `end_to_end/`)

Drive the real binary as a subprocess against an isolated `XDG_DATA_HOME`. Run
with `mise run e2e` (builds first, then `pytest`).

- `end_to_end/helper.py` defines the `diaria` fixture (resolves the binary from
  `DIARIA` env or a build dir) and the `vault` fixture (creates a fresh,
  `diaria init`-ed vault under `tmp_path` with `DIARIA_PASSWORD=password`).
  **Reuse `vault`/`diaria` fixtures**; only build env dicts by hand for
  legacy/uninitialized scenarios.
- The password env var is `DIARIA_PASSWORD` (`src/password.rs::PASSWORD_ENV`).
  Set it in the env dict you pass to `subprocess.run` to avoid TTY prompts.
- Two committed example vaults: `entry_data/` (legacy v0, no manifest — must be
  refused) and `entry_data_v1/` (v1, manifest present — must decode). Use these
  for format-compat tests rather than minting new key material.
- Tests assert on `result.stdout` / `result.stderr` / `returncode`. Prefer
  substring assertions (`"..." in out`) over exact full-output matches —
  formatting drifts, but the facts being tested shouldn't.

### When adding a command

1. Add unit tests in its `mod tests` (mock-based).
2. Add an e2e test in `end_to_end/test_<name>.py` using the `vault` fixture.
3. Run all three gates (fmt+clippy, `cargo test`, `mise run e2e`).

## Gotchas

- Entry filenames use a **local, offset-less** timestamp
  (`%Y-%m-%dT%H:%M:%S.diaria`). Parse as a `NaiveDateTime` then anchor with
  `Local.from_local_datetime(...)` — parsing straight to `DateTime` demands an
  offset the filename never carries and silently drops every entry. This was a
  real bug in `stats`.
- `sync` shells out to `git -C <entries> add/commit/push/pull`. It's a no-op
  (with a message) when `entries/.git` doesn't exist.
- `Manifest::parse` rejects version 0 and any version > `CURRENT_VERSION` (a
  future binary's vault). A missing manifest = legacy "v0" vault, refused with
  `LegacyUnversioned`. Never invent a version the binary doesn't know how to
  handle.

## Commits

Conventional Commits, enforced by `git-cliff` (`cliff.toml`) for changelog
generation. Use `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`,
`perf:`. Releases are tagged `v<semver>` (see release process below); the
version in `Cargo.toml` is bumped as part of the release flow, not by hand
during feature work.

Do not commit unless explicitly asked. When you do, stage only intended files
and never commit secrets.

## Release process

### Principles

- **Conventional commits drive the version.** `git-cliff` reads all commits
  since the last tag and derives the bump: `feat:` → minor, `fix:` → patch,
  `BREAKING CHANGE:` or `feat!:` → major.
- **All changes land via PR.** Never push to `main` directly. Feature work,
  fixes, and the release itself all go through separate branches and PRs.
- **The release script automates the cut.** `mise run tag` (wraps
  `tools/release-tag.sh`) bumps `Cargo.toml`, commits, opens a PR to `main`,
  waits for the squash merge, then pushes the tag to the merge commit so the CI
  release workflow fires.

### Steps

1. **Ensure `main` is up to date** and the working tree is clean.
2. **Run `mise run tag`.** This will:
   - Bump the version using `git-cliff --bumped-version` (or `VERSION=vX.Y.Z`
     env var to override).
   - Show the generated changelog notes and ask for confirmation.
   - Commit the `Cargo.toml`/`Cargo.lock` bump and push a `release/vX.Y.Z`
     branch.
   - Open a squash-merge PR (`release/vX.Y.Z` → `main`) with auto-merge
     enabled.
   - **Wait** for the PR to merge.
   - Tag the merge commit on `main` and push the tag — this triggers the
     release workflow.
3. **Verify the release CI** completes successfully.

The script requires `gh` (GitHub CLI) and `git-cliff` (provided via `mise`).
Override the bumped version: `VERSION=v1.2.3 mise run tag`.

## Sandboxed agent runs (Docker + clone)

`tools/agent-sandbox.sh` wraps opencode in a Docker container on a throwaway
**clone** of the repo.  A git worktree is not used because a worktree's `.git`
is just a pointer back into the main repo's object DB; mounting only the
worktree dir into a container leaves every git operation dangling.  A full
clone is self-contained, so the container owns a `.git` it can actually use.

```
tools/agent-sandbox.sh --agent sandbox --auto "refactor the sync module"
```

- Clones the repo to `/tmp/diaria-<ts>/` and creates a new `agent-<ts>` branch.
- Mounts that clone into a container with Rust, opencode, and `mise`.
- Passes all arguments through to `opencode` — use `--agent sandbox --auto`
  to run the sandbox agent with auto-approval.
- On exit the agent's branch is **fetched back into the main repo** as
  `refs/heads/agent-<ts>` — a purely local, host-side fetch of objects straight
  from the clone dir, so no `git push` or remote auth is needed either inside
  the container or on the host.  Review/merge/PR it from your normal checkout,
  then delete the branch when done.  No `agent-<ts>` ref is left dangling in
  the clone because the clone is removed.
- Rebuild the image: `docker build -t diaria-agent .` (the script builds it
  automatically otherwise).

Safety properties:
- **Container isolation.** Only the clone directory is writable.  The rest
  of the filesystem is read-only (except tmpfs mounts for `/tmp`).
- **Repo isolation.** The main repository (`src/`, refs, objects, `HEAD`) is
  never mounted.  The agent physically cannot touch `main` or any other
  existing ref; the only thing it can produce is the local `agent-<ts>` branch,
  which the host imports into the main repo on exit.
- **`external_directory: deny`.** The sandbox agent config blocks access to
  any path outside the workspace directory.
