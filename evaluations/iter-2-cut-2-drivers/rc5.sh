#!/bin/sh
# Repeat one window run N times; print rc, stdout digest, kiln stderr and xvfb-run stderr sizes.
# env: KILN_BIN, XLIBS, KILN_WORK; args: <init dir> <script> <N> <tag>
I=$1; S=$2; N=$3; T=$4
k=0
while [ $k -lt "$N" ]; do
  W="$KILN_WORK/fx/rc5-$T-$k"; mkdir -p "$W" "$W.home"; cp -a "$I/." "$W/"
  env -u WAYLAND_DISPLAY LD_LIBRARY_PATH="$XLIBS/usr/lib/x86_64-linux-gnu" MESA_SHADER_CACHE_DISABLE=true HOME="$W.home" \
    timeout 300 xvfb-run -a -s "-screen 0 1280x800x24" sh -c "exec '$KILN_BIN' --root '$W' --script '$S' 2>'$W.err'" >"$W.out" 2>"$W.xerr"
  echo "run $k rc=$? out=$(md5sum <"$W.out" | cut -c1-8) kiln_stderr=$(wc -c <"$W.err") xvfb_run_stderr=$(wc -c <"$W.xerr") $(head -c 200 "$W.err" "$W.xerr" 2>/dev/null | tr '\n' ' ')"
  k=$((k+1))
done
