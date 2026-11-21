# Design note: indexing, semantic search, and entity tracking

Status: **proposal** (in progress). Encryption, sync, SQLite layout, the
binary split, the open attribute model, and the division of labour (local NER
*suggests*, external LLM / manual *verifies*) are settled. The **embedding
model**, **MCP framework**, exact **subcommand details**, and the
**report/dashboard layer** are still open. No product code changed by this
note.

Scope: a new binary + a derived index. **No change to the entry format or the
manifest format version** — the index is a sidecar that is rebuilt from
decrypted entries, never part of an entry. Reports and dashboards are the
eventual consumer of this index; this note fixes only the storage/query
foundation they will read, not the reports themselves.

## Goals

- Ask natural-language / recall questions over the vault: "which movies did I
  watch", "who did I meet in March", "which cities have I actually been to".
- Track distinct **things** — people and places (later orgs/events) — and tie
  each to the **specific mention(s)** it appears in, with evidence.
- Sync the derived index across machines; keep embeddings local.
- Provide an **open attribute framework**: a future fact about an entry,
  entity, or mention is stored as data (key/value), not as a new column or
  format version.
- Make **reports** (time with people, co-occurrence, locations, projects,
  effort) queries over the index, without changing the write side.

## Decided: binary split (workspace)

`diaria` today is a single crate with no `lib.rs`. To keep the lean write/read
binary free of the ML stack, split into a workspace:

- `crates/diaria-core` — shared vault/crypto/entry codec (what both binaries
  need to decrypt/read).
- `crates/diaria` — the existing write/read CLI (commands unchanged).
- `crates/diaria-index` — extraction / search / MCP. **Only** this pulls
  `fastembed` / GLiNER / sqlite / LLM client deps; `diaria` never links them.

The main binary stays pure writing and reading; the extraction binary reuses
`diaria-core` for decryption.

## Decided: architecture overview

Pipeline (over decrypted entries, which requires the password). The **LLM comes
in from the outside** — the index binary never bundles one; it defines a
pluggable interface for semantic-context providers (external LLM or manual
review):

```
diaria-index index    decrypt → chunk → GLiNER (suggests) → write stores
diaria-index verify   review suggestions → fill semantic context (external LLM / manual)
                                                         │
                                  ┌──────────────────────┴──────────────┐
                                  ▼                                     ▼
                        synced index (SQLite,       local embeddings store
                        encrypted, spans+presence)  (SQLite, text+vectors, local only)
```

Two stores, split by what must sync:

1. **Synced index** (SQLite) — `entities` + `mentions` (spans, evidence,
   presence). Small, structured, synced, encrypted (below).
2. **Local embeddings store** (SQLite) — `chunks` with text + embedding
   vectors. Large, rebuildable from synced entries + password, never synced.

Because the synced index carries spans relative to chunks (see schema), a fresh
machine can re-materialize the local chunk store by decrypting its synced
entries — the same read path that already requires the password.

**Division of labour.** `index` is local and deterministic: GLiNER only
*suggests* candidate mentions (spans + surface text), leaving semantic context
(`presence`) empty. `verify` is the step that fills it in — it sends candidates
to an external LLM and/or presents them for manual review, then marks them
verified. This keeps the extractor offline/cheap and the judgment pluggable.
`verify` is exposed both as a CLI command and as an MCP tool (same code path).

## Decided: encryption & sync

The index is encrypted with the **same hybrid envelope the entries use**
(`src/entry/version01.rs`): a fresh ephemeral X448 keypair → ECDH with the
vault public key → HKDF-SHA256 over `(shared_secret || key.sym)` →
XChaCha20Poly1305, stored as `ephemeral_pub || nonce || ciphertext`.

- **Asymmetric** = the X448 key exchange binds the index to the vault's long-term
  key pair.
- **Symmetric** = the XChaCha20Poly1305 AEAD.
- Reuse the existing `encode`/`decode`, so no new crypto, no new format version,
  and no separate index key sitting in the vault.

On the remote the synced index is unreadable without the long-term private key
(password) **and** `key.sym` — identical protection to the entries.

**Placement & sync changes:**

- The encrypted index blob lives inside the existing synced git repo, e.g.
  `entries/.index/`. `list_entries` filters `*.diaria`
  (`src/entry/repository.rs`), so a `.index`-suffixed file is invisible to
  `read`/`add`/`dump`/`stats` — no per-command changes.
- `sync` currently runs `git -C <entries> add *.diaria` (`src/commands/sync.rs`),
  which would ignore the index. Widen the `add` to include the index path (e.g.
  `git add .` or add `.index`), keeping one git repo.

**Security notes (documenting, not yet patched):**

1. **Write authorization.** Entries are authorable with only `key.pub` + `key.sym`
   (the `add`-needs-no-password tradeoff). This must **not** hold for the index:
   a forged "you were in Moscow with X" index would be indistinguishable from
   real. Building/writing the index requires the private key/password, unlike
   `add`.
2. **Broadened remote footprint.** The index is a structured life-profile
   (people, places, presence, dates). It is encrypted like entries, so
   confidentiality holds, but it is a concentrated, high-value blob on the
   remote.
3. **Re-encryption churn.** Rebuilding the aggregate index rewrites the whole
   blob each sync (git sees churn). Fine at diary scale; if it ever bites,
   split into per-date encrypted records for incremental diffs.

## Decided: SQLite schema — things tracked to mentions

One SQLite database for the synced index. The model is deliberately
"separate things, each tracked to its specific mentions", so every fact is
auditable back to the entry, chunk, and span it came from.

```sql
-- A distinct "thing": a person, a place, later org/event/...
CREATE TABLE entities (
    id           INTEGER PRIMARY KEY,
    kind         TEXT NOT NULL,          -- 'person' | 'place' | ...
    canonical    TEXT NOT NULL,          -- display name, e.g. "Marie Curie"
    key          TEXT NOT NULL UNIQUE,   -- normalized dedup key
    first_seen   TEXT NOT NULL,          -- ISO date
    last_seen    TEXT NOT NULL,          -- ISO date
    mention_count  INTEGER NOT NULL DEFAULT 0,
    present_count  INTEGER NOT NULL DEFAULT 0
);

-- A single occurrence of an entity in one chunk of one entry.
CREATE TABLE mentions (
    id                 INTEGER PRIMARY KEY,
    entity_id          INTEGER NOT NULL REFERENCES entities(id),
    entry_id           TEXT NOT NULL,    -- entry filename, e.g. "2026-01-05T09:00:00.diaria"
    chunk_index        INTEGER NOT NULL, -- which chunk of that entry
    span_start         INTEGER NOT NULL, -- char offset into the chunk
    span_end           INTEGER NOT NULL,
    text               TEXT NOT NULL,    -- surface form, e.g. "Marie"
    snippet            TEXT,             -- surrounding context (evidence)
    extractor          TEXT NOT NULL,    -- 'gliner'
    extract_confidence REAL,
    status             TEXT NOT NULL DEFAULT 'suggested',  -- 'suggested' | 'verified' | 'rejected'
    presence           TEXT NOT NULL DEFAULT 'unknown',  -- 'present' | 'mentioned' | 'unknown'
    presence_confidence REAL,
    verified_by        TEXT,             -- 'llm' | 'manual' (external provider / user)
    indexed_at         TEXT NOT NULL
);

-- Which entries have been indexed, for incremental rebuilds.
CREATE TABLE indexed_entries (
    entry_id     TEXT NOT NULL PRIMARY KEY,
    content_hash TEXT NOT NULL,   -- so unchanged entries are skipped
    indexed_at   TEXT NOT NULL
);
```

**Span & evidence conventions (the part worth extra care):**

- **Offsets are char offsets into a chunk**, not bytes, and are relative to
  `(entry_id, chunk_index)`. Resolving a span requires the local chunk store;
  that is fine because the local store is re-derived from synced entries on each
  machine. Keep the convention explicit and consistent across extractors.
- **Normalization / dedup.** `entities.key` folds case and diacritics (and
  optionally strips common name articles) so "marie" and "Marie" collapse.
  Coreference resolution (e.g. "Marie" → "Marie Curie") is a post-processing
  step that updates `mentions.entity_id`; see the extraction section.
- **Evidence is per mention.** Each row carries the surface `text`, a `snippet`
  of surrounding context, the `extractor`, and a confidence. A `present` claim
  is reviewable: the user can open the entry/chunk and see why the system
  decided they were actually there.
- **Presence is stored per mention and aggregated.** `presence` lives on the
  mention; `entities.present_count` aggregates. "Actually there" is a property
  of a specific occurrence, not of the entity as a whole — a city you once
  visited and once mentioned should carry both facts.
- **Status lifecycle.** GLiNER writes `status='suggested'` with `presence='unknown'`
  (empty semantic evidence — just spans). `verify` flips each to `verified`
  (LLM or manual, `verified_by`) or `rejected` (a false positive). Aggregates
  and searches count only `verified` mentions unless told otherwise.
- **Re-indexing.** `indexed_entries.content_hash` lets a rebuild skip unchanged
  entries. Deleting/renaming an entry cascades to its mentions on the next
  rebuild. Re-indexing must not clobber the human/LLM `verify` verdicts: keep
  `verified`/`rejected` state stable across rebuilds (keyed by a stable mention
  identity, e.g. entry + chunk + span + text).

The **local embeddings store** is a separate SQLite DB (never synced):
`chunks(entry_id, chunk_index, text, embedding BLOB)`. The embedding BLOB is
`f32` bytes in model dimension order; see the embedding open question.

## Decided: open attribute model

The entity/mention tables model the facts known *today*. To avoid committing to
that closed set, everything not modelled there is written as an **attribute**:
a generic `(subject, key, value)` row. New ideas — `project`, `location`,
`duration`, `mood`, `book`, whatever comes next — are data, not schema changes.

```sql
-- A stable subject an attribute can hang off. `ref` is the identity that
-- survives a rebuild, never a row id that re-indexing may recycle.
CREATE TABLE attribute_subjects (
    id    INTEGER PRIMARY KEY,
    kind  TEXT NOT NULL,          -- 'entry' | 'entity' | 'mention' | ...
    ref   TEXT NOT NULL,          -- entry_id / entity.key / stable mention identity
    UNIQUE (kind, ref)
);

CREATE TABLE attributes (
    id           INTEGER PRIMARY KEY,
    subject_id   INTEGER NOT NULL REFERENCES attribute_subjects(id),
    key          TEXT NOT NULL,   -- 'project', 'location', 'duration', ...
    value        TEXT NOT NULL,   -- canonical text form
    value_kind   TEXT NOT NULL DEFAULT 'text', -- 'text' | 'number' | 'date' | 'bool' | 'entity'
    value_num    REAL,            -- numeric projection for number attributes
    unit         TEXT,            -- 'hours', 'km', ... (number only)
    source       TEXT NOT NULL,   -- 'user' | 'extractor' | 'llm' | 'derived'
    confidence   REAL,
    status       TEXT NOT NULL DEFAULT 'suggested', -- suggested | verified | rejected
    created_at   TEXT NOT NULL,
    UNIQUE (subject_id, key, value)
);
```

- **One mechanism, several subject kinds.** Entry attributes ("this entry was
  about `project = diaria`") and entity attributes ("`location = Berlin`,
  `visited = true`") share one table. Adding a subject kind (`chunk`, later
  `event`) only adds a `kind` value.
- **Stable refs, not row ids.** `ref` is the entry filename, the entity dedup
  `key`, or the stable mention identity (entry + chunk + span + text) already
  used to preserve verify verdicts. Re-indexing rebuilds rows but not
  identities, so verified attributes survive.
- **Same lifecycle as mentions.** Extractors/LLM write `suggested`; `verify`
  flips to `verified`/`rejected` and sets `source`. `value_kind` keeps
  filtering honest instead of using `CAST` as the type system. `value_num` is
  only for genuine number attributes (e.g. `duration = 2, unit = 'hours'`),
  never for derived effort metrics.
- **Presence stays a first-class column.** It is the one fact aggregated on
  every query (`present_count`), so promoting it into `attributes` would only
  add joins. Treat it as a built-in shortcut: everything new goes to
  `attributes`, so nothing needs a migration to become queryable.
- **Kinds and keys are data.** `entities.kind` and attribute keys are free
  text; an extractor declares which kinds it produces. No enum gates additions.
- **Same protection as the index.** Attributes live in the synced encrypted
  index, so confidentiality and the password-to-write rule apply unchanged.

## Deferred: reports and the query contract

Reports are the **goal** of the index, but not part of this foundation. The base
tables plus a set of **read-only SQL views** are the contract: a report is a
query plus a renderer, so a new question never changes the write side.

Settled now, so the base schema can support them:

- **Co-occurrence is derived, not stored.** "People mentioned together" is a
  self-join over `mentions` on the same `entry_id` (or the same
  `entry_id, chunk_index` for a tighter window). No pair table until a query
  measurably needs one.
- **Effort is derived from spans.** Per-mention span offsets and lengths (plus
  the entry/chunk text that already exists locally) are enough; no word/char
  scalars are added to the synced index.
- **Time spent is an attribute or a timestamp convention.** A duration is an
  explicit `duration` attribute (`source = 'user' | 'llm'`), or derived from
  entry timestamps at report time. It is not a column on `mentions`.
- **Views, not report tables.** When reports are built: seed views for
  entity/presence, per-entry attributes, and mention co-occurrence; `--json`
  output as the stable contract; text/table/graph renderers as a thin layer.
  Dashboards and graphs (mermaid/DOT) come after a JSON report works.

Explicitly out of scope here: report definitions, aggregation caching, chart
rendering, and any dashboard UI.

## Decided: extraction (NER) — suggests only

`index` uses **in-process GLiNER** (`gline-rs`, or `anno`'s stacked backends) to
*suggest* candidate mentions: zero-shot NER with custom types (PERSON, PLACE),
local ONNX, no training. It only writes spans + surface text; semantic context
is left empty (`presence='unknown'`, `status='suggested'`).

- Handles a personal diary's idiosyncratic names without a gazetteer.
- `anno` optionally adds coreference resolution (linking "Marie" → "Marie Curie")
  as a post-processing pass on the suggested mentions.
- Offline and deterministic; the judgment is deliberately deferred to `verify`.

Rejected here: single-pass LLM extraction (bundles an LLM into `index`, against
the "external LLM" decision) and pure deterministic gazetteer/rule NER (too
brittle for a diary's unbounded names).

## Decided: external LLM for semantic context (verify)

`verify` is where the semantic judgment happens. The **LLM comes in from the
outside**: `diaria-index` defines a pluggable `SemanticContext` trait (a
mockable interface in the DI style) but never owns or bundles a model. Concrete
providers are user-chosen and injected:

- **External LLM** — an OpenAI-compatible / Ollama HTTP provider, configured by
  the user (not compiled in). It reads each suggested candidate + its chunk
  context and returns `presence` (`present`/`mentioned`) with a confidence.
- **Manual review** — a `dialoguer`-based TUI ("were you actually there?",
  confirm/correct the canonical name) for the human to fill context directly.

Both set `status='verified'` and `verified_by` accordingly. Because the provider
is a trait, the same command works for an LLM or manual review, and tests inject
a mock.

## Open: embedding algorithm

Embeddings are **not** for the entity tracking (that is the GLiNER + verify
index's job). They are the *general recall net* for content no extractor
modelled. Three tools cover different recall classes:

| Tool | Catches | Fails on |
|---|---|---|
| Entity index (GLiNER+verify) | known types: people, places, presence | concepts you didn't define as a type (movies, feelings, "that trip") |
| Full-text / FTS over chunks | exact terms ("movie" appears literally) | synonyms / paraphrases |
| **Embeddings** | semantic/fuzzy recall of unmodelled concepts | — (complements the above) |

Practical sequencing: ship **entity index + FTS + external LLM first**, and add
embeddings only if semantic-recall gaps appear. If/when added:

- **Model:** `fastembed` (local ONNX, synchronous). Defaults like
  `bge-small-en-v1.5` / `all-MiniLM-L6-v2`; `bge-m3` or `multilingual-e5` if
  entries are not English. Models download once, then run offline.
- **Index:** at diary scale, brute-force cosine over the chunks (fastembed's
  `similarity::top_k`) is enough — no vector DB needed initially. If the corpus
  grows, `sqlite-vec` (persistent, `vec0`) or `voy` (pure-Rust ANN).
- **Retrieval quality:** optionally a reranker (`bge-reranker`) over top-k
  before feeding the external LLM.

## Decided: query surface — native CLI first, thin MCP on top

Prefer **native CLI commands for everything**, with a **thin MCP layer** that
exposes the same command logic as tools (no duplicate code paths). The CLI is
the source of truth; the MCP just maps tools onto it. `verify` in particular is
exposed as an MCP command (the "evaluate these suggestions" action).

- A long-running MCP server (a subcommand) fits "index in memory": one password
  prompt at startup, decrypted index held across requests, `zeroize`d on exit.
  The MCP host LLM does the reasoning/Q&A over the tool results.
- Rejected: RAM-dir + agent (leaves plaintext on a tmpfs, gives raw file access
  instead of controlled tools).

Reports follow the same shape when they are built: native queries over the
read-only views, exposed as MCP tools alongside the rest. They are not part of
the initial foundation.

## Proposed subcommands

- `diaria` (write/read, unchanged): `init`, `add`, `read`, `load`, `dump`,
  `sync`, `summarize`, `stats`, `status`.
- `diaria-index`:
  - `index` — decrypt → chunk → GLiNER → write `suggested` mentions to SQLite,
    encrypt the synced index. `--incremental` skips unchanged entries.
  - `verify` — run the external LLM / manual review over suggested candidates to
    fill `presence` + canonical name; mark `verified`/`rejected`. Also an MCP
    tool.
  - `search` — entity / keyword (FTS) / semantic (embeddings) search.
  - `ask` — natural-language question → retrieve (entity+FTS+embeddings) →
    external LLM answer.
  - `entities` — list tracked people/places with presence counts, drill into
    mentions/evidence.
  - `attributes` — list/set entry/entity/mention attributes (manual authoring,
    same lifecycle as `verify`).
  - `report` — future consumer over the read-only views (co-occurrence,
    per-entity presence/time, per-entry attributes, …), `--json` first. Not
    part of the initial foundation.
  - `mcp` — run the MCP server exposing the above as tools.

## Follow-up implementation tasks

> - **Workspace split**: introduce `crates/diaria-core` (shared vault/crypto/
>   entry codec), move `diaria` onto it, add `crates/diaria-index`. Only the
>   index crate depends on the ML stack.
> - **Index writer** (`diaria-index index`): decrypt entries → chunk → GLiNER →
>   write `suggested` mentions (spans only, empty semantic context) into the
>   synced SQLite store, encrypted with the entry envelope. DI-style:
>   `EntityExtractor` as an injected, mockable trait.
> - **Attribute store**: add `attribute_subjects` + `attributes`, a `SubjectRef`
>   value type for the stable entry/entity/mention refs, and a writer trait so
>   extractors, the external LLM, and manual authoring all write through one
>   path. Preserve verified attributes across re-indexing.
> - **Verify** (`diaria-index verify`): a `SemanticContext` trait
>   (external-LLM provider + `dialoguer` manual TUI) that fills `presence` and
>   canonical name, marking `verified`/`rejected`; preserve verdicts across
>   re-indexing. Expose as an MCP tool.
> - **Sync change**: widen `git -C <entries> add` in `src/commands/sync.rs` to
>   include the synced index; add a unit test asserting the index path is staged.
> - **Write-auth test**: assert building/writing the index requires the private
>   key/password (mocked), unlike `add`.
> - **Report views (later)**: after the base tables exist, add read-only views
>   for entity/presence, entry attributes, and mention co-occurrence, then build
>   `report` on top with `--json` first. No report tables, no effort scalars.
> - **Open items**: pick the embedding model + index only if semantic-recall
>   gaps appear; choose the MCP framework; finalize the remaining subcommands
>   and the report/dashboard output format.
