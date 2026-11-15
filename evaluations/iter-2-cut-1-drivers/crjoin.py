"""Enter in a `cr` document with a `\n` right after the selection's end (ui-editor.md join case)."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl
for name, text, sel in [("j1.rs", "a\rb\rc\n", (5, 5)), ("j2.rs", "a\rb\rcd\n", (5, 6)), ("j3.rs", "a\rb\rc\nd\re\r", (5, 5)), ("j4.rs", "a\rb\rc\r\n", (5, 5))]:
    root = kl.fresh("cj"); kl.write_files(root, {name: text})
    reqs = [{"id": "f", "ui": "focus", "target": {"label": "Editor"}},
            {"id": 0, "command": "cursor.set", "params": {"doc": 1, "anchor": sel[0], "head": sel[1]}},
            {"id": "k", "ui": "key", "key": "Enter"}, {"id": "s", "ui": "snapshot"},
            {"id": 1, "command": "doc.text", "params": {"doc": 1}}, {"id": 2, "command": "cursor.get", "params": {"doc": 1}},
            {"id": 3, "command": "doc.info", "params": {"doc": 1}}]
    rc, out, err, _ = kl.kscript(reqs, root=root, paths=[name], log=root + ".log")
    res = {r["id"]: r for r in kl.parse(out)}
    log = [json.loads(l) for l in open(root + ".log")]
    msg = kl.one(res["s"], label="Message")
    print(name, repr(text), "sel", sel, "->", json.dumps(res[1]["result"]["text"]), "cursor", (res[2]["result"]["anchor"], res[2]["result"]["head"], res[2]["result"]["line"], res[2]["result"]["col"]),
          "le", res[3]["result"]["line_ending"], "mut", [(l["command"], l["params"]) for l in log if l["command"] in ("doc.insert", "doc.replace", "cursor.set")][1:],
          "msg", msg.get("value"), "rc", rc)
