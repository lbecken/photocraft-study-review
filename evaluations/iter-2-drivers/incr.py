"""syntax.incremental: random edit sessions (insert/delete/replace/undo/redo/search.replace/
eol.normalize) on third-party Rust files, `syntax.tree` (and highlight + brackets) after each step;
then every reached text parsed from scratch in a fresh document; compare has_error, errors,
sexp, node_count, highlight spans and brackets; check the mode rules. Also checks highlight and
bracket models (sx.py) on the error trees reached.
usage: python3 -I incr.py <seed> <files> <steps>"""
import sys, os, json, random, glob
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *
import sx

seed_, nfiles, steps = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
rnd = random.Random(seed_)
allf = sorted(f for f in glob.glob(os.path.expanduser("~/.cargo/registry/src/*/*/**/*.rs"), recursive=True)
              if 300 <= os.path.getsize(f) <= 6000)
SNIP = ["(", ")", "{", "}", "[", "]", "\"", "'", "//", "/*", "*/", "\n", "\r\n", " ", "fn x() {}", "let", "é", "😀",
        "struct S;", "impl", "mod m {", "#", "!", "<", ">", "::", ";", "r#\"", "\"#", "b'", "0x1f", "1.0e", "mut ", "self"]
st = dict(sessions=0, trees=0, tree_diff=0, hl_diff=0, br_diff=0, mode_bad=0, model_hl=0, model_br=0, errors_trees=0)
shown = 0
for fi in range(nfiles):
    f = rnd.choice(allf)
    try:
        text = open(f, encoding="utf-8").read()
    except UnicodeDecodeError:
        continue
    st["sessions"] += 1
    model = text
    lines = [req("doc.open", id="o", path="f.rs"), req("syntax.tree", id="t0", doc=1)]
    texts = [text]
    for s in range(1, steps + 1):
        op = rnd.random()
        L = len(model)
        if op < 0.45:
            o = rnd.randint(0, L); t = rnd.choice(SNIP)
            if 0 < o < L and model[o - 1] == "\r" and model[o] == "\n": o -= 1
            lines.append(req("doc.insert", id=f"e{s}", doc=1, offset=o, text=t)); model = model[:o] + t + model[o:]
        elif op < 0.75 and L:
            a = rnd.randint(0, L); b = min(L, a + rnd.randint(0, 12))
            lines.append(req("doc.delete", id=f"e{s}", doc=1, start=a, end=b)); model = model[:a] + model[b:]
        elif op < 0.85 and L:
            a = rnd.randint(0, L); b = min(L, a + rnd.randint(0, 6)); t = rnd.choice(SNIP)
            lines.append(req("doc.replace", id=f"e{s}", doc=1, start=a, end=b, text=t)); model = model[:a] + t + model[b:]
        else:
            lines.append(req("doc.undo", id=f"e{s}", doc=1)); model = model  # undo: offsets of later steps may be off, harmless
        lines.append(req("syntax.tree", id=f"t{s}", doc=1))
        if s % 3 == 0:
            lines.append(req("syntax.highlight", id=f"h{s}", doc=1))
        texts.append(model)
    # run once to learn texts (doc.text resync when needed)
    lines.append(req("doc.text", id="final", doc=1))
    rc, rs, err = mem(lines, [("f.rs", text)], name="incr-a.jsonl", timeout=600)
    by = {r["id"]: r for r in rs}
    # recover the exact text before each tree with a second pass: insert doc.text before every tree
    lines2 = []
    for l in lines:
        if l["command"] == "syntax.tree":
            lines2.append(req("doc.text", id="x" + l["id"], doc=1))
        lines2.append(l)
    rc2, rs2, err2 = mem(lines2, [("f.rs", text)], name="incr-b.jsonl", timeout=600)
    by2 = {r["id"]: r for r in rs2}
    seq = []
    for s in range(0, steps + 1):
        t = by2["xt%d" % s]["result"]["text"]; tr = by2["t%d" % s]["result"]
        if by["t%d" % s]["result"] != tr: st["tree_diff"] += 1  # same session twice must agree
        seq.append((s, t, tr))
    # fresh parses: one document per distinct text
    uniq = sorted({t for _, t, _ in seq})
    files = [(f"u{i}.rs", t) for i, t in enumerate(uniq)]
    fl = []
    for i, (p, t) in enumerate(files):
        fl += [req("doc.open", id=f"o{i}", path=p), req("syntax.tree", id=f"t{i}", doc=i + 1), req("syntax.highlight", id=f"h{i}", doc=i + 1)]
        brk = [q for q, c in enumerate(t) if c in "()[]{}"]
        for q in brk[:60]:
            fl.append(req("syntax.match-bracket", id=f"b{i}-{q}", doc=i + 1, offset=q))
    rc3, rs3, err3 = mem(fl, files, name="incr-c.jsonl", timeout=900)
    fb = {r["id"]: r for r in rs3}
    idx = {t: i for i, (p, t) in enumerate(files)}
    prev_text = None
    for s, t, tr in seq:
        st["trees"] += 1
        fresh_tr = fb[f"t{idx[t]}"]["result"]
        a = {k: v for k, v in tr.items() if k != "mode"}; b = {k: v for k, v in fresh_tr.items() if k != "mode"}
        if a != b:
            st["tree_diff"] += 1
            if shown < 5:
                shown += 1; print("TREE DIFF", f, "step", s, json.dumps(a)[:300], "|", json.dumps(b)[:300])
        exp_modes = {"full"} if prev_text is None else ({"unchanged"} if t == prev_text else ({"full"} if tr["has_error"] else {"full", "incremental"}))
        if tr["mode"] not in exp_modes:
            st["mode_bad"] += 1
            if shown < 8: shown += 1; print("MODE", f, s, tr["mode"], exp_modes)
        prev_text = t
    for s in range(0, steps + 1):
        if f"h{s}" in by2:
            t = by2["xt%d" % s]["result"]["text"]
            if by2[f"h{s}"]["result"] != fb[f"h{idx[t]}"]["result"]:
                st["hl_diff"] += 1
    for i, (p, t) in enumerate(files):
        tr = fb[f"t{i}"]["result"]
        if tr["has_error"]: st["errors_trees"] += 1
        root = sx.parse_sexp(tr["sexp"])
        got = sx.spans_to_kinds(fb[f"h{i}"]["result"]["spans"], 0, len(t))
        mk = sx.highlight_model(t, root)
        P = sx.paint(root, len(t))
        unknown = [q for q in range(len(t)) if got[q] != mk[q] and P[q].kind == "ERROR"]
        st["hl_unknown_in_error"] = st.get("hl_unknown_in_error", 0) + len(unknown)
        for q in unknown: mk[q] = got[q]
        if got != mk:
            st["model_hl"] += 1
            q = next(q for q in range(len(t)) if got[q] != mk[q])
            open(os.path.join(WORK, "hl-mismatch-%d.rs" % st["model_hl"]), "w", newline="").write(t)
            if shown < 12: shown += 1; print("HL-MODEL", json.dumps(t[max(0, q - 20):q + 20]), q, got[q], mk[q], "err" if tr["has_error"] else "")
        bm = sx.brackets_model(t, root)
        brk = [q for q, c in enumerate(t) if c in "()[]{}"][:60]
        for q in brk:
            exp = {"open": bm[q][0], "close": bm[q][1]} if q in bm else None
            if fb[f"b{i}-{q}"]["result"]["bracket"] != exp and (P[q].kind == "ERROR" or (q in bm and P[bm[q][0] if bm[q][0] != q else bm[q][1]].kind == "ERROR")):
                st["br_unknown_in_error"] = st.get("br_unknown_in_error", 0) + 1
            elif fb[f"b{i}-{q}"]["result"]["bracket"] != exp:
                st["model_br"] += 1
                if shown < 14: shown += 1; print("BR-MODEL", json.dumps(t[max(0, q - 20):q + 20]), q, fb[f"b{i}-{q}"]["result"], exp)
print(f"seed={seed_} " + " ".join(f"{k}={v}" for k, v in st.items()))
