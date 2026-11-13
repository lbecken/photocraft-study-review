---
id: T-019
title: "List the project as a tree through the VFS, honouring .gitignore files and skipping target/ and .git, without following symlinks"
tier: high              # ADR-0002: reads directories on disk (paths that reach disk) inside the T-008 confinement boundary; adds the `ignore` dependency
state: draft
owner_role: planner
scorecard_items: [ws.open-project]
depends_on: [T-015]     # project.rs slots; T-008 confinement arrives transitively
touches:
  - crates/kiln-workspace/Cargo.toml         # + ignore (exact pin)
  - crates/kiln-workspace/src/vfs.rs         # Vfs::list_dir, DirEntry, EntryKind
  - crates/kiln-workspace/src/memory.rs      # MemoryFs::list_dir
  - crates/kiln-workspace/src/disk.rs        # DiskFs::list_dir (confined)
  - crates/kiln-workspace/src/project.rs     # project tree + gitignore (slot from T-015)
  - crates/kiln-workspace/tests/project.rs   # implementer's own integration tests
  - crates/kiln-commands/src/project.rs      # project.tree (slot from T-015)
  - docs/protocol/project.md                 # new
  - docs/commands.json                       # generated
  - Cargo.lock                               # generated
budget_tokens: 850000   # high (v1.1)
iteration: 1
found_by: planner
origin_task: null
---

# T-019 List the project as a tree

## Goal
The project is the session root (`--root`, or the `--memory` file system). After this task
`project.tree` lists its directories and files in a pinned order. It honours every `.gitignore`
inside the project with git's semantics, always skips the root's `target` directory and every
`.git` directory, and never follows a symbolic link. Directory reading goes through a new
`Vfs::list_dir`. On disk it stays inside T-008's confinement: a directory whose location is
outside the root is refused, and symlink entries are reported, not followed. `find.in-files`
(T-022) and workspace symbols (T-023) build on the same walk.

No gate path. Reading list: `tasks/T-008-*.md` Definitions and Interface; `crates/kiln-workspace/src/
{vfs,memory,disk}.rs`; `tasks/T-015-*.md` Ownership; gitignore(5).

## Interface

### Dependency
`crates/kiln-workspace/Cargo.toml`: `ignore = "=0.4.34"`, or the newest release that is at least
two weeks old with MSRV ≤ 1.97; record it in the log. Only its gitignore **matcher**
(`ignore::gitignore`) may be used. Its directory walkers (`Walk`, `WalkBuilder`, …) must not
be: every directory read goes through `Vfs`.

### `kiln_workspace` (L2)
```rust
// vfs.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind { File, Dir, Symlink, Other }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry { pub name: String, pub kind: EntryKind }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirListing { pub entries: Vec<DirEntry>, pub skipped: usize }   // entries sorted by name bytes
pub trait Vfs {
    // … existing methods unchanged …
    /// List `dir` (None = the root). `skipped` counts names that are not valid UTF-8 or not a
    /// valid RelPath segment; those are omitted.
    fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError>;
}

// project.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEntry { pub path: RelPath, pub kind: EntryKind }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectTree { pub entries: Vec<TreeEntry>, pub ignored: usize, pub skipped: usize, pub truncated: bool }
pub fn tree(vfs: &dyn Vfs, under: Option<&RelPath>, limit: usize) -> Result<ProjectTree, VfsError>;
```
`MemoryFs::list_dir`: the directories are implied by file paths. A directory that holds no file
does not exist. `list_dir` of a path that is a file → `NotAFile`; of a missing path →
`NotFound`. Kinds are only `File` and `Dir`.

`DiskFs::list_dir`: the location of `dir` is found by T-008's rules. Outside the root →
`OutsideRoot`; missing → `NotFound`; not a directory → `NotAFile`. Entry kinds come from the
entry itself, not followed: a symlink is `Symlink` whatever it points to, and FIFOs, sockets
and devices are `Other`. The call never blocks on a special file, and nothing is opened except
the directory.

### Walk rules (exact)
1. Start at `under` (default: root). Visit each directory's entries in name byte order; output
   is preorder (a directory, then its contents, then its next sibling).
2. Always skipped (counted in `ignored`, not descended): the entry `target` directly under the
   root when it is a `Dir`, and every entry named `.git`.
3. `.gitignore` files: one in the root or in any visited directory applies to that directory's
   subtree, with gitignore(5) semantics: patterns relative to the file's directory, `!`
   negation, a trailing `/` for directories only, `**`, and a deeper file overriding a
   shallower one. It is read through `Vfs::read` and decoded as UTF-8; lines that are not UTF-8
   are skipped. No other source is consulted: no `.git/info/exclude`, no global excludes, no
   `.ignore`. An ignored entry is counted once in `ignored` and not descended. A file under an
   ignored directory cannot be re-included, as in git. `.gitignore` files themselves are listed
   unless ignored.
4. `Symlink` and `Other` entries are listed with their kind and never descended or read.
5. At most `limit` entries are listed; then `truncated: true` and the walk stops.

### Command `project.tree` (read-only)
Params `{"path"?: string, "limit"?: integer 1..=1000000 (default 50000)}`. Result:
`{"entries":[{"path":"src/main.rs","kind":"file"}, …], "ignored": n, "skipped": n, "truncated": bool}`,
with kinds `file`, `dir`, `symlink`, `other`. Refusals: `invalid_path`, `not_found`,
`not_a_file` (the `path` is a file), `outside_root`, `io_error`. All are existing kinds; there is
no new one. `docs/protocol/project.md` documents the command and the walk rules.

## Pinned
- (a) Errors: through `VfsError` with T-008's precedence (`OutsideRoot` → `NotFound` →
  `NotAFile` → `Io`). An error while listing a subdirectory, or while reading a `.gitignore`,
  makes the whole command fail with that error. The path in the message is the RelPath, never a
  host path.
- (b) Files read: directories, and `.gitignore` files through `Vfs::read`. Nothing is written.
- (c) Order: sort a directory's entries by name bytes before applying any rule; rules 2 → 3 → 4
  per entry.
- (d) Unix, as T-008 (`DiskFs`). Names are compared as bytes. Hidden files are listed. No
  case folding.
- (e) Threat model: a hostile project directory (cloned repository). It must not make Kiln read
  or list anything outside the root: symlinks to `/`, to `..`, or a symlinked root subdirectory.
  It must not hang it: FIFOs, a `.gitignore` that is a FIFO (→ `not_a_file` through T-008), or
  10,000-deep nesting (iterative walk or bounded recursion; no stack overflow). And huge trees
  are bounded by `limit`. The reviewer probes each case.
- (f) Snapshots: none. `Vfs` gains a method, so every `Vfs` implementation (`MemoryFs`,
  `DiskFs`, and any in tests) implements it.
- (g) Worked example (executed against a naive Python model): files `.gitignore` (`*.log\n/target\n`),
  `B.txt`, `a.rs`, `a/b.rs`, `x.log`, `target/debug/k`, `src/lib.rs`, `src/.hidden`. Then
  `project.tree {}` gives the entries `.gitignore` file, `B.txt` file, `a` dir, `a/b.rs` file,
  `a.rs` file, `src` dir, `src/.hidden` file, `src/lib.rs` file, with `ignored: 2` (`target`,
  `x.log`), `skipped: 0`, `truncated: false`.
- (h) Evaluator call (`ws.open-project`): `kiln-cli exec project.tree '{}' --root <dir>` and
  `kiln-cli run` with `project.tree`; compare with `git ls-files -co --exclude-standard` in a
  fresh `git init` of the same directory.

## Acceptance criteria
- AC-1 (`ws.open-project`, CLI, git oracle): for ≥ 10 hand-built fixture trees and ≥ 100
  fixed-seed random trees with random `.gitignore` files (patterns from `*.x`, `/a`, `b/`,
  `!c`, `**/d`, `e/**`, nested `.gitignore`s), copy each to a temp dir and run `git init`. Then
  run `git ls-files --cached --others --exclude-standard`, with `GIT_CONFIG_GLOBAL=/dev/null`,
  `GIT_CONFIG_NOSYSTEM=1`, `-c core.excludesFile=` and a 30 s timeout. That set, minus paths
  under the root `target/`, equals the set of `file` and `symlink` paths of `kiln-cli exec
  project.tree '{}' --root <dir>`. Oracle: git.
- AC-2 (`ws.open-project`, order and counts): the worked example gives exactly the stated
  result. For random trees, the entries are the naive preorder with byte-sorted siblings, and
  `ignored` equals the naive count. Oracle: naive model + exact value.
- AC-3 (`ws.open-project`, memory = disk): the same fixture through `Host::in_memory` (files
  inserted) and through `Host::on_disk` gives identical `project.tree` results, apart from
  directories without files, which do not exist in memory. Oracle: differential.
- AC-4 (confinement, CLI): a root holding `out -> ../outside` (a real directory with a real file,
  whose presence the test asserts), `self -> .`, a FIFO `p` and a `.gitignore` that is a
  symlink to an outside file gives:
  - `project.tree` lists `out`, `self` and `p` with kinds `symlink`, `symlink`, `other`, and
    nothing under them;
  - `project.tree {"path":"out"}` → `outside_root`;
  - the call returns within 5 s, and nothing outside is read (the outside file's atime is not
    used; the test checks that the outside `.gitignore`'s patterns had no effect).
  Oracle: exact value.
- AC-5 (limits and refusals): `limit: 3` gives 3 entries and `truncated: true`. `path: "a.rs"`
  → `not_a_file`, `path: "nope"` → `not_found`, `path: "../x"` → `invalid_path`. Oracle: exact
  value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| ws.open-project | `kiln_cli::acceptance_t019::tree_equals_git_ls_files`, `kiln_cli::acceptance_t019::tree_order_and_counts`, `kiln_cli::acceptance_t019::memory_and_disk_trees_agree`, `kiln_cli::acceptance_t019::symlinks_are_listed_not_followed` |

## Out of scope and implementer notes
- Watching for changes; caching the tree; `.ignore`/global excludes; Windows; find-in-files
  (T-022). The project is the session root: there is no `project.open` command.
- Your tests: `crates/kiln-workspace/tests/project.rs`, unit tests in the touched files.
- Determinism: output depends only on the tree's names, kinds and `.gitignore` contents.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-056 · (new) · did: specified the project walk over a new `Vfs::list_dir`, so
  that MemoryFs stays the reference model and DiskFs keeps T-008's confinement. Only the
  matcher of `ignore` is used (its walker would read disk outside the Vfs), and the oracle is
  `git ls-files` with global config disabled. Tier high, by ADR-0002: paths that reach disk, the
  confinement boundary, and a new dependency. The worked example was executed against a naive
  model. Unsure: `ignored` counts top-level ignored entries only; whether the UI wants ignored
  files greyed rather than hidden is an iteration-2 question. · gate attempts: 0 · missing
  context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
