---
id: T-095
title: "On Windows, refuse as invalid_path the characters Windows cannot store in a name (':' with streams and drive prefixes, <>\"|?*, control characters), and state how reserved names, trailing dots and spaces, case and short names behave"
tier: high              # METHODOLOGY v1.5 §4: the confinement boundary (the lexical check of every request path) and a security property a document states. No gate path, no dependency change; no workflow edit (T-072's general reported Windows step runs the test file, D-254).
state: draft
owner_role: planner
scorecard_items: []     # none claimed (ws.* items verified on Linux; per-platform confirmation is the evaluator's)
depends_on: [T-074, T-093]   # T-074 merged (25054fe); T-093 (specified, low): shares docs/protocol/paths.md and project.md and states the macOS case sentence this task's Windows one sits beside (Q-5)
touches:                # one role per path (D-153)
  - crates/kiln-workspace/src/path.rs        # implementer: Interface (1) under #[cfg(windows)] in RelPath::new, and its #[cfg(test)] module (one new portable unit test; the three existing test names unchanged)
  - docs/protocol/paths.md                   # implementer: the sentences of Pinned (i) for paths.md
  - docs/protocol/project.md                 # implementer: the Windows clause of `skipped` (:26, :99) and the D-270 citation (:82-83)
  - docs/control-protocol.md                 # implementer: a Windows sentence after the lexical bullet (bullet verbatim), and the D-270 citation (:144-147)
  - crates/kiln-workspace/tests/guard_t095.rs   # test author: Interface (5); passes on Unix base
  - crates/kiln-workspace/tests/acceptance_t095.rs   # test author: AC-D1 to AC-D3, the document statements (ruling 4, D-272; added at the spec check); every test fails on base
  # pending Q-A (spec check): the four Windows-green targets of Pinned (f), last item, if ruled into this task
budget_tokens: 1100000  # high (v1.5); estimate about 450,000
iteration: 2
found_by: planner       # cut from T-074 at its re-specification (ADR-0009 (b), R-255; D-254 ruling 1)
origin_task: null
---

# T-095 Windows: names Windows gives a meaning, case and short names

Specified in full under v1.5 against main `4eb349d` (T-074 merged at `25054fe`; `path.rs` unchanged since `72522f2`:
`git diff --stat 72522f2 4eb349d -- crates/kiln-workspace/src/path.rs` empty). Measured base: the T-074 probe, run
44290286460 at `de8106f`, record `tasks/ci-runs/T-074-probe-1.md`, letter I, on `windows-2025`, through `DiskFs` with a
`\\?\` root (PROBE B4); cited as "PROBE <id>". In that probe `RelPath::new` accepted every name tried: the test prints
`PROBE I0` only for a name it refuses, and the record has no `I0` line.

## Goal
`RelPath::new` (`crates/kiln-workspace/src/path.rs:29-57`) refuses the same forms on every platform. On Windows, names
the walk passes to the file system then behave as follows at base:
- `:` reaches stream syntax and drive prefixes. `f.txt::$DATA` reads `f.txt` under a second name (PROBE I2
  `read "f.txt::$DATA": Ok([102])`), and `check_location` answers `Ok` for it (I3). `C:x`, `C:` and `D:x` answer
  `OutsideRoot` (I1-I3), but only after `Path::join` has left the root (I10 `canonical.join("C:x"): "C:x"`), so the
  file system outside was consulted.
- `<`, `?`, `*`, `|`, `"` and U+0001 answer `Io InvalidFilename`, os error 123, for write, read and check (I1-I3): an
  `io_error` after a file-system call, where the request was invalid from the start.

After this task, on Windows:
- These characters are refused as `invalid_path` before the file system is consulted, as `\` and NUL are now.
- Names Windows can store but treats specially are stated, not refused: reserved device names, a trailing dot or
  space.
- Case and 8.3 short names are stated as second spellings of one file.

Linux and macOS are unchanged. The two citations of D-270 are corrected.

Reading list:
- `tasks/ci-runs/T-074-probe-1.md` letter I, and B4.
- `docs/protocol/paths.md` "Lexical refusals", "The rule" (re-open by path text), and its Windows paragraphs.
- `docs/protocol/project.md:22-26`, `:79-84`, `:99`; `docs/control-protocol.md` "Paths".
- `crates/kiln-workspace/src/path.rs`, and `disk.rs` `list_dir` (names not a valid `RelPath` are skipped).
- `tasks/T-093-macos-document-sentences.md` rows 2-4.
- METHODOLOGY v1.5 §5.7, §6, §8.1.

## Interface (behaviour on Windows; Linux and macOS unchanged)
### (1) Lexical refusals added on Windows
`RelPath::new(p)` returns `Err(PathError)` on Windows if `p` contains any character of exactly this set:
- `:`, `<`, `>`, `"`, `|`, `?`, `*`;
- U+0001 to U+001F.

These join the existing refusals (`\`, NUL, empty, leading `/`, empty, `.` or `..` segments). The control protocol maps
them, as now, to `rejected` / `invalid_path`. The precedence is the existing one: lexical before everything.

This covers stream names (`f.txt:s`, `f.txt::$DATA`, `f.txt:s:$DATA`), drive-relative segments anywhere (`C:x`, `C:`,
`x/C:y`) and the characters Win32 does not allow in a name. The `PathError.reason` text is not specified, and no test
pins it. `RelPath` is shared by the disk and the in-memory file systems, so on Windows `--memory` and `--memory-seed`
(`apps/kiln-cli/src/main.rs:256`) refuse the same paths (corrected (spec check): stated). `--root` is a host path, not a
`RelPath`, and is not affected (`C:\proj` stays a valid root).

Reason for this set: it is the set of characters other than `\`, `/` and NUL that Win32 does not allow in a file name.
The rule rests on Microsoft's naming rules (read, not run). Measured samples: PROBE I1 gave os error 123 for `<`, `?`,
`*`, `|`, `"` and U+0001; every `:` write failed (`f.txt:s`, `f.txt:s:$DATA`, `f.txt::$DATA`: os error 123; `C:x`, `C:`,
`D:x`: `OutsideRoot`; `x/C:y`: `NotFound`, because `x` does not exist), and the reads reached a stream (`f.txt::$DATA`
`Ok([102])`; `f.txt:s` `NotFound`) or another drive (`OutsideRoot`) (I1-I3; corrected (spec check): the draft said every
`:` form reached a stream or another drive, which `x/C:y` did not). So no file a Windows project can hold through Win32
becomes unreachable; `>` and U+0002 to U+001F were not sampled at the file system. (`RelPath::new` at base has no
platform code, so that it accepts them on Windows at base is shown by the Linux run: the sketch's unit test passes on
Linux asserting acceptance.)
### (2) Not refused (stated)
Reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`…`COM9`, `COM¹`…, `LPT1`…, `CONIN$`, with or without an
extension: `aux.rs`, `con.txt`) and names ending in a dot or a space (`a.`, `b `, `c. .`) stay valid. Under the `\\?\`
paths Kiln uses they are ordinary files of the project, read and written like any other (PROBE I1-I3 `Ok` for
each name sampled, I5/I6 listed). Sampled at base: `CON`, `con.txt`, `NUL`, `nul.rs`, `COM1`, `LPT1.txt`, `AUX`, `PRN`,
`CONIN$`, `COM¹`, `a.`, `b `, `c. .`, through `write_atomic`, `read`, `check_location`, `list_dir`; not sampled:
`aux.rs` itself, `COM2`…`COM9`, `LPT2`…, and a `Document` save (corrected (spec check); `aux.rs` and the save are
shown by `guard_t095` in the CI round).
- A project holding `aux.rs` (common in Rust repositories made on Unix) can be opened, saved and listed by Kiln on Windows.
- Other Windows programs may not open such files: a non-`\\?\` write to `a.` made a file `a` (PROBE I11, I12).
- Kiln creates such a file when a request saves a new one under that name. This is stated, not refused (Q-2).
### (3) Case and short names (stated as limits; no code change)
On Windows the file system folds case: `F.TXT` names `f.txt`.
- At base, `read` and `check_location` of `F.TXT` answer as for `f.txt` (PROBE I7).
- A save of `F.TXT` replaces that file. Which spelling the file has afterwards is **to be measured by the test author on
  CI** (`MEASURED:` line, Interface (5)); the sentence follows the record.
- `doc.open` recognises a document already open by its path text (`paths.md` "The rule"). So `doc.open F.TXT` after
  `doc.open f.txt` opens a second document of the same file, and the later save replaces the earlier one's content
  without notice. This is the same as T-093's macOS row 4.
- 8.3 short names alias long names the same way where a volume has them. The runner's work volume has none (PROBE I8
  `NotFound`; I9: 8dot3 creation disabled there, enabled on `C:`). Whether a short name reaches the file is **to be
  measured by the test author on CI** in a fixture on `C:` (`std::env::temp_dir()`), else unchecked.
- Not refused (Q-3). A listing shows the stored spelling (PROBE I6: names as created).
### (4) Listings
Names that fail `RelPath::new` are omitted from listings and counted in `skipped`, as now (`disk.rs` `list_dir`). On
Windows that now includes names with a character of (1). Such a name cannot be created through Win32 (PROBE I1), so it
can only come from another system (for example WSL's own names); that case is unchecked. `project.md:26`, `:99` gain a
Windows clause; the quoted words "not valid UTF-8 or contain" stay (`xtask/platform-tests.toml`, four entries).
### (5) Tests
- Implementer, in `path.rs`'s `#[cfg(test)]` module: one new test with no platform cfg on the fn. Each name of (1) is
  refused exactly when `cfg!(windows)`, and each name of (2) plus `c~1.rs`, `x\u{7f}` and `é` is accepted everywhere.
  It runs on Windows in T-073's reported step (`-p kiln-workspace --lib`).
- Test author, `crates/kiln-workspace/tests/guard_t095.rs`: no platform cfg in the file (D-245); expectations by
  `cfg!(windows)`; fixtures below `CARGO_TARGET_TMPDIR`, except the short-name case. It checks:
  - The refusal set of (1) as an equality over U+0000 to U+00FF except `/` (the separator), one at a time inside a name
    `a<c>b`: refused exactly for the set of (1) plus `\` and NUL on Windows, and for `\` and NUL alone on Linux and
    macOS; and the look-alikes U+FF1A, U+2236, U+FF1F, U+FF0A, U+FF1C, U+FF02 accepted on every platform (corrected
    (spec check): the variant outside the list beyond U+00FF).
  - Every stream and drive form of (1) through `Document` open and save: `Err` before any file changes, the directory
    snapshot unchanged.
  - (2) through `DiskFs`: write, read, list and save of `aux.rs`, `CON`, `a.`, `b `.
  - (3): read and save of `F.TXT` against `f.txt`, one `MEASURED:` line for the stored spelling after the save, one for
    the short-name read on `C:`, and two `Document`s for one file, the second save winning.

  On Linux and macOS each case asserts today's behaviour (`:` and the other characters are accepted and stored as
  given). The expectations of (3) are chosen by `cfg!(target_os = "linux")`, not by `cfg!(windows)`: on Linux `F.TXT`
  and `f.txt` are two files; on macOS (the runner's default case-insensitive volume) and Windows they are one, as
  T-093's `apps/kiln-cli/tests/macos_t093.rs` chooses (corrected (spec check): with `cfg!(windows)` the macOS job would
  run the Linux branch of (3) and fail).

## Pinned
- (a) Errors: no new kind. Interface (1) is `PathError` → `invalid_path`, first in precedence.
- (b) Files: none written or read beyond what requests do.
- (c) Order: Interface (1) is part of the lexical check, before the root check.
- (d) Platform: Windows x86_64 MSVC (CI `windows-2025`; work volume `D:` with 8.3 creation disabled, `C:` enabled, PROBE
  I9). The new product lines are under `#[cfg(windows)]` (a statement in `RelPath::new`), so the Unix library is
  unchanged (reviewer: IR comparison as for T-073 and T-074).
- (e) Threat model: request paths from a client (control protocol, scripts) and names inside a hostile project. (1)
  stops a request from naming a stream, another drive or an invalid name. (2) and (3) are not escapes: every such
  name resolves inside the root (PROBE I3 `Ok` for each). Their cost is a second spelling of one file. Not defended:
  two documents of one file under case or short-name spellings (Q-3).
- (f) Tests pinning what this changes:
  - `xtask/tests/guard_t079.rs` pins `path.rs`'s three test names (`:1283-1285`): kept, the new test is added.
  - `apps/kiln-cli/tests/acceptance_t008_cli.rs:563-565` pins the lexical bullet verbatim: kept, the Windows sentence
    follows it.
  - The registry quotes four `project.md` passages ("not valid UTF-8 or contain") and three `control-protocol.md`
    passages: kept (S-4, measured).
  - Not kept on Windows (added at the spec check; Q-A): four test targets that exit 0 on Windows at base
    (`tasks/ci-runs/T-092-probe-1.md:49,50,57,124`) hold fixtures or requests with `:` or `"` and fail once the rule is
    on: `apps/kiln-cli/src/main.rs` test module (`manifest_refusals_name_the_problem`, path `b,{"`),
    `crates/kiln-commands/tests/acceptance_t032.rs` (two tests), `acceptance_t032_json.rs` (one), `guard_t032.rs` (one).
    Shown on Linux with the rule switched on for every platform (Spec check, point 1).
- (g) Worked examples (base: PROBE lines; after: the model "Windows: refused iff a character of (1)"):
  - `read "f.txt::$DATA"`: base `Ok([102])` (I2), after `PathError` (`invalid_path`).
  - `read "C:x"`: base `OutsideRoot` (I2), after `PathError`.
  - `write_atomic "x<y"`: base `Io InvalidFilename` (I1), after `PathError`.
  - `write_atomic "nul.rs"` (a reserved name with an extension, as `aux.rs`): base `Ok` (I1), after `Ok`.
  - `read "F.TXT"`: base `Ok([102])` (I7), after the same.
  - Prototype (scratch clone, branch `t095`, not committed): the rule of (1) and the unit test of (5). Measured there:
    `cargo test -p kiln-workspace --lib --locked path::` gives `4 passed`, and `cargo clippy -p kiln-workspace --lib
    --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings` is clean.
- (h) Evaluator: none.
- (i) Document sentences, worded to be copied. Each is "measured" (citing PROBE lines) or "unchecked", and is shown on
  the task's head by `guard_t095.rs` in the CI record before it stands (reviewer).
  - `paths.md` "Lexical refusals", new paragraph: "On Windows (T-095) a path is also refused this way if it contains
    `:`, `<`, `>`, `"`, `|`, `?`, `*` or a control character (U+0001 to U+001F), the characters Windows does not allow
    in a file name: so no request names an alternate data stream (`f.txt::$DATA` is a second name of `f.txt` there)
    or another drive (`C:x`). On Linux and macOS these characters are ordinary." Evidence: PROBE I1-I3, I10 (base); the
    refusal itself is shown by `guard_t095.rs` on the runner; "On Linux and macOS these characters are ordinary" by
    `guard_t095.rs` on `gate-linux` and `gate-macos` (its non-Windows branch); "the characters Windows does not allow in
    a file name" measured for `<`, `?`, `*`, `|`, `"`, U+0001 and `:` (I1), read from Microsoft's naming rules and not
    run for `>` and U+0002 to U+001F (corrected (spec check): the evidence line did not say so; Q-E). The heading
    "## Lexical refusals (unchanged)" loses "(unchanged)", which this paragraph makes false (corrected (spec check);
    no test or registry entry quotes the heading: `grep -rn "Lexical refusals" --include=*.rs --include=*.toml`
    gives none).
  - `paths.md`, replacing "Names that Windows gives a meaning, and case, are not yet stated for Windows (T-095).":
    "Names that Windows reserves for devices (`CON`, `NUL`, `aux.rs`, …) and names ending in a dot or a space are
    ordinary files of the project for Kiln, which reaches every file through `\\?\` paths. Other Windows programs may
    not open them (T-095; CI run 44290286460)." Evidence: PROBE I1-I3, I5, I11, I12 (for `CON`, `NUL`, `nul.rs`,
    `con.txt`, `a.`, `b `); `aux.rs` itself: `guard_t095` in the CI round (not in the probe; corrected (spec check)).
  - `paths.md` "The rule", after the re-open item, a Windows bullet: "On Windows names differ only by case on the
    default file system: `F.TXT` reads and saves `f.txt` (after the save the file is named <measured>), and an 8.3 short
    name, on a volume that has them, names its long one (<measured or unchecked>). A path already open is recognised
    by its text, so `doc.open` under another spelling opens a second document of the same file, and the later save
    replaces the other's content." Evidence: PROBE I7, I8, I9; the rest to be measured by `guard_t095.rs`.
    Beside T-093's macOS row 4 (Q-5).
  - After T-093 merges (added at the spec check): T-093's "Platforms" bullet in "Residual limits" ends "names and case
    on Windows are not yet stated (T-095)" (`task/T-093` at `234f98c`, `docs/protocol/paths.md`). That clause becomes
    false with this task and is replaced by "names and case on Windows: see Lexical refusals, The rule and the Windows
    paragraphs of The root (T-095)". To be re-checked against T-093's merged wording before dispatch.
  - `project.md:26` and `:99`, appended: "On Windows also names containing a character refused by
    `docs/protocol/paths.md` "Lexical refusals" (such names cannot be made through Windows itself; unchecked)."
  - `control-protocol.md`, after the lexical bullet: "On Windows (T-095) also a path containing `:`, `<`, `>`, `"`, `|`,
    `?`, `*` or a control character; see `docs/protocol/paths.md`."
  - D-270: `control-protocol.md:144-147` and `project.md:82-83` name CI run 44358453526
    (`tasks/ci-runs/T-074-round-1.md`, all 15 guards) for the `awa3_*` and `awa1_*` tests. `paths.md`'s "its four
    `awa1_*` tests … in CI run 44307677646" stays: four is what that run held. Exact replacements (corrected (spec
    check): the draft gave none, and AC-D3 needs them): in `control-protocol.md` "(shown in CI run 44307677646,
    `tasks/ci-runs/T-074-early-1.md`, for a junction" becomes "(shown in CI run 44358453526,
    `tasks/ci-runs/T-074-round-1.md`, for a junction"; in `project.md` "`guard_t074.rs` in CI run 44307677646,
    `tasks/ci-runs/T-074-early-1.md`;" becomes "`guard_t074.rs` in CI run 44358453526, `tasks/ci-runs/T-074-round-1.md`;".
    Evidence: `tasks/ci-runs/T-074-round-1.md:216-233` (Windows `guard_t074`: five `awa1_*`, five `awa3_*`, all `ok`) and
    `:78` (`disk::tests::link_at_temp_name_is_not_followed_on_every_platform ... ok`), which the control-protocol sentence
    also cites. The rest of each sentence is unchanged (its claims are a subset of what that run shows).

## Acceptance criteria
Local (implementer, re-run by the reviewer):
- AC-1: every existing `kiln-workspace` test passes on Linux and macOS. The product change is one `#[cfg(windows)]`
  statement in `RelPath::new`. The Linux and macOS library IR equals base's after normalising.
- AC-2: `cargo clippy -p kiln-workspace --lib --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings` and
  `cargo clippy -p kiln-workspace --test guard_t095 --locked --target x86_64-pc-windows-msvc -- -D warnings` are clean.
  No C stub is needed (measured, Pinned (g)).
- AC-3: `cargo xtask no-skip` prints `35 platform test(s) registered (10 pending)`, unchanged. The variant to reject is a
  `#[cfg(windows)]` on the new test fn (an unlisted platform test).

Through the join (test author, `crates/kiln-workspace/tests/acceptance_t095.rs`; ruling 4, D-272; added at the spec
check). The documents read from the repository (`CARGO_MANIFEST_DIR/../../docs/…`), whitespace squashed (so CRLF on a
Windows checkout reads the same); no platform cfg; each test fails on base on every platform, and **every test in the
file must fail on base** (the join refuses the whole join if any acceptance test passes on base: `xtask/src/join.rs:307-319`
`refused_base_passes`), so a "stays unchanged" check (for example `paths.md`'s four `awa1_*` in run 44307677646) belongs
in `guard_t095.rs` or the reviewer's diff check, not here:
- AC-D1: `docs/protocol/paths.md` contains Pinned (i) item 1 (the Windows paragraph of "Lexical refusals") as worded.
  Base: absent.
- AC-D2: `docs/protocol/paths.md` contains Pinned (i) item 2 (reserved names, trailing dot or space) as worded, and no
  longer contains "are not yet stated for Windows (T-095)". Base: absent / present. (Q-B: which paragraphs ruling 4 means.)
- AC-D3: the D-270 citations: `docs/control-protocol.md` contains "(shown in CI run 44358453526,
  `tasks/ci-runs/T-074-round-1.md`, for a junction" and `docs/protocol/project.md` contains "the `awa1_*` tests of
  `guard_t074.rs` in CI run 44358453526, `tasks/ci-runs/T-074-round-1.md`" (Pinned (i), last item). Base: both cite
  44307677646 / `T-074-early-1.md`.
- The variant each must reject: AC-D1 a paragraph without `>` (or saying "U+0000"); AC-D3 the run id changed but the
  record left at `T-074-early-1.md`.
Measured on the sketch (spec check): three sketch tests, `0 passed; 3 failed` with the documents at base, `3 passed` with
the Pinned (i) text in place; clippy `-D warnings` clean for the host and `x86_64-pc-windows-msvc` with `#![cfg(test)]` at
the top (without it the workspace's `unwrap_used`/`expect_used` deny fires in a helper fn, as for every integration
test file here).

Through CI (pending criterion, §8.1). Fail-on-base evidence on Windows is the probe record (as D-254 ruling 8 for
T-074).
- AC-W1 (Interface (1)): in `guard_t095`, the character equality and every stream and drive form `ok` on Windows.
  Fails on base: no `PROBE I0` line exists for `f.txt:s`, `f.txt::$DATA`, `C:x`, `x<y`, `q?`, `s*`, `p|q`, `dq"`,
  `ctl\u{1}`, `x/C:y`. Instead there are I2 `read "f.txt::$DATA": Ok([102])`, I1 `write_atomic "C:x": Err(OutsideRoot …)`,
  and I1 `write_atomic "x<y": Err(Io … InvalidFilename …)`. Unmeasured at base: `>` and U+0002 to U+001F; the rule
  covers them by construction.
- AC-W2 (Interface (2)): `aux.rs`, `CON`, `a.`, `b ` written, read, listed and saved on Windows. Holds at base (PROBE
  I1-I3, I5, I6 for `CON`, `a.`, `b ` by `write_atomic`/`read`/`list_dir`; `aux.rs` and the `Document` save are not in
  the probe and are inferred from `AUX`, `nul.rs` and `write_atomic`: corrected (spec check)), so it is a guard.
- AC-W3 (Interface (3)): `F.TXT` reads and saves `f.txt`; the two `MEASURED:` lines (spelling after the save, short name
  on `C:`) are present. The reading holds at base (PROBE I7); the spelling and the short name are unmeasured, and the
  test's first run measures them.
- AC-W4: the new unit test of `path.rs` `ok` on Windows in T-073's step.

### The pending CI criterion (v1.5 §8.1), for the approval row
The record `tasks/ci-runs/T-095-round-N.md`, from the artefacts of the task's head on `ci/candidate`, shows:
- `gate-linux`, `gate-macos`, `window-linux` and `window-probe (macos-15)` green, the test lists naming `guard_t095`'s
  tests.
- In `gate-windows`: fmt, layers, ui-commands, hygiene, schema and no-skip PASS. clippy, scorecard and test FAIL only
  at test targets T-092, T-100 or T-098 own, named in the record (corrected (spec check): T-100 holds two of T-092's
  former targets since D-273).
- T-073's step (`workspace-tests.txt`) names the new `path.rs` unit test `ok`.
- **T-072's general step** (`windows-tests.txt`, each `crates/kiln-workspace/tests/*.rs` run with `-- --show-output`)
  shows `==== guard_t095: exit status 0`, every `guard_t095` test `ok`, and its `MEASURED:` lines; and `====
  acceptance_t095: exit status 0` with its three tests `ok` (added at the spec check: the step runs every file there).

**One CI round expected, two if a measured sentence needs rewording.** If T-092 has merged first, `gate-windows`'s
`test` step may build `kiln-workspace`'s targets, and the criterion's "FAIL only at …" is re-read against T-092's end
state (T-092 makes 13 of its 15 targets exit 0; `acceptance_t019_workspace` and `project` are T-100's: D-273;
corrected (spec check)).

## Out of scope and implementer notes
- Refusing a second spelling of an open file (case, short name), which would be a change to `kiln-commands`' table of
  open documents or an exact-case check in `disk.rs`'s walk (Q-3). Refusing reserved names or trailing dots and spaces
  (Q-2). Names that are not Unicode (T-092 ruling 3, D-269: an unpaired-surrogate name is made and skipped by listings,
  `tasks/ci-runs/T-092-probe-1.md` P-A1 to P-A6). The access list (T-074 D-254 ruling 7).
- No platform cfg on a test fn; Windows-only product lines under `#[cfg(windows)]`. Do not rename `path.rs`'s tests.

## Collisions and order
- **T-092** (fresh spec check running) touches `disk.rs` (test module), eleven `kiln-workspace/tests/*.rs` files,
  `xtask/platform-tests.toml`, `xtask/tests/acceptance_t092.rs` and `guard_t092.rs`. **No path is shared with this
  task**, which avoids the registry, `disk.rs` and every existing test file. They may run side by side (§5.2). One
  semantic contact: none of T-092's rewritten tests should use a `:` or `<>"|?*` name on Windows. grep on `4eb349d`:
  `RelPath::new("…[:<>|?*]…")` in `crates`, `apps` gives no hit (S-4); T-092's re-check confirms against its own diff.
- **T-093** (specified, low, re-check running) touches `docs/protocol/paths.md` and `project.md`, both shared. Not in
  flight together. Recommended: **T-093 first** (low, about 150,000, ready sooner), and its macOS case row (row 4) and
  `skipped` rows (2, 3) are the macOS halves of this task's (3) and (4). This task then re-checks the sentences of
  Pinned (i) against T-093's text (Q-5).
- T-094 (draft; `disk.rs`, `paths.md`): after.

## Estimate
T-074 booked 920,909 (D-271): 30 product lines, seven portable tests that made links, nine registry entries, six
document places, two CI looks and one formal round. This task: one product statement and one unit test, a guard file
by an independent test author without links (names only), five document places, no registry, no gate path, one or two
CI rounds: **about 450,000** (ceiling 1,100,000). Arm: high → independent test author.

## Questions and rulings
- **Q-1 (scope of the refusal).** A: Windows only. A Linux or macOS file named `a:b.rs` stays openable there; on Windows
  such a file cannot exist (Win32), so a project moved there loses nothing Kiln could have opened. B: every platform
  (one rule for portable projects; but a Unix file named `a:b` becomes unopenable and is skipped in listings, a Linux
  behaviour change touching `ws.*` tests). **Rec. A.**
- **Q-2 (reserved names, trailing dot or space).** A: allowed and stated (2). They are ordinary files under `\\?\` (PROBE
  I), and refusing them makes `aux.rs` in a real project unopenable. B: refused as `invalid_path` on Windows. **Rec. A.**
- **Q-3 (case and short names).** A: stated as limits (3), like T-093's macOS row 4. B: on Windows the walk refuses a
  spelling other than the stored one (`not_found`); that is `disk.rs`, shared with T-092, so in series and about
  +300,000. C: the open-documents table compares paths case-insensitively on Windows (`kiln-commands`, the same file's
  identity is still missed for short names). **Rec. A**, with a draft for B if the owner wants the stronger rule.
- **Q-4 (fail on base).** Windows behaviour passes on Unix base, so the test author's file holds only guards (as T-073
  y1, T-074) and no `acceptance_t095.rs` exists. Fail-on-base is the probe record (AC-W1). Whether `cargo xtask join`
  accepts a join with zero acceptance targets is not checked here (T-073 and T-074 each had one). A: as written; the
  director confirms the join tool. B: an `acceptance_t095.rs` reading `path.rs` as text for the `#[cfg(windows)]` rule
  (brittle, golden rule 9). **Rec. A.** (Superseded by ruling 4, D-272: `acceptance_t095.rs` with the document
  statements; applied at the spec check in `touches` and AC-D1 to AC-D3.)
- **Q-5 (order with T-093).** A: T-093 first, then this task, beside T-092. B: this task first and T-093 re-checked.
  **Rec. A.**

### Spec check (planner, R-386, by runs in the scratch clone `/home/user/kiln-plan-T074`, branch `t095` = main `4eb349d` + the prototype of Pinned (g) and draft sentences of (i) in the three documents; not committed)
- **S-1 Shared with tasks in flight:** none by path (Collisions). No trait, type or public signature changes
  (`RelPath::new` keeps its signature; `PathError` unchanged).
- **S-2 Main's tool on the candidate (D-235):** no file under `xtask/` changes, so main's tool is the candidate's. No
  gate rule changes, so the tree uses none. Run on the prototype tree (`cargo xtask <step>` in the clone):
  - `PASS  no-skip: … 206 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending)` (the new
    unit test carries no platform cfg; the `#[cfg(windows)]` statement is in product code, not a test);
  - `PASS  hygiene: reviews: 64 of 76 … ok; session-paths: 716 tracked file(s) clean`;
  - `PASS  scorecard: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned.`;
  - `PASS  layers: 11 crates, 14 internal edges, layer table holds`;
  - `PASS  ui-commands: self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)`;
  - `PASS  schema: 30 commands; audit ok; docs/commands.json current`.

  The real candidate adds `guard_t095.rs` (no platform cfg): `no-skip` must stay at the same count. The builders
  repeat the six steps with main's tool before the join.
- **S-3 Criteria of absence or limit:** (1) is an equality over the stated set. The variant to reject is `>` accepted
  (unsampled at base), and also `CON` refused (a set wider than (1)).
- **S-4 Main's tests naming each path (D-249), run on the prototype** (`grep -rlE '<path>' --include=*.rs crates apps
  xtask` on `4eb349d`, then the hits run):
  - `path.rs` → `xtask/tests/guard_t079.rs` (pins the three `path.rs` test names): **6 passed**.
  - `xtask/tests/acceptance_t079.rs` (registry; T-079's 20, untouched): **2 passed**.
  - `xtask/tests/acceptance_t074.rs` (registry; T-074's nine, untouched): **2 passed**.
  - `kiln-workspace --lib` (the prototype's new unit test and the three old `path.rs` tests among them): **59 passed**.
    The command was `cargo test -q --locked -p xtask --test guard_t079 --test acceptance_t079 --test acceptance_t074 -p
    kiln-cli --test acceptance_t008_cli -p kiln-workspace --lib`, 0 failed in every binary.
  - `paths.md`, `control-protocol.md` → `apps/kiln-cli/tests/acceptance_t008_cli.rs` (`:549-578`: the lexical bullet and
    the save bullet verbatim, `outside_root`/`not_a_file` present, no claim that links are followed outside), with the
    prototype's Windows sentence appended to the lexical bullet: **9 passed**.
  - The registry's quoted sentences (a script over `xtask/platform-tests.toml` and the three documents, whitespace
    squashed): all 7 `paths.md`, 4 `project.md` ("not valid UTF-8 or contain") and 3 `control-protocol.md` quotes
    present after the prototype edits (and `no-skip` PASS above, which checks them).
  - `project.md` → five files, comments or `project.rs`'s own docs. No main test names `guard_t095.rs`.
  - Searched for names this rule would refuse in existing tests: `grep -rn 'RelPath::new("[^"]*[:<>|?*]' --include=*.rs
    crates apps` gives no hit. JSON `"path"` strings in other crates' tests are not searched; they run on Windows only
    after T-092/T-098, whose re-checks cover them.
- **S-5 Unmeasured and marked:** `>` and U+0002 to U+001F at base; the spelling after a save under another case; a
  short name on `C:`; names with refused characters from another system. None decides a choice, so there is no
  probe; the test author's first CI run measures the two `MEASURED:` items.

## Note from T-074's review (director, D-270)
`docs/control-protocol.md:145` cites "`awa3_*`" and `docs/protocol/project.md:82` cites "`awa1_*`" for run
44307677646, which did not yet hold the two tests added later (`reviews/T-074-r2.md`, the nit). This task, which
edits the same documents, names run 44358453526 (all 15 guards; `tasks/ci-runs/T-074-round-1.md`) there.
(Done in Pinned (i), last item.)

### Rulings (director, D-272)
- **1: Windows only.** The lexical refusals apply on Windows; a Unix file named `a:b` stays openable. The documents
  say that a project moved to Windows cannot reach such a name there.
- **2: allow.** Reserved device names and names ending in a dot or a space are allowed and stated (they are ordinary
  files under the paths Kiln uses; a project with `aux.rs` stays usable).
- **3: limits.** Case and 8.3 short names are stated limits with the two CI measurements, not a stricter check on
  the walk.
- **4: one acceptance test is required.** `cargo xtask join` refuses a task with no acceptance test
  (`xtask/src/join.rs:259`), and the methodology wants one that fails on base. The test author writes
  `crates/kiln-workspace/tests/acceptance_t095.rs`: the criteria that can fail on base on every platform, which are
  the document statements this task must add (the Windows names paragraph; the two citations of D-270 naming run
  44358453526), read from the repository's files, as T-073's manifest test was (ruling y1 of T-073). The runtime
  tests stay guards in `guard_t095.rs`. The planner adds the file to `touches` and the criterion to the spec at the
  spec check.
- **5: T-093 first.** `depends_on` T-074 and T-093; this task's sentences are re-checked beside T-093's when it
  has merged. T-092 runs beside both (no shared path).

### Spec check (fresh planner)
Read against main `76ccb9f` (ledger-only since `4eb349d` and `53be751`: `git diff --stat 53be751 76ccb9f` touches only
`docs/decisions.md` and five task files; `path.rs`, `disk.rs`, the three documents unchanged), the branches `task/T-093`
(`234f98c`) and `task/T-092` (`cbe3d21`, `79d5b76` on `-tests`), and D-235, D-249, D-256, D-266, D-267, D-270, D-272,
D-273. Runs in a clone `/home/user/kiln-check-T095` of the primary (at `53be751`), uncommitted, `target/` deleted at the
end. The sketch there: Interface (1) as one `#[cfg(windows)]` statement after the backslash check in `RelPath::new`
(`path.contains(|c| matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*' | '\u{1}'..='\u{1f}'))`), the unit test of (5),
the Pinned (i) sentences in the three documents, and two sketch test files `acceptance_t095.rs` (three document tests)
and `guard_t095.rs` (the character equality). **Verdict: ready after rulings** — one blocker (Q-A), 15 corrections made
in the text above (listed under each point, marked "corrected (spec check)"), seven questions.

**1. The refusal itself.** VERIFIED unless marked.
- *One gate.* Every path from outside becomes a `RelPath` through `RelPath::new` only (`RelPath(String)` is private;
  the other constructors are `root_placeholder()` = `(root)` and `parent()`, a prefix of a valid path:
  `path.rs:56,66-78`). The places that make one from outside input (`grep -rn "RelPath::new" crates apps`, product code):
  `doc.open` (`kiln-commands/src/doc.rs:235`) and `project.tree` (`kiln-commands/src/project.rs:230`) — which carry
  every control-protocol, `kiln-cli exec`/`run` request and, through `doc.open`, the desktop app's start paths and Path
  box (`apps/kiln/src/main.rs` and `kiln-ui` hold no `RelPath`; `kiln-ui/src/lib.rs:89`); the `--memory-seed` manifest
  (`apps/kiln-cli/src/main.rs:256`); listings (`disk.rs:626-631`) and the project tree (`kiln-workspace/src/project.rs:243-259`,
  `child` of a listed name). The two file systems share `RelPath`, so on Windows `--memory`/`--memory-seed` refuse the
  same paths (corrected (spec check): stated in Interface (1)).
- *Listings.* A name read back from `read_dir` that fails `RelPath::new` is skipped and counted
  (`disk.rs:626-631`), never an error: a listing of a directory holding such a name succeeds without it. Interface (4)
  says so. Whether NTFS can hold such a name at all: unchecked (Interface (4)). Its example "WSL's own names" is weak:
  WSL is commonly said to store such characters as private-use code points, which Kiln would list (INFERRED, not
  measured) — the example is not in any document sentence, so no correction; the clause stays "unchecked".
- *Link targets.* On Windows no target is read: every reparse point on the walk answers `OutsideRoot` before
  `read_link` (`disk.rs:337-340`, before `:350`). On Unix target components become walk steps, not `RelPath`s, and are
  unchanged. Nothing to state.
- *`--root`.* A host path, never a `RelPath` (`apps/kiln-cli/src/main.rs:122,412`; `DiskFs::new(&Path)`,
  `disk.rs:210`): `C:\proj` stays a valid root. `paths.md:3-4` already defines "path" as a request's `path`, so the new
  paragraph cannot be read as covering `--root`. Two implementers agree (the rule is in `RelPath::new` only).
- *`Path::join` before `RelPath`.* None: the walk joins one segment at a time (`disk.rs:332` `dir.join(&name)`, the
  segments of `RelPath::segments`), and `dir` is the verbatim canonical root. At base `C:x` makes `dir.join` return
  `C:x` (PROBE I10) and `fs::symlink_metadata("C:x")` is then called outside the root (`disk.rs:333`; the effect, a
  lookup on another drive, is INFERRED from the code, not measured) before `inside` refuses it — the Goal's "the file
  system outside was consulted" is right. After the rule a segment holds no `:`, `\` or `/`; with no `:` and no
  separator, `std` on Windows parses no prefix or root in a component (INFERRED from `std`'s documented prefix forms,
  `\\?\`, `\\.\`, `\\server\share`, `X:`, all needing `\`/`/` or `:`), so every join appends.
- *Does refusing `:` refuse something a user needs?* Not a file: Win32 cannot make a name with `:` (PROBE I1: every
  write of a `:` name failed or left the root). A request a Windows user may type — an absolute `C:\x\a.rs` or
  `C:/x/a.rs` — changes kind: base `outside_root` (`C:x`, I1-I3) or `invalid_path` (`\`), after `invalid_path` for
  both. Precedence says lexical first, so this is the specified answer; no test on main asserts `outside_root` for a drive
  form (`grep -rnP '"[A-Za-z]:[^\\"]' crates apps --include=*.rs`: only message-like names, see Q-A).
- *Separators and segments.* `a\b` is refused as one invalid path on every platform (`path.rs:42-44`; unchanged, the
  spec says "as `\` and NUL are now"); `/` is the only separator. `/`, `/a`, `a/`, `a//b`, `.`, `..`, `a/.`, `a/../b`,
  `""`, `a\0b` are refused today (`rejects_invalid_paths`, `path.rs:110-118`, in the 59 passing on Linux). The new
  statement adds nothing for them. Order inside the lexical check is unobservable (one kind; reason text unpinned).
- *Unicode look-alikes* (U+FF1A fullwidth colon, U+2236 ratio, U+FF1F, U+FF0A, U+FF1C, U+FF02): accepted by the sketch on
  every platform (the `guard_t095` sketch's equality, below). Rust's `std` calls the wide (UTF-16) Windows API, so no
  best-fit mapping turns them into `:` or `?` (INFERRED). The spec's equality covered U+0000-U+00FF only, so an
  implementer refusing look-alikes (for example by a Unicode category) would pass it: corrected (spec check): the
  guard's equality adds these six as accepted everywhere.
- *Two implementers.* With the exact set of (1), the equality over U+0000-U+00FF plus the look-alikes, and "no
  platform cfg on the test fn", two implementers build the same statement. Unspecified and harmless: where in the
  lexical sequence it sits, the reason text.
- *What the rule breaks elsewhere* (D-249; **blocker, Q-A**). With the sketch's statement under `#[cfg(all())]` in
  place of `#[cfg(windows)]` (so Linux shows what Windows will do), `cargo test --workspace --locked --no-fail-fast`:
  140 result lines, failures only in `path::tests::windows_name_characters` (expected) and in four other targets:
  `kiln-cli` bin `tests::manifest_refusals_name_the_problem` (`main.rs:1216`, a manifest path `b,{"`),
  `kiln-commands --test acceptance_t032` (`lower_layer_errors_map_to_pinned_reasons`, `refusals_carry_reasons_at_their_sources`:
  fixture `missing: not found`, request `x: is a directory, not a file`), `--test acceptance_t032_json`
  (`refusals_serialize_their_reason`) and `--test guard_t032` (`refusal_messages_and_codes_are_unchanged`). All four exit
  0 on Windows at base (`tasks/ci-runs/T-092-probe-1.md:49,50,57` and `:124` `kiln-cli:--bins=0`). None is in this
  task's, T-092's, T-098's, T-100's or T-099's `touches`. The spec's S-4 grep (`RelPath::new("…[:<>|?*]…")`) missed
  them because the names reach `RelPath::new` through helpers (`files_ctx`, `p(…)`, JSON params). They stay hidden in
  CI until T-098 (cargo stops at build errors), then T-099 would meet them as failures no task caused.
  `kiln-control`'s `"x: not found"` (`acceptance_t032_control.rs:42`) is formatting only and passed.

**2. Sentences resting on probe lines** (`tasks/ci-runs/T-074-probe-1.md`, lines 150-245; the probe source read at
`de8106f:crates/kiln-workspace/tests/probe_t074.rs:418-552`).
- Header: "`PROBE I0` only for a name it refuses, and the record has no `I0` line" — VERIFIED (source `p("I0", …)` in the
  `let Ok(rel) = r else` arm; record lines 153-230 hold I1-I3 for all 26 names). B4 (line 68): canonical root
  `\\?\D:\…` — VERIFIED.
- Goal bullets: I2 `read "f.txt::$DATA": Ok([102])` (199), I3 `Ok` (200), `C:x`/`C:`/`D:x` `OutsideRoot` (201-209), I10
  `"C:x"` (241), `<?*|"` and U+0001 os error 123 on all three calls (210-227) — VERIFIED as cited.
- Interface (1) "every `:` form … reached a stream or another drive": wider than the lines (`x/C:y` is `NotFound`,
  line 228-230, because `x` does not exist; the walk never joined `C:y`) — corrected in the text.
- Interface (2) "PROBE I1-I3 `Ok` for each": true for the 13 names sampled (153-191); `aux.rs`, `COM2`-`COM9`, a
  `Document` save were not sampled — corrected in Interface (2), AC-W2 and Pinned (i) item 2. "a non-`\\?\` write to `a.`
  made a file `a`" — VERIFIED (I11 line 244, I12 line 245 lists `"a"` where I5, line 232, had none). The non-verbatim
  `CON` write answered `Ok` (243) and made no file (I12 has one `CON`): it went to the console device (INFERRED).
- Interface (3): I7 (234-235) `F.TXT` read `Ok([102])` and check `Ok` — VERIFIED; I8 `AVERYL~1/g.txt` `NotFound` (236);
  I9 `C:` "8dot3 name creation is ENABLED" (237, line cut by `…` in the artefact itself), work volume `D:` DISABLED (239)
  — VERIFIED. That `std::env::temp_dir()` is on `C:` is in no record (INFERRED from the runner's usual `TEMP`): the
  short-name test must print the drive it used (Q-D).
- Pinned (g) examples: `nul.rs` `Ok` (162), `read "C:x"` `OutsideRoot` (202), `x<y` os error 123 (210), `F.TXT` (234) —
  VERIFIED. Prototype claims re-run: `path::` unit tests pass (59 in `--lib`), Windows clippy clean (point 4).
- AC-W1 names the ten forms with no `I0` line — VERIFIED (each in the probe's list).
- *Not sampled at base: `>` and U+0002-U+001F.* At the file system only. For the criterion that matters — `RelPath::new`
  refuses them on Windows after, accepts them at base — base needs no probe: `path.rs` has no platform code at base, so
  Windows base behaviour equals the Linux run (the sketch's unit test passes on Linux asserting acceptance). So AC-W1's
  fail-on-base holds for them by that run; what stays unchecked is only the document clause "the characters Windows does
  not allow in a file name" for `>` and U+0002-U+001F (read from Microsoft's naming rules, not run). Acceptable if the
  sentence's evidence line says so (it does not yet: Q-E offers a measurement for about 10,000 tokens).

**3. Acceptance criteria.**
- *Ruling 4 not yet applied* (blocker for the join, fixed): the spec had no acceptance test, and `cargo xtask join`
  stops with `no_acceptance_tests` (`xtask/src/join.rs:255-262`) — corrected (spec check): AC-D1 to AC-D3 added under
  "Through the join", the file in `touches` for the test author (point 6). Each fails on base on every platform (a file
  read; measured on Linux with the sketch: 3 failed at base, 3 passed with the text). Their oracle is the Pinned (i)
  wording, so the implementer must copy items 1 and 2 and the D-270 replacements exactly (they already say "worded to be
  copied"). Pinning these sentences makes the next task that rewords them change `acceptance_t095.rs` (the D-249 kind):
  acceptable, as `acceptance_t008_cli.rs:549-575` does the same for the lexical bullet.
- *Testable, fail on base:* AC-1 (reviewer's IR comparison; not re-run here: every changed product line is
  `#[cfg(windows)]`, the test fn is `#[cfg(test)]`), AC-2 (run, point 4), AC-3 (run, point 4: the count stays `35 (10
  pending)` also with the two new test files present) are local procedures, not fail-on-base tests. Windows-only AC-W1
  fails on base by the argument of point 2 (no platform code in `path.rs` at base) plus the probe lines for the file
  system's answers. AC-W2 and the reading half of AC-W3 hold at base (guards). AC-W4 fails on base by construction
  (the test does not exist).
- *Absence as equality:* (1) is exact: the set, plus "U+0000-U+00FF except `/`, one at a time in `a<c>b`", refused
  exactly for (1) ∪ {`\`, NUL} on Windows and {`\`, NUL} elsewhere. Variants outside the list it rejects: `>` accepted
  (S-3), `CON` refused, U+007F or U+0080-U+009F refused (an implementer using `char::is_control`), and now a look-alike
  refused (added, point 1). The sketch of this equality passes on Linux and is clippy-clean for Windows.
- *Guards in `guard_t095.rs`:* no platform cfg (D-245); expectations by `cfg!`; must pass on Unix base (the join runs
  guards on base on Linux: `join.rs:251-253`). On Windows they assert (1) the refusals (fail at base by design: a guard
  is required to pass on base only where the join runs it), (2) Kiln's answers for reserved and trailing names, (3) the
  case answers. The (3) expectations were chosen by `cfg!(windows)`, which sends macOS down the Linux branch although
  its default volume folds case (T-093 row 3, `macos_t093.rs` header) — corrected to `cfg!(target_os = "linux")`.
- *The two measurements:* the spec says what is printed, not what is asserted when the value cannot be predicted or
  the measurement cannot be made (no 8.3 name on the volume of `temp_dir()`, `temp_dir()` not on `C:`, `fsutil`
  failing). Two test authors would differ (one asserts the short name reaches the file and goes red on `D:`). Q-D
  gives a rule. Also on Linux and macOS the spec does not say where the short-name fixture lives
  (`std::env::temp_dir()` is `/tmp` there, outside `CARGO_TARGET_TMPDIR`): Q-D.
- *Marking:* "to be measured by the test author on CI" is not one of §5.7's marks (implementer, director, reviewer,
  "unchecked"); the implementer, who writes the sentence, cannot see the guard before the join. Q-C.

**4. Type-check and run** (clone at `53be751`; one build at a time; stand-in C compiler as `reviews/T-073-r1.md:50-57`,
`CC_x86_64_pc_windows_msvc=<stub-cc>`, `AR_x86_64_pc_windows_msvc=llvm-lib`):
- `cargo test -q --locked -p kiln-workspace --lib` → `59 passed; 0 failed` (the three old `path.rs` tests and the new
  one among them; on Linux the new one asserts acceptance).
- `cargo clippy -p kiln-workspace --lib --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings` → exit 0,
  no stub needed (AC-2's claim holds); `cargo clippy -p kiln-workspace --lib --locked --target x86_64-pc-windows-msvc --
  -D warnings` → exit 0; with the stub, `cargo clippy --workspace --lib --bins --locked --target x86_64-pc-windows-msvc
  -- -D warnings` → exit 0, no warning; `cargo clippy -p kiln-workspace --test guard_t095 --test acceptance_t095 --locked
  [--target x86_64-pc-windows-msvc] -- -D warnings` on the sketch files → exit 0 on both targets.
- Main's tool on the sketched tree (D-235), all `PASS`: `no-skip: … 206 files scanned, 0 allowlisted skip(s), 35
  platform test(s) registered (10 pending)` (with the two sketch test files: `208 files scanned`, same count);
  `hygiene: reviews: 64 of 76 … ok; session-paths: 716 tracked file(s) clean`; `scorecard: 64 items: 47 verified, 0
  implemented, 0 failing, 17 planned.`; `layers: 11 crates, 14 internal edges`; `ui-commands: self-test ok; 14 kiln-ui
  file(s) scanned, 5 allowlisted path(s)`; `schema: 30 commands; audit ok; docs/commands.json current`.
- Main's tests naming the touched paths (D-249): `cargo test -q --locked -p xtask --test guard_t079 --test
  acceptance_t079 --test acceptance_t074 -p kiln-cli --test acceptance_t008_cli -p kiln-workspace --lib` → 6, 2, 2, 9,
  59 passed, 0 failed. The registry's quotes in the three documents (a script over `xtask/platform-tests.toml`,
  whitespace squashed): 14 of 14 present after the sketch's edits (7 `paths.md`, 4 `project.md`, 3 `control-protocol.md`).
- The D-249 search the spec's S-4 did by grep, done by run instead: the rule under `#[cfg(all())]`, `cargo test
  --workspace --locked --no-fail-fast` → the four targets of point 1 (Q-A). Afterwards the statement was put back under
  `#[cfg(windows)]` for every other run here.
- Not run: anything on Windows; the LLVM-IR comparison of AC-1 (the reviewer's); `cargo xtask ci` as a whole; the join.

**5. Documents** (each sentence of Pinned (i), placed in the sketch; quotes and pins re-run above).
- Item 1 (`paths.md` "Lexical refusals"): measured for `:` and five characters (I1-I3, I10), the refusal by
  `guard_t095` on Windows; `>` and U+0002-U+001F as Windows' rule read, not run — its evidence line now says so; heading
  "(unchanged)" dropped (corrected). Placed after the existing paragraph, it does not touch the bullet
  `acceptance_t008_cli.rs:562-565` pins (that bullet is in `control-protocol.md`).
- Item 2 (reserved names): measured except `aux.rs` itself (guard, CI) — corrected. "Other Windows programs may not open
  them": I11/I12 show one such program behaviour (`a.` written as `a`); "may" keeps it within that.
- Item 3 (case and short names, "The rule"): two placeholders; their marking is invalid (Q-C) and what the tests
  assert is open (Q-D). Nothing in it contradicts T-093: T-093's `macos_t093.rs` expects folding on Windows too ("as
  macOS", its header line 6, an inference there), and its row 3 test writes with `std::fs::write`, so the listing keeps
  the first spelling; this item's spelling is after a Kiln save (a rename over the file), a different operation — the
  sentence must say "after a save by Kiln", which it does ("after the save").
- Item 4 (`project.md` `skipped`): "unchecked" as written. After T-093 the bullet at `:26` is six lines longer and ends
  with T-093's macOS clause; appending the Windows sentence after it keeps the quoted words (shown on the sketch at
  `53be751`; to be re-run on T-093's merged text). `:99` unchanged by T-093.
- Item 5 (`control-protocol.md`): placed as a continuation of the lexical bullet in the sketch; `acceptance_t008_cli`
  (squashed `contains`) passes either way (9 passed). Whether a continuation or its own bullet is left to the
  implementer; no test pins it.
- D-270: exact replacement text added (it was missing; AC-D3 needs it); evidence `T-074-round-1.md:78,216-233`.
- T-093's "Platforms" bullet ("names and case on Windows are not yet stated (T-095)"): not in the spec; a stale clause
  after this task — added as an item of Pinned (i) (corrected).
- Not in `touches`, stale after this task: `docs/adr/0009-platforms.md:111` ("not refused today and meaningful to
  Windows: … `<>:"|?*`"). An ADR records its decision date, so no change is proposed; noted for the director.
- Not stated anywhere, and outside (3) as written: on Windows git treats `.gitignore` patterns and `target` without case
  (`core.ignorecase`, set by `git init` on a case-insensitive volume: known, not verified here), while Kiln matches by
  bytes, so `project.md`'s equality with `git ls-files` (`:88-91`) may not hold for a pattern and a name differing in
  case. Q-F.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · draft · did: cut from T-074 at its re-specification under v1.5 · next: director (T-074 Q-1)
- 2026-11-18 · R-386 · draft · did: full specification under v1.5 against main `4eb349d`. Read: T-074's merged
  `disk.rs`, `paths.md`, `project.md`, `control-protocol.md`, `guard_t074.rs`, the records `T-074-round-1.md`,
  `T-092-probe-1.md`, D-256 to D-271, and T-092's and T-093's specs. Ran in the scratch clone (branch `t095` from the
  primary's main): the prototype of Pinned (g) (Windows clippy clean, `path::` 4 passed); main's six `xtask` steps PASS
  (S-2); main's pinning tests 6/2/2/9/59 passed (S-4); the quote script (S-4). Prototype reverted, `target/` deleted.
  No probe: no choice hangs on unmeasured behaviour; two items are to be measured by the test author on CI (Interface
  (3)). Tier high (confinement boundary; no gate path). Pinned (h) n/a. Not run: anything on Windows; the join tool
  with zero acceptance targets (Q-4) · next: director (Q-1 to Q-5), fresh spec check
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
