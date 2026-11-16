"""Documents at the size limit (1,048,576 chars) and one char over, a very long line and a file with
syntax errors, opened from the project panel: tab, bar, colours. env: KILN_BIN, KILN_CLI, KILN_WORK."""
import os, sys, time
sys.path.insert(0, os.path.dirname(__file__))
import kl
from k2 import Session, rows, status, tabs_view, editor, message
d = kl.fresh("limit")
line = "let x = 1; // c\n"
body = line * (1048576 // len(line))
body += "a" * (1048576 - len(body))
kl.write_files(d, {"at.rs": body, "over.rs": body + "b", "long.txt": "x" * 200000 + "\nend\n", "err.rs": "fn ( { let = ;\n\"unterminated\n"})
for p in ("at.rs", "over.rs", "long.txt", "err.rs"):
    t0 = time.time()
    s = Session([{"id": "p", "ui": "click", "target": {"role": "button", "label": "Project"}},
                 {"id": "o", "ui": "click", "target": {"role": "tree_item", "label": p}}, {"id": "s", "ui": "snapshot"},
                 {"id": "fe", "ui": "focus", "target": {"role": "multiline_text_input", "label": "Editor"}},
                 {"id": "e", "ui": "key", "key": "End"}, {"id": "s2", "ui": "snapshot"},
                 {"id": "c", "command": "cursor.get", "params": {"doc": 1}}, {"id": "i", "command": "doc.info", "params": {"doc": 1}},
                 {"id": "h", "command": "syntax.highlight", "params": {"doc": 1, "start": 0, "end": 3}}], root=d, name="limit", t=900)
    r = s.snap("s2")
    e = editor(r)
    cols = sorted({x["color"] for row in e.get("rows", [])[:2] for x in row})
    print(f"{p}: {time.time()-t0:.1f}s open={s['o'].get('ok')} tabs={[t[0] for t in tabs_view(r)]} bar={[v.get('value') for v in status(r).values() if isinstance(v, dict)]} "
          f"painted={[bool(rows(v)) for v in status(r).values() if isinstance(v, dict)]} cursor={s['c']['result']['line']},{s['c']['result']['col']} "
          f"len={s['i']['result']['len_chars']} colours={cols} highlight={s['h'].get('ok') or s['h']['error']['kind']} msg={message(r)!r}")
