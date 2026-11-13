---
id: T-019
title: "List the project as a tree through the VFS, honouring .gitignore files and skipping target/ and .git, without following symlinks"
tier: high              # ADR-0002: reads directories on disk (paths that reach disk) inside the T-008 confinement boundary; adds the `ignore` dependency
state: queued
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
  - docs/protocol/paths.md                              # implementer: the first sentence only (Q-6, D-095)
  - crates/kiln-workspace/tests/acceptance_t019_workspace.rs  # test author: AC-7 (Rust API)
  - apps/kiln-cli/tests/acceptance_t019_cli.rs          # test author: AC-1 … AC-6 through kiln-cli
  - apps/kiln-cli/tests/guard_t019_cli.rs               # test author: the guards under Acceptance criteria (pass on base)
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
    pub too_deep: usize,     // Q-1 (D-095)
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
1. Start at `under` (default: the root). `under` itself is not listed and is not subject to
   rules 2–3 (a named ignored, `target` or `.git` directory is listed). Entry paths are
   root-relative and begin with `under` as given, also when `under` is reached through an
   in-root link (Q-3). Visit each directory's entries
   in name byte order; output is preorder (a directory, then its contents, then its next
   sibling).
2. Always skipped (counted in `ignored`, not descended): the entry whose path is exactly
   `target` when it is a `Dir`, and every entry named `.git`, whatever its kind.
3. `.gitignore` files: an entry named `.gitignore` of kind `File` in a visited directory, and the
   `.gitignore` of the root and of every directory on `under`'s lexical prefix, each read at its lexical path (Q-3), applies to
   that directory's subtree: patterns relative to the file's directory, `!` negation, a
   trailing `/` for directories only, `**`, a deeper file overriding a shallower one, and a
   file under an ignored directory cannot be re-included, as in git. It is read through
   `Vfs::read`; a leading UTF-8 BOM is dropped and the content is split into lines; lines that
   are not valid UTF-8, and lines the matcher refuses, are skipped (Q-5). A
   `.gitignore` entry of kind `Symlink`, `Other` or `Dir` is not read and contributes no
   patterns (Q-2); on `under`'s prefix, a `.gitignore` is read only if `Vfs::list_dir` of its
   directory reports it as `File`. No other
   source is consulted: no `.git/info/exclude`, no global excludes, no `.ignore`. An ignored
   entry is counted once in `ignored` and not descended. `.gitignore` files themselves are
   listed unless ignored. Whether a `.gitignore` that its own or a shallower file's patterns
   ignore is still read is open (Q-8); AC-1's generator produces no pattern that matches the
   name `.gitignore` until it is ruled.
4. `Symlink` and `Other` entries are listed with their kind and never descended or read.
5. Depth (Q-1): a `Dir` entry whose path has 128 segments is listed and not descended;
   it is counted in `too_deep`.
6. Limit (Q-7): the walk stops when it would list entry `limit + 1`. Then `entries`
   holds exactly `limit` entries and `truncated` is true; `truncated` is false iff every entry
   was listed. `ignored`, `skipped` and `too_deep` count what the walk met before it stopped.

### Command `project.tree` (read-only, `MUTATING = false`)
Params `{"path"?: string, "limit"?: integer 1..=1000000 (default 50000)}`, unknown keys refused
(`invalid_params`, as other commands). Result:
`{"entries":[{"kind":"file","path":"src/main.rs"}, …], "ignored": n, "skipped": n, "too_deep": n, "truncated": bool}`
with kinds `file`, `dir`, `symlink`, `other`. Refusals (`rejected`):
`invalid_path` (the `path` fails the lexical `RelPath` check, before the file system),
`outside_root`, `not_found`, `not_a_file` (the `path`'s location is a file or special file),
`io_error`. All are existing kinds; there is no new one, and no `reason.rs` change: the kind
comes from the existing `From<&VfsError> for RejectReason`. The message names the path as
given and never a host path; its wording is not pinned in any test; for `not_a_file` it is
`cannot list <path>: not a directory` (Q-4), with no new `VfsError` variant. At least one `Example` (run by the
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
  followed), deep nesting (rule 5). Bounded by `limit`: the number of listed
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
  implementer to copy (wording may be joined into paragraphs, not widened). Each carries its
  evidence. No sentence states a time or a speed for a walk unless the implementer measured it
  on the built `kiln-cli` and recorded the command and output in its log (D-095, Q-1); the
  spec check's cost figures (P11, P14) are a model, not a measurement of Kiln.
  - S1. "`project.tree` reads directories and `.gitignore` files and nothing else; it never
    writes, creates or changes a file." Evidence: AC-4 (outside files byte-identical), AC-1
    (trees unchanged); reviewer's `grep` for write calls.
  - S2. "On disk, the `path` is located as `docs/protocol/paths.md` describes: a `path` whose
    location is outside the root is refused with `outside_root`, and nothing outside the root
    is listed or read." Evidence: AC-4.
  - S3. "Symbolic links found in the project are listed with kind `symlink` and never followed,
    whatever they point to; FIFOs, sockets and devices are listed with kind `other` and never
    opened, so they cannot make the command wait." Evidence: AC-4 (FIFO, loop, links to `.`
    and outside; its 5 s timeout is a hang detector, not a speed claim).
  - S4 (Q-2). "A `.gitignore` is read only when it is a regular file; a `.gitignore` that is a
    symbolic link, a FIFO or a directory is listed and contributes no patterns."
    Evidence: AC-4; git 2.43 also ignores an in-tree `.gitignore` symlink (probe P6).
  - S5. "For patterns of the forms `*.x`, `/a`, `b/`, `!c`, `**/d` and `e/**`, in `.gitignore`
    files at any depth, the `file` and `symlink` entries are exactly the paths
    `git ls-files --cached --others --exclude-standard` lists in a fresh repository of the same
    directory with no global or system configuration, apart from the root's `target`
    directory." Evidence: AC-1.
  - S6 (Q-5). Known differences from git, written as limits: "`{a,b}` in a pattern matches `a`
    and `b` (git matches the literal name `{a,b}`); a trailing tab is removed from a pattern
    (git keeps it); a directory that holds its own `.git` is listed with its contents (git
    lists it as one entry `dir/`); names that are not UTF-8 or contain `\` are not listed and
    are counted in `skipped` (git lists them)." Each difference goes into the document only
    when the implementer shows it on Kiln, by a test in `crates/kiln-workspace/tests/project.rs`
    or by a probe (the `kiln-cli` command, its output, and `git ls-files -z` on the same
    directory), recorded in its log. A bullet the test or probe does not confirm is dropped; a
    further difference the implementer finds is added the same way. The names case is also
    shown by AC-7. Spec-check evidence, git side only: probes P2, P4, P8, P9; Kiln side, by
    reading the source only: `globset` 0.4.21 always expands alternates, `add_line` trims with
    `trim_right` (`gitignore.rs`:470–475), rule 2 skips only the `.git` entry, `RelPath::new`
    refuses `\`.
  - S7. "`limit` bounds the number of entries listed, not the work: each directory visited is
    read whole and sorted before any rule applies, and ignored entries do not count toward
    `limit`." Evidence: by construction (rule order (c)); AC-5. A response size may be added
    only from the implementer's measurement of `kiln-cli exec project.tree` output (P14 is a
    Python model of the JSON).
  - S8 (Q-1). "A directory whose path has 128 segments is listed but not descended, and is
    counted in `too_deep`." Evidence: AC-6. The implementer measures the walk of AC-6's two
    chains on the built `kiln-cli` (debug build, `time`) and records the commands and times in
    its log; no time goes into the document unless measured so.
  - S9. "A directory whose location on the host has a path of 4,096 bytes or more cannot be
    listed, and the command fails with `io_error`; whether that happens depends on how long the
    root's own host path is." Evidence: probe P10 (`doc.open` of a directory at an absolute
    path of 4,094 bytes → `not_a_file`, of 4,096 bytes → `io_error`; PATH_MAX 4096). That probe
    is of `doc.open`; the implementer repeats it with `project.tree {"path": …}` on the built
    `kiln-cli` and records the command and output before copying the sentence.
  - S10. "A directory or a `.gitignore` that cannot be read (for example for lack of
    permission) makes the whole command fail with `io_error`." Evidence: not testable in the
    gate (uid 0, Pinned (e)); the implementer shows it by a probe as an unprivileged user
    (T-036's identity U) and records the command and output, and the reviewer repeats it. If
    neither can run it, the sentence is left out and the reviewer records that.
  - S11. "The listing is not a snapshot: if another process changes the tree while the command
    runs, the result can mix the states before and after, and the residual limits of
    `docs/protocol/paths.md` apply to each directory read." Evidence: by construction (each
    `list_dir` locates its directory again, T-008 "nothing is cached").
  - S12. "Entries are in preorder, each directory's entries in byte order of their names; no
    case folding." Evidence: AC-2.
  - S13, `docs/protocol/paths.md` (Q-6; its first sentence only, replaced by): "Every `path` in
    a request names an entry below the root (`--root`, or the current directory): a file for
    `doc.open` and for the file a `doc.save` writes, a directory for `project.tree`
    (`docs/protocol/project.md`)." Evidence: AC-4 (`outside_root` for `project.tree`), AC-5
    (`not_a_file` for a file). The rest of `paths.md` is unchanged; its rules apply to the
    `path` of `project.tree` as they stand (S2).

## Acceptance criteria
Acceptance-test protocol (high tier, METHODOLOGY §4, §6): an **independent test author** writes
`apps/kiln-cli/tests/acceptance_t019_cli.rs`, `crates/kiln-workspace/tests/acceptance_t019_workspace.rs`
and `apps/kiln-cli/tests/guard_t019_cli.rs` on `task/T-019-tests`, from this spec only, before
the first join. The implementer adds no `acceptance_t*` or `guard_t*` file and nothing under
`scorecard/`; its own tests go in `crates/kiln-workspace/tests/project.rs` and unit tests.
Every acceptance test **fails on base** (main without this task) by assertion or command
error: on base `kiln-cli exec project.tree` exits 2 with `unknown_command` (probe P0), so each
CLI test asserting `ok` fails by assertion. AC-7 drives new Rust API and fails on base only by
compiling, which is weak evidence (§6), made up for by the reviewer's mutation spot-check. The
guards **pass on base** and on the joined branch. The test author runs its tests against a
naive stub and ≥ 5 seeded bugs and lists the kills (§6). All CLI calls run under a timeout;
fixtures are built in fresh temporary directories.

- AC-1 (`ws.open-project`, CLI, git oracle; `acceptance_t019_cli.rs`, fails on base): for ≥ 10 hand-built fixture trees and ≥ 100
  fixed-seed random trees (a hand-written PRNG in the test; no new dev-dependency) with random
  `.gitignore` files: copy each to a temporary directory and run `git init`. Then run
  `git ls-files -z --cached --others --exclude-standard` with `GIT_CONFIG_GLOBAL=/dev/null`,
  `GIT_CONFIG_NOSYSTEM=1`, `-c core.excludesFile=` and a 30 s timeout. That set, minus paths
  under the root `target/`, equals the set of `file` and `symlink` paths of
  `kiln-cli exec project.tree '{}' --root <dir>`. Generator limits, outside which git and this
  task differ (S6, Pinned (e)): names from `[a-e0-9._-]` (no `\`, no braces, valid UTF-8), no
  name `.git`, patterns only of the forms of S5 (no braces, tabs, BOM or trailing spaces),
  no pattern that matches the name `.gitignore` (Q-8), depth below 128, no FIFO, and any
  symlink pointing inside the tree. Oracle: git.
- AC-2 (`ws.open-project`, order and counts; `acceptance_t019_cli.rs`, fails on base): the worked example (Pinned (g)) gives exactly the
  stated result. For the random trees of AC-1, the entries are the naive model's preorder with
  byte-sorted siblings, and `ignored` equals the model's count. Oracle: naive model + exact
  value.
- AC-3 (`ws.open-project`, memory = disk; `acceptance_t019_cli.rs`, fails on base): the same trees given as a `--memory-seed` manifest
  (`docs/protocol/cli-memory-seed.md`) and as a `--root` directory give identical
  `project.tree` results through `kiln-cli exec`, apart from directories without files, which
  do not exist in memory. Oracle: differential.
- AC-4 (`ws.open-project`, confinement, CLI; `acceptance_t019_cli.rs`, fails on base): a root holding `out -> ../outside` (a real directory holding a real
  file and a `.gitignore` containing `*`; the test first asserts both exist), `self -> .`,
  `loop1 -> loop2`, `loop2 -> loop1`, a FIFO `p`, and a `.gitignore` that is a symlink to an
  outside file containing `*`, plus a regular file `keep.rs`, gives:
  - `project.tree {}` lists `.gitignore`, `keep.rs`, `loop1`, `loop2`, `out`, `p`, `self` with
    kinds `symlink`, `file`, `symlink`, `symlink`, `symlink`, `other`, `symlink`, and nothing
    under any of them (so neither outside `*` pattern had an effect, Q-2);
  - `project.tree {"path":"out"}` → `rejected` / `outside_root`;
  - `project.tree {"path":"self"}` → the same seven entries, each path beginning `self/`
    (`self/.gitignore`, `self/keep.rs`, …, `self/self`), with the same kinds (Q-3);
  - each call finishes before a 5 s timeout (a hang detector for the FIFO and the loop, not a
    speed claim), and the outside files are byte-identical afterwards.
  Oracle: exact value.
- AC-5 (`ws.open-project`, limits and refusals; `acceptance_t019_cli.rs`, fails on base): on the
  worked example, `limit: 3` gives the first 3 entries and `truncated: true`; `limit: 8` gives
  all 8 and `truncated: false` (Q-7).
  `path: "a.rs"` → `not_a_file`, `path: "nope"` → `not_found`, `path: "../x"` →
  `invalid_path`, `path: "a"` → `a/b.rs` only. `limit: 0` → `invalid_params`. Oracle: exact
  value.
- AC-6 (`ws.open-project`, depth; `acceptance_t019_cli.rs`, fails on base): a chain of 130
  nested directories `d/d/…/d` with a file at the bottom gives 128 `dir` entries, `too_deep: 1`
  and no `file` entry. A chain of 2,100 nested directories gives `ok`, 128 `dir` entries and
  `too_deep: 1` (not `io_error`: the walk never reaches a 4,096-byte path). Each call runs
  under the test's ordinary process timeout of 30 s (AGENTS.md: every spawned process has
  one), which only detects a hang and states no speed. Oracle: exact value.
- AC-7 (Rust API, extra; `acceptance_t019_workspace.rs`, fails on base by compiling only): `Vfs::list_dir` on `MemoryFs` and `DiskFs` gives byte-sorted
  entries with their kinds; on disk a name containing `\` and a name that is not UTF-8 are
  omitted and counted in `skipped`; `list_dir` of a file → `NotAFile`, of a missing path →
  `NotFound`, of a link to an outside directory → `OutsideRoot`. Oracle: exact value.
- Guards (`guard_t019_cli.rs`, pass on base and joined): on AC-4's fixture, `doc.open` keeps
  T-008's results: `out/<outside file>` → `outside_root`, `self/keep.rs` → ok, `p` →
  `not_a_file` within the 5 s hang timeout, `loop1` → `io_error`; and `doc.open` of a
  directory still gives `not_a_file`. They show that adding `list_dir` left the open path
  confined. They assert kinds only, no message text (Pinned (f)).

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| ws.open-project | `kiln_cli::acceptance_t019_cli::tree_equals_git_ls_files` (AC-1), `kiln_cli::acceptance_t019_cli::tree_order_and_counts` (AC-2), `kiln_cli::acceptance_t019_cli::memory_and_disk_trees_agree` (AC-3), `kiln_cli::acceptance_t019_cli::symlinks_are_listed_not_followed` (AC-4), `kiln_cli::acceptance_t019_cli::limits_and_refusals` (AC-5), `kiln_cli::acceptance_t019_cli::deep_trees_stop_at_the_depth_bound` (AC-6) |

Names are intended; the test author may rename and lists the final names in its log. AC-7 and
the guards are not tied to the item.

The test author lists the tests in its log; the director ties them to the item at the join
(METHODOLOGY §6). The test author does not edit `scorecard/`.

## Out of scope and implementer notes
- Watching for changes; caching the tree; `.ignore`/global excludes/`.git/info/exclude`;
  Windows; find-in-files (T-022); declaring `project.tree`'s refusal kinds (T-038, after this
  task, D-092). The project is the session root: there is no `project.open` command.
- No `reason.rs` or `VfsError` variant change (Q-4), no `kiln-control` or `kiln-cli`
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

### Questions for the director (R-156; ruled in D-095 below, and the sections above follow the rulings)
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
- Q-8 (director, D-097): yes, a `.gitignore` is read even when a pattern ignores it. Measured by the director with git 2.43.0 on 2026-11-13: in a repository whose root `.gitignore` holds `.*` and whose `sub/.gitignore` holds `secret.txt`, `git ls-files -o --exclude-standard -z` lists `sub/keep.txt` and `top.txt` and not `sub/secret.txt`, and `git check-ignore -v sub/secret.txt` names `sub/.gitignore:1`. The implementer need not repeat the probe; the test author may now generate patterns that match the name `.gitignore`, and the listing itself omits an ignored `.gitignore` like any other ignored entry.
### Rulings on the implementer's questions of R-163 (director, D-100; answered to both builders)
- y1 (S1). The implementer found S1 untrue: while the ignore matcher is built, the Rust standard library asks the operating system for the number of available CPUs, which on Linux reads fixed system files under `/proc` and `/sys` (seen under `strace`). The dependency stays. S1 becomes: "`project.tree` reads the project through directory listings and `.gitignore` files inside the root, and never writes, creates or changes a file. The only other files the command reads are fixed system files that the Rust standard library reads once per process to learn how many CPUs are available; the project's content cannot choose them." Evidence the implementer records before copying it: the list of every path opened or read between the request and the response under `strace`, each either inside the root or one of those system files. If the list holds anything else, the sentence is not copied and the implementer reports the list. The "Dependency rule" is read with the same exception.
- y2 (Pinned (b)). The lock gains 11 packages, not 9: the two extra (`winapi-util`, `windows-sys`) are Windows-only dependencies of the same crate. Accepted; no locked version changes. The approval row at the merge states the count.
- y3 (a `.gitignore` whose patterns cannot be compiled together). Behaviour: such a `.gitignore` contributes no patterns and the listing goes on, as for a `.gitignore` that is not a regular file (Q-2); one unusable ignore file must not make a project unlistable. The implementer first tries to produce the case (very many patterns, very long patterns; every probe under a timeout). If it can be produced, an integration test in `tests/project.rs` shows the behaviour and `project.md` gains: "A `.gitignore` whose patterns cannot be compiled contributes no patterns." If it cannot be produced, the code still behaves so and the document says nothing about it.
- y4 (two readings, confirmed). A `path` that already has 128 or more segments descends no directory. A directory at the depth bound that is met as the entry after the last one returned (the entry that makes `truncated` true) is counted in `too_deep`, because the depth rule is applied before the limit rule (Pinned (c)); Q-7's "what was walked before the stop" includes that entry.
- y5 (S1 again; director, D-102). The sentence y1 gave is untrue too: under `strace` the command also calls `readlink` on each directory above the root, from the check that the root is still the same directory (T-008). The director wrote y1's list of what the command reads without the evidence, which is the error v1.2 §2.1 names. S1 is reduced to the half that is shown: "`project.tree` never writes, creates or changes a file." Evidence, already recorded by the implementer in R-163 and R-168: no write, create, mkdir, unlink or rename call between request and response under `strace`, and AC-4's outside files byte-identical. No sentence in the document lists what the command reads; S2 to S4 state the confinement. The "Dependency rule" stands as written for the `ignore` crate's own items, and the CPU-count read is recorded here as known and accepted.
### Rulings on review round 1 (R-174, `reviews/T-019-r1.md`; director, D-105). The implementer acknowledges each before acting.
- y6 (finding 1, defect). The paragraph on differences from git says what was observed and does not claim to be complete: it states that it lists the differences found so far. It gains the three the reviewer showed, each written from a case the implementer runs with `kiln-cli` and `git ls-files -z` on one fixture and records: trailing whitespace is removed from a pattern, not only a trailing tab (observed for vertical tab, U+00A0 and U+3000; git keeps them); POSIX character classes such as `[[:digit:]]` are not supported; for the pattern `***/z` git ignores both `z` and `p/z` and Kiln does not ignore `z`. The implementer names only characters and patterns it ran.
- y7 (finding 3, spec gap). The cost of `.gitignore` files is not bounded by this task. `project.md` says so in one sentence without a figure: "No bound exists on the time or memory that `.gitignore` files can cost." A bound is part of the follow-up task on deterministic limits that the director files (D-101).
- y8 (finding 4, spec gap). The refusal stays: the walk reads the `.gitignore` of every directory on the `path`'s prefix (Q-3), so a `path` with a prefix directory located outside the root is refused with `outside_root`, even when the path as a whole leads back inside and `doc.open` of a file under it succeeds. `project.md` says exactly that, in one sentence the implementer writes after running the reviewer's case (`esc/back`), and the implementer's own integration tests in `tests/project.rs` gain that case, asserting the error variant and not a message.
- y9 (findings 2 and 5). The wording follows the binary: wherever the documents and the command's description say that the bound is "128 segments", they say that a directory whose path has 128 or more segments is listed and not descended into; and the description's "skips the root's `target` directory" becomes a wording that is true when the root is listed through a link to it (the entry skipped is the one whose path is exactly `target`). `docs/commands.json` is regenerated. Finding 6 (the default `limit` is covered only by the schema comparison) is accepted as it is.

### Open after R-162 (planner)
- Q-8 (found re-reading the rules as a builder): is a `.gitignore` read when a pattern ignores
  it (for example `.gitignore` or `.*` in the root's file, or `*` in a shallower one)? Options:
  (A) yes, read whenever it is a `File` in a visited directory, ignored or not (believed to be
  git's behaviour; not probed in this run, which ran no git); it is then not listed but its
  patterns apply; (B) no: an ignored `.gitignore` contributes nothing. Recommended: (A), after
  the implementer confirms git's behaviour by a probe (`git ls-files -z` on a fixture whose
  root `.gitignore` holds `.gitignore` and `*.log`, and a subdirectory `.gitignore` holding
  `!keep.log`) and records it; if git differs, follow git. Until ruled, AC-1's generator
  avoids patterns that match the name `.gitignore` (rule 3).

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
- 2026-11-13 · R-162 · draft · did: finalised on D-095 (Q-1…Q-7 as recommended): PENDING marks removed and the rulings written into the rules, Interface, Pinned (i) and ACs; Q-6's `paths.md` sentence written as S13. Q-1 addition: no document sentence states a walk time; S7 lost its response-size figure (P14 is a Python model), S8 now asks the implementer to measure AC-6's two chains on the built `kiln-cli` and record them; AC-4's 5 s and AC-6's 30 s are stated as hang detectors, not speed claims (AC-6's former "within 10 s" removed). Q-5 addition: each S6 difference enters the document only when a test in `tests/project.rs` or a recorded `kiln-cli` + `git ls-files -z` probe shows it; S9 and S10 likewise need the implementer's recorded probe. Acceptance-test protocol stated for high tier (independent test author writes `acceptance_t019_cli.rs`, `acceptance_t019_workspace.rs`, `guard_t019_cli.rs`; implementer adds none); each AC names its file and fail-on-base (AC-7 compile-only, made up by the mutation spot-check); guards defined (pass on base); scorecard names for AC-1…AC-6. Re-read top to bottom: found one open point, Q-8 (is an ignored `.gitignore` still read), held out of AC-1's generator until ruled. spec check: booked to iter-2. · gate attempts: 0 · not run: git, cargo (as instructed) · missing context: none · next: director rules Q-8 (or accepts the recommendation), then dispatch.
<!-- end planner log -->
### implementer
- 2026-11-13T22:01Z · R-163 · ready · rulings: Q-1 (A, bound 128), Q-2 (A), Q-3 (as recommended), Q-4 (A), Q-5 (A), Q-6 (yes), Q-7 (as recommended), Q-8 (read when ignored; D-097) and the dependency ruling all accepted and implemented as written; one Pinned sentence (S1) found untrue of the built binary (QUESTION 1), not reworded, left out of the document. · criteria in my words: `project.tree` lists the root (or the directory `path`) through `Vfs::list_dir` in preorder with byte-sorted siblings; skips the root's `target` dir and every `.git`; applies the `.gitignore` files of visited directories and of `path`'s prefix (regular files only, read through `Vfs::read`, read even when ignored); lists links and special files without following or opening them; stops descending at 128 segments; stops listing at `limit`; refuses with the existing kinds in T-008's precedence, the path named as given; memory and disk agree except for file-less directories. · did: `ignore = "=0.4.34"` (released 2026-10-27T18:31Z, `globset` 0.4.21 the same minute, both `rust_version` 1.88, from the crates.io API: 17 days old, so kept); `Vfs::list_dir` with `EntryKind`/`DirEntry`/`DirListing` (vfs.rs), `MemoryFs::list_dir` (memory.rs), `DiskFs::list_dir` (disk.rs: `locate` generalised to `locate_steps` so the root is located with label `(root)`; kinds from `DirEntry::file_type`, never followed; module docs); the walk `project::{tree, TreeEntry, ProjectTree, MAX_DEPTH}` (matcher items used: `GitignoreBuilder::{new, add_line, build}`, `Gitignore::matched` only, builder root `.`, `from: None`); command `project.tree` (`MUTATING = false`, `path` null refused, `limit` 1..=1,000,000 by schema, two examples on MemoryFs; `not_a_file` message `cannot list <path>: not a directory`); `docs/commands.json` by `cargo xtask schema --write`; `docs/protocol/project.md` (new) and the first sentence of `docs/protocol/paths.md` (S13); unit tests in memory.rs, project.rs (both crates) and integration tests `crates/kiln-workspace/tests/project.rs` (10 tests; no pinned message, document or command list). `Cargo.lock`: 11 packages added, none removed, no locked version changed (`git diff Cargo.lock` has no `-` line): the 9 of Pinned (b) plus `winapi-util` 0.1.11 and `windows-sys` 0.61.2 (`cfg(windows)` dependencies of `ignore`/`same-file`/`walkdir`, never compiled here; see QUESTION 2). The build fetched them from the registry without error. · gate attempts: 2 (1: red `clippy`, `unwrap` in test helpers outside `#[cfg(test)]`; 2: green, `GATE PASSED in 282.6s (11 steps; 2 EMPTY: fuzz, budgets)`, `test 712 passed, 0 failed, 0 ignored in 87 suites`). · not run: the test author's acceptance and guard tests (not seen); a concurrent-modification probe for S11; a build of `Gitignore` failing on a huge pattern file (mapped to `io_error` naming the `.gitignore`, kind `InvalidData`; QUESTION 3); a non-Linux host. · evidence per document sentence (probes on `target/debug/kiln-cli` built from this branch, fixtures under a scratch dir `$W`, run as uid 0 unless said, all removed afterwards; `gitls` = `git init -q` then `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 timeout 30 git -c core.excludesFile= ls-files -z --cached --others --exclude-standard`):
  - S1 LEFT OUT (QUESTION 1). `strace -f -e trace=%file,%desc kiln-cli exec project.tree '{}' --root ex` (the worked example) shows, besides the root, its directories and `.gitignore`, `openat("/proc/self/cgroup")`, `statx("/sys/fs/cgroup/cpu/")`, `openat(".../cpu.cfs_quota_us")`, `openat(".../cpu.cfs_period_us")`, right after the `.gitignore` read; no `O_WRONLY|O_RDWR|O_CREAT`, `mkdir`, `unlink` or `rename` (count 0). Cause: `GitignoreBuilder::build` (an allowed item) calls regex-automata's `Pool::with_available_parallelism_capacity` → `std::thread::available_parallelism`, which reads the cgroup CPU quota (once per process). The "never writes, creates or changes" half holds.
  - S2 copied. `project.tree {"path":"out"}` (`out -> ../outside`) → `{"code":"rejected","kind":"outside_root","message":"cannot list the project: out: resolves outside the root"}` exit 2; `{"path":"out/f.rs"}` → `outside_root`; `{}` lists `out` as `symlink` and nothing under it; `sha1sum` of the outside files equal before and after.
  - S3 copied. AC-4 fixture, `timeout 5 kiln-cli exec project.tree '{}' --root ac4/root` → `.gitignore` symlink, `keep.rs` file, `loop1`, `loop2`, `out` symlink, `p` other, `self` symlink, `ignored 0`; `{"path":"self"}` → the same seven under `self/`; `{"path":"p"}` → `not_a_file` (`cannot list p: not a directory`); `{"path":"loop1"}` → `io_error` `too many levels of symbolic links`; each well under the 5 s timeout. Also `tests/project.rs` `disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names` (socket and FIFO `Other`, dangling link and loops `Symlink`).
  - S4 copied. The `.gitignore -> ../outside/ign` (`*`) had no effect (above); `g1/.gitignore` a FIFO with `g1/x.log` → `.gitignore` other, `x.log` file; `g2/.gitignore/` a directory holding `inner` = `*` → `.gitignore` dir, `.gitignore/inner` file, `x.log` file.
  - S5 copied. Fixture with root `.gitignore` `*.x`, `/a`, `b/`, `!c.x`, `**/d`, `e/**`, `s/.gitignore` `!m.x`, `/q`, `keep`, `s/t/.gitignore` `!/a`, `!keep`, `*.txt`, the same entries at depths 0, 1, 2 plus symlinks and a root `target/`: kiln `file`+`symlink` paths (23) `diff` `gitls` minus `target/` (23) → equal. Plus a randomized probe (Python, seeds 0–299, names from `[a-e0-9._-]`, patterns `*.x`, `/n`, `n/`, `!n`, `**/n`, `n/**`, `n`, `!*.x`, `*`, depth ≤ 5, in-tree symlinks): `trees 300 mismatches 0`.
  - S6 copied, all four bullets shown, each also by `tests/project.rs` `documented_differences_from_git`: `br/.gitignore` `{a,b}` with `a`, `b`, `{a,b}` → kiln lists `.gitignore`, `{a,b}` (`ignored 2`), git `.gitignore|a|b|`; `tab/.gitignore` `*.log\t` → kiln `.gitignore` only (`ignored 1`), git `.gitignore|x.log|`; `nest/sub` made by `git init -q sub` plus `sub/f`, `top` → kiln `sub` dir, `sub/f`, `top` (`ignored 1`), git `sub/|top|`; `odd/` with `b\s`, `n\xff`, `ok` → kiln `ok` only, `skipped 2`, git `b\s|n\xff|ok|`. No further difference found.
  - S7 copied (no response size). `limit:3` on the worked example → first 3, `truncated:true`; `limit:8` → all 8, `false`; `.gitignore` `*.log` with `1.log 2.log 3.log z`, `limit:2` → `.gitignore`, `z`, `ignored 3`, `truncated false`. Measured but not documented: 60,000 files via `--memory-seed`, `{}` → 1,700,060 bytes of output, 0.87 s.
  - S8 copied, no time stated. Chains of 130 and 2,100 `d` directories with a file at the bottom, `/usr/bin/time timeout 30 kiln-cli exec project.tree '{}' --root cN` (debug build), 3 runs each: c130 → 128 dirs, 0 files, `too_deep 1`, 0.04/0.04/0.05 s; c2100 → `ok`, 128 dirs, `too_deep 1`, 0.04/0.05/0.04 s; max RSS ≈ 15.5 MB.
  - S9 copied. Root host path 79 bytes; directories at absolute host path lengths 4,094 / 4,095 / 4,096 / 4,100, `project.tree {"path": <rel>}` → `ok` (empty) / `ok` / `io_error` / `io_error`; `{}` from the root, which reaches the 4,096 one → `io_error`. Messages name the relative path only (truncated by the CLI with `…`).
  - S10 copied. As uid 65534 (`setpriv --reuid=65534 --regid=65534 --clear-groups timeout 5 kiln-cli exec project.tree …`, fixtures chowned to 65534): `r1/locked` mode 000 → `{}` gives `io_error` `locked: I/O error: Permission denied (os error 13)`; `{"path":"locked"}` the same; `r2/.gitignore` mode 000 → `io_error` `.gitignore: … Permission denied`. As uid 0 both trees list normally.
  - S11 copied, by construction: the strace above shows a `statx` of the root path before each directory's `openat(O_DIRECTORY)` (each `list_dir` locates its directory again; nothing cached). No race was run.
  - S12 copied. `A/`, `B`, `_`, `a/`, `a-`, `a.rs`, `b`, `é` → `A`, `A/y`, `B`, `_`, `a`, `a/x`, `a-`, `a.rs`, `b`, `é`; worked example `a/b.rs` before `a.rs`.
  - S13 copied into `paths.md`: `{"path":"out"}` → `outside_root` (S2), `{"path":"a.rs"}` → `not_a_file`.
  - Sentences I added beyond Pinned (i), with evidence: the memory paragraph (`--memory-seed` of `d/f`, `g` → `d` dir, `d/f`, `g` file; `--memory` `{}` → empty, ok); the request paragraph (`{"path":null}`, `{"x":1}`, `{"limit":0}` → `invalid_params`; unit test `refusals_carry_existing_kinds` also `1000001` refused, `1000000` ok); walk rules 1–6 are the spec's rules restated.
  · missing context: none · next: director rules QUESTIONs 1–3, then join with `task/T-019-tests`.
  QUESTION 1 (S1 untrue): `project.tree` reads `/proc/self/cgroup` and `/sys/fs/cgroup/cpu/cpu.cfs_{quota,period}_us` once per process, through the allowed `GitignoreBuilder::build` (regex-automata 0.4.18 `util/pool.rs`:193–205 → `std::thread::available_parallelism`). Fixed system paths, read-only, not influenced by the project, but outside the VFS and contrary to S1 and to "every other item that touches the host file system or the environment is forbidden". Options: (A) amend S1 to name this read; (B) forbid `build` and drop `ignore` (hand-written matcher); (C) accept as is, S1 left out. S1 is not in the document until ruled.
  QUESTION 2 (Pinned (b) list): the lock also gained `winapi-util` 0.1.11 and `windows-sys` 0.61.2 (Windows-only dependencies; a lock file lists every target). New packages only, no version change; does the approval row need them named?
  QUESTION 3 (not in spec): if `GitignoreBuilder::build` fails (a pattern set too large for the regex size limit), I fail the command with `io_error` naming that `.gitignore`. Alternative: treat it as contributing no patterns. Not probed. Also two edge choices, made by the letter of the spec: a `path` already at ≥ 128 segments descends no `Dir` entry (rule 5 read as "≥ 128"); a too-deep directory met as entry `limit + 1` is counted in `too_deep` (Pinned (c): rule 5 before rule 6).
- 2026-11-13T22:14Z · R-168 · ready (rulings y1–y4, D-100, read in the primary checkout's copy of this file; this branch's copy does not hold them) · rulings: y1 accepted (dependency stays; S1 restated; copied only if the strace list holds nothing but in-root paths and the CPU-count system files); y2 accepted (11 packages; nothing to do); y3 accepted and implemented; y4 accepted, the code already does both readings (`>= MAX_DEPTH` in `visit_entries`; the rule-5 count comes before the rule-6 stop), no change. · did:
  - y1, probe: fixture in `target/y1probe` (worked example plus `src/.gitignore` `!keep.log`, `src/keep.log`, `out -> ../outside` whose `.gitignore` is `*`, FIFO `p`). `timeout 30 strace -f -qq -tt -e trace=%file,%desc -o st.trace kiln-cli control --root root < req.jsonl`, with requests 1 `buffer.text`, 2 `project.tree {}`, 3 `project.tree {"path":"src"}`, all `ok`. Every path named by a syscall between the write of response 1 and the write of response 3 (trace lines 53–210; syscalls: openat 12, statx 26, getdents64 10, read 16, readlink 72, fstat 5, lseek 3, close 12), counted: `<root>` 20, `<root>/.gitignore` 4, `<root>/a` 2, `<root>/src` 6, `<root>/src/.gitignore` 4, `/proc/self/cgroup` 1, `/sys/fs/cgroup/cpu/` 1, `/sys/fs/cgroup/cpu/cpu.cfs_quota_us` 1, `/sys/fs/cgroup/cpu/cpu.cfs_period_us` 1, `""` 7 (`statx`/`read` on an fd already opened). It also holds, 9 times each, `readlink` (result `EINVAL`, not links) of every ancestor of the root: `/home`, `/home/user`, `/home/user/photocraft-study-review`, `…/.worktrees`, `…/.worktrees/T-019`, `…/T-019/target`, `…/target/y1probe`. These come from `fs::canonicalize` of the root path in `DiskFs::current_root` (T-008's identity check of the root before each lookup: glibc `realpath` does a readlink on each component). `doc.open` does the same. Nothing under `outside/` and no FIFO open appear. The list holds something other than in-root paths and CPU-count files, so S1 is NOT copied (y1). The strace was run once; the fixture was removed afterwards.
  - y3, probe (fixtures in `target/y3probe`, each call under `timeout 60`, removed afterwards). Before the change: `N` lines `*a<i>*b*[c-x]?d*e` at the root, `kiln-cli exec project.tree '{}'`: N = 1,000 / 5,000 / 8,000 → `ok` (0.09 / 0.42 / 0.61 s); N = 12,000 / 20,000 (389 KB) → `io_error` `.gitignore: … the patterns cannot be used: error building NFA` (globset 0.4.21 `lib.rs`:287–299, NFA size limit 10 MiB). One long pattern (`a*a*…`, 10,000 and 100,000 bytes) → `ok`, so length alone did not produce it. Changed `parse_gitignore` to return `Option<Gitignore>` (`build().ok()`); `push_gitignore` pushes no layer on `None` (no new `ignore` item used, `Gitignore::empty` not used). After the change, with `*.log` appended and an `x.log`: N = 8,000 → `x.log` ignored (`ignored 1`); N = 20,000 → `x.log` listed (`ignored 0`), exit 0. Added the integration test `a_gitignore_that_cannot_be_compiled_contributes_no_patterns` in `tests/project.rs` (MemoryFs, 100 vs 20,000 patterns; a subdirectory's `.gitignore` still applies). It would fail on the R-163 code (the `unwrap` of an `Io` result). Added the ruling's sentence to `docs/protocol/project.md` after the S4 paragraph, and a module-doc line.
  · gate attempts: 2, both green. 1, after the code and test: `GATE PASSED in 236.1s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)`, test `713 passed, 0 failed, 0 ignored in 87 suites`. 2, after the document sentence: `GATE PASSED in 243.6s (…)`, the same counts. · not run: strace on a root whose path has no ancestors other than `/`; the test author's tests. · missing context: the rulings y1–y4 are not on this branch's task file; I read them in the primary checkout and did not copy them here (that section is the director's). · next: director (join).
  QUESTION 4 (y1): the only outside-root paths besides the CPU-count files are `readlink` calls on the root's own ancestors, which the root check does on each lookup. Is S1 to be restated again to cover them (for example "…and the directories above the root, whose symbolic links it resolves to check that the root is still the same directory"), or does it stay out?
- 2026-11-13T22:20Z · R-171 · ready (ruling y5, D-102, read in the primary checkout's copy) · rulings: y5 accepted (y1's sentence withdrawn; S1 reduced to its shown half). · did: copied "`project.tree` never writes, creates or changes a file." as the first paragraph of "Guarantees and limits" in `docs/protocol/project.md`. Re-read the recorded evidence: the R-163 trace of `kiln-cli exec project.tree '{}'` (whole process) had 0 calls matching `O_WRONLY|O_RDWR|O_CREAT|mkdir|unlink|rename`; the R-168 trace between the responses held only openat, statx, getdents64, read, readlink, fstat, lseek, close and the 2 `write(1, …)` calls of the responses on stdout; the outside files were byte-identical (R-163 S2 probe). No call that creates or changes anything. No sentence lists what the command reads. · checks: `cargo fmt --check` clean; `cargo xtask hygiene` PASS. · gate attempts: 0 (no full gate, as instructed; the join runs it) · not run: a fresh strace (the traces were removed after R-163/R-168) · missing context: none · next: director (join).
- 2026-11-13T22:56Z · R-176 · review round 1 request-changes (`reviews/T-019-r1.md`; rulings y6–y9, D-105, read in the primary checkout's copy) · rulings: y6, y7, y8, y9 accepted as written; no behaviour change. · did (probes on `target/debug/kiln-cli` of this branch; fixtures in `target/r176`, removed afterwards; git side `git init -q`, `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 timeout 30 git -c core.excludesFile= ls-files -z --cached --others --exclude-standard`):
  - y6, one fixture with subdirectories: `vt/.gitignore` `x<VT>` with files `x`, `x<VT>`; `nbsp/.gitignore` `a<U+00A0>` with `a`, `a<U+00A0>`; `ideo/.gitignore` `q<U+3000>` with `q`, `q<U+3000>`; `cls/.gitignore` `[[:digit:]].x` with `1.x`, `a.x`; `star/.gitignore` `***/z` with `z`, `p/z`. `kiln-cli exec project.tree '{}'` files: `cls/.gitignore cls/1.x cls/a.x ideo/.gitignore ideo/q<U+3000> nbsp/.gitignore nbsp/a<U+00A0> star/.gitignore star/z vt/.gitignore vt/x<VT>`, `ignored 4`. git -z: `cls/.gitignore cls/a.x ideo/.gitignore ideo/q nbsp/.gitignore nbsp/a star/.gitignore vt/.gitignore vt/x`. So Kiln trims the trailing VT, U+00A0 and U+3000 (git keeps them), does not support `[[:digit:]]`, and does not ignore `z` for `***/z` while git ignores `z` and `p/z`. The tab case is R-163's probe. The paragraph now opens "Differences from git found so far (this list is not complete)" and names only these characters and patterns.
  - y7: added the ruling's sentence verbatim after the `limit` paragraph (no figure; the ruling places the bound in a follow-up task).
  - y8: `root/esc -> ../outside`, `outside/back -> ../root/real`, `root/real/f`: `project.tree {"path":"esc/back"}` → `rejected`/`outside_root` ("esc: resolves outside the root"), exit 2; `doc.open {"path":"esc/back/f"}` → ok; `project.tree {"path":"real"}` → `real/f`. Sentence added after the S2 paragraph. New integration test `a_prefix_directory_outside_the_root_refuses_the_path` (`DiskFs::read("esc/back/f")` ok, `list_dir("esc/back")` lists `f`, `tree(.., Some("esc/back"), ..)` is `Err(VfsError::OutsideRoot { .. })`; no message asserted).
  - y9: a 130-deep chain, `path` of 127/128/129 segments → one `dir` of 128/129/130 segments each, `too_deep 1`; `{}` → 128 entries, deepest 128 segments, `too_deep 1`. With `self -> .` and `target/x/f`, `{"path":"self"}` lists `self/target`, `self/target/x`, `self/target/x/f` (`ignored 0`). Reworded rule 5, the S8 paragraph, the `too_deep` and `ignored` result descriptions and the command DOC ("skips the directory whose path is exactly `target`", "lists a directory whose path has 128 or more segments without descending into it"); `docs/commands.json` regenerated by `cargo xtask schema --write`. S5's "apart from the root's `target` directory" is Pinned and about `{}`, so I left it as is. Finding 6: nothing, as ruled.
  · gate attempts: 1, green: `GATE PASSED in 266.9s (11 steps; 2 step(s) EMPTY (nothing registered): fuzz, budgets)`, test `728 passed, 0 failed, 0 ignored in 90 suites` (the acceptance and guard tests are included). · not run: nothing else · missing context: rulings y6–y9 are only in the primary checkout's copy · next: reviewer round 2.
<!-- end implementer log -->
### test author
- 2026-11-13T22:05Z · R-164 · ready · did: ACKNOWLEDGED, all accepted, none questioned: D-095 Q-1 (bound 128, `too_deep`; AC-6), Q-2 (`.gitignore` read only as `File`; AC-4), Q-3 (in-root link `path` followed, entries under the path as given, prefix `.gitignore`s apply, `path` not listed or subject to rules 2–3; AC-4 `self`, `tree_under_a_path_applies_prefix_gitignores`), Q-4 (`not_a_file` for listing a file; kind only), Q-5 (generator kept to S5's forms: no braces, tabs, BOM, trailing spaces), Q-6 (doc only, no test), Q-7 (`truncated` iff an entry beyond `limit`; counts cover what was met before the stop; AC-5); D-097 Q-8 (generator now emits `*.gitignore`, `/.gitignore`, `**/.gitignore`, `!.gitignore`; hand tree `q8-ignored-gitignore-still-read`); D-100 y3 (no test: none assumes a failure for an uncompilable `.gitignore`), y4 (i) a `path` of 128 segments descends no directory, (ii) the bound directory that makes `truncated` true is counted in `too_deep`: both now asserted in AC-6 (`limit` 127/128, `path` of 127/128 segments, disk and memory); y1, y2: no test reads outside-root opens or `Cargo.lock`. Wrote `apps/kiln-cli/tests/acceptance_t019_cli.rs`, `apps/kiln-cli/tests/guard_t019_cli.rs`, `crates/kiln-workspace/tests/acceptance_t019_workspace.rs`. ws.open-project → AC-1 `kiln_cli::acceptance_t019_cli::tree_equals_git_ls_files`; AC-2 `…::tree_order_and_counts`; AC-3 `…::memory_and_disk_trees_agree`; AC-4 `…::symlinks_are_listed_not_followed`; AC-5 `…::limits_and_refusals`, `…::tree_under_a_path_applies_prefix_gitignores`; AC-6 `…::deep_trees_stop_at_the_depth_bound`. Not tied to the item: AC-7 `kiln_workspace::acceptance_t019_workspace::{memory_list_dir_sorts_by_bytes_and_reports_kinds, disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names, disk_root_errors_carry_the_root_placeholder, tree_gives_the_worked_example, tree_counts_skipped_names_on_disk}`; guards `kiln_cli::guard_t019_cli::{doc_open_stays_confined_on_the_tree_fixture, doc_open_of_a_directory_is_still_not_a_file}`. Oracles: AC-1 git (`git ls-files -z --cached --others --exclude-standard`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, `-c core.excludesFile=`, run after `git init`) on 13 hand trees and 120 SplitMix64 trees with fixed seeds; the test's naive model is asserted equal to git on every tree before Kiln is asked. AC-2 is the model plus Pinned (g)'s exact values (exec and `run`). AC-3 is a differential of `--memory-seed` against `--root`. AC-4 to AC-7 use exact values. On base: guards 2/2 pass; all 7 CLI acceptance tests fail by assertion: `project.tree` is refused with `unknown_command` (AC-3's assertion is `code` `unknown_command` ≠ `rejected`); AC-7 fails by compile (`list_dir`, `DirEntry`, `project::tree` missing): compile. Self-check: a scratch stub (vfs/memory/disk `list_dir`, a walk using `ignore` 0.4.34's matcher, the `project.tree` command) passed all 14 tests; it was then reverted, nothing committed. With the real matcher, AC-1 held on all 133 trees. The model agreed with git on every tree; across the trees there were 237 ignored entries, 189 links and 58 ignored `.gitignore` files. 16 seeded bugs, all killed: B1 case-folded sort → AC-2, AC-3, AC-5, AC-7; B2 `.git` not skipped → AC-1, AC-2; B3 every `target` skipped → AC-1, AC-2, AC-3; B4 `truncated` when len == limit → AC-5, AC-7; B5 symlinked `.gitignore` read → AC-4; B6 bound 129 → AC-6; B7 prefix `.gitignore` ignored → AC-5 path test; B8 an ignored `.gitignore` not read (Q-8 B) → AC-1, AC-2, AC-3; B9 kinds from a followed `metadata` → AC-1, AC-2, AC-4, AC-7; B10 memory file listed as NotFound → AC-3, AC-7; B11 `skipped` not counted → AC-7 ×2; B12 negation dropped → AC-1, AC-2, AC-3, AC-5 path test; B13 `limit` ignored → AC-5; B14 rule-3 ignores not counted → AC-2, AC-5, AC-7; B15 path not reported as given → AC-4; B16 limit before depth, and a bound of `== 128` only (y4) → AC-6. Interpretations: `invalid_params` refusals are asserted as `code` `invalid_params`, `kind` `schema` (the only kind of that code in `docs/control-protocol.md`); `limit` 1,000,001, -1 and `"3"`, `path` 3 and an unknown key are all `invalid_params`; `path` `""` and `/etc` are `invalid_path`. On disk, a dangling in-root link → `NotFound`, `out/x` → `OutsideRoot`, a loop → `Io` (variant and path only), a FIFO, socket or file-link as `dir` → `NotAFile`. A replaced root → `OutsideRoot` with path `(root)`. `path` at 128 segments: only "no directory descended" is asserted, not `too_deep`. Hand trees for AC-1 use names outside the generator charset (`B.txt`, `target`, …) but stay within S5's forms. Not tested: Pinned (a)'s "not logged" (whether a read-only command is logged is unspecified), unreadable directories (uid 0, Pinned (e)). · gate attempts: 0 (`cargo xtask ci` not run: disk; only my targets built, as instructed) · not run: `cargo xtask ci`; clippy of the AC-7 file on base (does not compile there; linted clean with the stub in place); fmt clean; CLI files clippy clean on base · missing context: none · next: director joins.
<!-- end test author log -->
### reviewer
- 2026-11-13T22:50Z · R-174 · tests_joined · rulings: Q-1 to Q-8 (D-095, D-097) and y1 to y5 (D-100, D-102), read in the primary checkout's copy (this branch's copy lacks y1 to y5); all accepted, none questioned · did: round 1 of 1d35a29238e747f5a2a5a6a89d155a2c0544841d, full; `reviews/T-019-r1.md`, verdict request-changes. Every AC was checked through `kiln-cli exec/run/control`, `--root` and `--memory-seed`. 24 mutants: 24 killed, 21 of them by T-019 acceptance or guard tests, 0 equivalent, 0 survived. Adversarial probes: links, a replaced root, FIFOs, sockets, devices, odd names, 4,096-byte paths, depth 2,100, 120,000 files, large `.gitignore` files, `capsh` without DAC capabilities, `strace`. Nothing outside the root was listed or read, and nothing was written. Findings: 2 defects (the known-differences paragraph misses trailing Unicode whitespace, POSIX classes and `***`; the `too_deep` and rule-5 wording says "128 segments" where y4 gives "128 or more"), 2 spec gaps (unbounded `.gitignore` cost; a prefix leaving the root refuses an in-root `path`), 2 nits. `Cargo.lock`: only additions (11 packages, as y2). `ignore` usage: only the allowed matcher items · gate attempts: 0 (`cargo xtask ci` not run; `cargo xtask hygiene` run) · not run: a uid-65534 run (used `capsh` instead), concurrent changes, 128 levels of large `.gitignore` files · missing context: none · next: implementer fixes findings 1 and 2; director rules on findings 3 and 4.
<!-- end reviewer log -->
