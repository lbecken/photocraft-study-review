---
id: ADR-0009             # file docs/adr/0009-platforms.md
title: "Kiln is built and tested on Linux, macOS and Windows; evidence for macOS and Windows comes from CI jobs on hosted runners, and a platform job counts for main's colour once it has been green"
status: accepted
date: 2026-11-15
task: null               # a plan, carried by the tasks it orders (T-069 to T-078)
charter_section: "§2 the product (platforms, new sentence); §4 the gate (identical locally and in CI) and CI; METHODOLOGY §8 (main green), the no-skip rule (D-166 ruling b8)"
approval: D-175
supersedes: null
superseded_by: null
---

# ADR-0009 Linux, macOS and Windows

Written by a planner (run R-255, booked to `iter-2`) on the owner's instruction D-171, with the
owner's ruling D-173 (no signed installers for now). Read at main `30917a53e264a17b33c1e429ef52b672c57a6470`
(the brief named `eefd9d0`; one ledger commit later, D-171); main moved to `60c20fa` during the run
(D-172, D-173, ledger only; no code path changed, checked with `git diff --stat 30917a5 60c20fa`).
Labels: **VERIFIED** (a command was run here and printed it; commands in "Evidence"), **KNOWN**
(the planner's own knowledge or a documentation page read through a summarising fetch; not checked
by a command), **INFERRED** (reasoning from verified facts), **PROPOSED** (this decision).

## Context

### What PhotoCraft did and what it cost (photocraft-study, data under study)
- Same toolkit family (egui on winit); UI crates (`egui`, `eframe`, `winit`, `rfd`) banned below the
  UI layer (`deliverables/03-architecture-and-process.md`, E04-002). Kiln has the same ban
  (`xtask/layers.toml` `headless_forbidden`).
- CI layers (`deliverables/06-testing-ci-release.md` §1.2): pull-request CI on Linux only; Windows
  `unit`/`integration` legs only when a path regex matched or a `full-ci` label was set; FreeBSD and
  Windows ARM64 nightly; macOS removed from PR and main CI and built **only** in the release workflow
  (row L11a; bb3108cfd2: "5 macOS slots in total"). A nightly release dry run of every release build
  except macOS (row L12, f1b5e7d727).
- The price (§3.2 "Cost-control moves and the regressions they caused"; `07-failures-productivity.md`
  F3, F10): three Windows-only reds on main (ee3ebfa748 "Windows CI red since #1882", 184c226a76,
  79a33dae9c "PR ran Linux only"); macOS breakage found only on release day (0.15.0 → 0.15.1 stapling,
  §6.3); at least three of 26 tagged releases needed a platform fix that no PR gate could catch (§6.3,
  INFERRED there). The study's own finding: "The economies that removed a *platform* … each caused at
  least one later red main or release-day failure. The economies that kept every case but ran it more
  cheaply … caused none" (§3.2).
- Release (§6.2): signed installers for every platform with 15 signing secrets (Apple certificate and
  notarisation, Windows certificate and Azure signing, a Linux key). For Kiln, D-173 puts signing and
  packaging out of scope.

### Kiln today
- The charter names no platform (`docs/CHARTER.md` §2, §3). `crates/kiln-workspace/src/disk.rs:1`
  says "supported platforms: Unix only, decision D-014"; `docs/protocol/paths.md:114` ends "Unix only";
  `docs/protocol/ui.md:53` "Linux is what the gate and the evidence of T-055 cover" and `:67` "`kiln`
  opens a window on X11 (not Wayland)". The windowing crate is built with `default_fonts`, `glow`, `x11`
  (`Cargo.toml`, `eframe` line). D-166 ruling b8 accepted a test file that compiles only on Unix,
  "since the gate and CI run on Linux". CI is one job on `ubuntu-24.04` (`.github/workflows/ci.yml:18`).
- **Main is red in CI, and has been for about 36 hours** (VERIFIED, e5 below): every run of `ci.yml` on
  main from run 257 (`47135bc`, 2026-11-14 04:34Z, the first push after T-036 merged at `1a808c9`) to
  run 353 (`93ad8ed`, 2026-11-15 16:16Z) concluded `failure`; the last success is run 256 (`805ebcb`).
  The failing step is `scorecard`: three tests of `ws.path-confinement`
  (`kiln_cli::acceptance_t036_cli::unreadable_root_is_refused_at_start`,
  `kiln_workspace::acceptance_t036_workspace::unreadable_root_is_refused_by_new`,
  `kiln_cli::guard_t036_removal_cli::root_made_unreadable_after_start_stays_protected_through_control`)
  fail on the runner, and the page is then stale. These tests re-run themselves as uid 65534 from uid 0
  (`acceptance_t036_workspace.rs:7,38,47,78`); this machine runs as uid 0, the hosted runner does not
  (KNOWN: the runner user is an ordinary user with `sudo`). They are the family of the deferred
  draft T-052 (tests that need a checkout another identity can reach; `evaluations/iter-2.md` NEW-3),
  which D-147 deferred "until what they remove starts to cost": on CI it has cost every run since. No decision row, incident or retrospective
  records it (`ls incidents`: absent; `retrospectives/iter-2-stage-1.md` "Red-main minutes: 0 recorded").
  INFERRED: the process has no step in which anyone reads a CI result. Evidence that can come only from
  CI (macOS, Windows) is worth nothing until that step exists. This is the first thing to fix, and it is
  a Linux identity assumption, the same kind of fault the platform work is about.

### Evidence: the feasibility probe (R-255)
Scratch directory `…/scratchpad/platform-probe/` (outside the repository), a `git archive` copy of
main `30917a5` (`ws/`) and a clone with `core.autocrlf=true` (`crlf/`); toolchain 1.97.0; every command
under `timeout`; logs in `logs/`. Nothing was installed on the system; four Rust standard libraries
were added to the existing toolchain with `rustup target add` (left installed). Builds deleted at the
end. Disclosure: the brief's limit of 8 GB in scratch was exceeded — the full gate in `crlf/` (c2) built
9.2 GB beside 4.1 GB in `ws/target`, 13.3 GB at the peak (`du -sh` before deletion); 27 GB were free.

| # | Question | Command (abridged) | Result |
|---|---|---|---|
| a0 | Targets installable | `rustup target add --toolchain 1.97.0 x86_64-pc-windows-msvc x86_64-pc-windows-gnu aarch64-apple-darwin x86_64-apple-darwin` | all four installed, 16 s (VERIFIED) |
| a1 | Baseline | `cargo check --workspace --all-targets --locked --keep-going` (host) | exit 0, 66.8 s cold (VERIFIED) |
| a2 | Cross check as the machine is | same with `--target <t>` for each of the four | exit 101 on all four: the C code of `tree-sitter 0.27.1` and `tree-sitter-rust 0.24.2` does not compile: msvc `failed to find tool "lib.exe"`; gnu `failed to find tool "x86_64-w64-mingw32-gcc"`; darwin `cc … -arch arm64` fails. `clang --target=<t>` on `tree-sitter/src/lib.c`: `fatal error: 'stdio.h' file not found` for `arm64-apple-macos11`, `x86_64-pc-windows-msvc`, `x86_64-w64-mingw32` — no SDK, CRT or mingw headers here (VERIFIED). **Missing cross toolchain, not a Kiln fault.** Without it nothing can be linked or run for macOS or Windows here; no Wine, no mingw (`which`, VERIFIED) |
| a3 | Rust code past the C stage | as a2 with `CC_<t>` = a stub that compiles an *empty* C unit for the target (`bin/stub-cc`) and `AR_<t>` = `llvm-lib`/`llvm-ar`; `cargo check` never links, so the stub changes no Rust diagnostic | Windows (msvc and gnu alike): **`kiln-workspace` (lib) fails with 6 errors**, all in `crates/kiln-workspace/src/disk.rs` (VERIFIED, `logs/a3-*.log`): `:86` `use std::os::unix::fs::{MetadataExt, OpenOptionsExt}`; `:189` and `:213` `dev()`/`ino()` (the root's identity); `:361` `OpenOptions::mode` (the temporary file's mode). Everything above `kiln-workspace` is then not checked. The windowing crate and its Windows dependencies (`windows-sys`, `egui-winit`, `eframe`) check without error with `x11` among the features (`logs/a2-x86_64-pc-windows-msvc.log`) |
| a4 | The rest, with a probe-only stand-in for those 6 lines | `disk.rs` given, in the scratch copy only, `#[cfg(not(unix))]` stand-ins for `dev`, `ino`, `mode` (`logs/probe-shim.diff`, 13 lines); same check | **macOS, both targets: exit 0 — every crate, binary and test target compiles** (VERIFIED). **Windows: every library and binary compiles** (`kiln-ui`, `kiln-control`, `kiln-commands`, both apps, `xtask`); **198 errors in 33 files, all in test code**: 29 integration-test files and the test modules of `disk.rs`, `kiln-commands/src/doc.rs`, `xtask/src/hygiene.rs`, `xtask/src/publish.rs` (`logs/a4-win-by-file.tsv`). Kinds: `std::os::unix` not found (70), `Permissions::from_mode` (24), `FileType::is_fifo` (19), `Metadata::ino`/`dev`/`uid`/`gid`/`mode` (52), `is_socket`/`is_char_device` (9), `OsStr::from_bytes`/`OsString::from_vec` (non-UTF-8 names, 8), `Command::uid` (6), and one consequence of a failed import (6) |
| a5 | Lints | `cargo clippy … -- -D warnings` for `aarch64-apple-darwin` `--all-targets`, and for `x86_64-pc-windows-msvc` `--lib --bins` (with a4's stand-in) | both exit 0 (VERIFIED) |
| a6 | A portable root identity | scratch copy: `same-file = "=1.0.6"` added to `kiln-workspace`, a function comparing `same_file::Handle`s; `cargo check -p kiln-workspace --lib` for the host and for `x86_64-pc-windows-msvc`; `diff` of `Cargo.lock` | both compile; the lock gains one line (`"same-file"` in `kiln-workspace`'s dependency list) and **no package and no version** — `same-file 1.0.6` is already in the lock through `ignore → walkdir` (VERIFIED, `cargo tree -i walkdir --target all`). Reverted afterwards |
| b1 | Unix APIs in product code | `grep -rn "std::os::unix\|cfg(unix)" --include=*.rs crates apps xtask` minus `/tests/` | product code: **only `disk.rs:86`** (the rest is inside `#[cfg(test)]` modules: `disk.rs` from `:535`, `doc.rs` from `:1312`, `registry.rs` from `:965`, `hygiene.rs`, `publish.rs`). 37 files in all, 31 of them under `tests/` (VERIFIED) |
| b2 | Items whose tests cannot build on Windows today | script over `scorecard/items/*.toml` (`logs/b-items-win.txt`) | every item with a test in `kiln-workspace` or a crate above it: **38 of the 44 tested items** (only `text.*`, `undo.*` items of `kiln-text` — 6 — are untouched); of these, 8 items list a test in a file that does not compile on Windows even with a4's stand-in: `ws.path-confinement` 28/28, `ws.open-project` 7/7, `ui.window` 7/7, `syntax.outline` 4/4, `nav.workspace-symbol` 3/3, `nav.goto-line` 2/2, `nav.goto-symbol` 1/1, `control.errors` 1/6 (VERIFIED) |
| c1 | Line endings on checkout | `git clone -c core.autocrlf=true` of main; `cargo xtask fmt`, `cargo xtask schema`, then `cargo xtask ci --keep-going` | every one of 574 tracked files gets CR bytes; `fmt` passes; **`schema` fails: "docs/commands.json is stale"** (VERIFIED; it compares the committed file with the generated text byte for byte, `xtask/src/schema.rs:89`; `scorecard.rs:383` does the same for `docs/scorecard.md`). `core.autocrlf=true` is the default of Git for Windows and of the Windows runner image (KNOWN, not verified here) |
| c2 | The whole gate on that checkout | `cargo xtask ci --keep-going --report …` in `crlf/` (236.5 s) | `GATE FAILED … schema, scorecard, test`. Caused by the line endings (VERIFIED by running the same test in the LF copy `ws/`, where it passes): `schema` and `kiln_cli::acceptance_t020_cli::syntax_commands_are_read_only` ("a valid corpus file without CR": every corpus file now has CR). Not caused by them (fail in `ws/` as well): five `ws.path-confinement` tests that re-run as another identity and cannot reach a checkout below `/tmp/claude-0` (mode 0700) — the known T-052 (`evaluations/iter-2.md` NEW-3). `hygiene`, `fmt`, `clippy`, `no-skip` pass |
| c3 | Gate steps and the platform | reading `xtask/src/` | see (c) below |
| d1 | Runners | docs.github.com "GitHub-hosted runners" (fetched, summarised) | public repositories: "free and unlimited"; `ubuntu-24.04` 4 CPU/16 GB/14 GB x64; `windows-2025` (`windows-latest`) 4/16/14 x64; `windows-11-arm` 4/16/14 arm64; `macos-15` (`macos-latest`) 3 CPU (M1)/7 GB/14 GB arm64; `macos-15-intel` 4/14/14 x64; "During periods of high demand, jobs that run on macOS runners may take longer to start" (KNOWN, summarised page) |
| d2 | Runner images | `actions/runner-images` readmes, Ubuntu 24.04 and Windows 2025 (fetched, summarised) | Ubuntu image 20261111: `xvfb` installed; `libxkbcommon-x11-0`, `libxcb-xkb1`, Mesa not listed; Windows image 20261111: Visual Studio 2022 with VC++ tools, Git 2.56.1; no mention of a desktop session, a screen or OpenGL (KNOWN, summarised) |
| d3 | CI's own wall time today | `actions_list` of `ci.yml` (100 latest completed runs, `logs/d-ci-runs-p1.txt`) | 97 failures, 3 successes; the successes (runs 254–256, whole gate) took 347–490 s; failing runs stop at `scorecard` (median 181 s) (VERIFIED) |
| e5 | Is CI green? | the same listing; `get_job_logs` of runs 257 and 352 | see "Kiln today" (VERIFIED) |

What the probe did **not** establish: whether anything links or runs on macOS or Windows (nothing can
be linked here); whether the tests pass there; whether a window opens on a hosted macOS or Windows
runner; wall times of macOS and Windows jobs; the release build. Only a CI run can establish these.

### (b) What Kiln's code and documents assume about the platform

Per topic: what is promised now, what differs on macOS (M) and on Windows (W). M and W statements
are KNOWN (platform knowledge) or INFERRED from the code read here; none is VERIFIED, since nothing ran
there.

| Topic | Code / document now | macOS | Windows |
|---|---|---|---|
| Unix-only API in product code | `disk.rs:86` (b1) | none (a4) | 6 compile errors (a3) |
| Root identity | `(dev, ino)` of the held root, compared on every call (`disk.rs:189,213`; `paths.md` "The root") | same | no stable `dev`/`ino` in stable `std` (the Windows `file_index` accessors are unstable, KNOWN); `same-file` gives a volume-serial-plus-file-index identity (a6) |
| Holding the root open | `fs::File::open(root)` on a directory (`disk.rs:184`) | same | `std` cannot open a directory without the backup-semantics flag (KNOWN): as written, **every root would be refused at start** (INFERRED); an open handle may also stop the root being renamed or removed, which changes the "root replaced" sentences of `paths.md` |
| Lexical path rules | `RelPath`: `/`-separated; refuses `\`, NUL, `.`, `..`, empty, leading `/` (`path.rs:39-52`; `paths.md` "Lexical refusals") | same | not refused today and meaningful to Windows: a drive prefix in a segment (`C:`, `C:x` — `Path::join` replaces the base, so the walk jumps; the final inside-check still refuses it, INFERRED), alternate data streams `f.rs:s`, reserved device names (`CON`, `NUL`, `COM1` …, with any extension), trailing dot or space (stripped by Win32), `<>:"|?*` |
| Links | the walk reads symlinks and follows targets "as the kernel does", absolute targets from `/` (`disk.rs:256-299`; `paths.md` "Location") | same; `/tmp` is itself a link to `/private/tmp` (canonicalisation handles it, INFERRED) | symlinks need a privilege or Developer Mode; file vs directory symlinks; junctions and other reparse points; absolute targets carry a drive or `\\?\` prefix, which the walk turns into "restart from `/`" (`disk.rs:291`) — wrong for another drive (INFERRED); `canonicalize` returns `\\?\` paths, so prefix forms must agree before `starts_with` |
| Case | the inside test is a component-wise, case-sensitive comparison (`disk.rs:236`); "already open" is by path text (`paths.md` "The rule") | default file system is case-insensitive: `A.rs` and `a.rs` are one file, two documents (INFERRED) | same as macOS; also the comparison of an absolute link target with the root is case-sensitive while the file system is not (fails safe: refused as outside, INFERRED) |
| Special files | FIFOs, sockets, devices refused as `not_a_file` without opening (`paths.md`); tests make FIFOs with `mkfifo` | same | no FIFOs or sockets in the tree; device names reach devices; the sentence needs a Windows form |
| Permissions and identities | temporary file `0600`, then the target's bits; new files `0666` minus umask; `0311`/`0000` roots refused (`disk.rs:14-17`; `paths.md` "The root"); tests switch to uid 65534 | same API; `nobody` is uid −2, not 65534 (KNOWN); runners are not root | only a read-only attribute and ACLs; mode sentences have no meaning; "a root that cannot be opened for reading" needs an ACL-based form |
| Temporary file and rename | `.kiln-tmp-N` beside the target, `create_new`, rename over (`control-protocol.md:133` "on Unix"); directory `fsync` best effort | same | rename over a file that another process holds open fails (KNOWN); directory sync does nothing; the `.` prefix does not hide the file |
| Path length | `PATH_MAX` 4,096 on Linux (`disk.rs:61`) | 1,024 (KNOWN) | 260 without long-path support (KNOWN) |
| Line endings | the document's own ending is kept and Enter inserts it (`io.crlf`, `ui-editor.md:69`); a new document's default: not platform-dependent in the code read | same | same in Kiln; **the checkout** changes: with `autocrlf` every tracked file gets CRLF and the gate's `schema` step fails (c1) |
| Processes | `NoTools` by default, no process spawned (`tools.rs:1-5`); tests use shell-script fakes (`registry.rs:1715`, `acceptance_t015_tools.rs:130`: `panic!` on non-Unix) | same | the fakes need a Windows form (`.cmd`/`.exe`); `cargo.exe` lookup |
| Environment | `PATH` in tests only; no display variable read in script mode (`ui.md:53`) | same | same |
| Fonts | built-in fonts (`default_fonts`), so layout does not depend on the system (`Cargo.toml` egui line) | same; whether float results of layout are identical on arm64 is not known: a cross-platform comparison of `kiln script` output answers it | same |
| Keys | `ctrl` or the toolkit's `command` both count as Ctrl (`editor.rs:354`; `ui.md:147` "on Linux the toolkit's `command` modifier is Ctrl"; `ui-editor.md:72-78`) | `command` is ⌘: ⌘Z/⌘S work, and so does Control-Z; Control-Arrow moves by word, where macOS users expect Option-Arrow (INFERRED) | as Linux |
| Window system | X11 only; `glow` (OpenGL); exit 2 and one line without a display (`ui.md:67-105`) | winit's AppKit backend; the event loop on the main thread (`apps/kiln/src/main.rs:190` calls `run_window` from `main`, INFERRED fine); OpenGL is deprecated but present (KNOWN); whether a hosted runner has a GUI session and GL: unknown | winit's Win32 backend; hosted runners have no GPU, and Windows' own OpenGL is version 1.1 (KNOWN), so `glow` is likely to fail on a runner without a GL driver such as Mesa's (INFERRED, to be measured) |

### (c) What the gate assumes

Read in `xtask/src/` (VERIFIED by reading; not run on another platform):
- `fmt`, `clippy`, `layers`, `ui-commands`, `test` call `cargo` and parse its output: neutral.
  `util::rel` turns `\` into `/` (`util.rs:150-154`), so the `no-skip` allowlist and the `ui-commands`
  paths match on Windows too.
- `hygiene` and `no-skip` call `git` (`git ls-files`; HEAD's commit date as "today"): needs `git` on
  `PATH`, present on all images (KNOWN).
- `schema` and `scorecard` compare generated text with committed files byte for byte: they fail on a
  CRLF checkout (c1). Fix by repository attributes or by the workflow's checkout settings.
- `scorecard` runs every listed test; a test that does not compile for a platform makes its item
  `planned` there and the committed page stale there. With platform-specific tests the page cannot be
  equal on all three platforms unless the rule says which platform the page is computed on.
- `no-skip` treats any `cfg` other than `cfg(test)` on a test or test module as a skip (`noskip.rs:9-10,81-90`),
  but not a `cfg` on a helper function or a statement: `acceptance_t002.rs:176-187` gives non-Unix
  `set_mode` an empty body and `mode_and_inode` a `None`, so a test that uses them runs there **without
  checking what it checks on Unix** — a skip `no-skip` cannot see. `acceptance_t015_tools.rs:130`
  instead `panic!`s on non-Unix, which fails loudly. The rule must say which of the two is allowed.
- The queue, `join`, `worktree`, `publish` and the metrics run only on the director's Linux machine:
  the queue's lock liveness reads `/proc/<pid>` and keeps a lock when `/proc` is absent
  (`queue.rs:312-314`); not needed on other platforms. Nothing in this plan changes them except where
  the CI result enters the queue (Decision 5).

## Options

| Option | For | Against |
|---|---|---|
| A. **Every platform on every code push, whole gate; macOS and Windows count for main's colour once green** | the study's line between the economies that cost and those that did not (06 §3.2); public repo: no minute cost (d1); one gate everywhere ("identical locally and in CI", CHARTER §4) | evidence arrives after the merge unless a candidate run is added (Decision 5); macOS start delays (d1); slower feedback per push |
| B. PhotoCraft's economies: Linux per push, macOS and Windows nightly or by path | fewer runs | exactly what produced three Windows reds and release-day macOS failures (06 §3.2, §6.3); Kiln has no runner cost to save |
| C. Linux only, the other platforms "best effort" (built in a release job, not tested) | least work | the owner asked for the three platforms "if that can be done"; it can be built (a4), so not testing it would be claiming what is not observed (C4) |
| D. Cross-compile and test under emulation here | evidence without CI | not possible here: no SDK, CRT or mingw (a2); no Wine (a2); installing system packages is not allowed in this environment |
| Keep the current contract | nothing to do | answers D-171 with "no"; D-166's b8 stays a hole |

## Decision (PROPOSED)

1. **Platforms and versions.** Supported: Linux x86_64 (CI `ubuntu-24.04`; window on X11 as today,
   Wayland not in scope), macOS arm64 (CI `macos-15`), Windows x86_64 with the MSVC toolchain (CI
   `windows-2025`). Not in scope without a new decision: macOS Intel (`macos-15-intel` exists, d1),
   Windows arm64 (`windows-11-arm` exists, d1), Linux arm64, Wayland. The CI labels are pinned (no
   `-latest`), as `ubuntu-24.04` is today. The Rust toolchain stays `rust-toolchain.toml`'s 1.97.0 on all three.

2. **What "supported" means, per layer.**
   - *Core crates* (`kiln-text`, `kiln-syntax`, `kiln-search`): build and pass the same tests on all
     three; no platform difference is allowed (they read no file system).
   - *Workspace, commands, control, CLI* (`kiln-workspace` … `kiln-cli`, `kiln script`): build and pass
     the gate on all three. Behaviour is identical except where `docs/protocol/` states a platform
     difference (rule 4), and every such sentence has a test that runs on that platform.
   - *Headless UI* (`kiln script`): identical output on all three for the same script, files and build
     settings, compared across platforms in CI (T-078); a difference is a defect or a stated limit.
   - *The real window*: Linux under Xvfb as ADR-0008 Q-2 A says. macOS and Windows: the window is
     **supported** only once a CI job has started it on that platform and run the window procedure of
     T-065 there; until then the documents say it is built but not observed on that platform, and
     `ui.window` is confirmed for Linux only (an evaluation states the platform of each confirmation).
   - *Release builds*: an unsigned release binary of `kiln` and `kiln-cli` per platform, built in CI and
     kept as a workflow artefact (T-078). No installer, no signing (D-173).

3. **How evidence for macOS and Windows is produced.**
   - A workflow with one job per platform runs `cargo xtask ci` — the whole gate, the same command as
     locally — on every push to `main` that changes a path outside the ledger (ledger-only pushes
     start no platform job, the idea of T-041 applied to the new jobs from the start), on every push to
     a branch `ci/T-NNN` (Decision 5), nightly at a fixed time on main, and on manual dispatch. The
     existing Linux job stays as it is until T-041. Each job uploads its gate report
     (`--report`) and the test output as an artefact, and checks out with line endings untouched.
   - A builder, a reviewer or the evaluator reads a CI result only through a record: the director
     writes, for each run it relies on, the run id, the head commit, and per platform the conclusion
     and the failed step, into the task's log or a decision row (and `metrics/` once T-033's records
     exist). A reviewer of a task that changes platform-dependent behaviour checks the run's job logs
     itself (read-only GitHub access) and names the run in the review. The evaluator confirms an item
     for a platform other than Linux only from a CI run at the evaluated commit whose log shows the
     item's tests ran and passed there, and says so per platform.
   - Until the macOS and Windows jobs have been green once, they are **tracking jobs**: their result
     is recorded at every merge but does not make main red. From the first green run on main of a
     platform, that platform's job counts (Decision 5).

4. **Platform differences in the documents and in tests (replaces D-166 ruling b8).**
   - A document states a platform difference in one sentence per platform where behaviour differs,
     naming the platform ("On Windows, …"), with evidence from that platform (a test that runs there).
     A sentence without a platform applies to all three and is tested on all three.
   - **Rule for tests.** Every test runs on every supported platform. Where the specified behaviour
     differs by platform, the test asserts each platform's specified behaviour (the expected value is
     chosen by platform inside the test); it never passes without checking on one platform what it
     checks on another. A test of a facility one platform does not have (a FIFO on Windows, a mode bit)
     is allowed to exist only on the platforms that have it if it is listed in a registry of
     platform tests with the document sentence it covers and the test that covers that sentence's
     counterpart on the other platforms (or "no counterpart" and the reason); `no-skip` fails an
     unlisted platform-conditional test, a listed one whose counterpart does not exist, and a helper
     that does nothing on one platform where it acts on another. A test that cannot run on a platform
     fails there with a message saying so, never passes (as `acceptance_t015_tools.rs:130` does today).
   - The scorecard page is computed on Linux. On macOS and Windows the scorecard step fails if a listed
     test that is built there fails or is missing without a registry entry; it does not compare the page.
     (Behaviour; the mechanism is T-071's.)

5. **Main is green on every counted platform.**
   - The queue keeps running the gate on Linux locally (CHARTER §4). For a task whose `touches`
     include a platform-sensitive path — `crates/kiln-workspace/`, any `tests/` file that uses the file
     system, processes or the window, `apps/`, `crates/kiln-ui/src/window.rs`, `xtask/`, `.github/`,
     `Cargo.toml`, `Cargo.lock`, `.gitattributes` — the director pushes the exact rebased head the queue
     would fast-forward to a branch `ci/T-NNN` and queues only after every counted platform's job is
     green on that commit. (Interim: rule only, recorded as in 3. Later, PROPOSED for T-033's scope: a
     queue precondition reading the recorded result.)
   - After every push to main the director reads the CI result before the next dispatch or queue run.
     A counted platform's job red on main is main red: METHODOLOGY §8 applies (stop dispatching and
     queueing, open `incidents/INC-NNN-*.md`, fix or revert through the queue, name a permanent fix).
     A red tracking job is recorded and filed as a task, not an incident.
   - The present red (runs 257–353) is the first such incident; T-069 fixes it.

6. **The real window per platform and ADR-0008's procedure in CI.** Linux: a CI step starts Xvfb, puts
   the two libraries ADR-0008 names in place (on the runner, where installing packages is allowed) and
   runs T-065's window procedure (six cases) with `kiln --script`, comparing with `kiln script` as the
   reviewer did. macOS and Windows: T-070 adds a probe step that starts `kiln --script` with a three-line
   script and records the exit status and stderr; T-077 makes the window work and runs the procedure
   there if the probe shows a session the window can open in. If a hosted runner cannot give a window
   (no session, or no OpenGL ≥ 2), the window on that platform stays "built, not observed", and the
   owner is asked whether to accept that, provide a runner, or allow a renderer change (a dependency
   decision, high tier).

7. **Not in scope without the owner.** Signed or notarised builds and installers (certificates and
   secrets only the owner can provide; D-173). An unsigned artefact per platform needs nothing from the
   owner: a release build in CI and an upload as a workflow artefact (pinned action, a stated
   retention). What a user then meets (KNOWN): on macOS, a downloaded unsigned binary is quarantined and
   Gatekeeper refuses it until the user clears the attribute or allows it in the system settings; arm64
   binaries carry the linker's ad-hoc signature, which is enough to run but not to pass Gatekeeper;
   on Windows, SmartScreen warns before the first run. The documents state this; nothing works around it.

### Table: what a platform changes, and which task carries it

| Scorecard item or document sentence | Platform | What differs (from b) | Task |
|---|---|---|---|
| CI is green on main (`ci.yml`; METHODOLOGY §8) | Linux (runner, non-root) | three `ws.path-confinement` tests assume uid 0 | T-069 |
| `docs/development.md` "CI" (one Linux job) and the gate's line-ending assumption | all | jobs per platform; CRLF checkout fails `schema` (c1) | T-070 |
| `no-skip` (D-166 b8), the scorecard step; `xtask`'s own Unix-only tests (`hygiene.rs`, `publish.rs` test modules, `acceptance_t005_r3/r4`, `acceptance_t013_hygiene`, `guard_t005`) | all / Windows | the rule of Decision 4; the gate's tests must run where the gate runs | T-071 |
| `ws.path-confinement` (28 tests); `paths.md` "The root", "The rule", "Residual limits: Unix only" | macOS | case-insensitive names; `nobody` uid; `PATH_MAX` 1,024; runner not root | T-072 |
| same; `disk.rs` module docs ("Unix only, D-014") | Windows | root identity and holding it open; directory open; reparse points, junctions, drive and `\\?\` prefixes, reserved names, streams, case, ACLs; no FIFOs | T-073 (build, identity), T-074 (confinement rules) |
| `io.save-atomic`, `ws.vfs`; `control-protocol.md:133` "(on Unix)"; `disk.rs:14-17` modes | Windows | modes are not bits; rename over an open file; directory sync | T-073 |
| `io.crlf`, `io.utf8`, `io.bom`, `io.mixed-eol`, `syntax.parse` (corpus with `crlf_lines.rs.txt`) | Windows | byte-exact fixtures need an untouched checkout | T-070 |
| `ws.open-project` (7), `nav.workspace-symbol` (3), `nav.goto-line` (2), `nav.goto-symbol` (1), `syntax.outline` (4), `control.errors` (1 of 6) | Windows | their test files use FIFOs, non-UTF-8 names or modes (b2) | T-075 |
| `diag.tool-fake`, `cmd.registry` (tests with shell-script fakes) | Windows | fake tools need a Windows form | T-075 |
| `ui.window` (7 tests, `acceptance_t065.rs` non-UTF-8 args); `ui.md` "The window" (X11, Xvfb libraries, display gone → exit 1) | macOS, Windows | window backend, session, OpenGL; non-UTF-8 arguments are not bytes on Windows | T-076 (tests), T-077 (window) |
| `ui.md:147` and `ui-editor.md:72-78` (`ctrl`/`command`) | macOS | ⌘ and Control both act; word moves on Control | T-077 |
| `ui.editor-view`, `ui.dispatch-only` (`kiln script` output) | all | identical output across platforms, to be measured | T-078 |
| unsigned binaries | all | release build and artefact | T-078 |

## Consequences

- **Tasks** (drafts filed with this ADR; tiers by ADR-0002 and D-162; estimates in builder and review
  tokens, from the figures under "Token cost" below):

  | Task | What | Tier | Depends on | Estimate |
  |---|---|---|---|---|
  | T-069 | CI green again on an ordinary runner (absorbs T-052) | low | — | 150,000 |
  | T-070 | macOS and Windows gate jobs (tracking), byte-exact checkout, reports kept, window probes, Linux window procedure in CI | high | T-069 | 850,000 |
  | T-071 | the platform-test rule in `no-skip` and the scorecard step; `xtask`'s tests portable | high | T-070 | 1,000,000 |
  | T-072 | gate green on macOS; macOS sentences | high (provisional) | T-070, T-071 | 600,000 |
  | T-073 | Windows: workspace builds; root identity, held root, save | high | T-070, T-071 | 1,100,000 |
  | T-074 | Windows confinement rules | high | T-073 | 1,300,000 |
  | T-075 | portable tests: `kiln-workspace`, `kiln-commands` | low (Q-8) | T-071, T-074 | 350,000 |
  | T-076 | portable tests: `kiln-cli`, `kiln` | low (Q-8) | T-071, T-074 | 300,000 |
  | T-077 | the window on macOS and Windows; per-target toolkit features; keys | high | T-070, T-068, T-076 | 1,100,000 |
  | T-078 | unsigned release binaries as artefacts; `kiln script` identical on three platforms | high | T-076 | 700,000 |

- **Order** (with reasons; the director decides it after the retrospective R-254, D-172):
  1. **T-069 first**: main is red in CI now (Context); no platform evidence means anything while the one
     existing CI job is unread and red. Independent of the platform decision.
  2. **T-070** (platform jobs, tracking): it is the only way any macOS or Windows fact becomes known; its
     first runs replace the unknowns of this ADR (link, test results, window probe, wall times).
  3. **T-071** (the test rule) **before T-057 to T-059 write their tests**: each UI module task adds an
     `apps/kiln/tests/acceptance_t05N.rs`; under today's b8 they may be Unix-only, and every such file
     is later rework (29 files already, a4). T-057 to T-059 can be specified meanwhile.
  4. Cut 2 (T-057, T-058, T-059, then evaluation B) **interleaved** with T-072 to T-076: their paths do
     not meet (`crates/kiln-ui/src/panels/*`, `apps/kiln/tests/acceptance_t05N.rs` against
     `crates/kiln-workspace/`, other `tests/` files, `xtask/`), so at most three builders compile at once
     (CHARTER §4) and nothing waits on a shared file. Reason not to put platforms first: cut 2 is the
     owner's other instruction (D-171), and its modules are platform-neutral code over a toolkit that
     already compiles on all three (a4). Reason not to put platforms last: every UI module adds tests
     that must run on three platforms; doing the rule first is cheaper than converting later.
  5. **T-077** (window on macOS and Windows) after T-068 (both edit `crates/kiln-ui/src/window.rs`) and
     after T-070's window probe; **T-078** last.
- **Collisions with drafts.** T-041 (ledger-only pushes start no CI): T-070 does not edit `ci.yml`;
  T-041 later covers both workflows (its scope note changes). T-054 (lighter debug profile): the
  runners have 14 GB of storage (d1) and the workspace's debug test build was 6.3 GB (D-097); if T-070's
  first Windows run runs out of disk, T-054 moves before T-073. T-064 edits `docs/development.md` like
  T-070 (the "CI" section): sequence them, one role per path (D-153). T-066 and T-068 edit
  `crates/kiln-ui/src/observe.rs`, `script.rs`, `window.rs`; T-077 follows T-068.
- **CHARTER text to change (exact, proposed):** §2, after "It must be usable: …": "It runs on Linux
  (x86_64, X11), macOS (arm64) and Windows (x86_64); `docs/protocol/` states where behaviour differs,
  per platform (ADR-0009)." §4, Gate: "identical locally and in GitHub Actions" stays; add "CI runs it on
  every supported platform (ADR-0009)." METHODOLOGY §8: the sentences of Decision 5 (at the next
  methodology edit; until then the director applies them as an interim rule).
- **Checks.** Mechanical: the platform jobs (T-070), `no-skip` and the scorecard step (T-071).
  Rule only, until a tool exists: the director reads CI after every push and records it (Decision 3, 5);
  the `ci/T-NNN` candidate run before queueing a platform-sensitive task.
- **Cost per push** (estimates; to be replaced by T-070's measurements): Linux job 347–490 s for the whole
  gate today (d3); macOS and Windows unknown — KNOWN rule of thumb, Windows builds of Rust workspaces are
  slower than Linux on the same core count and macOS arm64 runners have 3 cores (d1): planning figure
  10–20 min each. Runner minutes cost the owner nothing (d1, public repository). Wall time added to a
  platform-sensitive task by the candidate run: the slowest platform job, plus macOS start delays.
  Number of pushes: 100 CI runs between 2026-11-14 04:08Z and 2026-11-15 16:16Z (36 h, d3), almost all
  ledger-only commits; with the path filter only merges and `ci/` branches start platform jobs.
- **Token cost** (from the record): high tasks of this stage 887,544 (T-065), 997,800 (T-055), 1,190,305
  (T-056) (D-168, D-155, D-162); stage-1 medians high 810,888, medium 686,472, low 84,466–124,444; a fresh
  spec check about 327,000 on average (`retrospectives/iter-2-stage-1.md` §7, Metrics), a spec check by
  the planner that drafted the task 34,769 (D-163). Estimates per task are in each draft; their sum for
  T-069 to T-078 is about **7.45 M** builder and review tokens; with spec checks of the eight non-low
  tasks by the planner that drafted them (about 35,000 each, D-163) about 7.7 M, with fresh spec checks
  (about 327,000 each) about 10.1 M — between 1.7 and 2.2 times the whole of cut 1 (4,645,569 booked,
  D-170). The largest uncertainties are T-074 (Windows confinement) and T-072 (unknown until T-070 runs).
  A cut line: after T-073 and T-075 the CLI and `kiln script` would be built and tested on Windows
  without the Windows confinement rules being stated, which is not shippable (paths could be refused or
  followed differently from the document); the first useful cut is after T-074 and T-076 (about 5.6 M).

## Verification

- T-070's first runs: per platform, how far the gate gets, wall time, the window probe's line.
- After T-076: the gate green on all three platforms on one commit of main; the scorecard page computed
  on Linux; `no-skip` fails a planted unlisted platform-conditional test (self-test).
- Retrospective: platform-only failures caught by a candidate run before the merge, against those first
  seen after it (the study's measure, 06 §3.2); red-main minutes per platform from `incidents/`; runner
  wall time per push.

## Rollback

A new ADR. Dropping a platform removes its job and its sentences; the platform-test registry and the
scorecard rule stay harmless on one platform. Code made portable (root identity through `same-file`,
lexical rules) stays correct on Linux, so rollback costs nothing there. If a counted platform's job
turns out unreliable (flaky start, no macOS slot), it goes back to tracking by a decision row, never by
`continue-on-error` in the workflow.

## Questions for the director

- **Q-1. When do macOS and Windows jobs count for "main is green"?** A: from their first green run on
  main (tracking until then; Decision 3, 5). B: tracking for the rest of iteration 2, counted from
  iteration 3. C: counted from the start, so main is "red" until T-076 merges and nothing can be queued
  except platform work. **Recommendation: A** — C stops cut 2 for weeks; B lets a platform regress
  unseen for an iteration, the study's F3.
- **Q-2. Candidate runs before the merge.** A: for platform-sensitive paths only (Decision 5). B: for
  every code task. C: none; post-merge red is an incident. **Recommendation: A** — B adds 10–20 minutes
  to UI module tasks whose code compiles on all three already (a4); C repeats PhotoCraft's three
  Windows reds on main (06 §3.2).
- **Q-3. The Linux window in CI.** A: T-070 runs T-065's procedure under Xvfb in a CI job on every code
  push (the libraries are installed on the runner). B: the reviewer and the evaluator keep doing it here
  as now. **Recommendation: A**, keeping B for the evaluator (ADR-0008 Q-2 A stays): it makes the 9 of 18
  mutants that only a real window kills (D-165) visible on every push.
- **Q-4. Order against cut 2.** A: T-069, T-070, T-071 first, then cut 2 interleaved with T-072–T-076
  (Consequences). B: cut 2 first, platforms after evaluation B. C: platforms first, cut 2 after.
  **Recommendation: A** — B makes T-057–T-059 add three more Unix-only test files; C delays the owner's
  other instruction for the uncertain T-074.
- **Q-5. If a hosted runner cannot open a window.** A: the window on that platform is "built, not
  observed" and the owner is told. B: a renderer change (`wgpu`, Direct3D/Metal) for that platform,
  a dependency decision with its own probe. C: the owner provides a runner. **Recommendation: A first**,
  with B put to the owner with figures from T-070's probe.
- **Q-6. Windows symlinks.** A: Kiln follows symlinks and junctions on Windows by the same rule as on
  Unix (location inside the root), tested on the runner (KNOWN: hosted Windows runners allow creating
  symlinks as administrator). B: on Windows every reparse point on the walk is refused as outside the
  root. **Recommendation: B for this stage** — it is fail-safe, one sentence, and a fraction of T-074's
  cost; A can follow if a user needs it.
- **Q-7. T-069's review.** T-069 changes tests only, so it is low by ADR-0002, but the tests are the
  evidence of a high confinement item. **Recommendation:** low, and the director has the diff read by a
  reviewer before queueing (no tier change), because weakening a confinement test there would be
  invisible to the gate.
- **Q-8. Who writes the platform-test registry's entries.** The registry is under `xtask/` (a gate
  path), so a test-only task that adds an entry becomes high (T-075, T-076). A: T-071 adds every entry
  the existing tests need (it knows the list from probe a4), so T-075 and T-076 stay low. B: T-075 and
  T-076 are high. C: the registry lives outside `xtask/` (a non-gate path read by the gate), which
  weakens the gate-path rule. **Recommendation: A.**
- **Q-9. T-052.** T-069 covers T-052's tests and its document paragraph. **Recommendation:** abandon
  T-052 in favour of T-069 (one decision row), rather than run both.

## Rulings at acceptance (director, D-175)
- Q-1 A: the macOS and Windows jobs count for main's colour from their first green run on main; until then they
  are tracked and their results recorded.
- Q-2 A: a pre-merge run on a `ci/T-NNN` branch is made for tasks that touch platform-sensitive paths.
- Q-3 A: the Linux window procedure runs under Xvfb in CI on every code push (this is also the retrospective's
  change 8, D-174: window evidence in the gate where it can be).
- Q-4 A: T-069 first (INC-001), then T-070 and T-071, then cut 2 (T-057 to T-059) interleaved with T-072 to
  T-076; T-066 and T-043 are in that batch too (D-174).
- Q-5 A: where a hosted runner cannot open a window, the item is "built, not observed" on that platform and the
  owner is told.
- Q-6 B: on Windows, reparse points are refused as outside the root in this stage.
- Q-7 A: T-069 is low tier with a reviewer's read of the diff.
- Q-8 A: T-071 writes all entries of the platform-test registry, so T-075 and T-076 stay low; their budgets are
  restated when they are specified (the planner expects them over the low ceiling).
- Q-9: T-052 is abandoned in favour of T-069.
- Signed installers, notarisation and packaging are out of scope (the owner, D-173); T-078 ends at unsigned
  workflow artefacts.
- From now on the director reads the CI run of every pushed commit and records its result (INC-001).
