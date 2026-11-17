#!/usr/bin/env bash
# Retrospective of iteration 2, stage 2, cut 1 to cut 2 (R-304): facts from git history of main. Read-only.
# Usage: bash retrospectives/scripts/iter2s2c2_git.sh   (from the repository root)
# Range: D-171's commit 30917a5 to D-217's commit c45a17f (the brief's state); D-216 is 995b176.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
START=30917a5; CUT2=995b176; END=c45a17f9b6142f422f0ca2eaf8487da3975e22b4
R="$START^..$END"
echo "## commits $R: $(git rev-list --count "$R"); first-parent: $(git rev-list --first-parent --count "$R")"
echo "## commits after END (drift): $(git rev-list --count "$END..HEAD")"; git log --format='  %h %cI %s' "$END..HEAD"

echo "## methodology, AGENTS.md, development.md, CHARTER, role briefs changed in $R"
git log --format='  %h %cI %s' "$R" -- methodology/ AGENTS.md docs/CHARTER.md | head -20
echo "## METHODOLOGY.md lines at START and END: $(git show $START:methodology/METHODOLOGY.md | wc -l) $(git show $END:methodology/METHODOLOGY.md | wc -l)"

echo "## decision rows: commit time of rows that must precede a run, a join or the queue"
for pat in '| D-176 |' '| D-177 |' '| D-178 |' '| D-179 |' '| D-185 |' '| D-189 |' '| D-195 |' '| D-196 |' '| D-198 |' '| D-207 |' '| D-210 |' '| D-213 |' '| D-217 |'; do
  echo "  $pat $(git log --format='%h %cI' -S"$pat" "$R" -- docs/decisions.md | tail -1)"
done

echo "## Role: director commits reachable from END in $R whose files include a task file's front matter, by subject"
for h in $(git rev-list "$R" --grep='Role: director'); do
  if git show --format= --name-only "$h" | grep -q '^tasks/T-'; then
    on_fp=$(git rev-list --first-parent "$R" | grep -c "^$h" || true)
    echo "  $(git log -1 --format='%h %cI' $h) first-parent=$on_fp files=$(git show --format= --name-only $h | tr '\n' ' ' | cut -c1-120) | $(git log -1 --format=%s $h | cut -c1-80)"
  fi
done | head -40
echo "## director commits on task branches (Role: director, not on main's first-parent line): subjects"
fp=$(git rev-list --first-parent "$R")
for h in $(git rev-list "$R" --grep='Role: director'); do
  if ! echo "$fp" | grep -q "^$h"; then echo "  $(git log -1 --format='%h %s' $h | cut -c1-110) [$(git show --format= --name-only $h | tr '\n' ' ' | cut -c1-90)]"; fi
done

echo "## merge commits on main's history in $R: $(git rev-list --merges --count "$R")"
echo "## builder commits by Role line in $R"
git log --format='%B%x00' "$R" | tr '\0' '\n' | grep -o '^Role: [a-z-]*' | sort | uniq -c

echo "## tests-first order of the implementer-first tasks (first acceptance commit vs first other implementer commit)"
for t in T-057 T-059; do
  a=$(git log --reverse --format='%h %cI %s' "$R" --grep="Task: $t" --grep='Role: implementer' --all-match -- "apps/kiln/tests/acceptance_${t/T-/t}.rs" | head -1)
  b=$(git log --reverse --format='%h %cI %s' "$R" --grep="Task: $t" --grep='Role: implementer' --all-match -- crates/kiln-ui | head -1)
  echo "  $t acceptance first: $a"; echo "  $t crates/kiln-ui first: $b"
  echo "  $t commits touching its acceptance file by the implementer: $(git log --format=%h "$R" --grep="Task: $t" --grep='Role: implementer' --all-match -- apps/kiln/tests/acceptance_${t/T-/t}.rs | wc -l)"
done

echo "## files using std::os::unix, cfg(unix) or cfg(target_os) at the cut-1 head eefd9d0 and at END (apps crates xtask)"
for c in eefd9d0 $END; do
  U=$(git grep -l -E 'cfg\(unix\)|std::os::unix|cfg\(target_os' "$c" -- apps crates xtask | sed "s/^$c://")
  echo "  $c: $(echo "$U" | wc -l) files, under tests/: $(echo "$U" | grep -c '/tests/')"
done
echo "  new test files of the period (acceptance_t05[4789], t057, t058, t059, t069, t070) that match:"
git grep -l -E 'cfg\(unix\)|std::os::unix|cfg\(target_os' "$END" -- 'apps/*/tests/*t05[4789]*' 'apps/*/tests/*t070*' 'xtask/tests/*t054*' 'xtask/tests/*t070*' 'crates/*/tests/*t069*' | sed 's/^/    /' || echo "    none"

echo "## workflows at END"
git ls-tree -r --name-only "$END" .github | sed 's/^/  /'
echo "## platforms.yml: lines with head -200, no-fail-fast, cargo test -p"
git show "$END:.github/workflows/platforms.yml" | grep -n -E 'head -[0-9]+|no-fail-fast|cargo test --locked -p|cargo xtask ci' | sed 's/^/  /' | head -20
echo "## xtask gate test step: does it pass --no-fail-fast?"
git grep -n -E 'no-fail-fast' "$END" -- xtask/src | sed "s/^$END:/  /" | head -5 || echo "  none in xtask/src"

echo "## Cargo.toml profile tables at END"
git show "$END:Cargo.toml" | sed -n '/^\[profile/,/^$/p' | sed 's/^/  /'
