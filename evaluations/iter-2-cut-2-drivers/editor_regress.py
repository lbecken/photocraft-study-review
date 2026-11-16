"""Regression check of ui.editor-view: text with combining marks and a ligature through the snapshot:
caret positions after clicks on every column and after shift-arrows, selection and painted rows
against cursor.get and doc.text, colours against syntax.highlight. env: KILN_BIN, KILN_CLI, KILN_WORK."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl
from k2 import Session, rows, editor

COL = {"plain": "#dcdcdc", "keyword": "#569cd6", "type": "#4ec9b0", "function": "#dcdcaa", "string": "#ce9178",
       "number": "#b5cea8", "comment": "#6a9955"}
ED = {"role": "multiline_text_input", "label": "Editor"}
LINES = ["let é́ = \"ﬁﬂ\";", "fn ẍ() { 1 }", "// á ﬁ ‍ z", "struct Ş;"]
text = "\n".join(LINES) + "\n"
for path in ("c.rs", "c.txt"):
    d = kl.fresh("edreg")
    kl.write_files(d, {path: text})
    reqs = [{"id": "s0", "ui": "snapshot"}, {"id": "h", "command": "syntax.highlight", "params": {"doc": 1}}]
    starts = [0]
    for l in LINES:
        starts.append(starts[-1] + len(l) + 1)
    for r, l in enumerate(LINES):
        for c in range(len(l) + 1):
            reqs += [{"id": f"c{r}-{c}", "ui": "click", "target": ED, "at": {"row": r, "col": c}},
                     {"id": f"g{r}-{c}", "command": "cursor.get", "params": {"doc": 1}}, {"id": f"s{r}-{c}", "ui": "snapshot"}]
    reqs += [{"id": "home", "ui": "click", "target": ED, "at": {"row": 0, "col": 0}}]
    for k in range(8):
        reqs += [{"id": f"x{k}", "ui": "key", "key": "ArrowRight", "modifiers": ["shift"]},
                 {"id": f"xg{k}", "command": "cursor.get", "params": {"doc": 1}}, {"id": f"xs{k}", "ui": "snapshot"}]
    reqs += [{"id": "ty", "ui": "type", "text": "ﬁé"}, {"id": "t", "command": "doc.text", "params": {"doc": 1}},
             {"id": "gt", "command": "cursor.get", "params": {"doc": 1}}, {"id": "st", "ui": "snapshot"}]
    s = Session(reqs, root=d, paths=[path], name="edreg")
    mism = clicks = 0
    e0 = editor(s.snap("s0"))
    if rows(e0) != LINES:
        mism += 1
        print("  rows", rows(e0))
    spans = s["h"]["result"]["spans"]
    kind = [None] * len(text)
    for sp in spans:
        for i in range(sp["start"], sp["end"]):
            kind[i] = sp["kind"]
    colour_bad = []
    for r, row in enumerate(e0["rows"]):
        i = starts[r]
        for run in row:
            for ch in run["text"]:
                want = COL[kind[i]] if path.endswith(".rs") else COL["plain"]
                if not ch.isspace() and run["color"] != want and ch not in "̧́̈‍":
                    colour_bad.append((r, i - starts[r], ch, run["color"], want))
                i += 1
    for r, l in enumerate(LINES):
        for c in range(len(l) + 1):
            clicks += 1
            g = s[f"g{r}-{c}"]["result"]
            sel = editor(s.snap(f"s{r}-{c}"))["selection"]
            if not s[f"c{r}-{c}"]["ok"] or g["head"] != starts[r] + c or g["anchor"] != g["head"] or sel != {"anchor": g["anchor"], "head": g["head"]}:
                mism += 1
                print("  click", r, c, s[f"c{r}-{c}"].get("ok"), g, sel)
    for k in range(8):
        g = s[f"xg{k}"]["result"]
        sel = editor(s.snap(f"xs{k}"))["selection"]
        if g["anchor"] != 0 or g["head"] != k + 1 or sel != {"anchor": 0, "head": k + 1}:
            mism += 1
            print("  shift-right", k, g, sel)
    t = s["t"]["result"]["text"]
    want_t = "ﬁé" + text[8:]
    if t != want_t or editor(s.snap("st"))["value"] != t or s["gt"]["result"]["head"] != 3:
        mism += 1
        print("  typed over selection:", repr(t[:20]), s["gt"]["result"])
    print(f"{path}: rows equal lines: {rows(e0) == LINES}; clicks {clicks}; shift-arrows 8; typed over selection; mismatches {mism}; "
          f"colour mismatches (non-blank, non-mark chars) {len(colour_bad)} {colour_bad[:6]}")
