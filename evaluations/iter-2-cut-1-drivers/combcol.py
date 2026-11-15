"""Colours of painted rows next to combining marks and other zero-width chars (.rs), per char
against syntax.highlight. Prints, per line, the chars whose reported colour differs from the table."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl

COL = {"plain": "#dcdcdc", "keyword": "#569cd6", "type": "#4ec9b0", "function": "#dcdcaa",
       "string": "#ce9178", "number": "#b5cea8", "comment": "#6a9955"}
cases = ["let x = 1;", "́let x = 1;", "́ let x = 1;", "á let x = 1;", "x ́ let x = 1;", "é́ let x = 1;",
         "\t́let x = 1;", "élet x = 1;", "̈́ let x = 1;", "‍ let x = 1;", "a‍ let x = 1;",
         "​ let x = 1;", "﻿ let x = 1;", "// é let x = 1;", "let ́x = 1;", "⃣ let x = 1;", "̀̀̀ let x = 1;",
         "ि let x = 1;", "ั let x = 1;", "x́́́́ let x = 1;"]
text = "\n".join(cases) + "\n"
root = kl.fresh("cc"); kl.write_files(root, {"c.rs": text})
reqs = [{"id": "s", "ui": "snapshot"}, {"id": "h", "command": "syntax.highlight", "params": {"doc": 1}}]
rc, out, err, _ = kl.kscript(reqs, root=root, paths=["c.rs"])
r = {x["id"]: x for x in kl.parse(out)}
kinds = [None] * len(text)
for sp in r["h"]["result"]["spans"]:
    for q in range(sp["start"], sp["end"]):
        kinds[q] = sp["kind"]
ed = kl.one(r["s"], label="Editor")
off = 0
starts = []
for c in cases:
    starts.append(off); off += len(c) + 1
bad_lines = 0
for c, s, row in zip(cases, starts, ed["rows"]):
    chars = [(ch, run["color"]) for run in row for ch in run["text"]]
    txt = "".join(ch for ch, _ in chars)
    diffs = []
    for i, (ch, col) in enumerate(chars):
        if ch.isspace():
            continue
        exp = COL[kinds[s + i]]
        if col != exp:
            diffs.append(f"{i}:{ch!r}:{col}!={exp}")
    bad_lines += bool(diffs)
    print(f"{json.dumps(c)[:40]:42} rowtext_ok={txt == c} wrong_colours={len(diffs)} {' '.join(diffs[:4])}")
print("lines", len(cases), "lines_with_wrong_colours", bad_lines)
