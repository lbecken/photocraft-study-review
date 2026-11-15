#!/usr/bin/env bash
# Retrospective of iteration 2, stage 2 to cut 1 (R-254): facts from git history of main. Read-only.
# Usage: bash retrospectives/scripts/iter2s2_git.sh   (from the repository root)
# Range: from the D-143 commit (43e9162) to the head the brief names (eefd9d0, D-170); drift after it is listed.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
START=43e9162   # ledger: usage limit ... analyst resumed (D-143), 2026-11-14T18:16:17Z
PLAN=9d5f6d6    # ledger: plan of the second stage (D-147), 2026-11-14T19:06:27Z
END=eefd9d0     # ledger: cut 1 reached and evaluated (D-170), 2026-11-15T05:54:40Z
R="$START..$END"

echo "## commits $R: $(git rev-list --count "$R"); from D-147 ($PLAN..$END): $(git rev-list --count "$PLAN..$END")"
echo "## commits after END: $(git rev-list --count "$END..HEAD")"; git log --format='  %h %cI %s' "$END..HEAD"

echo "## commits by subject prefix in $R"
git log --format='%s' "$R" | sed -E 's/^([A-Za-z-]+):.*/\1/' | sort | uniq -c | sort -rn | head -12

echo "## commits that change only ledger paths in $R"
n=0; t=0; for h in $(git rev-list "$R"); do
  t=$((t+1)); f=$(git diff-tree --no-commit-id --name-only -r "$h")
  if [ -n "$f" ] && ! echo "$f" | grep -qvE '^(metrics/|docs/decisions.md|tasks/|reviews/|handoffs/|STATE.md|evaluations/|retrospectives/|docs/adr/)'; then n=$((n+1)); fi
done; echo "$n of $t (merge commits show no files and are not counted as ledger-only)"

echo "## methodology, AGENTS.md, development.md, CHARTER changes in $R"
git log --format='  %h %cI %s' "$R" -- methodology/ AGENTS.md docs/development.md docs/CHARTER.md || true

echo "## decision rows: commit time of the rows that must precede a run or the queue"
for pat in '| D-159 |' '| D-152 |' '| D-161 |' '| D-167 |' '| D-149 |' '| D-150 |' '| D-158 |' '| D-165 |' '| D-163 |' '| D-156 |' '| D-170 |'; do
  echo "  $pat $(git log --format='%h %cI' -S"$pat" "$R" -- docs/decisions.md | tail -1)"
done

echo "## queue records: commit time of each T-055/T-056/T-065/T-067 queue line"
for t in T-055 T-056 T-065 T-067; do
  git log --format="  $t %h %cI %s" -S"\"task\":\"$t\",\"attempt\"" "$R" -- metrics/queue.jsonl | tail -3
done

echo "## director (Role: director) commits that touch a task file's front matter on a task branch: commits not on main's first-parent line touching tasks/ with Role: director"
for t in T-055 T-056 T-065; do
  for ref in $(git for-each-ref --format='%(refname)' "refs/heads/task/$t" "refs/remotes/origin/task/$t" 2>/dev/null); do echo "  ref $ref"; done
done
echo "  (task branches were deleted after merge; the merged history is checked instead)"
for t in T-055 T-056 T-065; do
  c=$(git log --format='%h' "$R" --grep="Task: $t" --grep='Role: director' --all-match -- tasks/ | wc -l)
  echo "  $t: Role: director commits touching tasks/ reachable from END: $c"
done

echo "## merge commits on main's history in $R"
git rev-list --merges --count "$R"

echo "## Task:/Role: lines in builder commits reachable from END in $R (count by Role)"
git log --format='%B%x00' "$R" | tr '\0' '\n' | grep -o '^Role: [a-z-]*' | sort | uniq -c

echo "## files of the product changed in $R (diffstat, product paths only)"
git diff --shortstat "$START" "$END" -- crates apps xtask Cargo.toml Cargo.lock
git diff --stat "$START" "$END" -- crates/kiln-ui apps/kiln Cargo.toml Cargo.lock | tail -15

echo "## Cargo.lock packages at START, after T-055, after T-065"
for c in $START e34ba1f d35078c; do echo "  $c $(git show "$c:Cargo.lock" | grep -c '^\[\[package\]\]')"; done

echo "## files using std::os::unix, cfg(unix) or cfg(target_os) at END (apps crates xtask)"
U=$(git grep -l -E 'cfg\(unix\)|std::os::unix|cfg\(target_os' "$END" -- apps crates xtask | sed "s/^$END://")
echo "  all: $(echo "$U" | wc -l); under a tests/ directory: $(echo "$U" | grep -c '/tests/'); others:"
echo "$U" | grep -v '/tests/' | sed 's/^/    /'
echo "  lines outside a tests/ directory (unit-test modules included):"
git grep -n -E 'cfg\(unix\)|std::os::unix|cfg\(target_os' "$END" -- crates apps xtask/src | sed "s/^$END://" | grep -v '/tests/' | cut -c1-110 | sed 's/^/    /'
echo "  CI workflows at END:"
git ls-tree -r --name-only "$END" .github | sed 's/^/    /'
git show "$END:.github/workflows/ci.yml" | grep -n -E 'runs-on|os:|matrix' | sed 's/^/    /'
echo "## files that set permission modes or read the uid (PermissionsExt, from_mode, geteuid) at END"
git grep -l -E 'geteuid|getuid|PermissionsExt|from_mode|set_mode' "$END" -- apps crates xtask | sed "s/^$END:/    /"

echo "## no-skip / cfg rules in xtask (how no-skip treats cfg)"
git grep -n -i -E 'cfg|ignore' "$END" -- xtask/src/noskip.rs xtask/src/no_skip.rs 2>/dev/null | head -20 || true
git ls-tree -r --name-only "$END" xtask/src | sed 's/^/    /'
