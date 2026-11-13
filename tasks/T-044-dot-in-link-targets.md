---
id: T-044
title: "Walk a `.` in a symlink target as the kernel does: after anything but a directory, the path is not found and a save creates nothing"
tier: high              # ADR-0002: product code on the file system's persistence path (the walk decides where `write_atomic` creates a file) and the confinement boundary (the same walk yields the location judged inside or outside the root)
state: draft
owner_role: planner
scorecard_items: []     # no item claimed: the change corrects the walk inside the root; the director may tie the tests to ws.path-confinement at the join
depends_on: [T-036, T-037]   # never in flight with either (shared disk.rs, docs/protocol/paths.md, confinement.rs); runs after both (D-111)
touches:
  - crates/kiln-workspace/src/disk.rs                     # implementer: the walk (`locate_steps`) and the module-doc bullet on link targets
  - crates/kiln-workspace/tests/confinement.rs            # implementer's own integration tests
  - docs/protocol/paths.md                                # implementer: the link-target sentence only (Pinned (i))
  - crates/kiln-workspace/tests/acceptance_t044_workspace.rs  # test author: AC-1 (Rust API)
  - apps/kiln-cli/tests/acceptance_t044_cli.rs            # test author: AC-2, AC-3 (exec, control); AC-5 (document)
  - apps/kiln-cli/tests/guard_t044_cli.rs                 # test author: AC-4 (cases that must not change)
  # scorecard/ and docs/scorecard.md: the director's, at the join (METHODOLOGY §6)
budget_tokens: 1100000  # high (v1.2, METHODOLOGY §11)
iteration: 2
found_by: reviewer      # T-008 r2 (R-138) N-1; T-008 ruling v12 (5); split from T-036 by D-111
origin_task: T-008
---

# T-044 Walk a `.` in a symlink target as the kernel does

## Goal
When Kiln walks a path, it follows each symlink by reading its target and walking the target's
components (`disk.rs` `locate_steps`). It splits the target with `Path::components`, which drops
every `.` component that is not the first. So a link whose target is `real/t.txt/.` is walked as
`real/t.txt`, and Kiln reads the file, where the kernel refuses with "Not a directory"
(VERIFIED: `reviews/T-008-r2.md` N-1; probe P3 below).

The spec check of T-036 (R-181) found two more consequences, both observable today:
- **A save through a link to `real/new.txt/.`** (when `real/new.txt` does not exist) creates
  `real/new.txt`. The kernel refuses with "No such file or directory" and creates nothing
  (kernel side VERIFIED by P3; Kiln side INFERRED from `disk.rs`:281–314, where the missing last
  component is `Found::Missing`, which `write_atomic` creates).
- **`project.tree` of a link to `real/t.txt/.`** gives `not_a_file`, because the walk reached
  the file. The kernel says "Not a directory", which Kiln reports as `not_found` everywhere else
  (VERIFIED, P3).

After this task a `.` component in a link target requires that what the walk has reached before
it is an existing directory, as a trailing `/` already does (T-008 ruling v7). Otherwise the
walk fails there: `not_found` inside the root, `outside_root` once outside. Nothing is read, and
a save creates nothing.

This task is not in the list of stage 1 in ADR-0005 (`docs/adr/0005-iteration-2-in-two-stages.md`:40–49,
which names T-036 and the evaluator's findings T-037 to T-039). The director decides when it is
dispatched. No gate path and no dependency in `touches`; std only.

Reading list: `tasks/T-008-path-confinement.md` Definitions ("as the kernel does"), Pinned (c),
ruling v7; `reviews/T-008-r2.md` N-1 and mutants M6, M7; `tasks/T-036-root-identity-follow-ups.md`
"Spec check R-181" (blocker 11) and the ruling "Split (director, D-111)";
`crates/kiln-workspace/src/disk.rs` module docs :35–62, `Step` :117–129, `locate_steps`
:218–323 (the trailing-`/` rule :277–280, the components loop :281–290), `list_dir` :467–517;
`docs/protocol/paths.md` "Location" and the precedence paragraph (:76–80);
`docs/protocol/project.md` "Refusals" (:30–37). Line numbers refer to main
`d162413e14236111c9bb6d7c5503d29c1dd1cb87`; T-036 and T-037 merge first and move
`paths.md` lines, so find the sentences by their text.

## Definitions
- **Root, location, inside, the walk**: as `docs/protocol/paths.md` "Location" and T-008
  Definitions.
- **Dot component**: a component `.` of a symlink's target, in any position (`a/.`, `a/./b`,
  `a/./`, `./a`, `.`).
- **What the walk has reached**: the entry the walk stands on after following everything before
  the dot component, links included (the dot after `fl` with `fl -> real/t.txt` stands on the
  file `real/t.txt`).

## Interface
`Vfs`, `VfsError`, every public signature, every kind and `reason.rs` are unchanged. No change
in `kiln-commands`, `kiln-control` or `kiln-cli`.

1. **A dot component after a directory** changes nothing: the walk stays where it is. A target
   that starts with `.` (`./t.txt`, `.`) begins at the link's directory, which is a directory.
2. **A dot component after anything that is not an existing directory** (a regular file, a FIFO,
   socket or device, a missing entry) ends the walk there. This includes a link that leads to
   such an entry.
   - If the walk is inside the root at that point, the result is as for a non-directory before
     the last component today (`Found::NoParent`). So `read` and `write_atomic` give `NotFound`,
     `is_file` is false, and `list_dir` gives `NotFound`.
   - If the walk is outside the root at that point, the result is `OutsideRoot` (T-008
     precedence).
   - `write_atomic` creates nothing: no target and no temporary file.
3. **Through the protocol**, the same results carry the existing kinds:
   - `doc.open`, `doc.save` and `project.tree` with such a `path` give `rejected` / `not_found`
     inside the root, and `outside_root` outside;
   - a refused save leaves the document dirty.
4. Every path without a dot component in a link target walks exactly as on base.

### Observable changes from base (each fails on base)
| Request | Base | After (the kernel's answer as oracle) |
|---|---|---|
| `read` / `doc.open` of `dot -> real/t.txt/.` (also `real/t.txt/./.`, `fl/.`) | the file's content | `not_found` (kernel: `Not a directory`) |
| `write_atomic` / `doc.save` through `dot` | the file `real/t.txt` replaced | `not_found`, `real/t.txt` unchanged (kernel: `Not a directory`) |
| `write_atomic` / `doc.save` through `dn -> real/new.txt/.`, `real/new.txt` absent | `real/new.txt` created (INFERRED, code) | `not_found`, nothing created (kernel: `No such file or directory`) |
| `list_dir` / `project.tree {"path":"dot"}` | `not_a_file` | `not_found` |

## Pinned
- (a) **Errors and precedence.** No new variant or kind. A dot component after a non-directory:
  `NotFound` (`not_found`) inside, `OutsideRoot` (`outside_root`) outside. The precedence of
  `docs/protocol/paths.md` is unchanged: `invalid_path`, `outside_root`, `not_found`,
  `not_a_file`, `io_error`. `project.md` already orders `not_found` before `not_a_file`, so a
  `project.tree` answer moving from `not_a_file` to `not_found` contradicts none of its
  sentences.
- (b) **Files.** Nothing is created, replaced or removed by a refused request; no `.kiln-tmp-*`
  is left. Test stems `acceptance_t044_workspace`, `acceptance_t044_cli` and `guard_t044_cli` are
  unique (no `*_t044*` file exists at `d162413`).
- (c) **Order of checks.** T-008 (c), unchanged: root identity, then location (the walk, which
  now also checks dot components), then outside, then existence, then type, then I/O.
- (d) **Path base, platform, file types.** As T-008 (d), Unix only. Tests use regular files,
  directories and symlinks only; no FIFO, socket or device.
- (e) **Threat model.** As T-008 (e). The change is inside the walk. It can only turn a success
  into `not_found`, or a creation into a refusal, never widen what is reached. A dot component
  after an outside entry was already `outside_root` (P3: `od -> ../outside/secret.txt/.` gives
  `outside_root` on base), and it stays so. The reviewer probes dot components after files,
  missing entries, links to files, links to links, FIFOs and outside entries, in first, middle
  and last position, and combined with `..` and a trailing `/`.
- (f) **Tests that pin what this changes.** None found. `grep -rn '/\."'` over every `tests/`
  directory finds no link target with a dot component. The trailing-`/` tests (`disk.rs` unit
  `link_target_with_trailing_slash_requires_a_directory`, T-019 AC-7's link to `keep.rs/`)
  keep their results.
- (g) **Worked examples (probe P3, T-036 spec check R-181, main `d162413`).** Run as root in a
  scratch directory holding `root/real/t.txt` = `target\n` and `outside/secret.txt`, with links
  in `root/`. The kernel side used `head`; the Kiln side used the debug `kiln-cli`.
  - Kernel, reading through the link:
    - `dot -> real/t.txt/.`, `dots -> real/t.txt/./`, `dotdot -> real/t.txt/./.`, `fldot -> fl/.`
      (with `fl -> real/t.txt`) and `od -> ../outside/secret.txt/.` → `Not a directory`;
    - `dn -> real/new.txt/.` and `md -> missing/.` → `No such file or directory`;
    - `dd -> real/.`, `self -> .` and `selfs -> ./` → `Is a directory`;
    - `mid -> real/./t.txt` → `target`.
  - Kernel, writing through the link (`echo X > link`):
    - `dot` and `fldot` → `Not a directory`;
    - `dn` and `md` → `No such file or directory`;
    - afterwards `real/` held only `t.txt`, still `target\n`.
  - Kiln on base, `exec doc.open` / `exec project.tree {"path":…}`:
    - `dot`, `dotdot`, `fldot` → open ok (content read) / `not_a_file`;
    - `dots`, `dn`, `md` → `not_found` / `not_found`;
    - `dd` → `not_a_file` / lists `dd/t.txt`;
    - `mid` → ok / `not_a_file`;
    - `od` → `outside_root` / `outside_root`;
    - `self` → `not_a_file` / lists `self/…`.
  - Not executed: a save through `dn` on base (no protocol request reaches it without an open,
    and no Rust API call was run). INFERRED by reading `disk.rs`:281–314; AC-1 and AC-3 show it
    on base.
- (h) **Scorecard.** No item is claimed (front matter). If the director ties the tests to
  `ws.path-confinement`, the evaluator's call is `kiln-cli exec doc.open '{"path":"dot"}' --root
  root` with `dot -> real/t.txt/.`, which gives exit 2 and `kind` `not_found`, while `cat
  root/dot` says `Not a directory`.
- (i) **Document sentences.** In `docs/protocol/paths.md`, the precedence paragraph:
  - Removed: "A link whose target ends in `/` must lead to a directory, as the kernel requires;
    a file reached that way is `not_found`."
  - Replaced by: "A link whose target ends in `/`, or has a `.` component, must lead to a
    directory at that point, as the kernel requires: a file there, or nothing, is `not_found`,
    and a save through such a link creates nothing."
  - Evidence: P3 (kernel) and AC-1 to AC-3.
  - In the same file nothing else changes. T-037 adds a sentence on already-open paths to this
    paragraph; keep it.
  - The `//!` bullet of `disk.rs` (:62, "Link targets ending in `/` require a directory, as the
    kernel does.") says the same. No other document changes. `docs/protocol/project.md` and
    `docs/control-protocol.md` are not edited.

## Acceptance criteria
**Protocol (high tier, METHODOLOGY §6).** An independent test author writes the three test files
on `task/T-044-tests`; the implementer adds no `acceptance_t044*` or `guard_t044*` file and does
not edit `scorecard/`. The test author lists the final test names in its log, and runs its tests
against a naive stub and at least 5 seeded bugs.

**Fixtures.** A fresh `work/` per test with `work/root/real/t.txt` = `b"target\n"` and
`work/outside/secret.txt` = `b"s3cr3t-content\n"`, with the links of Pinned (g) under
`work/root/`. "Snapshot" means every entry under `work/`, recursively, links not followed: type,
link target and bytes.

**Oracle.** Each case first asks the kernel through `std::fs` on the same link (for example
`fs::read(root.join("dot"))` must fail, and opening `root.join("dn")` for writing with `create`
must fail and leave `real/` unchanged). It then asserts that Kiln refuses where the kernel
refuses and succeeds where it succeeds, with the kinds and variants below.

**How errors are asserted.** In the Rust API, by variant and `path()`. Through the CLI, by `ok`,
`code`, `kind` and exit code. No test pins a message text, a whole document, the command list
or `error_kinds`. Every child process has a 10 s timeout and every `control` response is awaited
for at most 10 s; no test asserts a duration. No `return` inside a conditional in a test
(`xtask/src/noskip.rs`).

Base = main as it is when the tests branch is cut (after T-036 and T-037). **Acceptance** = fails
on base; **guard** = passes on base.

- **AC-1 (acceptance; `acceptance_t044_workspace.rs::dot_after_a_non_directory_is_not_found`).**
  Rust API on `DiskFs::new(work/root)`.
  - For `dot`, `dotdot`, `fldot`: `read` → `NotFound`, `is_file` → false, `write_atomic(b"X")` →
    `NotFound`, `list_dir(Some(..))` → `NotFound`.
  - For `dn` and `md`: `write_atomic(b"X")` → `NotFound`.
  - Afterwards the snapshot is unchanged: `real/` holds only `t.txt` = `b"target\n"`, there is no
    `real/new.txt` and no `.kiln-tmp-*`.
  - Fails on base: `read("dot")` returns the content, and `write_atomic("dn")` creates
    `real/new.txt`.
- **AC-2 (acceptance; `acceptance_t044_cli.rs::dot_after_a_file_is_not_found_through_exec`).**
  - `kiln-cli exec doc.open '{"path":"dot"}' --root work/root` → exit 2, stdout empty, stderr a
    JSON object with `code` `rejected` and `kind` `not_found`.
  - `kiln-cli exec project.tree '{"path":"dot"}' --root work/root` → exit 2, `kind` `not_found`.
  - Fails on base: `doc.open` exit 0; `project.tree` `kind` `not_a_file`.
- **AC-3 (acceptance; `acceptance_t044_cli.rs::save_through_a_dot_link_creates_nothing_through_control`).**
  - One `kiln-cli control --root work/root` session: `doc.open x.txt` (a regular file `x.txt` =
    `b"x\n"`), then `doc.insert`.
  - Between two requests, `x.txt` is replaced by the link `x.txt -> real/new.txt/.`, with
    `real/new.txt` absent. The kernel check: writing through it fails.
  - `doc.save` of document 1 → `rejected` / `not_found`, and `doc.info` → `dirty: true`.
  - No `real/new.txt` and no `.kiln-tmp-*` exist; the snapshot is otherwise unchanged.
  - Fails on base: the save succeeds and creates `real/new.txt` (INFERRED, Pinned (g)).
- **AC-4 (guard; `guard_t044_cli.rs::dot_after_a_directory_and_other_links_are_unchanged`).**
  Through `exec`:
  - `mid` opens with `b"target\n"`;
  - `dd` → `not_a_file` for `doc.open`, and `project.tree {"path":"dd"}` lists `dd/t.txt`;
  - `self -> .`: `project.tree {"path":"self"}` lists `self/…` (as T-019's rule 1 says);
  - `od` → `outside_root`;
  - `dots` and `trail -> real/t.txt/` → `not_found`;
  - a link `cur -> ./real/t.txt` opens with `b"target\n"`.
  - Passes on base and must keep passing.
- **AC-5 (document; acceptance; `acceptance_t044_cli.rs::paths_doc_drops_the_slash_only_rule`).**
  `docs/protocol/paths.md`, whitespace squashed, does not contain "A link whose target ends in
  `/` must lead to a directory", which is present on base. Absence only; the reviewer checks the
  new sentence against the binary.

## Scorecard items and intended test names
No item is claimed. If the director ties tests to `ws.path-confinement` at the join, the
intended names are `kiln_workspace::acceptance_t044_workspace::dot_after_a_non_directory_is_not_found`,
`kiln_cli::acceptance_t044_cli::dot_after_a_file_is_not_found_through_exec` and
`kiln_cli::acceptance_t044_cli::save_through_a_dot_link_creates_nothing_through_control`. AC-4
and AC-5 are not tied.

## Out of scope and implementer notes
- `..` components: already handled physically (a `..` after a file ends the walk as `NoParent`;
  `disk.rs`:302). Not changed.
- Request paths: a `.` segment in a request `path` is refused lexically (`invalid_path`,
  `paths.md` "Lexical refusals") and is not this task's subject.
- The root's identity (T-036), re-opens (T-037), `MemoryFs` (no links).
- How the walk records a dot component is the implementer's choice (for example a step like the
  trailing-`/` `DirMark`). Splitting the target text must not rely on `Path::components`
  keeping `.`, because it drops every `.` but the first.
- Implementer's own tests: unit tests in `disk.rs`, integration tests in `confinement.rs`.
  Determinism: no clock or thread timing; no new dependency.

## Conflicts and dispatch
- **T-036** (draft, then merged first) and **T-037** (draft) share `disk.rs`,
  `docs/protocol/paths.md` and, for T-036, `confinement.rs`. This task is never in flight with
  either (METHODOLOGY §5.2) and runs after both (D-111). It needs a pre-dispatch check against
  main after they merge (§5.7), mainly to re-find the `paths.md` sentence and the `disk.rs`
  lines.
- It is outside stage 1 of ADR-0005; the director schedules it.
- `docs/scorecard.md` is generated.

## Questions and rulings
None open.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T23:58Z · R-186 · (new) · did: filed from the section "Split-off item 5" of T-036
  (spec check R-181), as ruled in D-111. Written by the template, with:
  - the kernel as the oracle and probe P3 as evidence;
  - the two behaviour changes the check found (a save through a link to `real/new.txt/.`;
    `project.tree` of a link to `real/t.txt/.` changing from `not_a_file` to `not_found`) as
    acceptance criteria;
  - one `paths.md` sentence replaced, worded;
  - unique stems, no `scorecard/` in `touches`, no message pins.
  - Tier high by ADR-0002: product code on the persistence path (the walk decides where
    `write_atomic` creates a file) and the confinement boundary (the same walk gives the
    location judged inside or outside).
  - No scorecard item claimed. Outside ADR-0005 stage 1.
  - Pinned lines n/a: none.
  - Unsure: the base save through `dn` creating `real/new.txt` is INFERRED from code, not run.
  - · gate attempts: 0 · not run: cargo, any probe of new code · missing context: none · next:
  director orders it after T-036 and T-037; spec check against main after they merge.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
