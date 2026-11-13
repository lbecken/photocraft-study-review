#!/usr/bin/env bash
# Iteration-0 retrospective (R-052): facts from git history. Read-only.
# Usage: bash retrospectives/scripts/iter0_git.sh   (from the repository root)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

echo "## commits on main: $(git log --oneline | wc -l)"

echo "## Task:/Role: lines per commit (count, has Task line | Role line)"
git log --format='%H' | while read -r h; do
  b=$(git log -1 --format=%B "$h")
  t=$(echo "$b" | grep -c '^Task: T-' || true)
  r=$(echo "$b" | grep -o '^Role: .*' | head -1 || true)
  echo "$t|${r:-(no role)}"
done | sort | uniq -c

echo "## co-author trailer variants by Role line (hash of trailer text; names not printed)"
git log --format='%H' | while read -r h; do
  b=$(git log -1 --format=%B "$h")
  v=$( (echo "$b" | grep "^Co-Authored-By:" || true) | sed "s/<.*//" | md5sum | cut -c1-6)
  r=$(echo "$b" | grep -o '^Role: .*' | head -1 || true)
  echo "$v ${r:-(no role)}"
done | sort | uniq -c
echo "(d41d8c = no trailer)"

echo "## approval row commit vs queue merge commit, per high task"
for t in T-001 T-002 T-003 T-004 T-005; do
  a=$(git log --format='%h %ad' --date=iso-strict -S "Approve $t" -- docs/decisions.md | tail -1)
  q=$(git log --format='%h %ad' --date=iso-strict -S "\"task\":\"$t\"" -- metrics/queue.jsonl | tail -1)
  echo "$t approval: $a | queue record: $q"
done

echo "## director regenerations of docs/scorecard.md"
git log --oneline | grep -i 'regenerate docs/scorecard.md' || true

echo "## lines in tracked docs containing a session-scoped scratch path (unchecked rule, METHODOLOGY 12)"
git grep -c -E '/tmp/claude-0/[^/]+/[0-9a-f]{8}-' -- reviews docs tasks evaluations || true
