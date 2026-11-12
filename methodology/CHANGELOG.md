# Methodology changelog

Each version lists what changed and the evidence behind it. From v1.1 on, every change cites a
metric (file, field, value) or an incident (`INC-NNN`) from this repository; v1.0 cites the
PhotoCraft study (`/home/user/photocraft-study/deliverables/`), because no Kiln data exists yet.

## v1.0 — 2026-11-12

Initial version, written before iteration 0's first queued task. All rules are untested
proposals; the study's own verification lists its process proposals as having no effect data
(`VERIFICATION.md` §5). Starting values (tier budgets 150k / 400k / 700k tokens, concurrency cap
3, stop after 3 consecutive failed gate attempts) are director choices to be recalibrated.

| Principle | Study finding it answers | Measured by (claim → metric, METHODOLOGY §13) |
|---|---|---|
| P1 Contracts and checks before fan-out; contract change is its own task | Planned validated command schema and op-history never built (E04-003); fan-out from day 1 (D03, D10); a field added in one PR broke a concurrent one (E07-003) | C1 → rework ratio, escaped defects traced to contract changes, count of ADRs after iteration 0 |
| P2 Only the queue moves `main` | Semantic merge skew, 11 of 21 traced red-main incidents, no mechanical queue (E07-003, E05-052, F1) | C2 → first-pass queue success, red-main minutes, lead time |
| P3 Builder does not certify | Vacuous tests (E07-018, F4); fixes merged without tests (E07-015) | C3 → acceptance-caught defects; review findings of class test gap |
| P4 Status derived from tests plus evaluator | "625/625 live" measured wiring (E07-013, F5); 66 of 129 settings inert (E07-014, F6) | C4 → share verified; evaluator rejections of `implemented` items; escaped defects |
| P5 Review by risk tier | Review never blocked a merge; ~4 explicit approvals in ~22 sampled PRs (E06-041, F12, D06) | C5 → review findings by class and tier; escaped defects by origin tier; tokens per merged task by tier |
| P6 Every rule names its check | Stated-only rules are where practice diverged (E02 §4, errata #5); gates that could not fail (F11) | all claims → METHODOLOGY §16; retrospective lists drift at unchecked rules |
| P7 Budgets gate | 0 of 102 perf budgets enforced (errata #11, E04-051); effort estimate revised ~10× (E08-005) | C6 → budget overruns, tokens per merged task, enforced latency budgets |
| P8 Repository is the memory | Gitignored devlog and `plan/` (E02-004, E01-005, F14); session trailers on few commits (E06-011, F13); informal decision log (E02-025) | C7 → runs with log entry and run record, transitions recorded, runs reporting missing context |
| P9 Constructed oracles, public interfaces | No reference binary for an IDE (D10 §4); agents rarely verified through the control channel (E04-024) | C8 → escaped defects on items whose tests use an oracle vs not (director classification in v1) |
| P10 Stop and report | Out-of-scope edits caused duplicates and broken builds (E07-019, E06-050); hidden contract changes (E07-003) | C6, C7 → blocked runs by stop condition, budget overruns, out-of-`touches` diffs found at join |

Known gaps carried as open questions O-1 … O-8 (METHODOLOGY §15).
