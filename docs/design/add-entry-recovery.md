# Design note: recovering an entry when saving fails

Status: **proposal** (no product code changed by this note).
Scope: the `add` command (`src/commands/add.rs`). Sandbox is intentionally
dropped from the Rust rewrite; this note only concerns not losing freshly
written text.

## Problem

`add` obtains entry text (from `-i <file>` or the interactive `Editor`), then
runs a series of steps that can each fail *after* the text exists only in
memory:

```
load_manifest_version()?            // (1) before input — safe
input = file_loader / Editor        // (2) text now exists in memory
reject empty                        // (3)
load_symmetric_key() / load_public_key()   // (4) FsKeyManager .unwrap()s internally
encode(...)?                        // (5) compression / crypto
repository.add_entry(&encoded)?     // (6) disk write
```

For the **interactive editor** path this is a real data-loss risk: a user can
type a long entry, then have step (4)–(6) fail (missing/corrupt key files, a
key-manager `unwrap` panic, disk full, a read-only vault, an encode error) and
lose everything they just wrote. The `-i <file>` path is lower-risk because the
plaintext still exists in the source file, but the same failure modes apply.

Two things already reduce the blast radius and should be preserved:

- The manifest check (1) runs **before** input is gathered, so a legacy/unknown
  vault fails fast before the editor ever opens.
- Empty/whitespace input is rejected (3) before any crypto, so we never spend
  work — or trigger recovery — on nothing.

## Options

**A. Write raw plaintext to a predictable recovery file, then report it.**
On any error after step (2), persist `input` to e.g.
`$XDG_DATA_HOME/diaria/recovery/<timestamp>.txt` (created `0600`) and append the
path to the surfaced error.
- Pro: nothing is lost; the user has a concrete file to recover from and re-`add`.
- Con: writes **unencrypted** plaintext to disk — must be `0600`, clearly
  labelled, and ideally in the vault dir (already the secret-bearing location),
  not `/tmp`. If the failure *is* a disk/permission problem, this write may also
  fail (see hybrid below).

**B. Print the plaintext to stdout/stderr on failure.**
Dump the text so it survives in terminal scrollback / can be redirected.
- Pro: no new on-disk secret; trivial.
- Con: easy to miss, may be lost if the terminal is closed, and can land in logs
  or CI capture. Poor UX for a long entry.

**C. Re-run / retry affordance.**
Keep the text in memory and offer to reopen the editor or retry the save.
- Pro: best interactive UX; no plaintext hits disk unless the user chooses.
- Con: only helps the interactive path; useless for non-interactive/`-i` runs
  and for hard failures (bad keys) where retrying can't succeed; more complex.

## Recommendation

**Option A as the primary mechanism, with B as a fallback** ("hybrid"):

1. Structure `add` so that once `input` is in hand, the remaining fallible work
   runs inside a guard. On `Err`, before propagating:
   - write `input` to `<vault>/recovery/<%Y-%m-%dT%H:%M:%S>.txt` with `0600`
     permissions;
   - wrap/annotate the error so the user is told: the save failed, the entry was
     **not** encrypted, and here is the recovery file path.
2. If writing the recovery file *also* fails (e.g. the vault is read-only),
   fall back to printing the plaintext to stderr so it is not silently lost.
3. Do **not** pursue C now; a retry loop can be layered on later for the
   interactive path but is not required to close the data-loss hole.

Rationale: A covers both entry paths and all failure modes with a durable
artifact; B alone is too easy to lose; C alone doesn't cover non-interactive
use or unrecoverable errors. The security cost of A (plaintext on disk) is
bounded by `0600` perms, placement inside the already-sensitive vault dir, and
an explicit warning — and it only ever triggers on failure.

## Follow-up implementation task

> **Implement recovery-on-save-failure in `add` (`src/commands/add.rs`).**
> After input is captured and the empty-check passes, run the key-load / encode
> / `add_entry` steps inside a guard. On any error:
> 1. Write the raw `input` to `<base_dir>/recovery/<timestamp>.txt`, creating the
>    `recovery/` dir if needed, with `0600` file permissions.
> 2. Return an error whose `Display` states the entry was not saved/encrypted and
>    names the recovery file.
> 3. If the recovery write fails, additionally print the plaintext to stderr.
>
> Notes:
> - Add a `RecoveryError`/extend `AddError` as needed; keep `main()`'s
>   `Error: {e}` Display-based reporting.
> - Prefer routing the recovery write through `DiariaMetaRepository`
>   (it already owns `get_base_dir()`), so it stays unit-testable with a mock
>   rather than touching the real filesystem in tests.
> - Tests: unit-test that a failing `encode`/`add_entry` (mocked) triggers a
>   recovery write carrying the exact plaintext; e2e-test that a wrong/missing
>   key or read-only vault leaves a `recovery/*.txt` with the entry text.
