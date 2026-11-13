---
id: T-019
title: "List the project as a tree through the VFS, honouring .gitignore files and skipping target/ and .git, without following symlinks"
tier: high              # ADR-0002: reads directories on disk (paths that reach disk) inside the T-008 confinement boundary; adds the `ignore` dependency
state: draft
owner_role: planner
scorecard_items: [ws.open-project]
depends_on: [T-015, T-008]   # both merged (T-008 at d4915606f9a789b42b99a1eea053554f1f9dd8a7); project.rs slots; T-008 confinement (vfs.rs, disk.rs)
touches:
  - crates/kiln-workspace/Cargo.toml                    # implementer: + `ignore = "=0.4.34"` in [dependencies] (crate manifest, as regex/tree-sitter); a dependency change, not a gate path
  - crates/kiln-workspace/src/vfs.rs                    # implementer: Vfs::list_dir, DirEntry, DirListing, EntryKind
  - crates/kiln-workspace/src/memory.rs                 # implementer: MemoryFs::list_dir
  - crates/kiln-workspace/src/disk.rs                   # implementer: DiskFs::list_dir (confined), module docs
  - crates/kiln-workspace/src/project.rs                # implementer: project::tree (slot from T-015)
  - crates/kiln-workspace/tests/project.rs              # implementer's own integration tests
  - crates/kiln-commands/src/project.rs                 # implementer: `project.tree` (slot from T-015)
  - docs/protocol/project.md                            # implementer: new; the sentences of Pinned (i)
  - docs/protocol/paths.md                              # implementer: the first sentence only, PENDING Q-6
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs  # test author: AC-7 (Rust API)
  - apps/kiln-cli/tests/acceptance_t019_cli.rs          # test author: AC-1 … AC-6 through kiln-cli
  - apps/kiln-cli/tests/guard_t019_cli.rs               # test author: guards, if any (pass on base)
  - docs/commands.json                                  # generated (`cargo xtask schema --write`)
  - Cargo.lock                                          # generated; new packages only, no locked version changes (Pinned (b))
budget_tokens: 1100000  # high (D-091 change 3)
iteration: 2
found_by: planner
origin_task: null
---

# T-019 List the project as a tree

## Goal
The project is the session root (`--root`, or the in-memory file system of `--memory` /
`--memory-seed`). After this task `project.tree` lists its directories and files in a pinned
order. It honours the `.gitignore` files inside the project (pattern semantics as Pinned (i)
states them), always skips the root's `target` directory and every `.git` entry, and never
follows a symbolic link found in the tree. Directory reading goes through a new `Vfs::list_dir`.
On disk the requested `path` is located by T-008's rules (`docs/protocol/paths.md`): a `path`
whose location is outside the root is refused, and entries found while walking are reported by
their own kind, never followed. `find.in-files` (T-022) and workspace symbols (T-023) build on
the same walk.

No gate path. A dependency change (`ignore`), hence high. No new error kind, no `reason.rs`
change, no CLI change (so D-091 change 18 does not apply: `docs/control-protocol.md` is not
edited).

Reading list: `docs/protocol/paths.md`; `tasks/T-008-*.md` Definitions, Interface (error table,
precedence), Out of scope (the `list_dir` message note) and rulings v4–v13;
`crates/kiln-workspace/src/{vfs,memory,disk,path}.rs` (`locate`, `Found`, `RelPath::root_placeholder`);
`crates/kiln-commands/src/{doc,reason}.rs` (how `doc.open` maps `VfsError` and
`invalid_path`); `docs/protocol/cli-memory-seed.md`; `tasks/T-015-*.md` Ownership;
`tasks/T-036-*.md` Interface 1 (an unreadable root is refused at construction once T-036
merges); gitignore(5); this file's "Questions and rulings".

## Interface

### Dependency
`crates/kiln-workspace/Cargo.toml` `[dependencies]`: `ignore = "=0.4.34"` (its `rust-version`
is 1.88 ≤ the workspace's 1.97: VERIFIED in `ignore-0.4.34/Cargo.toml` of the local registry
cache). Record its release date in the log; if it is less than two weeks old, use the newest
release that is, with MSRV ≤ 1.97. Only its gitignore **matcher** may be used:
`ignore::gitignore::GitignoreBuilder::{new, add_line, build}` and
`ignore::gitignore::Gitignore::{matched, matched_path_or_any_parents}`. Every other item that
touches the host file system or the environment is forbidden: the walkers (`Walk`,
`WalkBuilder`, `WalkParallel`, …), `Gitignore::new`, `Gitignore::global`,
`GitignoreBuilder::add` and `build_global` (each opens files itself, `gitignore.rs`:103–118,
141–146, 408–411 of 0.4.34). Every directory read and every `.gitignore` read goes through
`Vfs`. The reviewer checks this by `grep`.

### `kiln_workspace` (L2)
```rust
// vfs.rs (exported at the crate root by the existing `pub use vfs::*`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind { File, Dir, Symlink, Other }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry { pub name: String, pub kind: EntryKind }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirListing { pub entries: Vec<DirEntry>, pub skipped: usize }   // entries sorted by name bytes
pub trait Vfs: std::fmt::Debug {
    // … read, write_atomic, is_file unchanged …
    /// List `dir` (None = the root). `skipped` counts names that are not valid UTF-8 or not a
    /// valid RelPath segment (for example a name containing `\`); those are omitted.
    fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError>;
}

// project.rs (reached as `kiln_workspace::project::…`; lib.rs is not edited)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEntry { pub path: RelPath, pub kind: EntryKind }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectTree {
    pub entries: Vec<TreeEntry>,
    pub ignored: usize,
    pub skipped: usize,
    pub too_deep: usize,     // PENDING Q-1
    pub truncated: bool,
}
pub fn tree(vfs: &dyn Vfs, under: Option<&RelPath>, limit: usize) -> Result<ProjectTree, VfsError>;
```
Errors about the root itself (`dir: None`, for example while the root path leads elsewhere)
carry the existing placeholder path `(root)` (`RelPath::root_placeholder`, as `DiskFs::new`
does), never a host path.

`MemoryFs::list_dir`: directories are implied by file paths (unchanged). A directory that holds
no file does not exist. The root always exists (an empty file system lists as empty). `list_dir`
of a path that is a file → `NotAFile`; of a missing path, or of a path below a file → `NotFound`
(as `MemoryFs::read`). Kinds are only `File` and `Dir`; `skipped` is always 0.

`DiskFs::list_dir`: the location of `dir` is found by T-008's rules (`locate`, root identity
check first), so a symlink on the way that stays inside the root is followed. Precedence as
T-008: location outside the root (or the root path no longer leading to the root) →
`OutsideRoot`; missing → `NotFound`; not a directory (file, FIFO, socket, device) → `NotAFile`;
any other failure → `Io`. More than 40 link hops → `Io` (`too many levels of symbolic links`),
as `read` (probe P12). Entry kinds come from the entry itself, not followed: a symlink is
`Symlink` whatever it points to (a loop, outside the root, dangling), FIFOs, sockets and
devices are `Other`. Nothing is opened except the directory being listed; no entry is
followed, opened or read.

### Walk rules (exact; `tree` and `project.tree`)
1. Start at `under` (default: the root). `under` itself is not listed. Entry paths are
   root-relative and begin with `under` as given (PENDING Q-3). Visit each directory's entries
   in name byte order; output is preorder (a directory, then its contents, then its next
   sibling).
2. Always skipped (counted in `ignored`, not descended): the entry whose path is exactly
   `target` when it is a `Dir`, and every entry named `.git`, whatever its kind.
3. `.gitignore` files: an entry named `.gitignore` of kind `File` in a visited directory, and the
   `.gitignore` of the root and of every directory on `under`'s path (PENDING Q-3), applies to
   that directory's subtree: patterns relative to the file's directory, `!` negation, a
   trailing `/` for directories only, `**`, a deeper file overriding a shallower one, and a
   file under an ignored directory cannot be re-included, as in git. It is read through
   `Vfs::read`; a leading UTF-8 BOM is dropped and the content is split into lines; lines that
   are not valid UTF-8, and lines the matcher refuses, are skipped (PENDING Q-5). A
   `.gitignore` entry of kind `Symlink`, `Other` or `Dir` is not read (PENDING Q-2). No other
   source is consulted: no `.git/info/exclude`, no global excludes, no `.ignore`. An ignored
   entry is counted once in `ignored` and not descended. `.gitignore` files themselves are
   listed unless ignored.
4. `Symlink` and `Other` entries are listed with their kind and never descended or read.
5. Depth (PENDING Q-1): a `Dir` entry whose path has 128 segments is listed and not descended;
   it is counted in `too_deep`.
6. Limit (PENDING Q-7): the walk stops when it would list entry `limit + 1`. Then `entries`
   holds exactly `limit` entries and `truncated` is true; `truncated` is false iff every entry
   was listed. `ignored`, `skipped` and `too_deep` count what the walk met before it stopped.

### Command `project.tree` (read-only, `MUTATING = false`)
Params `{"path"?: string, "limit"?: integer 1..=1000000 (default 50000)}`, unknown keys refused
(`invalid_params`, as other commands). Result:
`{"entries":[{"kind":"file","path":"src/main.rs"}, …], "ignored": n, "skipped": n, "too_deep": n, "truncated": bool}`
with kinds `file`, `dir`, `symlink`, `other` (`too_deep` PENDING Q-1). Refusals (`rejected`):
`invalid_path` (the `path` fails the lexical `RelPath` check, before the file system),
`outside_root`, `not_found`, `not_a_file` (the `path`'s location is a file or special file),
`io_error`. All are existing kinds; there is no new one, and no `reason.rs` change: the kind
comes from the existing `From<&VfsError> for RejectReason`. The message names the path as
given and never a host path; its wording is not pinned (PENDING Q-4 proposes
`cannot list <path>: not a directory` for `not_a_file`). At least one `Example` (run by the
audit on `MemoryFs`). `docs/protocol/project.md` documents the command, the walk rules and the
sentences of Pinned (i).

## Pinned
- (a) Errors: through `VfsError` with T-008's precedence (lexical `invalid_path` →
  `OutsideRoot` → `NotFound` → `NotAFile` → `Io`) for the `path` itself. An error while
  listing a visited subdirectory, or while reading a `.gitignore` of kind `File`, makes the
  whole command fail with that error; the message names that entry's RelPath. A refused
  command leaves the session unchanged and is not logged.
- (b) Files read: directories, and `.gitignore` files of kind `File` through `Vfs::read`.
  Nothing is written, created or opened for writing. `Cargo.lock` gains new packages only
  (`ignore`, `globset`, `bstr`, `log`, `crossbeam-deque`, `crossbeam-epoch`,
  `crossbeam-utils`, `same-file`, `walkdir`): every requirement of `ignore` 0.4.34 and
  `globset` 0.4.21 on an already locked package (`regex-automata` ≥ 0.4.18, `regex-syntax`
  ≥ 0.8.0, `memchr` ≥ 2.6.3, `aho-corasick` ≥ 1.1.1, `serde` ≥ 1.0.188) is met by the locked
  version (0.4.18, 0.8.11, 2.8.3, 1.1.5, 1.0.229). VERIFIED by reading the two manifests and
  `Cargo.lock`; the resolution itself was not run. If `cargo` changes a locked version, stop
  and report (the queue's `dependencies` rule refuses it without `dependency versions` in the
  approval row). Test files: those in `touches`, names distinct across crates.
- (c) Order: sort a directory's entries by name bytes before applying any rule; per entry,
  rules 2 → 3 → 4 → 5 → 6.
- (d) Unix, as T-008 (`DiskFs`). Names are compared as bytes. Hidden files are listed. No case
  folding. `path` is relative to the root as T-008 defines it.
- (e) Threat model: a hostile project directory (a cloned repository or an unpacked archive)
  with no concurrent attacker. It must not make Kiln list or read anything outside the root:
  links to `/`, to `..`, a symlinked subdirectory, a `.gitignore` that is a link to an outside
  file. It must not hang Kiln: FIFOs anywhere, a `.gitignore` that is a FIFO (listed `other`,
  never opened; `git ls-files` itself hangs on it, probe P7), link loops (listed, not
  followed), deep nesting (rule 5; PENDING Q-1). Bounded by `limit`: the number of listed
  entries, not the work (Pinned (i) S7). Accepted (T-008 D-3): a change by another process
  while the command runs. Not testable in the gate (it runs as uid 0, probe P13): an
  unreadable subdirectory; the reviewer probes it as an unprivileged user (T-036's identity U).
  The reviewer probes every other case adversarially.
- (f) Snapshots: none found. No test on main names `project.` (`grep -rn project` over every
  `tests/` directory: only prose). `Vfs` gains a required method; the only implementations
  are `MemoryFs` and `DiskFs` (`grep -rn 'impl Vfs for'`). Tests of this task must not pin
  what a drafted task changes (D-091 change 6): not the `kiln-cli commands` output or the
  command count, not `error_kinds` (T-038 publishes only declared kinds), not any message
  text, not a whole document, and not the behaviour of an unreadable root (T-036 refuses it
  at construction).
- (g) Worked example, executed (probe P1: naive Python model of rules 1–4 and `git ls-files -z`
  on the same directory): files `.gitignore` (`*.log\n/target\n`), `B.txt`, `a.rs`, `a/b.rs`,
  `x.log`, `target/debug/k`, `src/lib.rs`, `src/.hidden`. `project.tree {}` gives the entries
  `.gitignore` file, `B.txt` file, `a` dir, `a/b.rs` file, `a.rs` file, `src` dir,
  `src/.hidden` file, `src/lib.rs` file, with `ignored: 2` (`target`, `x.log`), `skipped: 0`,
  `too_deep: 0`, `truncated: false`. Model output:
  `[('.gitignore','file'), ('B.txt','file'), ('a','dir'), ('a/b.rs','file'), ('a.rs','file'), ('src','dir'), ('src/.hidden','file'), ('src/lib.rs','file')] ignored: 2`;
  git lists the same six files. Note `a/b.rs` before `a.rs` (preorder by name, not a sort of
  whole paths).
- (h) Evaluator call (`ws.open-project`): `kiln-cli exec project.tree '{}' --root <dir>` and
  `kiln-cli run <script> --root <dir>` with `project.tree`; compare the `file`/`symlink` paths
  with `git ls-files -z --cached --others --exclude-standard` in a fresh `git init` of the same
  directory (global config disabled as in AC-1), within the pattern forms of S5.
- (i) Sentences `docs/protocol/project.md` adds that state a guarantee or a limit, for the
  implementer to copy (wording may be joined into paragraphs, not widened). A sentence marked
  PENDING follows the ruling named. Each carries its evidence.
  - S1. "`project.tree` reads directories and `.gitignore` files and nothing else; it never
    writes, creates or changes a file." Evidence: AC-4 (outside files byte-identical), AC-1
    (trees unchanged); reviewer's `grep` for write calls.
  - S2. "On disk, the `path` is located as `docs/protocol/paths.md` describes: a `path` whose
    location is outside the root is refused with `outside_root`, and nothing outside the root
    is listed or read." Evidence: AC-4.
  - S3. "Symbolic links found in the project are listed with kind `symlink` and never followed,
    whatever they point to; FIFOs, sockets and devices are listed with kind `other` and never
    opened, so they cannot make the command wait." Evidence: AC-4 (5 s bound, FIFO, links to
    `..`, to `.` and outside).
  - S4 (PENDING Q-2). "A `.gitignore` is read only when it is a regular file; a `.gitignore`
    that is a symbolic link, a FIFO or a directory is listed and its patterns are not used."
    Evidence: AC-4; git 2.43 also ignores an in-tree `.gitignore` symlink (probe P6).
  - S5. "For patterns of the forms `*.x`, `/a`, `b/`, `!c`, `**/d` and `e/**`, in `.gitignore`
    files at any depth, the `file` and `symlink` entries are exactly the paths
    `git ls-files --cached --others --exclude-standard` lists in a fresh repository of the same
    directory with no global or system configuration, apart from the root's `target`
    directory." Evidence: AC-1.
  - S6 (PENDING Q-5). Known differences from git, each kept only if the implementer's unit test
    shows it (record the test and its output in the log; drop a bullet that does not hold, add
    one that a test finds): "`{a,b}` in a pattern matches `a` and `b` (git matches the literal
    name `{a,b}`); a trailing tab is removed from a pattern (git keeps it); a directory that
    holds its own `.git` is listed with its contents (git lists it as one entry `dir/`); names
    that are not UTF-8 or contain `\` are not listed and are counted in `skipped` (git lists
    them)." Evidence, git side: probes P2, P4, P8, P9. Kiln side, by reading: `globset` 0.4.21
    always expands alternates; `add_line` trims with `trim_right` (`gitignore.rs`:470–475);
    rule 2 skips only the `.git` entry; `RelPath::new` refuses `\`.
  - S7. "`limit` bounds the number of entries listed, not the work: each directory visited is
    read whole and sorted before any rule applies, and ignored entries do not count toward
    `limit`. A response with 50,000 entries is about 2 MB." Evidence: by construction (rule
    order (c)); probe P14 (50,000 entries `wide/f000000.rs` … : 2,050,013 bytes of JSON).
  - S8 (PENDING Q-1). "A directory 128 levels below the root is listed but not descended, and
    counted in `too_deep`." Evidence: AC-6; probe P11 (cost model).
  - S9. "A directory whose location on the host has a path of 4,096 bytes or more cannot be
    listed, and the command fails with `io_error`; whether that happens depends on how long the
    root's own host path is." Evidence: probe P10 (`doc.open` of a directory at an absolute
    path of 4,094 bytes → `not_a_file`, of 4,096 bytes → `io_error`; PATH_MAX 4096).
  - S10. "A directory or a `.gitignore` that cannot be read (for example for lack of
    permission) makes the whole command fail with `io_error`." Evidence: reviewer probe as an
    unprivileged user (not testable in the gate, Pinned (e)); the implementer records the
    command and output if it shows it.
  - S11. "The listing is not a snapshot: if another process changes the tree while the command
    runs, the result can mix the states before and after, and the residual limits of
    `docs/protocol/paths.md` apply to each directory read." Evidence: by construction (each
    `list_dir` locates its directory again, T-008 "nothing is cached").
  - S12. "Entries are in preorder, each directory's entries in byte order of their names; no
    case folding." Evidence: AC-2.

## Acceptance criteria
All CLI calls run under a timeout; fixtures are built in fresh temporary directories.
- AC-1 (`ws.open-project`, CLI, git oracle): for ≥ 10 hand-built fixture trees and ≥ 100
  fixed-seed random trees (a hand-written PRNG in the test; no new dev-dependency) with random
  `.gitignore` files: copy each to a temporary directory and run `git init`. Then run
  `git ls-files -z --cached --others --exclude-standard` with `GIT_CONFIG_GLOBAL=/dev/null`,
  `GIT_CONFIG_NOSYSTEM=1`, `-c core.excludesFile=` and a 30 s timeout. That set, minus paths
  under the root `target/`, equals the set of `file` and `symlink` paths of
  `kiln-cli exec project.tree '{}' --root <dir>`. Generator limits, outside which git and this
  task differ (S6, Pinned (e)): names from `[a-e0-9._-]` (no `\`, no braces, valid UTF-8), no
  name `.git`, patterns only of the forms of S5 (no braces, tabs, BOM or trailing spaces),
  depth below 128, no FIFO, and any symlink pointing inside the tree. Oracle: git.
- AC-2 (`ws.open-project`, order and counts): the worked example (Pinned (g)) gives exactly the
  stated result. For the random trees of AC-1, the entries are the naive model's preorder with
  byte-sorted siblings, and `ignored` equals the model's count. Oracle: naive model + exact
  value.
- AC-3 (`ws.open-project`, memory = disk): the same trees given as a `--memory-seed` manifest
  (`docs/protocol/cli-memory-seed.md`) and as a `--root` directory give identical
  `project.tree` results through `kiln-cli exec`, apart from directories without files, which
  do not exist in memory. Oracle: differential.
- AC-4 (confinement, CLI): a root holding `out -> ../outside` (a real directory holding a real
  file and a `.gitignore` containing `*`; the test first asserts both exist), `self -> .`,
  `loop1 -> loop2`, `loop2 -> loop1`, a FIFO `p`, and a `.gitignore` that is a symlink to an
  outside file containing `*`, plus a regular file `keep.rs`, gives:
  - `project.tree {}` lists `.gitignore`, `keep.rs`, `loop1`, `loop2`, `out`, `p`, `self` with
    kinds `symlink`, `file`, `symlink`, `symlink`, `symlink`, `other`, `symlink`, and nothing
    under any of them (so neither outside `*` pattern had an effect; PENDING Q-2);
  - `project.tree {"path":"out"}` → `rejected` / `outside_root`;
  - `project.tree {"path":"self"}` → the root's entries again, each path beginning `self/`
    (PENDING Q-3);
  - each call returns within 5 s, and the outside files are byte-identical afterwards.
  Oracle: exact value.
- AC-5 (limits and refusals): on the worked example, `limit: 3` gives the first 3 entries and
  `truncated: true`; `limit: 8` gives all 8 and `truncated: false` (PENDING Q-7).
  `path: "a.rs"` → `not_a_file`, `path: "nope"` → `not_found`, `path: "../x"` →
  `invalid_path`, `path: "a"` → `a/b.rs` only. `limit: 0` → `invalid_params`. Oracle: exact
  value.
- AC-6 (depth, PENDING Q-1): a chain of 130 nested directories `d/d/…/d` with a file at the
  bottom gives 128 `dir` entries, `too_deep: 1` and no `file` entry; a chain of 2,100 nested
  directories returns within 10 s with `ok` and `too_deep: 1`. Oracle: exact value.
- AC-7 (Rust API, extra): `Vfs::list_dir` on `MemoryFs` and `DiskFs` gives byte-sorted
  entries with their kinds; on disk a name containing `\` and a name that is not UTF-8 are
  omitted and counted in `skipped`; `list_dir` of a file → `NotAFile`, of a missing path →
  `NotFound`, of a link to an outside directory → `OutsideRoot`. Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| ws.open-project | `kiln_cli::acceptance_t019_cli::tree_equals_git_ls_files`, `kiln_cli::acceptance_t019_cli::tree_order_and_counts`, `kiln_cli::acceptance_t019_cli::memory_and_disk_trees_agree`, `kiln_cli::acceptance_t019_cli::symlinks_are_listed_not_followed` |

The test author lists the tests in its log; the director ties them to the item at the join
(METHODOLOGY §6). The test author does not edit `scorecard/`.

## Out of scope and implementer notes
- Watching for changes; caching the tree; `.ignore`/global excludes/`.git/info/exclude`;
  Windows; find-in-files (T-022); declaring `project.tree`'s refusal kinds (T-038, after this
  task, D-092). The project is the session root: there is no `project.open` command.
- No `reason.rs` or `VfsError` variant change (PENDING Q-4), no `kiln-control` or `kiln-cli`
  change, no `docs/control-protocol.md` change.
- Your tests: `crates/kiln-workspace/tests/project.rs`, unit tests in the touched files.
- Determinism: output depends only on the tree's names, kinds and `.gitignore` contents (and,
  on disk, on the limit of S9).
- Dispatch: never in flight with T-036 (`disk.rs`, `paths.md`; T-036 runs after this task,
  D-092) or T-037 (`paths.md`). `docs/commands.json` and `Cargo.lock` are generated and
  regenerated at the queue.

## Questions and rulings

### Spec check R-156 (planner, against main 891ae13734574e43275cb387ab7b1440479bd451)
Blockers found in the R-066 text, each fixed above or turned into a question:
1. Pinned (e) asked the walk to survive 10,000-deep nesting. On disk every `list_dir` locates
   its directory again from the root (T-008, "nothing is cached"), so a walk to depth d costs
   about d²/2 `lstat`s, and an absolute path of 4,096 bytes fails with `io_error` (P10, P11).
   Not meetable as written → Q-1.
2. AC-4 expected a `.gitignore` symlinked outside to have no effect, while rule 3 read every
   `.gitignore` through `Vfs::read` and Pinned (a) made a read error fail the whole command
   (`outside_root`); Pinned (e) wanted a FIFO `.gitignore` to give `not_a_file` while rule 4
   says `Other` entries are never read. Contradiction → Q-2.
3. AC-3 named `Host::in_memory`/`Host::on_disk` (Rust API of `kiln-control`), not observable
   through the CLI from `kiln_cli` tests as the scorecard row claims. Fixed: `--memory-seed`
   versus `--root` (T-011 merged).
4. Test names `kiln_cli::acceptance_t019::…` would collide with any other crate's
   `acceptance_t019`; and no acceptance or guard file was in `touches`. Fixed:
   `acceptance_t019_cli.rs`, `acceptance_t019_workspace.rs`, `guard_t019_cli.rs`.
5. AC-1 parsed `git ls-files` without `-z`: git quotes names with `\`, newlines or non-UTF-8
   bytes (P9). Fixed, and the generator restricted to where git and the matcher agree (P2, P4,
   P5, P7, P8).
6. The dependency text allowed any non-walker API; `Gitignore::new`, `GitignoreBuilder::add`
   and `Gitignore::global` read the host file system directly, bypassing confinement. Fixed:
   the allowed items are named.
7. `path` semantics were unstated where T-008 makes them matter (a `path` naming an in-root
   symlink to a directory is followed by T-008's rule; ancestors' `.gitignore`; whether `path`
   itself is listed) → Q-3.
8. "gitignore(5) semantics" (a document sentence, D-091 change 5) is wider than the matcher:
   braces, trailing tabs and a BOM differ from git 2.43 (P2, P4, P5) → Q-5 and Pinned (i) S6.
Fixed mechanically as well: front matter (budget 1,100,000, iteration 2, `touches`),
`Vfs: std::fmt::Debug` signature, `(root)` placeholder for root errors, `too_deep`, BOM, limit
semantics (pending), `limit: 0`, Pinned (b) lock analysis, (f) pins forbidden by D-091
change 6, (i) the document sentences, scorecard ownership (METHODOLOGY §6), AC-4 wording (the
atime clause replaced by an outside `*` pattern), AC-6/AC-7 added. Not split: the walk needs
`list_dir`, and both serve one item (`ws.open-project`); they are not independent items in
D-091's sense. If the director wants smaller high-tier tasks, the line would be `Vfs::list_dir`
(vfs/memory/disk) before the walk and command (project.rs ×2, `ignore`), each high.

Probes (all under a fresh scratch directory, removed afterwards; `kiln-cli` is
`target/debug/kiln-cli` built 18:15Z from main; git 2.43.0; run as uid 0):
- P0 `kiln-cli exec project.tree '{}' --root p7` → `unknown_command`, exit 2 (fails on base).
- P1 worked example, naive model and git: see Pinned (g).
- P2 `.gitignore` `{a,b}` with files `a`, `b`, `{a,b}`: git lists `a`, `b` (the literal name
  ignored).
- P3 `.gitignore` `*.log\r\n`: git ignores `x.log` (CR dropped).
- P4 `.gitignore` `*.log\t`: git lists `x.log` (tab kept).
- P5 `.gitignore` with a UTF-8 BOM then `*.log`: git ignores `x.log`.
- P6 `.gitignore -> real-ignore` (`*.log`): git warns `unable to access '.gitignore': Too many
  levels of symbolic links` and lists `x.log`.
- P7 FIFO `.gitignore`: `git ls-files` blocks until `timeout 30` (exit 124).
- P8 `sub/` with its own `git init`: git lists `sub/` and `top`, not `sub/f`.
- P9 names `n\xff`, `b\s`, `nl\nx`: git lists all three, quoted without `-z`.
- P10 `doc.open` of a directory at absolute path length 4,076/4,086/4,092/4,094 →
  `not_a_file`; at 4,096, 4,104, 4,106, 4,116 → `io_error`; of a file at relative depth 1,000
  → ok in 0.03–0.05 s, at depth 2,100 → `io_error`.
- P11 cost model of a re-walk from the root per directory (Python `os.lstat` of every prefix,
  as `locate` does): depth 64 → 2,080 lstats 0.01 s; 128 → 8,256, 0.02 s; 256 → 32,896,
  0.15 s; 512 → 131,328, 1.04 s; 1,000 → 500,500, 6.93 s; 1,998 → 1,997,001, 62.3 s. A model,
  not Kiln: no build was allowed in this check; the implementer measures the real walk.
- P12 `doc.open` of `l1` (`l1 -> l2 -> l1`) → `io_error` `too many levels of symbolic links`;
  `out -> ../outside` and `out/secret` → `outside_root`; `self/self/self/a` → `not_found`;
  41 × `self/` → `io_error`.
- P13 as uid 0 a directory of mode 000 is read normally (`doc.open u/x` ok).
- P14 a directory of 200,000 empty files: `scandir` + byte sort in Python 0.13 s; JSON of
  50,000 entries 2,050,013 bytes, of 200,000 entries 8,200,013 bytes.

### Questions for the director (R-156). The spec above is written for each recommendation, marked PENDING.
- Q-1 (deep trees; blocker 1). Options: (A) a depth bound: a directory whose path has N
  segments is listed and not descended, counted in a new result field `too_deep`; with N = 128
  the cost model is 0.02 s (P11) and a 4 KB path cannot be reached with short names. (B) No
  bound; document the cost (model: ~1 s at depth 512, ~1 min at ~2,000) and S9's `io_error`;
  Pinned (e)'s "must not hang" is then dropped for deep trees. (C) A handle-based walk
  (`openat`), which needs `unsafe` (denied) or a new dependency, against T-008 D-3.
  Consequences: (A) adds a result field and AC-6; consumers (T-022, T-023) see a truncated
  subtree counted, not silently. (B) keeps the interface but lets a 4 KB hostile tree stall a
  session for about a minute. Recommended: (A), N = 128 (real trees are far shallower; the
  number rests on P11 and the implementer re-measures the real walk at depth 128 and 2,100
  and records it).
- Q-2 (a `.gitignore` that is not a regular file; blocker 2). (A) Only a `.gitignore` entry of
  kind `File` is read; a symlink, FIFO or directory named `.gitignore` is listed by its kind
  and its patterns are unused. (B) Read every `.gitignore` through `Vfs::read` (follow in-root
  links as T-008 does): an outside link then fails the whole command with `outside_root`, a
  FIFO with `not_a_file`, so a hostile repository can make `project.tree` unusable; AC-4 is
  rewritten to expect those refusals. Recommended: (A): it matches rule 4, git's own refusal
  of an in-tree `.gitignore` symlink (P6) and the threat model; it never reports anything
  about the outside.
- Q-3 (`path`). (i) A `path` naming an in-root symlink to a directory: (A) followed, as
  T-008's location rule (`paths.md`: "Symlinks whose location stays inside the root are
  followed"); entries carry the path as given (`self/…`), and rule 2's `target` applies only
  to the lexical path `target`; (B) refused (`not_a_file`) unless the last component is a real
  directory, an exception to `paths.md`. (ii) `.gitignore` files of the root and of every
  directory on `path`'s lexical prefix apply (as git), or only those at and below `path`.
  (iii) `path` itself is not listed and not subject to rules 2–3 (a named ignored, `target` or
  `.git` directory is listed), or it is (an ignored `path` gives an empty listing).
  Recommended: (i) A, (ii) the root's and every prefix's, (iii) not listed and not subject.
- Q-4 (message for `list_dir` of a file; T-008 Out of scope note 19). (A) The command words
  its own message for `NotAFile` (`cannot list <path>: not a directory`); `VfsError` and
  `reason.rs` unchanged, kind `not_a_file`, whose generated `when` already covers "a file
  where a directory is needed". (B) A new `VfsError::NotADirectory` variant with its own
  `Display`, mapped to `NotAFile` in `reason.rs` (shared with T-020's row) and in every
  exhaustive match. Recommended: (A): no shared file, no contract change; messages are not
  stable anyway (`docs/control-protocol.md`).
- Q-5 (matcher differences from git). (A) Use the matcher as is; drop a leading BOM (as git,
  P5); document the remaining differences (S6) as limits. (B) Pre-process each line to git's
  rules (escape `{`/`}`, trim only spaces) and test them against git. (C) No `ignore`; a
  hand-written matcher tested against git (no dependency, so possibly not high on that
  ground; the confinement criterion keeps it high). Recommended: (A); (B) only if the
  director wants S5 to hold for every pattern, which AC-1 would then have to generate.
- Q-6 (`docs/protocol/paths.md`). Its first sentence says "Every `path` in a request
  (`doc.open`, and the file a `doc.save` writes) names a file below the root"; after this
  task a `path` can name a directory. (A) This task replaces that sentence by one that names
  `project.tree` and points to `docs/protocol/project.md` (one sentence; `paths.md` is in
  `touches`, shared with T-036 and T-037, which run after). (B) Leave it to T-036, which
  rewrites "The root" anyway. Recommended: (A), so no merged document is narrower than the
  code at any point.
- Q-7 (limit edge and counts). `truncated` true iff an entry beyond `limit` exists (so
  exactly `limit` entries with nothing more is `false`), and the counts cover only what was
  walked before the stop; or `truncated` true whenever `entries.len() == limit`.
  Recommended: the first (AC-5's `limit: 8`).
### Rulings on Q-1 to Q-7 (director, D-095)
- Q-1: option (A), a depth bound of 128 path segments; a directory at the bound is listed, not descended, and counted in `too_deep`. The documents state the bound as behaviour. No document states a time for a walk unless the builder measured it on Kiln; the spec check's figures are a model, not a measurement of Kiln.
- Q-2: (A). A `.gitignore` is read only when its kind is `File`; a link, FIFO or directory of that name is listed by its kind and contributes no patterns.
- Q-3: as recommended. A `path` naming an in-root link to a directory is followed; entries are reported under the path as given; the `.gitignore` of the root and of every directory on the path's prefix applies; `path` itself is not listed and is not subject to the skip and ignore rules.
- Q-4: (A). The command reports listing a file under the existing kind `not_a_file`; no new error variant and no change to `reason.rs`. Consequence for T-038, noted there at its pre-dispatch check: after this task a command can produce `not_a_file` for a file where a directory is needed.
- Q-5: (A). The matcher is used as it is, a leading byte-order mark is dropped, and the remaining differences from git are documented as limits (Pinned (i) S6). Each documented difference is shown by a test or a probe the implementer records.
- Q-6: yes. `docs/protocol/paths.md` joins `touches` for its first sentence only; T-036 and T-037 run after this task.
- Q-7: as recommended. `truncated` is true only if an entry beyond `limit` exists; the counts cover what was walked before the stop.
- Dependency: the `ignore` crate named in Pinned (b) is accepted for this task by the director as the owner's delegate, subject to the review; the approval row is written at the merge as for every high-tier task.

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
- 2026-11-13 · R-060 · draft · did: T-008 no longer precedes T-015 (contracts first), so it is now an explicit dependency: shared `vfs.rs`/`disk.rs` and `OutsideRoot`. The `ignore` crate stays in the crate manifest (R-060 answer 1). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: needs T-032 transitively (through T-008); the `not_a_file` wording for `list_dir` of a file is in T-008 (finding 23). It can now export its types without touching `kiln-workspace/src/lib.rs` (T-015 glob re-exports, finding 6). · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-156 · draft · did: spec check: read the spec against main 891ae13 (T-008, T-010, T-011, T-015, T-017, T-018, T-032, T-035 merged); 8 blockers (listed under "Spec check R-156"): 4 fixed in place (3–6), 4 turned into questions: Q-1 (deep trees: T-008's per-call walk costs ~d²/2 lstats, 4,096-byte paths fail), Q-2 (non-regular `.gitignore` contradicts AC-4), Q-3 (`path` semantics), Q-5 (matcher wider than "gitignore(5)"); 7 questions with recommendations, spec written for them and marked PENDING. Applied D-091 changes 3 (ceiling), 5 (Pinned (i) S1–S12 with evidence), 6 (no pins of commands/kinds/messages/unreadable root), 18 (not applicable: no CLI change, stated). Tier stays high (dependency; confinement boundary). Not split (one item; walk needs list_dir). Probes P0–P14 recorded above; no cargo build or test run, so the walk's real cost, the Kiln side of S6 and the lock resolution are unmeasured; `ignore` 0.4.34's release date not checked (offline). spec check: booked to iter-2. · gate attempts: 0 · not run: cargo build/test, cargo update · missing context: none · next: director rules Q-1…Q-7, then dispatch.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
