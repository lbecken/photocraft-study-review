#!/bin/sh
# T-065 window procedure (test author, R-251): AC-2, AC-3, b1, b2, b4. Usage: sh .github/scripts/window-procedure.sh <kiln> <fresh scratch dir>
# The window procedure of record (T-070 Interface (3)): R-251's block of tasks/T-065-window.md
# with these changes only: this usage line, the exit status, and (ruling m2, D-182) a window
# case's and AC-3's exit status is kiln's own, written by the wrapped command into <case>.st in
# the scratch directory and read from there (not the virtual-display wrapper's; not written =
# the case fails and says so). The three R-251 lines that run kiln under xvfb-run with a verdict
# are replaced, and one line is added after the CASE line; every other case, fixture, comparison
# and verdict line below is R-251's, inside the function `procedure`.
# Ruling g1 (D-193): what kiln prints is never a verdict. R-251 echoes kiln's stderr in two lines
# (`window stderr:` and AC-3's `stderr:`); `echo` below prints its text without reading escapes
# and indents every line after the first, so an echoed line of kiln's never starts a line of the
# output. A verdict is a line that starts at column 0. The wrapper at the end passes the output
# through unchanged and exits 0 only if it printed `WINDOW CASES PASS`, `PLAIN PASS: …` and
# either `AC-3 PASS` or `AC-3 PRECONDITION FAILED: …` (a library is installed on this machine, so
# AC-3 is not checked here: ruling Q-4 A, D-180), and no FAIL or NOT RUN verdict
# (`WINDOW CASES FAIL`, `AC-3 FAIL`, `PLAIN FAIL: …`, `PLAIN NOT RUN: …`); otherwise it exits 1.
# Changing this file is a high-tier change (.github/ is a gate path).
NL='
'
echo() {
  _e=$*; _f=${_e%%"$NL"*}; printf '%s\n' "$_f"
  while [ "$_f" != "$_e" ]; do _e=${_e#*"$NL"}; _f=${_e%%"$NL"*}; printf '    %s\n' "$_f"; done
}
procedure() {
set -u
K=$(realpath "$1"); W=$2; rm -rf "$W"; mkdir -p "$W"; cd "$W" || exit 99; W=$(pwd)
# --- fixtures: root d ---
mkdir -p d/src
printf 'fn main() {\n    let x = 1;\n}\n' > d/a.rs
{ printf 'fn b() {\n'; i=0; while [ $i -lt 58 ]; do printf '    let v%d = %d;\n' $i $i; i=$((i+1)); done; printf '}\n'; } > d/src/b.rs
{ printf 'fn l() {\n'; i=0; while [ $i -lt 1000 ]; do printf '    let w%d = %d;\n' $i $i; i=$((i+1)); done; printf '}\n'; } > d/src/l.rs
cat > s.jsonl <<'J'
{"id":"s1","ui":"snapshot"}
{"id":"c","ui":"click","target":{"label":"Editor"},"at":{"row":1,"col":4}}
{"id":"t","ui":"type","text":"y"}
{"id":"s2","ui":"snapshot"}
{"id":"k","ui":"key","key":"S","modifiers":["ctrl"]}
{"id":"sc","ui":"scroll","target":{"label":"Editor"},"direction":"down","steps":2}
{"id":"s3","ui":"snapshot"}
{"id":1,"command":"doc.text","params":{"doc":2}}
{"id":2,"command":"cursor.get","params":{"doc":2}}
{"id":3,"command":"doc.list"}
{"id":"f","ui":"frames","count":2}
{"id":"s4","ui":"snapshot"}
J
{ cat s.jsonl; echo '{"id":"x","ui":"click","target":{"label":"no such node t065"}}'; } > s3.jsonl
cat > dc.jsonl <<'J'
{"id":"f","ui":"focus","target":{"label":"Path"}}
{"id":"t","ui":"type","text":"src/a.rs"}
{"id":"c1","ui":"click","target":{"label":"Path"}}
{"id":"w","ui":"frames","count":20}
{"id":"c2","ui":"click","target":{"label":"Path"}}
{"id":"z","ui":"type","text":"Z"}
{"id":"s","ui":"snapshot"}
J
cat > sc.jsonl <<'J'
{"id":"s0","ui":"snapshot"}
{"id":"sc","ui":"scroll","target":{"label":"Editor"},"direction":"down","steps":100}
{"id":"s","ui":"snapshot"}
{"id":1,"command":"cursor.get","params":{"doc":1}}
J
cat > q.jsonl <<'J'
{"id":"c","ui":"click","target":{"label":"Editor"},"at":{"row":0,"col":0}}
{"id":"t","ui":"type","text":"q"}
{"id":"kq","ui":"key","key":"Q","modifiers":["ctrl"]}
{"id":"f","ui":"frames","count":5}
{"id":"t2","ui":"type","text":"w"}
{"id":"kq2","ui":"key","key":"Q","modifiers":["ctrl","shift"]}
{"id":"ks","ui":"key","key":"S","modifiers":["ctrl"]}
{"id":"s","ui":"snapshot"}
{"id":1,"command":"doc.list"}
J
cat > norm.py <<'P'
import json, sys
def load(p, drop):
    out = []
    for l in open(p, encoding="utf-8").read().splitlines():
        v = json.loads(l); r = v.get("result")
        if drop and isinstance(r, dict):
            if "settled" in r: r.pop("settled"); r.pop("frames", None)   # an action
            elif "root" in r: r.pop("frames", None)                       # a snapshot
        out.append(v)
    return out
def node(n, lab):
    if n.get("label") == lab: return n
    for c in n.get("children", []):
        m = node(c, lab)
        if m: return m
w, h = sys.argv[1], sys.argv[2]
a, b = load(w, True), load(h, True)
for i, (x, y) in enumerate(zip(a, b)):
    if x != y: print("  DIFF at line", i + 1, "\n   window  :", json.dumps(x)[:400], "\n   headless:", json.dumps(y)[:400])
ok = a == b and len(a) > 0
if len(sys.argv) > 3:   # the first snapshot of s.jsonl: Title src/b.rs, Message not_found
    s1 = next((x for x in a if x.get("id") == "s1"), None)
    t = node(s1["result"]["root"], "Title") if s1 else None
    m = node(s1["result"]["root"], "Message") if s1 else None
    first = bool(t and t.get("value", "").startswith("src/b.rs") and m and m.get("value", "").startswith("not_found"))
    print("  first snapshot Title src/b.rs and Message not_found:", first); ok = ok and first
full = load(w, False) == load(h, False) and len(a) > 0
print("  lines", len(a), len(b), "| without counts:", "EQUAL" if ok else "NOT EQUAL", "| with counts:", "EQUAL" if full else "NOT EQUAL")
sys.exit(0 if ok else 1)
P
# --- the two libraries, unpacked into scratch (nothing installed) ---
X="$W/xlibs"; mkdir -p "$X/debs" && (cd "$X/debs" && apt-get download libxkbcommon-x11-0 libxcb-xkb1 >/dev/null 2>&1) && for f in "$X"/debs/*.deb; do dpkg-deb -x "$f" "$X/root"; done
LIB="$X/root/usr/lib/x86_64-linux-gnu"; ls "$LIB"/libxkbcommon-x11.so.0 "$LIB"/libxcb-xkb.so.1 >/dev/null || echo "LIBRARIES MISSING"
xv() { env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET LD_LIBRARY_PATH="$LIB" MESA_SHADER_CACHE_DISABLE=true timeout 180 xvfb-run -a -s "-screen 0 1280x800x24" "$@"; }
# case <name> <script> <root|noroot> <paths…>: headless in copy h_<name>, window in copy w_<name>
FAIL=0
case_() {
  n=$1 s=$2 mode=$3; shift 3; cp -r d "h_$n"; cp -r d "w_$n"
  if [ "$mode" = root ]; then
    timeout 180 "$K" script "$s" --root "h_$n" "$@" > "h_$n.out" 2> "h_$n.err"; hc=$?
    xv sh -c '"$0" "$@" 2>"w_'"$n"'.err"; k=$?; echo $k > "w_'"$n"'.st"; exit $k' "$K" --root "w_$n" "$@" --script "$s" > "w_$n.out"; xw=$?; wc=$(cat "w_$n.st" 2>/dev/null)
  else  # no --root: the fixture copy is the current directory, the script is given relative to it
    (cd "h_$n" && timeout 180 "$K" script "../$s" "$@" > "../h_$n.out" 2> "../h_$n.err"); hc=$?
    (cd "w_$n" && xv sh -c '"$0" "$@" 2>"../w_'"$n"'.err"; k=$?; echo $k > "../w_'"$n"'.st"; exit $k' "$K" "$@" --script "../$s" > "../w_$n.out"); xw=$?; wc=$(cat "w_$n.st" 2>/dev/null)
  fi
  echo "CASE $n: exit window=$wc headless=$hc"
  if [ -z "$wc" ]; then echo "  WINDOW EXIT NOT RECORDED: kiln's exit status was not written (wrapper exit=$xw)"; FAIL=1; elif [ "$xw" != "$wc" ]; then echo "  wrapper exit=$xw (not kiln's; not used)"; fi
  extra=""; [ "$n" = ac2 ] && extra=first
  python3 -I norm.py "w_$n.out" "h_$n.out" $extra || FAIL=1
  [ "$wc" = "$hc" ] || { echo "  EXIT DIFFERS"; FAIL=1; }
  if diff -r "w_$n" "h_$n" >/dev/null; then echo "  files EQUAL"; else echo "  FILES DIFFER"; diff -r "w_$n" "h_$n" | head -5; FAIL=1; fi
  [ -s "w_$n.err" ] && echo "  window stderr: $(head -c 300 "w_$n.err")"
}
case_ ac2 s.jsonl root a.rs src/b.rs missing.rs
case_ ac2x3 s3.jsonl root a.rs src/b.rs missing.rs
case_ dblclick dc.jsonl root a.rs
case_ scroll100 sc.jsonl root src/l.rs
case_ ctrlq q.jsonl root a.rs
case_ noroot s.jsonl noroot a.rs src/b.rs missing.rs
[ $FAIL = 0 ] && echo "WINDOW CASES PASS" || echo "WINDOW CASES FAIL"
# --- AC-3: Xvfb without the two libraries (precondition: neither installed on the system) ---
if ldconfig -p | grep -q -e 'libxkbcommon-x11\.so' -e 'libxcb-xkb\.so'; then echo "AC-3 PRECONDITION FAILED: a library is installed"; else
  cp -r d n
  env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u LD_LIBRARY_PATH MESA_SHADER_CACHE_DISABLE=true timeout 120 \
    xvfb-run -a -s "-screen 0 1280x800x24" sh -c "'$K' --root n a.rs 2>n.err; echo \$? > n.st" > n.out; e=$(cat n.st 2>/dev/null || echo "not-recorded")
  l=$(awk 'END{print NR}' n.err); p=$(grep -c panicked n.err); b=$(wc -c < n.out)
  echo "AC-3: exit=$e stdout_bytes=$b stderr_lines=$l panicked=$p (pass: 2 0 1 0)"; echo "  stderr: $(cat n.err)"
  [ "$e $b $l $p" = "2 0 1 0" ] && diff -r d n >/dev/null && echo "AC-3 PASS" || echo "AC-3 FAIL"
fi
# --- extra, Pinned (c) under a display: the script is checked before the window ---
xv sh -c "exec '$K' --root d a.rs --script missing.jsonl 2>m.err" > m.out; e=$?
echo "missing script: exit=$e stdout_bytes=$(wc -c < m.out) (pass: 2 0; 124 = a window opened)"
# --- b4, plain window (no --script): does it show its start path? Framebuffer of the virtual display ---
plain() { # plain <tag> <paths…>: start Xvfb with -fbdir, start kiln, wait until the screen is drawn and stable, copy it
  t=$1; shift; mkdir -p "fb_$t"; rm -f "disp_$t"
  Xvfb -displayfd 3 -screen 0 1280x800x24 -fbdir "$W/fb_$t" -nolisten tcp 3>"disp_$t" 2>/dev/null & xp=$!
  i=0; while [ ! -s "disp_$t" ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i+1)); done
  cp "fb_$t/Xvfb_screen0" "blank_$t"
  env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET DISPLAY=":$(cat "disp_$t")" LD_LIBRARY_PATH="$LIB" MESA_SHADER_CACHE_DISABLE=true \
    "$K" --root d "$@" > "plain_$t.out" 2> "plain_$t.err" & kp=$!
  i=0; st=0; : > "prev_$t"
  while [ $i -lt 120 ] && [ $st -lt 4 ]; do sleep 0.5; i=$((i+1)); cp "fb_$t/Xvfb_screen0" "cur_$t"
    if cmp -s "cur_$t" "blank_$t"; then st=0; elif cmp -s "cur_$t" "prev_$t"; then st=$((st+1)); else st=0; fi; cp "cur_$t" "prev_$t"; done
  kill -0 $kp 2>/dev/null && alive=yes || alive=no
  kill $kp 2>/dev/null; wait $kp 2>/dev/null; kill $xp 2>/dev/null; wait $xp 2>/dev/null
  echo "plain $t: drawn_and_stable=$([ $st -ge 4 ] && echo yes || echo no) still_running=$alive"; [ $st -ge 4 ] && [ $alive = yes ]
}
plain none && plain none2 && plain apath a.rs && {
  cmp -s cur_none cur_none2 && echo "  two runs without a path: same screen" || echo "  two runs without a path differ (screen not deterministic; comparison below not meaningful)"
  cmp -s cur_none cur_apath && echo "PLAIN FAIL: the screen with start path a.rs equals the one without" || echo "PLAIN PASS: the start path changes the screen"
} || echo "PLAIN NOT RUN: a window was not drawn, or kiln ended"
}
procedure "$@" | {
  window=0 plain=0 ac3=0 failed=0
  while IFS= read -r line || [ -n "$line" ]; do
    printf '%s\n' "$line"
    case $line in
      "WINDOW CASES PASS") window=1 ;;
      "PLAIN PASS: "*) plain=1 ;;
      "AC-3 PASS" | "AC-3 PRECONDITION FAILED: "*) ac3=1 ;;
      "WINDOW CASES FAIL" | "AC-3 FAIL" | "PLAIN FAIL: "* | "PLAIN NOT RUN: "*) failed=1 ;;
    esac
  done
  [ "$window$plain$ac3$failed" = 1110 ]
}
exit $?
