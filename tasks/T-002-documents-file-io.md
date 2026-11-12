---
id: T-002
title: "Open and save documents through a virtual file system, preserving encoding and line endings, atomically"
tier: high              # persistence/save path; Cargo.lock (new dependency edge)
state: merged
owner_role: director
scorecard_items: [ws.vfs, io.utf8, io.bom, io.encoding, io.save-atomic]
depends_on: []
touches:
  - crates/kiln-workspace/src
  - crates/kiln-workspace/Cargo.toml
  - Cargo.lock
budget_tokens: 700000
iteration: 0
found_by: planner
origin_task: null
---

# T-002 Open and save documents through a virtual file system, preserving encoding and line endings, atomically

## Goal
After this task a caller can open a file from an in-memory or on-disk file system into a
`Document` (a `kiln_text::Buffer` plus what is needed to write it back), edit its buffer, ask
whether it differs from the file, and save it so that an unedited document is written back
byte-for-byte, the encoding (UTF-8 with or without BOM, UTF-16 LE/BE with BOM) is kept, and a
failed save never leaves a half-written or missing file. Undecodable files are refused with the
offset of the first bad byte, never silently repaired.

## Reading list
- `docs/CHARTER.md` §3 (L2 responsibilities; decisions 4 determinism, 5 round-trip oracles).
- `crates/kiln-text/src/buffer.rs` **as on main** — this task may use only `Buffer::from_text`,
  `text`, `insert`, `delete`, `slice`, `len_chars`, `len_bytes`, `is_empty` (all exist on main
  and are kept by T-001), so it runs in parallel with T-001.
- `scorecard/items/{ws.vfs,io.utf8,io.bom,io.encoding,io.save-atomic}.toml`.

## Interface
All items exported from the crate root (`kiln_workspace::…`). `kiln-workspace` gains a normal
dependency on `kiln-text` (`kiln-text.workspace = true`).

```rust
/// A validated path relative to a file-system root: non-empty, `/`-separated, no leading `/`,
/// no empty, `.` or `..` segment, no `\` and no NUL. Display/as_str give it exactly as given.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelPath(/* private */);
impl RelPath {
    pub fn new(path: &str) -> Result<RelPath, PathError>;
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for RelPath { … }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathError { pub path: String, pub reason: String }   // + Display, Error

/// File-system errors. Display names the RelPath only, never an absolute or host path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VfsError {
    NotFound { path: RelPath },          // file (or, for write, its parent directory) missing
    NotAFile { path: RelPath },          // the path is a directory
    Io { path: RelPath, kind: std::io::ErrorKind, message: String },
}                                        // + Display, Error

/// Files only; directories exist implicitly (MemoryFs) or on disk (DiskFs).
pub trait Vfs: std::fmt::Debug {
    fn read(&self, path: &RelPath) -> Result<Vec<u8>, VfsError>;
    /// Replace the file's whole content atomically: afterwards the file holds exactly `bytes`,
    /// or (on Err) exactly what it held before (or still does not exist). Creates the file if
    /// absent; never creates directories.
    fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError>;
    fn is_file(&self, path: &RelPath) -> bool;
}

/// In-memory VFS for deterministic tests. A directory exists iff some file lies below it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryFs { /* private */ }
impl MemoryFs {
    pub fn new() -> Self;
    pub fn insert(&mut self, path: &RelPath, bytes: &[u8]);   // set content directly
    pub fn get(&self, path: &RelPath) -> Option<Vec<u8>>;
    pub fn paths(&self) -> Vec<RelPath>;                       // sorted
    /// While set, every write_atomic fails with VfsError::Io (kind Other) and changes nothing.
    pub fn set_fail_writes(&mut self, fail: bool);
}
impl Vfs for MemoryFs { … }

/// On-disk VFS rooted at a directory; RelPaths resolve below `root`.
#[derive(Debug)]
pub struct DiskFs { /* private */ }
impl DiskFs {
    pub fn new(root: &std::path::Path) -> Result<DiskFs, VfsError>; // Err if root is not a dir
}
impl Vfs for DiskFs { … }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding { Utf8, Utf8Bom, Utf16Le, Utf16Be }
impl Encoding {
    /// "utf-8", "utf-8-bom", "utf-16le", "utf-16be".
    pub fn name(self) -> &'static str;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError { pub encoding: Encoding, pub byte_offset: usize }  // + Display, Error

/// Detect and decode: EF BB BF ⇒ Utf8Bom; FF FE ⇒ Utf16Le; FE FF ⇒ Utf16Be; otherwise Utf8.
/// The BOM is not part of the text. Invalid input ⇒ Err with the byte offset (in `bytes`,
/// BOM included) of the first byte of the first invalid sequence or unpaired surrogate unit, or
/// of the trailing odd byte of a UTF-16 input.
pub fn decode(bytes: &[u8]) -> Result<(String, Encoding), DecodeError>;
/// Inverse of decode: BOM (if the encoding has one) followed by the text in that encoding.
pub fn encode(text: &str, encoding: Encoding) -> Vec<u8>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError { Vfs(VfsError), Decode { path: RelPath, error: DecodeError } } // + Display, Error

#[derive(Clone, Debug)]
pub struct Document { /* private */ }
impl Document {
    pub fn open(vfs: &dyn Vfs, path: &RelPath) -> Result<Document, OpenError>;
    pub fn path(&self) -> &RelPath;
    pub fn encoding(&self) -> Encoding;
    pub fn buffer(&self) -> &kiln_text::Buffer;
    pub fn buffer_mut(&mut self) -> &mut kiln_text::Buffer;
    /// True iff the buffer text differs from the text last opened or successfully saved.
    pub fn is_dirty(&self) -> bool;
    /// Write encode(buffer text, encoding()) to path() with write_atomic; returns bytes written.
    /// Writes even when clean. On Err the document is unchanged (still dirty if it was).
    pub fn save(&mut self, vfs: &mut dyn Vfs) -> Result<usize, VfsError>;
}
```

Line endings are stored verbatim: the buffer text contains exactly the file's `\n`, `\r\n` and
`\r` characters, mixed line endings survive a round trip, and save writes the buffer's
characters without conversion; an inserted `\n` stays `\n` (D-2).

## Acceptance criteria
- AC-1 (path validation): `RelPath::new` accepts `"a"`, `"a/b.rs"`, `"src/main.rs"`, `"a b/c"`;
  rejects `""`, `"/a"`, `"a/"`, `"a//b"`, `"./a"`, `"a/../b"`, `".."`, `"a\\b"`, `"a\0b"`.
  Oracle: exact value.
- AC-2 (VFS contract, differential): the same fixed-seed sequence of `write_atomic`, `read` and
  `is_file` calls (on files and nested files, missing files, missing parent directories, and paths
  that are directories) gives identical results and identical final file contents on `MemoryFs`
  and on `DiskFs` over a fresh empty directory; reads return exactly the last bytes written.
  Oracle: MemoryFs as the reference model of DiskFs, plus round-trip read∘write = identity.
- AC-3 (UTF-8 round-trip, io.utf8): for a corpus of valid UTF-8 files (empty; ASCII; multi-byte
  incl. 4-byte chars; LF, CRLF, CR and mixed line endings; no final newline; containing NUL), on
  both VFSs: `open` then `save` without edits writes bytes equal to the original, and
  `encoding()` is `Utf8`. Oracle: round-trip.
- AC-4 (BOM, io.bom): a file starting with `EF BB BF` opens with `encoding() == Utf8Bom`, the
  buffer text does not start with U+FEFF, and save (unedited or edited) writes `EF BB BF` followed
  by the UTF-8 text; a BOM-only file opens as empty text and saves back as exactly the 3 BOM
  bytes. Oracle: round-trip + exact value.
- AC-5 (UTF-16, io.encoding): files `FF FE` + UTF-16LE and `FE FF` + UTF-16BE (including a
  surrogate pair) open to the expected text with `Utf16Le` / `Utf16Be` and save back byte-equal;
  after inserting `"é😀"` the saved bytes equal `encode(expected_text, enc)` built independently.
  Oracle: round-trip + exact value.
- AC-6 (invalid input is reported, never replaced): exact `DecodeError`s: `b"ab\xffcd"` →
  `{Utf8, 2}`; `EF BB BF 61 C3` → `{Utf8Bom, 4}`; `b"a\xe2\x82"` (truncated) → `{Utf8, 1}`;
  `FF FE 61 00 62` (odd length) → `{Utf16Le, 4}`; `FF FE 00 D8 61 00` (unpaired high surrogate)
  → `{Utf16Le, 2}`; `FE FF DC 00` (lone low surrogate) → `{Utf16Be, 2}`. `Document::open` on such a
  file returns `OpenError::Decode` with that error; no document is created. Oracle: exact value.
- AC-7 (encode/decode inverse): for generated strings `s` and each encoding `e`,
  `decode(&encode(s, e)) == Ok((s, e))`, except that `encode(s, Utf8)` of a string starting with
  U+FEFF decodes as `Utf8Bom` (documented ambiguity; the test asserts exactly that). Oracle:
  round-trip.
- AC-8 (dirty): a freshly opened document is clean; after a buffer edit it is dirty; after an
  edit followed by the inverse edit (insert then delete of the same range) it is clean; after a
  successful `save` it is clean and `read` of the path equals `encode(text, encoding)`. Oracle:
  naive model "text ≠ last saved text" + exact value.
- AC-9 (failed save, io.save-atomic): with `MemoryFs::set_fail_writes(true)`, `save` of an edited
  document returns `Err(VfsError::Io{..})`, the file still holds its original bytes, the document
  is still dirty, and after `set_fail_writes(false)` a second `save` succeeds. On `DiskFs`, when
  the document's path has been replaced by a directory since open, `save` returns an error, the
  directory and its contents are unchanged, and no other entry has appeared in the parent
  directory (listing before == listing after). Oracle: exact value.
- AC-10 (atomic replace on disk, io.save-atomic): after a successful `DiskFs` save of an existing
  file, the parent directory's listing is unchanged (no temporary file left), the file holds the
  new bytes, its Unix permission bits are unchanged, and (Unix) its inode number differs from the
  one before the save (the content was swapped in, not rewritten in place). Saving through a
  symlink updates the symlink's target file and leaves the symlink in place. Oracle: exact value.
- AC-11 (errors): `open` of a missing path → `OpenError::Vfs(NotFound)`; of a directory →
  `NotAFile`; `write_atomic` into a missing parent directory → `NotFound`, nothing created; error
  `Display` text contains the RelPath and not the DiskFs root path. Oracle: exact value.
- AC-12 (determinism): two `DiskFs` instances rooted at different directories, given the same
  sequence of operations, return identical results including identical error `Display` strings.
  Oracle: differential (root as the only difference).

## Scorecard items and intended test names
Tests in `crates/kiln-workspace/tests/acceptance_t002.rs` (`kiln_workspace::acceptance_t002::<fn>`).

| Item | Tests |
|---|---|
| ws.vfs | `memory_and_disk_vfs_agree_on_random_operation_sequences`, `relpath_accepts_and_rejects_exact_cases`, `vfs_errors_name_relative_paths_only` |
| io.utf8 | `unedited_utf8_files_round_trip_byte_for_byte`, `line_endings_round_trip_verbatim` |
| io.bom | `utf8_bom_detected_stripped_and_restored_on_save`, `bom_only_file_round_trips` |
| io.encoding | `utf16_le_and_be_with_bom_open_and_save`, `invalid_bytes_are_reported_with_offset_never_replaced`, `encode_decode_round_trip` |
| io.save-atomic | `failed_save_leaves_file_intact_and_document_dirty`, `disk_save_replaces_via_rename_without_leftovers`, `save_through_symlink_updates_target` |

Also (no item; `ws.dirty-tracking` is claimed by T-003 where undo exists): `dirty_follows_text_versus_last_saved`.

## Out of scope
- Commands (`doc.open`, …), multiple open documents, document ids — T-003.
- Line-ending detection and reporting of the dominant line ending: computed by kiln-text
  (T-001 `dominant_line_ending()`) and reported per document by T-003, which also claims
  `io.crlf` (D-2). Conversion/normalisation (`io.mixed-eol`) and "Enter inserts the document's
  line ending" — later editing tasks.
- Legacy encodings, UTF-16 without BOM, UTF-32, encoding guessing, lossy open — not supported.
- External-change detection (`io.external-change`), save-as, file watching, project tree,
  find-in-files, diagnostics, external tools — later tasks.
- Confining DiskFs against symlinks that point outside its root — later hardening task.
- Any change to `kiln-text`, `Cargo.toml` (root) or other crates.

## Implementer notes
- Disk tests: use a fresh directory below `env!("CARGO_TARGET_TMPDIR")` (integration tests) or a
  unique directory under `std::env::temp_dir()` (unit tests); no new dependency (no `tempfile`).
- No randomness or clock in behaviour: any temporary file name must be derived deterministically
  (and must not collide with the target name); a leftover from an earlier crash must not make
  save fail forever — state in your log what you chose.
- Durability (`fsync`) is not observable in tests; say in your log whether you sync and why.
- `Vfs` must stay object-safe (`&dyn Vfs`, `Box<dyn Vfs>` are used by T-003).
- `Cargo.lock` conflict: T-001 also changes `Cargo.lock` (adds `ropey`) and runs in parallel.
  Whichever reaches the queue second conflicts there; until T-005 lands, the director resolves it
  by regenerating the lock file (D-3).

## Decisions
Director's answers to the planner's questions of R-003 (2026-11-12).
- D-1 (was Q-1): dirty is judged by content — a document is dirty exactly when its current text
  differs from the text last loaded or saved. How that is computed is the implementer's choice.
- D-2 (was Q-2): bytes are preserved exactly as read; mixed line endings survive a round trip; an
  inserted `\n` stays `\n`. The document reports the detected dominant line ending; `io.crlf` is
  claimed as round-trip preservation plus detection. Because this task may use only the kiln-text
  API on main, the detection lives in kiln-text (T-001) and is reported, and `io.crlf` claimed, by
  T-003; this task's round-trip ACs (AC-3) supply the preservation half.
- D-3 (was Q-3): T-002 runs in parallel with T-001; the resulting `Cargo.lock` conflict is
  resolved by the director regenerating the lock file until T-005 lands.
- Director rulings after review r1 (2026-11-12, decision D-014) — these are part of the spec:
  1. A save never reads, writes, replaces or deletes any file other than its target and a temp file it created itself. The temp file is created exclusively (if anything already exists at the candidate name, including a symlink, try the next name) and its name is short and independent of the target's name length.
  2. New content is never more readable than the file it replaces or becomes: when the target exists, the temp file is created owner-only and receives the target's mode before the rename; when the target is new, the temp file is created with the platform default mode under the umask, which is the mode the new file keeps. (Final wording after review r2; an intermediate wording was not implementable without reading the umask.)
  3. Saving to a target the user cannot write returns `PermissionDenied` and leaves the file untouched.
  4. Replacing the file splits hard links and may change the owner: documented limitation.
  5. Supported platforms: Unix (Linux, macOS). Windows is out of scope.
  6. `NotFound.path` is the requested path; `PathError.path` is the input verbatim, `reason` free text; saving onto a directory returns `NotAFile`.

## Questions
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · tokens (if known) · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-12 · R-003 · (new) · did: specified RelPath, the Vfs trait with MemoryFs/DiskFs, BOM
  detection and exact decode errors for UTF-8 (±BOM) and UTF-16 LE/BE (with BOM), Document
  open/save/dirty against main `48683de`. Oracles: MemoryFs as the reference model of DiskFs,
  byte round-trips, exact error offsets. Tier high: persistence/save path plus a `Cargo.lock`
  change. Uses only kiln-text methods present on main, so parallel with T-001 if T-001 adds no
  dependency. `ws.documents` and `ws.dirty-tracking` moved to T-003 (need several documents and
  undo). Unsure: dirty semantics (Q-1), verbatim line endings (Q-2). · gate attempts: 0 ·
  not run: gate (no code) · missing context: none · next: director review.
- 2026-11-12 · R-003 · draft · did: applied the director's answers (D-1…D-3): content-based
  dirty confirmed; verbatim line endings with dominant-ending detection placed in kiln-text and
  reported by T-003 (so this task stays on main's kiln-text API); `Cargo.lock` conflict
  procedure noted. · gate attempts: 0 · next: director review.
<!-- end planner log -->
### implementer
- 2026-11-12 · R-007 · ready · did: restated ACs: RelPath validates exactly the listed cases
  (AC-1); MemoryFs is the reference model for DiskFs incl. NotFound for missing file/parent
  (a file used as a parent counts as missing) and NotAFile for directories (AC-2, AC-11);
  open+save without edits is byte-identical for UTF-8, UTF-8+BOM, UTF-16LE/BE (AC-3..5);
  decode is strict with the exact offsets of AC-6 (earliest error wins, so an unpaired
  surrogate before a trailing odd byte is reported first); encode∘decode identity except
  Utf8+leading U+FEFF ⇒ Utf8Bom (AC-7); dirty = buffer text ≠ last opened/saved text (AC-8,
  D-1); failed save leaves file and dirty flag (AC-9); disk save = temp + rename (AC-10);
  error Display names only the RelPath (AC-11/12). Built modules path, vfs, memory, disk,
  encoding, document in crates/kiln-workspace/src (+ dep on kiln-text, Cargo.lock), 27 unit
  tests incl. a fixed-seed MemoryFs-vs-DiskFs differential test, inode/permission/symlink
  checks, stale-temp recovery and the AC-9 directory-replacement case through Document.
  Choices: temp file `.<name>.kiln-tmp` in the destination's directory, derived only from the
  name, opened with truncate, so a crash leftover is overwritten by the next save (cost: a
  user file of exactly that name would be clobbered); symlink chains are resolved (max 40
  hops) and the final target replaced; existing permissions copied to the temp file (owner
  and xattrs not preserved); temp file fsynced before rename (else a crash could leave an
  empty file under the real name), parent directory fsynced best-effort after rename (failure
  not reported because the replace already happened). DiskFs::new errors use the placeholder
  RelPath `(root)` (NotFound for a missing root, Io{NotADirectory} for a non-directory). ·
  gate attempts: 3 (1: red no-skip, a `#[cfg(unix)]` test, rewritten to run on every
  platform via cfg'd helper fns; 2: red clippy, compile error I did not build before running
  the gate; last: green) · not run: kiln-cli / control-protocol check (no doc commands exist
  until T-003; behaviour exercised only through the public API in unit tests); non-Unix
  builds (Windows symlink creation in the test needs privileges); very long file names (a
  255-byte name makes the temp name exceed NAME_MAX, so save fails with Io) · missing
  context: spec gives no RelPath for DiskFs::new errors (chose `(root)`); NotFound vs Io for a
  write whose parent is a file not stated (chose NotFound, matching MemoryFs); how the AC-2
  test creates directories in MemoryFs vs DiskFs not stated (MemoryFs::insert of a file below
  vs create_dir on disk assumed) · next: join with T-002 acceptance tests, review.
- 2026-11-12 · R-012 · request-changes (review r1) · did: fixed D-1..D-3 per D-014 in
  `disk.rs`. Temp file is now created exclusively (`create_new` = O_CREAT|O_EXCL, refuses
  existing files and symlinks, dangling or not) under the first free name of `.kiln-tmp-0` …
  `.kiln-tmp-99` in the target's directory: short, independent of the target name length, no
  clock/randomness; leftovers are never touched, only skipped; all 100 taken ⇒ `Io{AlreadyExists}`
  with nothing changed (D-014 (1), D-1, D-2). For an existing target the temp is created 0600
  and gets the target's bits via fchmod before fsync and rename; a new file is created 0666 &
  ~umask like a plain create (D-014 (2), D-3). An existing regular target that cannot be opened
  for writing (open O_WRONLY, no truncate) ⇒ `Io{PermissionDenied}`, untouched (D-014 (3); root
  can write anything, so root is never refused). Own temp removed on any failure after
  creation, including a failed rename (rename made injectable via a private
  `write_atomic_with` seam). Hard-link split, owner/group change, lost xattrs/ACLs documented
  in the module docs (D-014 (4)); Unix-only APIs used directly, the cross-platform test
  helpers removed (D-014 (5)). Error shapes unchanged (D-014 (6)). New unit tests: leftover
  (longer, read-only) untouched; symlink and dangling symlink at temp names not followed and
  outside file unchanged; name exhaustion fails cleanly then recovers; 245/246/250/255-byte
  names save; temp mode is the target's (0600/0640/0644/0755) with full content just before
  rename; injected rename failure leaves no own temp and keeps the target and a foreign
  `.kiln-tmp-0`; read-only target refused (as non-root) / written (as root); new-file default
  mode. · gate attempts: 1 (last: green) · not run: fsync remains unobservable (TG-2 not
  addressed); real crash injection; the PermissionDenied branch is not reached by `cargo xtask
  ci` here because the sandbox runs as root — I ran the lib test binary as `nobody` via
  `setpriv --reuid=65534 --regid=65534 --clear-groups` and all 13 disk tests passed, including
  that branch · missing context: none (D-014 answered it); note that a new file's mode follows
  the umask, not 0600, which D-014 (2) does not state explicitly · next: join with the
  test author's D-014 tests, review round 2.
- 2026-11-12 · R-016 · request-changes (review r2, tests only) · did: added a second private
  seam to `write_atomic_with` (`created`: runs on the temp file right after `create_temp`,
  may return Err; `write_atomic` passes a no-op, so production behaviour is unchanged). New
  unit tests in `disk.rs`: `save_creates_temp_owner_only_for_an_existing_target` (TG-4: mode
  is 0600 and the temp file is empty at creation, for targets 0600/0644/0666/0755) and
  `failed_save_after_writability_check_leaves_target_and_listing_unchanged` (TG-5: listing,
  target bytes and target mode identical after three root-provokable failures: all 100 temp
  names taken, injected failure after create, injected rename failure). Mutants applied
  locally and reverted (file restored from a backup, diff checked): N3 (temp created 0666 for
  an existing target) killed by the TG-4 test; N7 (writability check opens with truncate)
  killed by the TG-5 test, plus `all_temp_names_taken…` and `failed_rename…`. · gate
  attempts: 1 (last: green) · not run: non-root runs of the new tests (they have no root-only
  branch) · missing context: see QUESTION · next: director ruling on the QUESTION, review r3.
  QUESTION: the reworded D-014 (2) says the temp file is *always* created owner-only and a new
  file gets the platform default mode under the umask. The code creates a new file's temp with
  `0666 & ~umask` (r2 SG-8), so for new files it does not meet the reworded ruling. Meeting it
  is a behaviour change: create 0600, then chmod to `0666 & ~umask` before the rename, which
  needs the process umask. std cannot read it; the options are the `libc` crate (new
  dependency, `umask(2)` sets the mask while it reads it, so it is racy for threads) or parsing
  `Umask:` from `/proc/self/status` (Linux ≥ 4.7 only). As instructed I stopped on this point.
  The module docs describe what the code does (owner-only for an existing target, default mode
  for a new file); I left them unchanged.
<!-- end implementer log -->
### test author
- 2026-11-12 · R-008 · in_progress · did: wrote 15 acceptance tests in
  `crates/kiln-workspace/tests/acceptance_t002.rs` from the Interface and ACs only (did not read
  `task/T-002`); set `tests` of ws.vfs, io.utf8, io.bom, io.encoding, io.save-atomic to the
  spec's table names. Criterion → tests → result on base (all `compile`: the whole interface is
  new, so the single target fails with E0432/E0425, 32 errors; reviewer revert check needed):
  AC-1 → `relpath_accepts_and_rejects_exact_cases`; AC-2 → `memory_and_disk_vfs_agree_on_random_operation_sequences`
  (600 fixed-seed ops, MemoryFs vs DiskFs vs a byte-map model, final contents, no dirs created);
  AC-3 → `unedited_utf8_files_round_trip_byte_for_byte` (11-file corpus, both VFSs),
  `line_endings_round_trip_verbatim`; AC-4 → `utf8_bom_detected_stripped_and_restored_on_save`,
  `bom_only_file_round_trips`; AC-5 → `utf16_le_and_be_with_bom_open_and_save` (hand-written
  bytes incl. D834 DD1E; edited bytes built with std `encode_utf16`); AC-6 →
  `invalid_bytes_are_reported_with_offset_never_replaced` (the 6 named + 14 more); AC-7 →
  `encode_decode_round_trip` (2008 strings × 4 encodings, leading-U+FEFF ambiguity asserted
  exactly, also `name()`); AC-8 → `dirty_follows_text_versus_last_saved` (incl. 400-step naive
  model on a UTF-16BE doc, save-when-clean); AC-9 → `failed_save_leaves_file_intact_and_document_dirty`;
  AC-10 → `disk_save_replaces_via_rename_without_leftovers`, `save_through_symlink_updates_target`;
  AC-11 → `vfs_errors_name_relative_paths_only`; AC-12 → `disk_vfs_results_do_not_depend_on_root`
  (not in a scorecard item; the spec's table names none for AC-12). Edge/hostile: empty, BOM-only,
  double BOM, NUL, lone CR, CR/LF split by edit, 4-byte chars, U+10FFFF, overlong/surrogate/
  out-of-range UTF-8, odd UTF-16, lone/unpaired surrogates at end and mid, paths `.`, `a/.`,
  `../a`, `/`, `\`, NUL; symlink; path replaced by directory; root not a directory. Sanity:
  compiled and ran the file against a throwaway reference stub outside the repository
  (15/15 pass, clippy `-D warnings` clean, rerun-stable); the stub is not committed.
  Unix-only checks (mode bits, inode, symlink) go through `#[cfg(unix)]` helpers; on non-Unix the
  mode/inode asserts are vacuous. · gate attempts: 1 (last: red clippy — only
  `acceptance_t002.rs`, compile case; fmt pass; `cargo xtask no-skip` pass run separately) ·
  not run: gate steps after clippy (blocked by the compile case) · missing context: none ·
  next: join.
  QUESTION: AC-11/VfsError — for `write_atomic` into a missing parent, is `NotFound.path` the
  requested file or the missing parent? Tests assert only `NotFound{..}` and that Display contains
  `nodir`. Proposed wording: "`path` is the RelPath passed to the call".
  QUESTION: `PathError.path`/`reason` contents are unspecified; tests assert only rejection and a
  non-empty Display. Proposed: "`path` equals the input string".
  QUESTION: AC-9 DiskFs (path became a directory) says only "returns an error"; tests do not
  assert the variant. Proposed: "returns `VfsError::NotAFile`", consistent with `write_atomic` on
  MemoryFs.
- 2026-11-12 · R-013 · in_progress (round 2, rulings D-014) · did: added 10 tests in
  `crates/kiln-workspace/tests/acceptance_t002_r2.rs`; first file and scorecard items unchanged
  (no item names these tests). Every check is a whole-tree snapshot (entries, bytes, mode,
  inode, symlink targets, no symlink followed) before and after; decoys are planted under 35
  plausible temp names (incl. `.<name>.kiln-tmp`); no naming scheme assumed. Ruling → tests
  (base: all `compile`, 2 errors from unresolved imports):
  (1) `save_leaves_regular_files_at_temp_names_untouched`,
  `save_never_writes_through_symlinks_at_temp_names` (links → victims outside the dir),
  `save_ignores_self_pointing_and_dangling_symlinks_at_temp_names` (links → the document; dangling
  links whose targets must not appear), `save_through_symlink_with_decoys_changes_only_the_link_target`,
  `documents_named_like_temp_files_do_not_clobber_each_other`; (2)
  `saved_file_is_never_more_readable_than_the_target` (modes 600, 640, 604, 644, 660, 700, 755);
  (3) `save_to_unwritable_target_is_permission_denied_and_untouched` (modes 444, 400, 555; it
  probes whether permission bits stop writes; unprivileged ⇒ `Io{PermissionDenied, path}`,
  tree unchanged, still dirty; privileged ⇒ the same refusal or a complete save with the mode
  kept, never a partial state); (4) `file_with_255_byte_name_opens_edits_and_saves` (ASCII and
  2-byte chars); (5) `not_found_names_the_requested_path`, `save_onto_directory_is_not_a_file`.
  Validation against the throwaway stub outside the repo: the naive stub from R-008 fails 6/10
  (the decoy and long-name tests); a stub fixed to use `create_new` 0600 temps with short names
  passes 10/10 (and still 15/15 on round 1), both as root and, run as `nobody`, in the
  unprivileged branch. · gate attempts: 1 (last: red clippy, only the acceptance targets,
  compile case; fmt and no-skip pass) · not run: gate steps after clippy · missing context: none ·
  next: join.
  Not testable: the mode of the transient temp file (ruling 2, "no file created during the save
  is group- or world-readable"). It cannot be seen without a file watcher or a race; only the
  final state is checked. A failing *rename* cannot be provoked deterministically through the
  public interface (a directory target is caught as `NotAFile` before any temp exists; root
  ignores permission bits). Failed saves are checked for no extra entries only in the
  `NotAFile` and `PermissionDenied` cases. Decoys cover only the guessed names.
<!-- end test author log -->
### reviewer
- 2026-11-12 · R-011 · self_verified · did: review round 1 of joined `ccd3d52` →
  `reviews/T-002-r1.md`, verdict request-changes. Checked every AC through the public API.
  Probed data loss with a temporary probe test (not committed), run as root and as `nobody`.
  Mutation spot-check (D-011): 16 bugs introduced, 13 killed; survivors were the fsync drop,
  missing temp cleanup on a failed rename, and a non-truncating temp open. Defects:
  D-1 the temp is opened following symlinks and accepting an existing file, so a symlink at
  `.<name>.kiln-tmp` gets the document written into its target and replaces the file by the
  symlink; D-2 names of 246–255 bytes can never be saved; D-3 permissions are applied after
  the data is written (exposure window). Test gaps TG-1…TG-3 (stale or hostile temp
  leftovers; fsync; rename-failure cleanup). · gate attempts: 1 (last: green) · not run:
  non-Unix, real crash injection · missing context: supported platforms · next: implementer
  fixes D-1…D-3 and adds tests for TG-1; director rules on the spec gaps.
  QUESTION: SG-1 — may `write_atomic` modify or delete any path other than `path`? Proposed:
  no; use `create_new` over a bounded deterministic name sequence.
  QUESTION: SG-2 — should saving onto a read-only file fail with `Io{PermissionDenied}`?
  Proposed: yes.
  QUESTION: SG-3 — state that save splits hard links and changes the owner and xattrs; copy
  the owner when permitted?
  QUESTION: SG-4 — which platforms are supported? Until ruled, record Unix-only.
  QUESTION: SG-5/6/7 (test author's) — proposed rulings: `path` is the requested RelPath;
  `PathError.path` is the input verbatim and `reason` is free text; a save onto a directory
  returns `NotAFile`.
- 2026-11-12 · R-014 · round 2 · did: reviewed delta `4a45196..65ff172` against D-014 →
  `reviews/T-002-r2.md`, verdict request-changes (test-only). Round-1 defects D-1, D-2 and D-3
  (existing targets) are resolved, as are TG-1 and N-3; TG-2 is waived. TG-3 waiver stands: a
  failing rename can be provoked as non-root via a sticky directory and another uid's 0666 file
  (cleanup verified), but the setup needs chown, so it cannot be an acceptance test. Mutation
  spot-check: 12 bugs; the acceptance tests killed 6 (9 counting unit tests). Survivors:
  fsync; rename cleanup; temp created 0666 (survives every test); chmod after rename; fill-failure
  cleanup; writability check that truncates. New findings: TG-4 (owner-only temp mode on the
  save path not asserted; the named unit test checks the helper only), TG-5 (no acceptance save
  that fails after the check on a writable target, e.g. a 0555 directory as non-root),
  SG-8 (new-file temp follows the umask; reword D-014 (2)), nits N-4…N-7 (the check opens the
  target O_WRONLY; the exhaustion message gives no cleanup hint; the sticky-directory
  limitation is undocumented; the branch is not rebased on main and the D-014 text is missing
  in the branch's task file). · gate attempts: 1 (last: green) · not run: crash injection,
  ENOSPC/EFBIG path · missing context: none · next: test-only fixes for TG-4/TG-5 or a director
  waiver; rebase.
  QUESTION: SG-8 — reword D-014 (2) as "never more readable than the resulting file
  (owner-only for an existing target, `0666 & ~umask` for a new one)"?
- 2026-11-12 · R-017 · round 3 · did: reviewed delta `2fdce23..587bfae` →
  `reviews/T-002-r3.md`, verdict approve. The new private `created` hook cannot be reached
  outside `disk.rs`, and production passes a no-op. Re-ran survivors N3 and N7, plus N5, as
  root and as `nobody`: all three are killed by the new unit tests, so TG-4 and TG-5 are
  resolved (TG-5 by unit test, as the director chose). The code and module docs match the
  final D-014 (2) wording. Outstanding: N-7, the branch is not rebased on `main`
  (task-file conflict expected at the queue). · gate attempts: 1 (last: green) · not run:
  crash injection · missing context: none · next: director approval, then rebase and queue.
<!-- end reviewer log -->
