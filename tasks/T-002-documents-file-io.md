---
id: T-002
title: "Open and save documents through a virtual file system, preserving encoding and line endings, atomically"
tier: high              # persistence/save path; Cargo.lock (new dependency edge)
state: in_progress
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
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
