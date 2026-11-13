#!/usr/bin/env bash
# Iteration-1 retrospective (R-153): facts from git history of main. Read-only.
# Usage: bash retrospectives/scripts/iter1_git.sh   (from the repository root)
# Range: from the D-035 commit (retrospective 0 accepted, methodology v1.1 adopted) to the
# iteration-close commit of D-089. Commit ids are fixed so the numbers do not drift.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
START=e1f1a2b   # "Retrospective 0; decision D-035: methodology v1.1 adopted", 03:11:05Z
END=b3d9dd2     # "ledger: T-018 merged ...; iteration 1 closed (D-089)", 17:57:29Z
R="$START..$END"

echo "## commits on main in iteration 1 ($R): $(git rev-list --count "$R")"
echo "## commits per hour (UTC)"
git log --format='%ad' --date=format:'%H' "$R" | sort | uniq -c | tr '\n' ' '; echo

echo "## commits by subject prefix"
git log --format='%s' "$R" | sed -E 's/^(ledger|queue|Record|Decision|T-[0-9]+[^:]*|State|WIP)[: ].*/\1/' \
  | sed -E 's/^(T-[0-9]+).*/T-NNN (task branch or task-named)/' | sort | uniq -c | sort -rn | head -12

echo "## queue-made commits ('queue: ...') and merges recorded"
git log --format='%s' "$R" | grep -c '^queue: ' || true

echo "## Task:/Role: lines per commit (count | has Task line | Role line)"
git log --format='%H' "$R" | while read -r h; do
  b=$(git log -1 --format=%B "$h")
  t=$(echo "$b" | grep -c '^Task: \(T-\|iter-\)' || true)
  r=$(echo "$b" | grep -o '^Role: .*' | head -1 || true)
  echo "$t|${r:-(no role)}"
done | sort | uniq -c

echo "## commits whose message carries a session-identifier trailer (value not printed)"
git log --format='%B%x00' "$R" | tr '\0' '\n' | grep -c '^Claude-Session: ' || true

echo "## approval row commit vs queue merge record commit, per high task of iteration 1"
for t in T-013 T-015 T-016 T-032 T-017 T-008; do
  a=$(git log --format='%h %ad' --date=format:'%H:%M:%S' -S "Approve $t" "$R" -- docs/decisions.md | tail -1)
  q=$(git log --format='%h %ad' --date=format:'%H:%M:%S' -S "\"task\":\"$t\",\"attempt\"" "$R" -- metrics/queue.jsonl | tail -1)
  echo "$t approval: $a | queue record: $q"
done

echo "## Extend rows vs queue merge record"
for t in T-015 T-013 T-010 T-008 T-018; do
  a=$(git log --format='%h %ad' --date=format:'%H:%M:%S' -S "Extend $t to" "$R" -- docs/decisions.md | tail -1)
  q=$(git log --format='%h %ad' --date=format:'%H:%M:%S' -S "\"task\":\"$t\",\"attempt\"" "$R" -- metrics/queue.jsonl | tail -1)
  echo "$t extend: $a | queue record: $q"
done

echo "## commits that rewrite an earlier run record (a '-{\"kind\":\"run\"' line in metrics/tasks.jsonl)"
git log -p --format='COMMIT %h %ad %s' --date=format:'%H:%M:%S' "$R" -- metrics/tasks.jsonl \
  | grep -E '^(COMMIT|-\{"kind":"run")' | grep -B1 '^-{' | grep '^COMMIT' || true

echo "## director corrections of handoff times (commit subjects)"
git log --format='%h %ad %s' --date=format:'%H:%M:%S' "$R" | grep -iE 'handoff.*(time|stamp).*correct|correct.*(time|stamp)' || true

echo "## ledger commits touching only handoffs/ or STATE.md (pushes that start CI for no product change)"
n=0; for h in $(git rev-list "$R"); do
  f=$(git diff-tree --no-commit-id --name-only -r "$h")
  if [ -n "$f" ] && ! echo "$f" | grep -qvE '^(handoffs/|STATE.md)'; then n=$((n+1)); fi
done; echo "$n"

echo "## lines in tracked files with a session-scoped scratch path (METHODOLOGY 12)"
git grep -c -E '/tmp/claude-0/[^/]+/[0-9a-f]{8}-' "$END" -- . || echo 0

echo "## task files with origin_task (rework ratio): iteration, origin, found_by, first commit time"
for f in tasks/T-*.md; do
  o=$(grep -m1 '^origin_task:' "$f" | awk '{print $2}')
  it=$(grep -m1 '^iteration:' "$f" | awk '{print $2}')
  fb=$(grep -m1 '^found_by:' "$f" | awk '{print $2}')
  c=$(git log --diff-filter=A --format='%ad' --date=format:'%Y-%m-%dT%H:%M' -- "$f" | tail -1)
  echo "$(basename "$f" .md | cut -c1-5) iteration=$it origin=$o found_by=$fb first_commit=$c"
done | awk '{n++; if ($3!="origin=null") {k++; print}} END {print "task files:", n, "with origin_task:", k}'
echo "iteration-1 task files: $(grep -l '^iteration: 1' tasks/T-*.md | wc -l)"
