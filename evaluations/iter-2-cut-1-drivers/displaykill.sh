#!/bin/sh
# Kill the display while the window is open (ui.md "Limits"). env: KILN_BIN, XLIBS, OUT (fresh), DPY, WITH_SCRIPT (0/1), DELAY
set -u
mkdir -p "$OUT/d" "$OUT/home"; printf 'fn a() {}\n' >"$OUT/d/a.rs"
i=0; : >"$OUT/long.jsonl"; while [ $i -lt 400 ]; do echo '{"ui":"frames","count":600}' >>"$OUT/long.jsonl"; i=$((i+1)); done
Xvfb ":$DPY" -screen 0 1280x800x24 -nolisten tcp >"$OUT/xvfb.log" 2>&1 &
XP=$!
i=0; while [ ! -S "/tmp/.X11-unix/X$DPY" ] && [ $i -lt 50 ]; do sleep 0.1; i=$((i+1)); done
S=""; [ "$WITH_SCRIPT" = 1 ] && S="--script $OUT/long.jsonl"
( env -u WAYLAND_DISPLAY DISPLAY=":$DPY" LD_LIBRARY_PATH="$XLIBS/usr/lib/x86_64-linux-gnu" MESA_SHADER_CACHE_DISABLE=true HOME="$OUT/home" \
    timeout 120 "$KILN_BIN" --root "$OUT/d" a.rs $S >"$OUT/stdout.txt" 2>"$OUT/stderr.txt"; echo $? >"$OUT/rc.txt" ) &
KP=$!
sleep "$DELAY"
kill -9 $XP
wait $KP
echo "with_script=$WITH_SCRIPT delay=$DELAY rc=$(cat "$OUT/rc.txt") stdout_bytes=$(wc -c <"$OUT/stdout.txt") stderr_lines=$(wc -l <"$OUT/stderr.txt")"
sed 's/^/   stderr: /' "$OUT/stderr.txt" | cut -c1-250 | head -6
rm -f "/tmp/.X11-unix/X$DPY" "/tmp/.X$DPY-lock"
