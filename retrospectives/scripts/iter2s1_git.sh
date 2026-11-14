#!/usr/bin/env bash
# Retrospective of iteration 2, stage 1 (R-223): facts from git history of main. Read-only.
# Usage: bash retrospectives/scripts/iter2s1_git.sh   (from the repository root)
# Range: from the D-089 commit (iteration 1 closed) to the stage-close commit of D-142.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
START=b3d9dd2   # ledger: T-018 merged; iteration 1 closed (D-089), 2026-11-13T17:57:29Z
V12=1c0946d     # ledger: methodology v1.2 (D-091 applied; D-093), 2026-11-13T21:24:05Z
END=e7d6ef9     # ledger: first stage of iteration 2 closed (D-142)
R="$V12..$END"

echo "## commits $START..$END: $(git rev-list --count "$START..$END"); v1.2 in force ($R): $(git rev-list --count "$R")"
echo "## commits after END on main: $(git rev-list --count "$END..HEAD")"; git log --format='  %h %cI %s' "$END..HEAD"

echo "## commits per hour (UTC) in $R"
git log --format='%cd' --date=format:'%dT%H' "$R" | sort | uniq -c | tr '\n' ' '; echo

echo "## commits by subject prefix in $R"
git log --format='%s' "$R" | sed -E 's/^([A-Za-z-]+):.*/\1/' | sort | uniq -c | sort -rn | head -12

echo "## Task:/Role: lines per commit in $R (count | Task line | Role line)"
git log --format='%H' "$R" | while read -r h; do
  b=$(git log -1 --format=%B "$h")
  t=$(echo "$b" | grep -c '^Task: \(T-\|iter-\)' || true)
  r=$(echo "$b" | grep -o '^Role: .*' | head -1 || true)
  echo "$t|${r:-(no role)}"
done | sort | uniq -c

echo "## ledger commits touching only handoffs/ or STATE.md in $R"
n=0; for h in $(git rev-list "$R"); do
  f=$(git diff-tree --no-commit-id --name-only -r "$h")
  if [ -n "$f" ] && ! echo "$f" | grep -qvE '^(handoffs/|STATE.md)'; then n=$((n+1)); fi
done; echo "$n"

echo "## commits that change only ledger paths (metrics/ docs/decisions.md tasks/ reviews/ handoffs/ STATE.md evaluations/ retrospectives/ docs/adr/) in $R"
n=0; for h in $(git rev-list "$R"); do
  f=$(git diff-tree --no-commit-id --name-only -r "$h")
  if [ -n "$f" ] && ! echo "$f" | grep -qvE '^(metrics/|docs/decisions.md|tasks/|reviews/|handoffs/|STATE.md|evaluations/|retrospectives/|docs/adr/)'; then n=$((n+1)); fi
done; echo "$n"

echo "## Extend rows and approval rows vs round-1/round-2 and queue records (commit times)"
for t in T-020 T-021 T-023 T-038; do
  a=$(git log --format='%h %cd' --date=format:'%d %H:%M:%S' -S "Extend $t to" "$R" -- docs/decisions.md | tail -1)
  q=$(git log --format='%h %cd' --date=format:'%d %H:%M:%S' -S "\"task\":\"$t\",\"attempt\"" "$R" -- metrics/queue.jsonl | tail -1)
  echo "$t extend: $a | queue record: $q"
done
for t in T-019 T-036 T-037 T-038; do
  a=$(git log --format='%h %cd' --date=format:'%d %H:%M:%S' -S "Approve $t" "$R" -- docs/decisions.md | tail -1)
  q=$(git log --format='%h %cd' --date=format:'%d %H:%M:%S' -S "\"task\":\"$t\",\"attempt\"" "$R" -- metrics/queue.jsonl | tail -1)
  echo "$t approval: $a | queue record: $q"
done

echo "## commits that rewrite or remove an earlier run record (a '-{\"kind\":\"run\"' line) in $R"
git log -p --format='COMMIT %h %cd %s' --date=format:'%d %H:%M:%S' "$R" -- metrics/tasks.jsonl \
  | grep -E '^(COMMIT|-\{"kind":"run")' | grep -B1 '^-{' | grep '^COMMIT' || true

echo "## T-021 tests-before-code order (odd arm)"
git log --format='%h %cI %s' --reverse 46bc1358821367f3580328203ce940ad76edae06 -- 'apps/kiln-cli/tests/acceptance_t021*' 'crates/*/tests/acceptance_t021*' | head -5 || true
echo "## T-023 tests-before-code order (odd arm)"
git log --format='%h %cI %s' --reverse e8423ff94426f97e6508cec7689887954d8f1578 -- 'apps/kiln-cli/tests/acceptance_t023*' | head -5 || true

echo "## lines in tracked files with a session-scoped scratch path (METHODOLOGY 12) at END"
git grep -c -E '/tmp/claude-0/[^/]+/[0-9a-f]{8}-' "$END" -- . || echo 0

echo "## reason.rs commits in $R (ADR-0006 verification: kind-adding tasks that edited reason.rs)"
git log --format='%h %cI %s' "$R" -- crates/kiln-commands/src/reason.rs || true

echo "## task files first committed in $R (new drafts): id found_by origin_task"
for f in tasks/T-*.md; do
  c=$(git log --diff-filter=A --format='%cI' "$START..$END" -- "$f" | tail -1)
  [ -n "$c" ] || continue
  o=$(grep -m1 '^origin_task:' "$f" | awk '{print $2}')
  fb=$(grep -m1 '^found_by:' "$f" | awk '{print $2}')
  s=$(grep -m1 '^state:' "$f" | awk '{print $2}')
  echo "$(basename "$f" .md | cut -c1-5) first=$c state=$s found_by=$fb origin=$o"
done
echo "## states of all task files at END"
for f in tasks/T-*.md; do (git show "$END:$f" 2>/dev/null || true) | sed -n 's/^state: *\([a-z_]*\).*/\1/p' | head -1; done | sort | uniq -c
