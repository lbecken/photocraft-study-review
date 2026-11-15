#!/bin/sh
# Plain window: open a.txt, type, then replace a.txt on disk (directory / link out / link in),
# then ctrl+S; screenshot of Message and Title. Fresh dirs: <name>-out and <name>-fix under $BASE.
# env: KILN_BIN, XLIBS, DRV, BASE
set -u
one() {
  name=$1; dpy=$2; change=$3
  out="$BASE/$name-out"; fix="$BASE/$name-fix"
  mkdir "$out" "$fix" || return
  printf 'abc\n' >"$fix/a.txt"; printf 'line\n' >"$fix/b.txt"
  printf 'focus\nclick 30 280\ntype Q\nsh %s\nkey s ctrl\nshot s2\nclose\n' "$change" >"$out/actions"
  OUT="$out" FIX="$fix" DPY="$dpy" SCREEN=1280x800x24 ACTIONS="$out/actions" ARGS="a.txt" timeout 100 sh "$DRV/plainwin.sh"
  echo "== $name: after: $(cd "$fix" && ls -la | tail -n +4 | awk '{print $1, $NF, $(NF-1), $(NF-2)}' | tr '\n' ';')"
  echo "   b.txt: $(od -c "$fix/b.txt" | head -1)"
  convert "$out/s2.png" -crop 700x30+0+0 +repage "$out/top.png"
  convert "$out/s2.png" -crop 700x60+0+245 +repage "$out/ed.png"
  convert "$out/top.png" "$out/ed.png" -append "$out/strip.png"
}
one rdir 93 'mv a.txt a.old && mkdir a.txt'
one rlnkout 94 'mv a.txt a.old && ln -s /etc/hostname a.txt'
one rlnkin 95 'mv a.txt a.old && ln -s b.txt a.txt'
one rgone 96 'mv a.txt a.old'
