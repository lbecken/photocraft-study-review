---
id: T-041
title: "Start no CI run for a push to main that changes only ledger paths, with a test that keeps the workflow's list equal to xtask/ledger.toml"
tier: high              # ADR-0002 "a gate path": .github/ and xtask/tests; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []
depends_on: []          # nothing unmerged; no spec check is required by METHODOLOGY §5.7
touches:
  - .github/workflows/ci.yml          # GATE PATH: implementer: the `on.push` filter and the header comment
  - xtask/tests/acceptance_t041.rs    # GATE PATH: test author (also the permanent drift check, AC-1)
  - xtask/tests/guard_t041.rs         # GATE PATH: test author (behaviour main already has that this task must keep)
  - docs/development.md               # implementer: the "CI" section only; not a gate path
budget_tokens: 1100000  # high (METHODOLOGY §11, v1.2)
iteration: 2
found_by: planner       # retro-1 change 14, accepted by D-091
origin_task: null
---

# T-041 No CI run for a ledger-only push

## Goal
After this task a push to `main` whose changes all lie under the ledger paths starts no GitHub
Actions run; every other push starts one, as today. The list of paths in the workflow is kept
equal to the tool's own definition of the ledger by a test in the gate.

Evidence (retro-1 "Tooling and environment", change 14; D-091): every push to `main` starts a run
of about 3 minutes; the run number had reached 203 at the push of `b2ee623` (D-091); 15 commits of
iteration 1 changed only `handoffs/` or `STATE.md`. On main
`b53d370f4a5f22eeffd34969b49f34170e0edf57`, 306 of 534 first-parent commits change only paths
under the mode-`any` ledger entries (command in Pinned (g)); the number of pushes is not recorded,
so the number of runs saved is not known.

**Gate paths.** `.github/workflows/ci.yml` and `xtask/tests/` are gate paths, so the task is tier
high; the approval row must begin `Approve T-041` and contain the words `gate paths`. No
dependency changes: the test uses the `toml` crate xtask already depends on.

**Dispatch.** `docs/development.md` is also in the `touches` of T-033 and T-043 (§5.2). No stage-1
product task touches these paths.

Reading list: `methodology/METHODOLOGY.md` §8; `methodology/CHANGELOG.md` v1.2 row 14;
`docs/decisions.md` D-003, D-091; `docs/development.md` "Policy files" and "CI";
`xtask/ledger.toml`; `xtask/src/policy.rs` (`path_under`, `ledger_allows`);
`.github/workflows/ci.yml`; GitHub's documentation of `on.push.paths-ignore`
(https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions).

## Interface

### (1) The workflow's rule
`.github/workflows/ci.yml` keeps `push` to `branches: [main]` and `workflow_dispatch`, and adds
under `push` a `paths-ignore` list. For every entry of `xtask/ledger.toml` with `mode = "any"`, in
file order, with `e` its `path` without the trailing `/`, the list holds two patterns, in this
order: `'e'` and `'e/**'`. Entries with `mode = "add-only"` (today only `scorecard/items/`)
contribute nothing (Q-2). Today's list is therefore, in order: `tasks`, `tasks/**`, `metrics`,
`metrics/**`, `docs/decisions.md`, `docs/decisions.md/**`, `docs/adr`, `docs/adr/**`, `STATE.md`,
`STATE.md/**`, `handoffs`, `handoffs/**`, `retrospectives`, `retrospectives/**`, `evaluations`,
`evaluations/**`, `methodology`, `methodology/**`.

The pair `e`, `e/**` matches exactly the paths `path_under(p, e)` accepts (`p == e`, or `p` below
`e/`), so the workflow does not need to know whether an entry is a file or a directory. Nothing
else in the workflow changes: no `concurrency` group, the same job, steps, pins and permissions.
The header comment says what the filter does and that `xtask/tests/acceptance_t041.rs` checks it
against `xtask/ledger.toml`.

### (2) What GitHub then does (documented behaviour; not testable here)
From GitHub's documentation (fetched 2026-11-13; summarised, so INFERRED): for a push to an
existing branch GitHub compares the pushed head with the previous head by a two-dot diff; the
workflow does not run when every changed path matches a `paths-ignore` pattern, and runs when any
path does not; it always runs for a push of more than 1,000 commits or when the diff times out;
it does not run when the diff has more than 3,000 files and no unmatched path is among the first
3,000; it does not run when no file changed. Consequences for this repository:
- a push of several commits runs if any of them leaves a non-ledger change at the pushed head;
  ledger commits pushed together with a queue merge run (the merge changes code);
- a publish (`cargo xtask publish`) changes `docs/scorecard.md`, which is not a ledger path, so
  pushing it runs CI; so does every queue merge;
- a change made and reverted inside one push is not in the two-dot diff and starts no run;
- the 3,000-file limit: `git ls-files | wc -l` prints 409 and the largest first-parent commit of
  the last 400 changes 47 files (commands in Pinned (g)), so a push diff is far from it today.

### (3) `docs/development.md` "CI"
The sentence "runs `cargo xtask ci` on every push to `main`" is replaced by what (1) and (2) say:
the run starts for a push that changes a path outside the mode-`any` ledger entries of
`xtask/ledger.toml`, and on manual dispatch; a ledger-only push starts none, and its state is
checked by the next run or a manual dispatch; `scorecard/items/` is not excluded; the test of
AC-1 keeps the list equal to `xtask/ledger.toml`. Wording is the implementer's; the reviewer checks
it against the workflow and GitHub's documentation (METHODOLOGY §7).

## Pinned
- (a) **Errors and precedence.** n/a for a command: the change is configuration. The tests fail
  with an assertion; the drift test fails closed (b).
- (b) **Layout of `ci.yml` the test relies on.** Under `on:` at two spaces, `push:` at two, then
  at four spaces `branches: [main]` and `paths-ignore:`, then one item per line at six spaces in
  the form `- '<pattern>'` (single quotes), then `workflow_dispatch:` at two spaces. The test
  extracts the items between `paths-ignore:` and the next line indented four spaces or less. It
  fails, naming the reason, when `paths-ignore:` occurs other than once, when `on.push` has a
  `paths:`, `branches-ignore:` or `tags:` key, when `branches` is not exactly `[main]`, when
  `workflow_dispatch:` is missing, or when an item is not in the form above. A reformatting of
  the file therefore fails the test instead of passing it silently. The test reads
  `xtask/ledger.toml` with the `toml` crate and fails if an entry's path contains a character
  GitHub's patterns treat specially (`*`, `?`, `+`, `[`, `]`, `!`) or starts with `/`.
- (c) **Order of checks.** n/a.
- (d) **Paths and platform.** Both files are read from the checkout the test is built in
  (`CARGO_MANIFEST_DIR`'s parent), as `acceptance_t013_docs.rs` reads `docs/development.md`.
- (e) **Threat model.** What a skipped run gives up: CI is the only check of `main` after a push
  (D-003), and the gate reads ledger paths: the `hygiene` step's `session-paths` check reads every
  tracked file, its `reviews` check reads task front matter under `tasks/`, the `scorecard` step
  reads `evaluations/`, and `no-skip` takes today's date from HEAD's commit date. So a ledger-only
  push can turn `main` red without a run. It is then found by the next CI run (the next push that
  changes code) or, earlier, by the next queue run, whose main gate runs on the rebased result and
  would report it as that task's `gate_failed`. Options in Q-1. A wrong pattern could also skip a
  push that changes code; AC-1 and AC-2 cover that.
- (f) **Existing tests this task changes.** None. No test reads this checkout's
  `.github/workflows/ci.yml` today: `grep -rn 'workflows/ci.yml' xtask crates apps --include=*.rs`
  on main `b53d370f4a5f22eeffd34969b49f34170e0edf57` finds the path only as a gate-path example
  written into toy repositories or unit-test lists (`precheck.rs:1243`, `queue.rs:1169`,
  `acceptance_t005_r3.rs:538`, `acceptance_t005.rs:1108`), which this task does not affect.
- (g) **Worked examples and measurements** (on main `b53d370f4a5f22eeffd34969b49f34170e0edf57`).
  Ledger-only first-parent commits:
  `git log --first-parent --format=%H main | while read c; do git diff-tree --no-commit-id --name-only -r --root $c | grep -qvE '^(tasks|metrics|docs/adr|handoffs|retrospectives|evaluations|methodology)/|^(docs/decisions\.md|STATE\.md)$' || echo $c; done | wc -l`
  → 306 (of `git log --first-parent --format=%H main | wc -l` → 534; an empty commit counts as
  ledger-only here). Size: `git ls-files | wc -l` → 409; the largest first-parent commit diff of
  the last 400: 47 files. Model cases (AC-2): {`tasks/T-1-a.md`, `STATE.md`} → no run;
  {`tasks/T-1-a.md`, `docs/scorecard.md`} → run; {`scorecard/items/x.toml`} → run;
  {`docs/decisions.md.bak`} → run; {`docs/adrx/a.md`} → run; {`metrics`} → no run;
  {} → no run.
- (h) **Scorecard items.** None.
- (i) **Guarantee sentences in `docs/protocol/`.** None.

## Acceptance criteria
The acceptance test cannot call GitHub. It checks the workflow file against the tool's definition
and checks the rule's outcome against a naive model of GitHub's documented `paths-ignore`
behaviour. Both in `xtask/tests/acceptance_t041.rs`, each failing on base (today's workflow has no
`paths-ignore`). No test asserts the wording of a comment or of `docs/development.md`.

- AC-1 (workflow equals the ledger; permanent drift check): read as Pinned (b), the `paths-ignore`
  items of the checkout's `.github/workflows/ci.yml` equal, in order, the list (1) derives from
  the checkout's `xtask/ledger.toml`; `branches` is `[main]`; `workflow_dispatch` is present; no
  pattern matches any path under an `add-only` entry. Oracle: the derivation of (1), computed in
  the test from the parsed TOML.
- AC-2 (model of the outcome): with a naive matcher (a pattern without `*` matches the identical
  path; a pattern ending in `/**` matches every path that begins with the part before `**`; other
  forms are rejected by AC-1's checks) and the rule "a run starts iff the set of changed paths
  contains a path no pattern matches": (i) the model cases of Pinned (g) give the stated results;
  (ii) for every path `p` of `git ls-files` in the checkout, "some pattern matches `p`" equals "`p`
  lies under a mode-`any` entry" with the naive definition `p == e || p.starts_with(e + "/")`;
  (iii) in a throw-away git repository, for pushes modelled as `git diff --name-only <before>
  <after>`: one ledger commit → no run; two ledger commits → no run; a code commit then a ledger
  commit → run; a ledger commit then a code commit → run; a commit that adds and a later commit
  that removes a code file (net change ledger-only) → no run; a deletion of a task file → no run;
  a change of `.github/workflows/ci.yml` → run.
- AC-3 (kept behaviour; guard): in `xtask/tests/guard_t041.rs`, the workflow still has one job
  that runs `cargo xtask ci --report gate-report.json`, `permissions: contents: read`, no
  `concurrency` key, and its two `uses:` lines pinned to a 40-hex commit id. (This pins the
  structure that this task must leave alone, not a text a drafted task changes; if a later task
  must change one of these, it changes this guard with a ruling.)
- AC-4 (no regression; named command): `cargo xtask ci` green in `.worktrees/T-041` and in the
  queue's main and own gates.

After the merge, not an acceptance criterion: the director checks on the repository's Actions page
that the next ledger-only push to `main` started no run and the next push with a queue merge
started one, and writes the two observations in a decision row.

## Out of scope and implementer notes
- Generating `ci.yml` from `xtask/ledger.toml` (Q-3, option B); running CI only on queue merge
  commits by commit message (retro-1's first wording of change 14).
- The add-only exception, if the director rules otherwise (Q-2), changes only the derivation in
  (1) and AC-1.
- The implementer's own work is the YAML edit, the header comment and the CI section; if the
  implementer adds a unit test it goes in `xtask/tests/` only under a name in `touches`.
- Never set or export `CARGO_TARGET_DIR` (D-019).

## Questions and rulings
Planner's options for the director (R-160); the spec above is written for the recommendation.
- Q-1 (which ledger paths to exclude, given the gate reads some). (A, recommended) every mode-`any`
  entry: the observable result the director asked for; a ledger push that breaks the gate is found
  by the next CI run or the next queue run (Pinned (e)), recorded here as an accepted residual
  risk. (B) Exclude only the paths no gate step reads beyond `session-paths` (`handoffs/`,
  `STATE.md`, `retrospectives/`, `methodology/`, `docs/adr/`, `docs/decisions.md`, `metrics/`),
  keep CI for `tasks/` and `evaluations/`: fewer misses, but `tasks/` commits are the commonest
  ledger commits, so most of the saving goes, and `session-paths` still reads everything.
  (C) No exclusion (today).
- Q-2 (`scorecard/items/`, mode `add-only`). A `paths-ignore` pattern cannot tell an addition
  from a modification. (A, recommended) exclude nothing for add-only entries: a direct item
  addition on `main` changes the generated page and makes the gate red (D-034), so the run is
  useful, and legitimate additions arrive by `publish` with `docs/scorecard.md` anyway. (B) Exclude
  them: then a forbidden modification pushed to `main` also starts no run.
- Q-3 (drift: test or generation). (A, recommended) the test of AC-1 in the gate's `test` step:
  any task that changes `xtask/ledger.toml` (itself a gate path) must update the workflow in the
  same branch or its gate is red; no new command. (B) An xtask command writes the list into the
  workflow and a `--check` runs in the gate: a new check in the gate changes CHARTER §4's step
  list, which needs an ADR; making `ci.yml` a generated path would exempt a gate path from the
  `touches` check and let the queue rewrite it with main's xtask. Neither is needed for a list of
  nine entries.
### Rulings (director, D-098)
- Q-1: all mode-`any` ledger paths are excluded, and the consequence is recorded as an accepted residual risk: a ledger-only push that breaks a gate step which reads ledger paths (`hygiene`, `session-paths`, `scorecard`, `no-skip`'s date) is reported only by the next CI run or the next queue run. Mitigation, a director rule: `cargo xtask hygiene` and `cargo xtask scorecard --check` pass on main before a ledger push. This task reduces a check that runs on the owner's account; it is dispatched last of the tooling tasks, and the owner is told before it is queued.
- Q-2: `scorecard/items/` is not excluded.
- Q-3: the drift check is a test in the gate that compares the workflow's list with `xtask/ledger.toml`; no generation, no new gate step.

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13T21:40Z · R-160 · (new) · did: drafted from v1.2 change 14 (D-091). Tier high:
  `.github/` and `xtask/tests/` are gate paths (`precheck.rs` `is_gate_path`). The ledger list is
  taken from `xtask/ledger.toml` and the matching rule from `policy.rs` `path_under`; the
  `e`/`e/**` pair makes the patterns exact without knowing file from directory. GitHub's
  `paths-ignore` behaviour was read with a web fetch of its documentation (a summary, so
  INFERRED; the file limit it gives is 3,000). Measured the ledger-only commit share and the repo
  size (Pinned (g)). Found that the gate reads ledger paths (Pinned (e), Q-1). Pinned (a), (c)
  n/a: configuration, no command; (h), (i) n/a. · gate attempts: 0 · not run: no cargo; no push;
  GitHub's behaviour not observed · missing context: the number of pushes per iteration (not
  recorded) · next: director rules Q-1 to Q-3; no spec check required.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
