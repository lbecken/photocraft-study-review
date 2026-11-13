#!/usr/bin/env python3
"""Iteration-1 retrospective (R-153): the director as a failure source.

Each row below is one director error or correction found in the record of iteration 1
(docs/decisions.md D-036..D-089, handoffs/2026-11-13-director-handoff.md, task-file rulings,
run-record notes and git history). The source column names where it is recorded; the analyst
read each one. The script only tallies the rows, so every count in the retrospective's
director section is reproducible and the classification is open to inspection.

Usage (from the repository root): python3 -I retrospectives/scripts/iter1_director.py
Classes of cause:
  ruling      a ruling, answer or instruction to builders whose content was wrong
  acceptance  a spec accepted for dispatch that could not be built as written
  brief       an operator or dispatch-brief error (wrong id, wrong path, wrong wording)
  time        a timestamp written from memory instead of the clock
  record      a metrics, booking or bookkeeping error
  omission    a required step not taken
Caught by: tool (a command refused or failed), agent (a builder, reviewer, planner or checker),
director (the director on re-reading, before or after commit), owner, none (still in the
record or found only by this retrospective), not recorded.
Reached a builder: yes if any agent worked on the wrong version.
"""
from collections import Counter

ROWS = [
    # (source, error, cause, caught_by, reached_builder, subtype)
    ("D-036", "D-035 decision wrong or incomplete in nine places; change 12 would have turned main red", "ruling", "agent", "yes", "unchecked"),
    ("D-040", "twelve answers to T-013 given at once, (a) and (h) contradictory; implementer used reset --soft on main", "ruling", "agent", "yes", "unchecked"),
    ("D-041", "instruction to write the spec-check report under reviews/, not a ledger path", "brief", "not recorded", "yes", "-"),
    ("D-041", "T-015 accepted in principle (D-037) with depends_on [] wrong and six blockers", "acceptance", "agent", "no", "-"),
    ("D-042/D-049", "first spec check booked to T-015, later needing an Extend row", "record", "director", "no", "-"),
    ("D-043", "spec edit written under a wrong section name, failed silently after builders were told it was made", "brief", "director", "yes", "-"),
    ("D-044", "corpus ownership first given to the implementer, who cannot see it before the join (R-077 ruling replaced by R-079)", "ruling", "agent", "no", "unchecked"),
    ("D-044", "edited specs of T-016/T-017 not re-checked after the spec-check edits", "omission", "none", "yes", "-"),
    ("D-045", "two handoff timestamps written from memory, ahead of the clock", "time", "not recorded", "no", "-"),
    ("D-046 / 2e1c206", "ruling point 2 not testable from outside the crate; amended 8 minutes later by message", "ruling", "not recorded", "yes", "unchecked"),
    ("D-050", "7-character reviewed_sha copied from the director's brief into a review", "brief", "tool", "yes", "-"),
    ("D-055", "R-102/R-103 booked with director-entered start times; R-103 52 s early", "time", "director", "no", "-"),
    ("D-058", "approval row D-057 did not begin with the approval", "brief", "tool", "no", "-"),
    ("D-059", "T-010 spec drafted before T-032 merged, accepted without a check; both builders stopped (about 226,000 tokens)", "acceptance", "agent", "yes", "-"),
    ("D-059 q2 / D-069", "wording change dropped because of a merged test instead of questioning the test; implementer blocked (R-122)", "ruling", "agent", "yes", "unchecked"),
    ("D-061", "D-052 assignment (test author edits merged tests; red gate expected before join) reversed", "ruling", "director", "no", "unchecked"),
    ("D-065 / D-068 v2", "re-check edit applied as worded, forbidding 'outside' although the pinned message contains it", "ruling", "agent", "yes", "unchecked"),
    ("D-070 s4", "ruling s1 claimed the 100-group cap bounds memory (bound stated without a measurement)", "ruling", "agent", "yes", "unmeasured bound"),
    ("D-070", "first version of the ruling was about to cite T-021/T-022 limits as the bound", "ruling", "director", "no", "unmeasured bound"),
    ("D-074 v8", "ruling v4 offered a mechanism contradicting its own observable", "ruling", "agent", "yes", "mechanism pinned"),
    ("D-076 v9", "message to the implementer forbade a held directory handle; the handle was needed", "ruling", "agent", "yes", "mechanism pinned"),
    ("D-077 s8", "ruling s5 generalised a 5 s bound from one probe; the documented sentence was false (245 s)", "ruling", "agent", "yes", "unmeasured bound"),
    ("D-078", "first wording of w1 defined the mapping through a call that refuses the case", "ruling", "director", "no", "mechanism pinned"),
    ("D-080", "R-134 booked from an interim notification (692 tokens, 1 tool call low)", "record", "director", "no", "-"),
    ("D-081", "document not read before the join although ruling v9 required it to state the limit", "omission", "agent", "yes", "-"),
    ("D-081", "first wording of v11 cited the threat model as a reason although it covers the case", "ruling", "director", "no", "unchecked"),
    ("D-075", "METHODOLOGY section 5, docs/development.md and T-008's spec disagree on who edits scorecard/; two joins failed at the scorecard step", "omission", "tool", "yes", "-"),
    ("D-089", "brief for R-150 told the implementer not to write its log entry; director wrote it on main; rebase conflict", "brief", "tool", "yes", "-"),
    ("handoff 13:55Z", "R-123 booked after the merged transition; metrics refused; a commit named the run without containing it", "record", "tool", "no", "-"),
    ("handoff 16:42Z / 3976c53", "handoff line stamped by hand two minutes ahead of the clock", "time", "director", "no", "-"),
    ("handoff 17:21Z / b9350ee", "start times in the handoff table typed from memory; two wrong", "time", "director", "no", "-"),
    ("handoff 16:16Z", "no scheduled check-in during the 14:20-16:10 usage stop; work resumed because the owner wrote", "omission", "owner", "no", "-"),
    ("R-061 note", "dispatch brief told the agent to set CARGO_TARGET_DIR, which AGENTS.md forbids", "brief", "agent", "yes", "-"),
    ("ea1761c / 70f652b", "R-082 committed with tokens 200,000 (a round placeholder), replaced 33 s later with 243,114", "record", "director", "no", "-"),
    ("R-056 record", "tool_calls 1 is a placeholder (stated in its note), still in the record", "record", "none", "no", "-"),
    ("D-055", "spec checks R-067 and R-069 (354,110 tokens) never booked; T-017 is over its ceiling once R-067's half is added, with no Extend row", "record", "none", "no", "-"),
    ("metrics/tasks.jsonl", "no role: director run recorded although METHODOLOGY 2.1 and T-013 provide for it", "omission", "none", "no", "-"),
    ("metrics report", "T-006 (1.71x) and T-014 (1.74x) over budget without an Extend row (interim rule of section 11)", "omission", "none", "no", "-"),
    ("metrics/tasks.jsonl", "T-015 builders R-072/R-073 started 06:26Z, before the ready transition at 06:34:20Z (section 5.1)", "omission", "none", "yes", "-"),
    ("D-089", "T-012 listed among iteration-1 merges; its front matter says iteration 0 and it merged at 02:56Z, before v1.1", "record", "none", "no", "-"),
]

if __name__ == "__main__":
    print("rows:", len(ROWS))
    for src, err, cause, by, reached, sub in ROWS:
        print(f"- {src}: {err} [{cause}; caught by {by}; reached a builder: {reached}]")
    print("\nby cause:", dict(Counter(r[2] for r in ROWS)))
    print("by catcher:", dict(Counter(r[3] for r in ROWS)))
    print("reached a builder:", dict(Counter(r[4] for r in ROWS)))
    rul = [r for r in ROWS if r[2] == "ruling"]
    print("ruling errors:", len(rul), "by subtype:", dict(Counter(r[5] for r in rul)),
          "reached a builder:", sum(r[4] == "yes" for r in rul))
    print("cause x catcher:")
    for c in sorted({r[2] for r in ROWS}):
        print(f"  {c}: {dict(Counter(r[3] for r in ROWS if r[2] == c))}")
