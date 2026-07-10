# diaria

An encrypted, plain-text personal diary on the command line. Entries are
compressed and encrypted at rest so that only the holder of the password can
read them back.

## Getting started

Requires the Rust toolchain (pinned in `rust-toolchain.toml`). Install the
binary:

```sh
cargo install --path .
```

### Arch Linux

A VCS package is published on the AUR as [`diaria-rs-git`](https://aur.archlinux.org/packages/diaria-rs-git),
kept in sync with `main` by the `.github/workflows/aur.yml` workflow. Install
with your AUR helper:

```sh
paru -S diaria-rs-git   # or: yay -S diaria-rs-git
```

The PKGBUILD template lives in `dist/aur/`; see there and the workflow file for
the one-time AUR/SSH-key setup the maintainer needs.

Set up your diary — this generates your keys and asks for a password:

```sh
diaria init
```

Then write your first entry:

```sh
diaria add
```

That's it. Use `diaria read` to decrypt an entry, and `diaria --help` to see
everything else (stats, summaries, importing, and git sync).

## How it works

Entries are protected by two keys:

- An **asymmetric** keypair encrypts entries. The public key is enough to write
  new entries, so adding an entry never needs your password. The private key —
  required to read entries back — is itself encrypted with your password.
- A **symmetric** key is mixed into the key derivation: it is concatenated with
  the per-entry ephemeral Diffie-Hellman shared secret and fed into HKDF-SHA256
  as input key material. It is a local secret (it never leaves the vault
  directory and is not synced to git), so reading entries back requires both the
  X448 private key *and* this symmetric key.

Contents are compressed before encryption, and each entry is versioned on disk
so the format can evolve.

Because entries are just encrypted files under your data directory, the whole
diary can be kept in a git repository and synced across machines with
`diaria sync`.

### Security limitations

- **No sender authentication.** Anyone with access to the vault directory's
  `key.pub` and `key.sym` (both unencrypted) can author diary entries that
  decrypt cleanly and are indistinguishable from yours — the XChaCha20Poly1305
  tag only proves integrity, not authorship. This is a deliberate trade-off for
  the "adding needs no password" UX. If you sync entries to an untrusted git
  remote (a public repo, a shared server), a peer with write access can inject
  forged entries. Keep the vault directory local and only sync the `entries/`
  subtree to remotes you trust, or consider adding an out-of-band signature if
  authorship matters to you.

## Development

The toolchain is pinned via `rust-toolchain.toml`. [mise](https://mise.jdx.dev)
provides task shortcuts:

```sh
mise run build     # cargo build
mise run test      # cargo test
mise run lint      # clippy, warnings as errors
mise run fmt       # cargo fmt
mise run check     # everything CI runs: fmt check + clippy + tests
```

CI (`.github/workflows/ci.yml`) runs formatting, clippy, and tests on every
push to `main` and every pull request.
