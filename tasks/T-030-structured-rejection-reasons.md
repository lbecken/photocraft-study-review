---
id: T-030
title: "Give every refusal a structured reason, generate the error-kind list from code, and measure the error cap without the echoed id"
tier: high              # ADR-0002: CommandError = registry contract (CHARTER §3, ADR needed) and a gate path (xtask/generated.toml); approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: [T-004, T-007, T-008, T-010]
touches:
  - crates/kiln-commands/src                  # CommandError, reasons at every refusal site
  - crates/kiln-control/src                   # kinds from reasons; cap without id; the kind-list renderer (D-2)
  - crates/kiln-control/tests/reasons.rs      # implementer's own integration tests
  - docs/protocol/error-kinds.md              # NEW generated file: the list of (code, kind) pairs (D-2)
  - xtask/generated.toml                      # GATE PATH: one new [[generated]] entry for that file (D-2)
  - docs/control-protocol.md                  # table → pointer; kind-derivation, id/cap, stale sentences
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: reviewer
origin_task: T-004
---

# T-030 Give every refusal a structured reason so the control protocol's error kinds never depend on message text; measure the error cap without the echoed id

## Goal
`CommandError::Rejected` carries only a message, so `kiln-control` matches message text to
derive the 23 documented `(code, kind)` pairs (plus T-008's `outside_root`). A reworded message
silently becomes `other`. After this task every refusal and internal error carries a structured
reason set where it is raised, and `kiln-control` reports `kind` from that reason alone. The
documented kind names stay exactly as they are. The task also closes three T-004 follow-ups
from D-032. The error-size cap no longer counts the echoed request `id`. A test sits on the
exact 4096-byte boundary. One misplaced sentence in the protocol document moves back to the id
rules. Split from T-007 (T-007 Decisions D-1).

## Reading list
- `docs/decisions.md` D-031 (9)–(10), D-032; T-007 Decisions D-1.
- `reviews/T-004-r3.md` R3-SG1, R3-N1, R3-N2, R3-N3.
- `crates/kiln-commands/src/{registry,doc,buffer}.rs` (every `CommandError::rejected` and
  `Internal` site); `crates/kiln-control/src/lib.rs` (`Failure::from_command`,
  `rejected_kind`, `PANICKED`, `cap_error`, `command_error_json`, `ERROR_KINDS`).
- `docs/control-protocol.md` "Response", "Error codes and kinds", "Request".
- `tasks/T-008-path-confinement.md` Interface (`VfsError::OutsideRoot`, kind `outside_root`).
- Tests that must stay green **unchanged**: every `acceptance_t003*`, `acceptance_t004*`,
  `acceptance_t007*`, `acceptance_t008*`, `acceptance_t010*` and guard file.

## Interface

### Rust (`kiln_commands::…`, re-exported from the crate root)
```rust
/// Why a command refused a request. `as_str()` and the serde form are the protocol `kind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    InvalidPath, NoSuchDocument, UnsavedChanges, NotFound, NotAFile, OutsideRoot,
    IoError, DecodeError, OutOfRange, InvertedRange, NotABoundary, Other,
}
impl RejectReason { pub fn as_str(self) -> &'static str; }   // "invalid_path", …, "other"

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalReason { ResultSchema, Panic, Other }
impl InternalReason { pub fn as_str(self) -> &'static str; }  // "result_schema", "panic", "other"

pub enum CommandError {                     // derives unchanged (Clone, Debug, PartialEq, Eq, Serialize)
    UnknownCommand { message: String },                          // unchanged
    InvalidParams { message: String, details: Vec<String> },     // unchanged
    Rejected { reason: RejectReason, message: String },          // CHANGED: + reason
    Internal { reason: InternalReason, message: String },        // CHANGED: + reason
}
impl CommandError {
    /// CHANGED: the reason is required (the one-argument form is removed).
    pub fn rejected(reason: RejectReason, message: impl Into<String>) -> Self;
    /// `{"code":…,"message":…[,"details":…][,"reason":…]}`; `reason` only for `rejected`/`internal`.
    pub fn to_json(&self) -> serde_json::Value;
}
// Display of CommandError: unchanged.
```
Patterns written as `CommandError::Rejected { .. }` / `Internal { .. }`, as the existing
acceptance tests use them, keep compiling.

### Where reasons come from (this table is the specification)
| Source of the refusal | Reason |
|---|---|
| `RelPath::new` fails on a request path | `InvalidPath` |
| no open document has the id | `NoSuchDocument` |
| `doc.close` of a dirty document without `discard` | `UnsavedChanges` |
| `VfsError::NotFound` (open or save) | `NotFound` |
| `VfsError::NotAFile` (directory or other non-regular file, T-008) | `NotAFile` |
| `VfsError::OutsideRoot` (T-008) | `OutsideRoot` |
| `VfsError::Io` | `IoError` |
| `OpenError::Decode` | `DecodeError` |
| `EditError::OutOfBounds` (`doc.insert/delete/replace`, `buffer.insert`) | `OutOfRange` |
| `EditError::InvertedRange` | `InvertedRange` |
| `PositionError::{OutOfBounds, LineOutOfBounds, ColumnOutOfBounds}` (`doc.line`, `doc.position`, T-010) | `OutOfRange` |
| `PositionError::{NotCharBoundary, InsideSurrogatePair, InsideLineBreak}` (T-010) | `NotABoundary` |
| result fails its schema (T-007 rule (c)) | `InternalReason::ResultSchema` |
| result does not serialize | `InternalReason::Other` |
| a panic caught by the `kiln-control` host | `InternalReason::Panic` |

`RejectReason::Other` is produced by no current command. The mapping from the lower layers'
error enums to reasons has no catch-all arm, so a variant added later fails to compile until it
is mapped (the reviewer checks this; a future task that adds a variant must list the mapping
file in its `touches`).

### Control protocol
- The `kind` of a `rejected` or `internal` error is `reason.as_str()`. **No message text is
  consulted** to choose a `code` or `kind`. Every protocol response, and every `kiln-cli exec`
  error object, keeps today's bytes (same codes, kinds, messages, details) for every request,
  except for the cap change below.
- The `(code, kind)` set in `docs/control-protocol.md` is unchanged. The sentence "The `kind`
  of a `rejected` error is currently derived … from the message" is replaced by one saying the
  kind comes from a structured reason set where the error is raised.
- Cap (D-032): for a host response, the decision to replace `details` by
  `{"truncated":true}` uses the byte length of the canonical response with `"id":null` in place
  of the echoed id. The id is still echoed in full. For `command_error_json` (no id) the
  measured text is the canonical error object, as today. A measured length of exactly 4096
  bytes keeps `details`; 4097 truncates. The 200-character string cap is unchanged.
- Document: the sentence "Integer ids are any JSON integer …" moves from the character-offset
  paragraph to the id rules of "Request". The "Response" text on caps states the id rule above.

### Generated error-kind list (Decisions D-2)
- `docs/protocol/error-kinds.md` becomes the one list of `(code, kind, when)` rows. It is
  rendered from code: the fixed request-level pairs of `kiln-control` plus every
  `RejectReason`/`InternalReason` with a one-line description that lives next to the variant.
  The file starts with a "generated, do not edit" comment, like `docs/scorecard.md`.
- A renderer runnable from the workspace writes it (for example a binary target under
  `crates/kiln-control/src/bin/`; the exact form is the implementer's, inside `touches` and
  without a new dependency or a `Cargo.toml` change). `xtask/generated.toml` gains, after the
  existing entries, `path = "docs/protocol/error-kinds.md"` with that command, so a rebase
  conflict on it is regenerated by the queue.
- Currency is checked in the gate: a `kiln-control` test (run by the gate's `test` step)
  fails, naming the file, when the committed file differs from the rendering. It replaces
  `every_kind_is_documented`.
- `docs/control-protocol.md` "Error codes and kinds": the table is replaced by one sentence
  pointing to `docs/protocol/error-kinds.md`. After this task, a task that adds a kind adds a
  reason variant and regenerates the file; it no longer edits `docs/control-protocol.md`.
- `docs/control-protocol.md` also gets the stale sentences that other tasks were not allowed to
  edit (D-6 of T-007/T-008/T-010/T-011): the "a command that returns an error is not recorded"
  sentence gains T-007's rule (c) exception, and the `kiln-cli` section gains one sentence
  pointing to `docs/protocol/` for options and subcommands documented there (`replay`,
  `--memory-seed`). Only those that are on `main` when this task starts.

## Pinned
- (a) Errors, payloads, precedence: the reason table above is the specification; `kind` =
  `reason.as_str()`; `to_json` adds `"reason"` for `rejected`/`internal` only; cap decision on
  the id-null canonical response, `≤ 4096` keeps `details`. Precedence of codes unchanged.
- (b) Files: `docs/control-protocol.md` (table → pointer; kind derivation, caps, moved id
  sentence, the stale sentences above); new generated `docs/protocol/error-kinds.md`; one entry
  in `xtask/generated.toml` (a gate path: the approval row must contain `gate paths`); no file
  system access by product code changes.
- (c) Order of checks: unchanged in every command; only the error value carries more.
- (d) Path base, platform, file types: n/a (no path handling changes; `OutsideRoot` comes from
  T-008).
- (e) Threat model: n/a for the gate. Integrity property: no message text can change a `kind`;
  the reviewer checks that no catch-all arm maps a lower-layer error variant.
- (f) Tests pinning a snapshot this changes: `kiln-control` unit tests of `rejected_kind` (go
  away with it; replaced, disclosed); `every_kind_is_documented` (kept); every T-003, T-004,
  T-007, T-008, T-010 acceptance and guard test, unchanged (they match `Rejected { .. }`).
- (g) Worked examples: the 4096-byte boundary is computed by the test from
  `command_error_json`'s own output (AC-8), not from a hand-made number; AC-3's example messages
  are today's: `"no open document 1"` maps to `no_such_document` by message on today's `main`
  (`rejected_kind`, `crates/kiln-control/src/lib.rs:357`), so with `r = NotFound` it must become
  `not_found`.
- (h) Scorecard items: none (the test author may add a test to `control.errors`; if it does,
  AC-3 and AC-7 are observable through `Host::handle_line`, the control protocol's host).

## Acceptance criteria
- AC-1 (reasons at their sources): through `standard_registry().execute`, each refusal row of
  the table gives exactly that reason:
  - `doc.open "../x"`; `doc.text {"doc":9}`; a dirty `doc.close`;
  - `doc.open` of a missing file; of a directory (`MemoryFs` with `d/x`, open `d`); of
    `b"\xff"`;
  - `doc.insert` at offset 99; `doc.delete` 2..1; `buffer.insert` at offset 99;
  - `doc.position {"at":{"byte":14}}` and `{"at":{"byte":5}}` on T-010's worked-example text
    → `OutOfRange`, `NotABoundary`;
  - `doc.save` on a `MemoryFs` with `set_fail_writes(true)` → `IoError`;
  - on a `DiskFs` root holding `link -> ../outside/f` (a real file whose presence the test
    asserts), `doc.open link` → `OutsideRoot`.

  `to_json()` of each has `"reason"` equal to `as_str()`. Oracle: exact value.
- AC-2 (internal reasons): T-007's `test.bad-result` liar → `Internal{reason: ResultSchema, ..}`;
  a liar whose output fails to serialize → `Internal{reason: Other, ..}`. Oracle: exact value.
- AC-3 (kinds come from reasons, not text): for every `RejectReason` `r`,
  `kiln_control::command_error_json("doc.open", &json!({}), &CommandError::rejected(r, m))` has
  `code: "rejected"` and `kind == r.as_str()`. This holds for `m` = `"zz unrelated"` and for a
  `m` that today maps to a different kind (for example `"no open document 1"` with
  `r = NotFound`). The same holds for every `InternalReason`, with `code: "internal"`.
  Oracle: exact value.
- AC-4 (documented set is exactly the reasons): the `rejected` and `internal` kinds listed in
  `docs/protocol/error-kinds.md` are exactly the `as_str()` values of `RejectReason` and
  `InternalReason`, and each serde form equals `as_str()`. Oracle: exact value (the test parses
  the table).
- AC-5 (protocol bytes unchanged): every T-004, T-007, T-008 and T-010 acceptance test passes
  unmodified, and the worked example in `docs/control-protocol.md` still gives the documented
  output byte for byte. Oracle: regression + exact value.
- AC-6 (panic kind): a panic in a command is still reported as `internal`/`panic` with message
  `` command `<name>` panicked ``, now via `InternalReason::Panic`. The test author may assert
  this through a unit-level seam only if one is public; otherwise it stays untested through the
  public interface (accepted in D-031 (13)), and the reviewer checks it by reading.
- AC-7 (id not counted, D-032): through `Host::handle_line`, send the same failing request (for
  example a `doc.insert` whose params give `invalid_params` details) once with `"id":1` and once
  with a 5000-character string id. The `error` objects are identical, and neither has
  `{"truncated":true}` when the `id: 1` line is under 4096 bytes. Oracle: differential.
- AC-8 (exact boundary, D-032): take a hand-built `CommandError::InvalidParams` whose capped
  canonical error object, from `kiln_control::command_error_json`, is exactly 4096 bytes: it
  keeps its `details`. One byte more gives `"details":{"truncated":true}`. Through `Host`, with
  requests whose id-null response length crosses 4096 in one-byte steps, `details` is kept
  exactly while that length is ≤ 4096. Oracle: exact value.
- AC-10 (generated list): with a `RejectReason` variant added in a scratch copy, the currency
  test fails naming `docs/protocol/error-kinds.md`; running the `xtask/generated.toml` command
  regenerates it and the test passes; on the task branch the committed file equals the
  rendering and `docs/control-protocol.md` has no kind table. Every pair the host can produce
  (the T-004 kind tests) is in the file. Oracle: exact value.
- AC-9 (document): the integer-id sentence sits with the id rules; the caps text says the id is
  not measured; the kind-derivation sentence names structured reasons. Checked by the reviewer
  against AC-3, AC-7 and AC-8.

## Scorecard items and intended test names
No new item. Tests (no item) in `crates/kiln-commands/tests/acceptance_t030.rs`
(`refusals_carry_reasons_at_their_sources`, `internal_errors_carry_reasons`) and
`crates/kiln-control/tests/acceptance_t030_control.rs` (distinct target name: two
`acceptance_t030` targets in two packages are miscounted by the join until T-009 (6) lands)
(`kinds_follow_reasons_not_messages`,
`documented_kinds_equal_reasons`, `echoed_id_does_not_count_toward_cap`,
`cap_boundary_is_exactly_4096_bytes`). The test author may also add
`kinds_follow_reasons_not_messages` to the `control.errors` item.

## Out of scope
- New error kinds beyond T-008's `outside_root`; renaming any kind.
- Replay, read-only and logging semantics (T-007).
- `apps/kiln-cli` (it uses `command_error_json`; no change expected. If one is needed, stop and
  report).
- Any change to `Cargo.toml`, `Cargo.lock`, `xtask/`, `.cargo/`, `.github/`, lint tables or
  build keys. This task touches **no gate path and no dependency**.

## Implementer notes
- Your tests: unit tests in `crates/kiln-commands/src` and `crates/kiln-control/src/lib.rs`, and
  `crates/kiln-control/tests/reasons.rs`. The unit tests that pin `rejected_kind`'s message
  matching go away with that function. Replace them with tests of the reason mapping, and say
  so in your log (golden rule 4 disclosure).
- Left to the implementer: how `kiln-control` stops using the message-classification state
  (for example the per-document path tracking), as long as the outputs are unchanged.

## Decisions
- D-1 (director, 2026-11-13, R-051): created by splitting T-007. It holds part (d), structured
  rejection reasons with stable documented kind names, and part (e), the three D-032 follow-ups
  moved here from T-006. T-007 lands first (reasons in T-007 Decisions D-1). This task also
  needs T-008 merged for `VfsError::OutsideRoot`. `outside_root` is approved (T-008 Decisions D-1).
- Dispatch note (R-055, revised R-057): depends on T-010, whose refusals it maps
  (`not_a_boundary` approved, T-010 D-1). `touches` overlap T-006, T-007, T-010
  (`crates/kiln-commands/src`) and T-008, T-010 (`crates/kiln-control/src`); it is disjoint from
  T-009, T-011 and T-013 (`xtask/generated.toml` is in neither xtask task's `touches`). As a
  CHARTER §3 contract change it needs an ADR before `ready`.
- D-2 (director R-057): `docs/control-protocol.md` stops being the file every product task
  edits: tasks document commands in their own `docs/protocol/<group>.md` and touch that file only
  for an error-kind line. T-030 also makes the list of error kinds generated from code (a
  generated path, with the gate checking it is current). Until T-030 lands, two tasks that each
  add a kind may conflict on rebase in that list; the director resolves it by hand.

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T03:05Z · R-051 · (new) · did: wrote T-030 from T-007's former parts (d) and (e),
  per the director's split: `RejectReason`/`InternalReason` with a source table (including
  `OutsideRoot` from T-008 and the panic path), kinds from reasons only, byte-identical protocol
  output, and the D-032 cap rule (id measured as null; exact 4096 boundary via the public
  `command_error_json`). Read `kiln-control`: today `Internal` kinds (`panic`, `result_schema`)
  are also derived from message text, so the internal reasons are in scope too. Depends on T-007
  (shared files, final `internal` path) and T-008 (`OutsideRoot`). Tier high: `CommandError`
  contract, L3 + L4. No gate path, no dependency. · gate attempts: 0 · missing context: none ·
  next: director review.
- 2026-11-13T03:50Z · R-055 · draft · did: brought the spec to v1.1: tier high under ADR-0002
  (criterion: it changes the registry's error contract `CommandError`, i.e. how the registry
  works, a CHARTER §3 contract change; an ADR is due); budget 850k; Pinned; `depends_on` +T-010
  with two reason rows and `NotABoundary` for its position refusals; renamed the control test
  target to `acceptance_t030_control.rs`. Arm n/a (high). · gate attempts: 0 · missing context:
  none · next: director review.
- 2026-11-13T03:56Z · R-057 · draft · did: applied D-2: generated `docs/protocol/error-kinds.md`
  with a currency test and a `generated.toml` entry (gate path, so `gate paths` in the
  approval), table in `docs/control-protocol.md` replaced by a pointer, and the stale sentences
  other tasks may not edit assigned here; AC-10; dispatch note revised. · gate attempts: 0 ·
  missing context: none · next: director review, ADR.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
