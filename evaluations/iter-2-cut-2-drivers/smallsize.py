"""Layout at small --size values: is the Editor visible, is the status bar painted, does any other
node's text land in the Editor's rows? usage: smallsize.py   env: KILN_BIN, KILN_CLI, KILN_WORK."""
import os, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl
from k2 import Session, find, rows, status, editor, tabs_view

d = kl.fresh("small")
lines = [f"line{i:02}" for i in range(40)]
kl.write_files(d, {"a.txt": "\n".join(lines) + "\n", "b.txt": "b\n", "c.txt": "c\n"})
for ndocs in (1, 2, 3):
    for size in ("200x200", "300x200", "300x300", "400x300", "480x320", "640x400", "800x320", "800x600", "1280x800"):
        paths = ["b.txt", "c.txt"][: ndocs - 1] + ["a.txt"]
        reqs = [{"id": "g", "ui": "focus", "target": {"role": "multiline_text_input", "label": "Editor"}},
                {"id": "k", "ui": "key", "key": "ArrowDown"}, {"id": "s", "ui": "snapshot"}]
        s = Session(reqs, root=d, paths=paths, size=size, name="small")
        root = s.snap("s")
        e = editor(root)
        er = rows(e) if e else None
        foreign = [r for r in (er or []) if r and r.rstrip() not in lines]
        sb = status(root)
        painted = [k for k, v in sb.items() if isinstance(v, dict) and rows(v)]
        tabs_off = sum(1 for t in tabs_view(root) if t[5].get("offscreen"))
        print(f"docs={ndocs} size={size:9} focus={s['g'].get('ok')}/{(s['g'].get('error') or {}).get('kind')} "
              f"key={s['k'].get('result')} editor_offscreen={bool(e and e.get('offscreen'))} editor_rows={len(er or [])} "
              f"foreign_rows={foreign[:2]} bar_painted={painted} tabs_offscreen={tabs_off}")
