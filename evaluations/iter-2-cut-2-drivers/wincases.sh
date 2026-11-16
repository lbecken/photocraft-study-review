#!/bin/bash
# Window start-up and failure cases (ui.md "The window").
# env: KILN_BIN, XLIBS, DRV, OUT (fresh output dir)
set -u
LIB="$XLIBS/usr/lib/x86_64-linux-gnu"
mkdir -p "$OUT/d/src" && printf 'fn main() {\n    let x = 1;\n}\n' >"$OUT/d/src/a.rs"
cd "$OUT"
printf '{"id":"s","ui":"snapshot"}\n{"id":1,"command":"doc.list"}\n' >s.jsonl
show() { echo "== $1: rc=$2 stdout_bytes=$(wc -c <o.txt) stderr_lines=$(wc -l <e.txt)"; sed 's/^/   stderr: /' e.txt | cut -c1-300 | head -5; }

env -u DISPLAY -u WAYLAND_DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d src/a.rs >o.txt 2>e.txt; show "no display" $?
env -u DISPLAY -u WAYLAND_DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d --script s.jsonl >o.txt 2>e.txt; show "no display, --script" $?
env -u WAYLAND_DISPLAY DISPLAY=:77 LD_LIBRARY_PATH="$LIB" HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d >o.txt 2>e.txt; show "unreachable display :77" $?
env -u DISPLAY WAYLAND_DISPLAY=wayland-9 XDG_RUNTIME_DIR="$OUT" HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d >o.txt 2>e.txt; show "wayland only" $?
env -u WAYLAND_DISPLAY DISPLAY=garbage HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d >o.txt 2>e.txt; show "DISPLAY=garbage" $?
# under Xvfb without the two libraries
env -u WAYLAND_DISPLAY -u LD_LIBRARY_PATH MESA_SHADER_CACHE_DISABLE=true HOME="$OUT/h1" timeout 60 xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$KILN_BIN' --root d src/a.rs --script s.jsonl 2>e.txt" >o.txt; show "xvfb without the two libraries" $?
# non-UTF-8 arguments
for form in "--root d $(printf 'a\377.rs')" "--root $(printf 'd\377')" "$(printf '\377')" "script s.jsonl --root d $(printf '\377.rs')" "script $(printf 's\377.jsonl')" "--script $(printf 's\377')"; do
  env -u WAYLAND_DISPLAY DISPLAY=:77 HOME="$OUT/h1" timeout 30 "$KILN_BIN" $form >o.txt 2>e.txt; rc=$?; show "non-UTF-8 argument [$(printf '%s' "$form" | od -An -c | tr -s ' ' | head -c 80)]" $rc
done
# a first argument `script`
printf 'not a script\n' >d/script
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" script >o.txt 2>e.txt; show "kiln script (no script)" $?
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d script >o.txt 2>e.txt; show "kiln --root d script (script as 2nd arg, no display)" $?
# shader cache: no MESA_SHADER_CACHE_DISABLE, fresh HOME, no XDG_CACHE_HOME
mkdir -p "$OUT/h2"
env -u WAYLAND_DISPLAY -u XDG_CACHE_HOME -u MESA_SHADER_CACHE_DISABLE LD_LIBRARY_PATH="$LIB" HOME="$OUT/h2" timeout 60 xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$KILN_BIN' --root d src/a.rs --script s.jsonl 2>e.txt" >o.txt; show "cache: window with fresh HOME, cache not disabled" $?
echo "   files under HOME:"; find "$OUT/h2" -type f | sed "s|$OUT|\$OUT|" | head -10; echo "   count: $(find "$OUT/h2" -type f | wc -l)"
mkdir -p "$OUT/h3" "$OUT/xdg"
env -u WAYLAND_DISPLAY MESA_SHADER_CACHE_DISABLE=true XDG_CACHE_HOME="$OUT/xdg" LD_LIBRARY_PATH="$LIB" HOME="$OUT/h3" timeout 60 xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$KILN_BIN' --root d src/a.rs --script s.jsonl 2>e.txt" >o.txt; show "cache disabled" $?
echo "   files under HOME and XDG_CACHE_HOME: $(find "$OUT/h3" "$OUT/xdg" -type f | wc -l)"
mkdir -p "$OUT/h4"
env -u WAYLAND_DISPLAY -u XDG_CACHE_HOME -u MESA_SHADER_CACHE_DISABLE LD_LIBRARY_PATH="$LIB" HOME="$OUT/h4" timeout 60 xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$KILN_BIN' script s.jsonl --root d src/a.rs 2>e.txt" >o.txt; show "kiln script under a display, cache not disabled" $?
echo "   files under HOME: $(find "$OUT/h4" -type f | wc -l)"
# the root: did anything appear in d besides the fixture?
echo "   files in root d: $(cd d && find . -type f | sort | tr '\n' ' ')"
# R-302 additions: usage errors of the window form, a missing start path, a bad root
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d --size 800x600 >o.txt 2>e.txt; show "--size without script" $?
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d --memory >o.txt 2>e.txt; show "--memory without script" $?
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d --log l.txt >o.txt 2>e.txt; show "--log without script" $?
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d --root d >o.txt 2>e.txt; show "--root twice" $?
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root nowhere >o.txt 2>e.txt; show "bad root, no display" $?
env -u WAYLAND_DISPLAY -u DISPLAY HOME="$OUT/h1" timeout 30 "$KILN_BIN" --root d --script missing.jsonl >o.txt 2>e.txt; show "unreadable --script, no display" $?
env -u WAYLAND_DISPLAY LD_LIBRARY_PATH="$LIB" MESA_SHADER_CACHE_DISABLE=true HOME="$OUT/h1" timeout 60 xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$KILN_BIN' --root d missing.rs src/a.rs nothere --script s.jsonl 2>e.txt" >o.txt; show "window, start paths missing.rs src/a.rs nothere" $?
cat o.txt | cut -c1-200
