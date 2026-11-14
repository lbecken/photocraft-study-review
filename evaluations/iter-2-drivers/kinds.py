"""Sweep: every command with many hostile/boundary param values (generated from its params
schema), on disk and in memory, after opening a CRLF document; collect every (command, code,
kind) seen; compare `rejected` kinds with the command's published `refusals`; count `internal`.
usage: python3 -I kinds.py"""
import sys, os, json, itertools, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

d = json.loads(subprocess.run(["timeout", "20", KILN, "commands", "--schema"], capture_output=True).stdout)
VALS = {"integer": [0, 1, 2, 3, 5, 99, 2 ** 63, 2 ** 64 - 1, -1, None, "1", 1.5],
        "string": ["", "a", "a.txt", "../x", "sub", "missing", "fifo", "out", "bad.txt", "sub/x.txt/y", "(", "$9", "\r\n", "x" * 1001, "a//b", None, 1],
        "boolean": [True, False, None, "true"]}


def variants(c):
    props = c["params_schema"].get("properties", {})
    base = {}
    for k in c["params_schema"].get("required", []):
        base[k] = 1 if props[k].get("type") == "integer" else ("a" if props[k].get("type") == "string" else {"char": 1})
    out = [dict(base), {}]
    for k, sch in props.items():
        t = sch.get("type")
        vals = VALS.get(t, [None, {}, {"char": 3}, {"byte": 4}, {"utf16": 2}, {"line_col": {"line": 0, "col": 9}}, {"char": 2}, "x"])
        if "enum" in sch: vals = sch["enum"] + ["bogus", None]
        if "oneOf" in sch or "$ref" in sch or "anyOf" in sch: vals = vals + ["char", "word", "line", "lf"]
        for v in vals:
            p = dict(base); p[k] = v
            out.append(p)
    return out


root = fresh("kinds")
write(root, "a.txt", "ab\r\ncd\n"); write(root, "b.rs", "fn f() { (x) }\n"); write(root, "sub/x.txt", "x"); write(root, "bad.txt", b"\xff")
os.mkfifo(os.path.join(root, "fifo")); os.symlink("/etc/hostname", os.path.join(root, "out"))
seen = {}; internal = []; n = 0
for c in d["commands"]:
    for p in variants(c):
        lines = [req("doc.open", id="o1", path="a.txt"), req("doc.open", id="o2", path="b.rs"), req("doc.insert", id="e", doc=2, offset=0, text=" "),
                 req(c["name"], id="x", **{k: v for k, v in p.items()})]
        for mode in ("disk", "mem"):
            if mode == "disk":
                rc, rs, err = runj(lines, root=root, name="kinds.jsonl")
            else:
                rc, rs, err = mem(lines, [("a.txt", "ab\r\ncd\n"), ("b.rs", "fn f() { (x) }\n"), ("sub/x.txt", "x"), ("bad.txt", b"\xff")], name="kinds-m.jsonl")
            r = rs[-1]; n += 1
            if not r["ok"]:
                key = (c["name"], r["error"]["code"], r["error"]["kind"])
                seen[key] = seen.get(key, 0) + 1
                if r["error"]["code"] == "internal": internal.append((c["name"], p, r))
undeclared = sorted(k for k in seen if k[1] == "rejected" and k[2] not in next(c for c in d["commands"] if c["name"] == k[0])["refusals"])
never = sorted((c["name"], k) for c in d["commands"] for k in c["refusals"] if (c["name"], "rejected", k) not in seen)
print(f"requests={n} distinct (command, code, kind)={len(seen)} internal={len(internal)}")
print("rejected kinds not declared by the command:", undeclared)
print("declared kinds not seen in this sweep:", never)
print("codes seen:", sorted({k[1] + "/" + k[2] for k in seen}))
