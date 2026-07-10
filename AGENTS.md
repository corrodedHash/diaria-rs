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
  comments (`///`) explain *why*, never *what*. Match that tone.
- `rustfmt.toml` sets `edition = "2024"`. Run `cargo fmt` before committing.
- Clippy is `-D warnings` — any warning fails CI. Watch for `redundant_closure`
  (`|| vec![]` → `std::vec::Vec::new`) and similar.
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
through the injected trait so tests stay deterministic. The injected traits:

| Trait | File | Real impl | Mock |
|---|---|---|---|
| `DiariaEntryRepository` | `entry/repository.rs` | `DiariaFsRepository` | `MockDiariaEntryRepository` |
| `DiariaMetaRepository` | `entry/repository.rs` | `DiariaFsRepository` | `MockDiariaMetaRepository` |
| `DiariaKeyManager` | `entry/key_manager.rs` | `FsKeyManager` | `MockDiariaKeyManager` |
| `PasswordService` | `password.rs` | `TerminalPasswordService` | `MockPasswordService` |
| `Environment` | `environment.rs` | `SystemEnvironment` | `MockEnvironment` |
| `UserOutput` | `stdout_printer.rs` | `RealUserOutput` | `MockUserOutput` |
| `FileLoader` | `file_loader.rs` | `RealFileLoader` | `MockFileLoader` |

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
→ XChaCha20Poly1305, with brotli compression before encryption. Body layout:
`ephemeral_pub(56) || nonce(24) || ciphertext`.

### Keys

`init` generates an X448 keypair, a 32-byte symmetric salt, and an Argon2
salt; the private key is encrypted with a key derived from the password
(`src/crypto.rs::derive_key_from_password`). Reading entries needs the
password (to decrypt the private key); **adding** entries only needs the public
key + symmetric salt, so `add` never prompts for a password.

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

- `list_entries()` is raw `read_dir` — it returns **everything** in `entries/`,
  including a `.git` directory. Use `list_entry_metadata()` when you need only
  real diary entries (it parses each filename's timestamp and drops the rest).
  This bit `status`: a git repo under `entries/` inflated the raw count.
- `read` and `summarize` still call `list_entries()`; the fuzzy-select / date
  match partly hides the `.git` issue, but filtering to `*.diaria` would be
  cleaner.
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
- Clippy's `redundant_closure` lint fires on `|| vec![]` / `|| Vec::new()`;
  use `std::vec::Vec::new` as a function pointer in mock returners.

## Commits

Conventional Commits, enforced by `git-cliff` (`cliff.toml`) for changelog
generation. Use `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`,
`perf:`. Releases are tagged `v<semver>` (see `tools/release-tag.sh`); the
version in `Cargo.toml` is bumped as part of the release flow, not by hand
during feature work.

Do not commit unless explicitly asked. When you do, stage only intended files
and never commit secrets.
