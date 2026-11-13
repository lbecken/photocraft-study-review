---
id: T-027
title: "Detect before save that a document's file changed or vanished on disk since it was read, refuse the save unless forced, and report the disk state on request"
tier: high              # ADR-0002: product code on the file system's persistence path (save)
state: draft
owner_role: planner
scorecard_items: [io.external-change]
depends_on: [T-032, T-008]   # D-041: ChangedOnDisk + reason.rs mapping; T-008's OutsideRoot (T-015 transitively)
touches:
  - crates/kiln-workspace/src/document.rs     # DiskState, SaveError, disk_state, save_checked
  - crates/kiln-workspace/tests/external.rs   # implementer's own integration tests
  - crates/kiln-commands/src/doc.rs           # doc.save `force`; new doc.disk-status
  - crates/kiln-commands/src/reason.rs        # ONLY the SaveError → RejectReason mapping (T-032 rule: no catch-all)
  - docs/protocol/doc.md                      # new: the save check and doc.disk-status
  - docs/commands.json                        # generated
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-027 Detect external changes before save

## Goal
Today `doc.save` overwrites the file whatever happened to it since `doc.open`. Another program
(git checkout, a formatter, a second editor) loses its change silently. After this task
`doc.save` first compares the file's current bytes with the bytes the document was read from
(or last saved). If they differ, or the file is gone, the save is refused with
`changed_on_disk` and nothing is written. `"force": true` saves anyway. `doc.disk-status`
reports `unchanged`, `changed` or `missing` without saving. The comparison is by content, never
by modification time (CHARTER §3 decision 4: no clock). Line endings and encoding stay
bytes-as-read.

No gate path. Reading list: `tasks/T-002-*.md` Interface (`Document::open/save`);
`tasks/T-008-*.md` (confinement, precedence); `crates/kiln-workspace/src/document.rs`;
`crates/kiln-commands/src/doc.rs` (`Save`); `tasks/T-032-*.md` (reasons, `reason.rs` mapping rule).

## Interface

### `kiln_workspace` (L2)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskState { Unchanged, Changed, Missing }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    Vfs(VfsError),
    /// The file's bytes differ from the baseline (`missing: false`), or it no longer exists (`true`).
    ChangedOnDisk { path: RelPath, missing: bool },
}
impl Document {
    /// UNCHANGED signature and behaviour (unchecked write; existing tests use it).
    pub fn save(&mut self, vfs: &mut dyn Vfs) -> Result<usize, VfsError>;
    /// The baseline = the exact bytes read at `open`, or written by the last successful save.
    pub fn disk_state(&self, vfs: &dyn Vfs) -> Result<DiskState, VfsError>;
    /// `disk_state` first: `Unchanged` → `save`; otherwise `ChangedOnDisk`, nothing written.
    pub fn save_checked(&mut self, vfs: &mut dyn Vfs) -> Result<usize, SaveError>;
}
```
`disk_state`: `vfs.read(path)` → bytes equal the baseline → `Unchanged`, else `Changed`;
`NotFound` → `Missing`; any other `VfsError` (`OutsideRoot`, `NotAFile`, `Io`) is returned
as is. Display of `ChangedOnDisk`: `{path}: changed on disk since it was read` or
`{path}: removed from disk since it was read`.

### Commands
- `doc.save {doc, force?: bool}` (default false): `force: false` → `save_checked`;
  `force: true` → `save`, today's behaviour, which also recreates a missing file. The result is
  unchanged. Refusal: `rejected`/`changed_on_disk` with message `cannot save <SaveError
  Display>`. Other refusals as today. The `doc.save` examples in `docs/commands.json` stay
  valid. Add one example with `force`.
- `doc.disk-status {doc}` (read-only) → `{doc, status: "unchanged"|"changed"|"missing"}`;
  `VfsError`s → today's kinds (`outside_root`, `not_a_file`, `io_error`).
- Mapping: `SaveError::Vfs(e)` → e's reason (T-015's `VfsError` mapping); `ChangedOnDisk` →
  `RejectReason::ChangedOnDisk` (T-032). An exhaustive `impl From<&SaveError> for RejectReason` in `reason.rs`.

No new error kind. `docs/protocol/doc.md` documents both, the content-not-time rule and the race
in (e).

## Pinned
- (a) Errors and precedence for `doc.save`: schema → `no_such_document` → (unless `force`) the
  read of `disk_state`, giving T-008's kinds (`OutsideRoot` → `NotAFile` → `Io`), then
  `changed_on_disk` → the write (today's kinds). A failed check leaves the document dirty and
  the session log unchanged.
- (b) Files: one extra read of the target before each unforced save; no other file. The
  baseline stays in memory.
- (c) Order: check, then write, in one command.
- (d) Unix, as T-008. Comparison is byte-exact; a touched mtime with the same bytes is
  `unchanged`.
- (e) Threat model: protects against accidental loss, not a racing adversary. A change between
  the check and the rename is not detected (TOCTOU, accepted and documented). The check never
  reads outside the root (T-008 rules apply to the read). It never follows a path other than
  the document's. A file replaced by a FIFO is refused `not_a_file` without blocking. The
  reviewer probes: file replaced by a symlink to outside, by a directory, by a FIFO; file
  deleted; same bytes rewritten; a BOM or UTF-16 file changed only in encoding.
- (f) Snapshots: the T-002, T-004 and T-008 acceptance tests stay unchanged and green. Before
  coding, the implementer lists every existing test that changes a file on disk between
  `doc.open` and a `doc.save` and expects the save to succeed. If there is one, stop and
  report: it would need `force`, and those files are not in `touches`.
- (g) Worked example (naive model: compare byte strings). Root with `a.txt` = `hello\n`:
  `doc.open a.txt` (doc 1), `doc.insert {doc:1, offset:5, text:"!"}`. The test writes `HELLO\n`
  to `a.txt`. `doc.disk-status {doc:1}` → `changed`. `doc.save {doc:1}` →
  `{"code":"rejected","kind":"changed_on_disk","message":"cannot save a.txt: changed on disk
  since it was read"}`, and the file still holds `HELLO\n`. `doc.save {doc:1, force:true}` →
  `bytes_written: 7`, the file holds `hello!\n`, and `doc.disk-status` → `unchanged`.
- (h) Evaluator call: `kiln-cli control --root d`: `doc.open`, edit, change the file from
  outside, then `doc.disk-status`, `doc.save`, `doc.save {force:true}`.

## Acceptance criteria
- AC-1 (`io.external-change`, control): the worked example holds exactly. Oracle: exact value +
  bytes on disk.
- AC-2 (`io.external-change`, all states): through `kiln-cli control`, for each external action
  between open (or a save) and the next save, `doc.disk-status` and `doc.save` give the naive
  model's result. The actions: none → unchanged, save ok; same bytes rewritten → unchanged, ok;
  one byte changed → changed, refused; file deleted → missing, refused, message `removed`;
  replaced by a directory → `not_a_file`; replaced by a symlink to an outside file (whose
  presence the test asserts) → `outside_root`; replaced by a FIFO → `not_a_file` within 5 s.
  Each refused save leaves the disk unchanged and the document dirty. Oracle: naive model +
  disk state.
- AC-3 (`io.external-change`, baseline moves with saves): open, save (ok), external change,
  save → refused; open, external change, `force` save, save again → ok. The baseline is the
  last written bytes, including for BOM and UTF-16 files. Oracle: naive model.
- AC-4 (no regression): every T-002/T-004/T-008 acceptance test passes unchanged, and a
  `kiln-cli run` session with no external change gives byte-identical output to the base
  binary. Oracle: regression + differential.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| io.external-change | `kiln_cli::acceptance_t027::external_change_refuses_save_worked_example`, `kiln_cli::acceptance_t027::disk_states_match_naive_model`, `kiln_cli::acceptance_t027::baseline_follows_saves` |

## Out of scope and implementer notes
- Reloading from disk, merging, watching for changes, notifying at open time; mtime or hash
  shortcuts; the replace-in-files staleness rule (T-022, in memory).
- Your tests: `crates/kiln-workspace/tests/external.rs`, unit tests in `document.rs`.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified a content-based external-change check on save
  with `force`. `Document::save` is kept as is, because existing acceptance tests call it, and
  `save_checked` is added. Tier high, by ADR-0002 "persistence path". It touches `doc.rs` and
  the mapping in `reason.rs`, so it must not be in flight with any other task editing those;
  none in T-015…T-029 does after T-015. The worked example was executed against a byte-compare
  model. Unsure: whether the director wants `doc.open`/`doc.info` to show the disk state too
  (left out). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-060 · draft · did: the mapping moves to T-015's `reason.rs`; `depends_on` gains T-008 (contracts first means T-008 no longer precedes T-015, and AC-2 needs `outside_root`). Shares `doc.rs` with T-007 and T-010, and `reason.rs` with T-008 and T-010: never in flight with them. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: reasons are T-032's (D-041): `depends_on` gains T-032. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
