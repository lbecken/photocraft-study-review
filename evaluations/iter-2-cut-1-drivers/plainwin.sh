#!/bin/sh
# A plain window (no --script) under Xvfb with -fbdir, driven by XTest through xprobe.py.
# env: KILN_BIN, XLIBS (dir holding usr/lib/x86_64-linux-gnu), DRV (drivers dir), OUT (output dir),
#      FIX (fixture root), DPY (display number), SCREEN (e.g. 1280x800x24), ACTIONS (file of
#      xprobe.py argument lines; a line "shot <name>" takes a framebuffer screenshot, "wait <s>" waits),
#      ARGS (paths and options after --root)
set -u
mkdir -p "$OUT/home"
Xvfb ":$DPY" -screen 0 "$SCREEN" -fbdir "$OUT" -nolisten tcp >"$OUT/xvfb.log" 2>&1 &
XP=$!
i=0; while [ ! -S "/tmp/.X11-unix/X$DPY" ] && [ $i -lt 50 ]; do sleep 0.1; i=$((i+1)); done
export DISPLAY=":$DPY"
( cd "$OUT" && env -u WAYLAND_DISPLAY LD_LIBRARY_PATH="$XLIBS/usr/lib/x86_64-linux-gnu" MESA_SHADER_CACHE_DISABLE=true \
    HOME="$OUT/home" timeout 120 "$KILN_BIN" --root "$FIX" $ARGS >"$OUT/stdout.txt" 2>"$OUT/stderr.txt"; echo "$?" >"$OUT/rc.txt" ) &
KP=$!
i=0; while [ $i -lt 100 ]; do
  if timeout 5 python3 "$DRV/xprobe.py" tree 2>/dev/null | grep -q "name="; then break; fi
  sleep 0.1; i=$((i+1))
done
sleep 1
while read -r line; do
  set -- $line
  case "$1" in
    shot) convert "xwd:$OUT/Xvfb_screen0" "$OUT/$2.png" ;;
    wait) sleep "$2" ;;
    sh) shift; (cd "$FIX" && sh -c "$*") >>"$OUT/xprobe.log" 2>&1; sleep 0.3 ;;
    *) timeout 10 python3 "$DRV/xprobe.py" "$@" >>"$OUT/xprobe.log" 2>&1; sleep 0.3 ;;
  esac
done <"$ACTIONS"
i=0; while kill -0 $KP 2>/dev/null && [ $i -lt "${WAITMAX:-100}" ]; do sleep 0.1; i=$((i+1)); done
kill $XP 2>/dev/null; wait $XP 2>/dev/null
echo "kiln rc: $(cat "$OUT/rc.txt" 2>/dev/null)"
