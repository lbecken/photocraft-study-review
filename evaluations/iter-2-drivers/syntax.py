"""syntax.tree / syntax.highlight / syntax.match-bracket on third-party compiling Rust files (cargo
registry sources, not Kiln's corpus): has_error, sexp well-formed, highlight partition + clipping
+ kinds vs sx.highlight_model, brackets vs sx.brackets_model at every bracket char and some other
offsets. usage: python3 -I syntax.py <seed> <files> [maxbytes]"""
import sys, os, json, random, glob
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *
import sx

seed_, nfiles = int(sys.argv[1]), int(sys.argv[2])
maxb = int(sys.argv[3]) if len(sys.argv) > 3 else 20000
rnd = random.Random(seed_)
allf = sorted(f for f in glob.glob(os.path.expanduser("~/.cargo/registry/src/*/*/**/*.rs"), recursive=True)
              if os.path.getsize(f) <= maxb)
files = rnd.sample(allf, nfiles)
stats = dict(files=0, has_error=0, hl_chars=0, hl_kind_mismatch_files=0, partition_bad=0, clip_bad=0,
             br_checks=0, br_mismatch=0, mode_bad=0)
kind_conf = {}
examples = []
for f in files:
    try:
        text = open(f, encoding="utf-8").read()
    except UnicodeDecodeError:
        continue
    if text.startswith("﻿"):
        continue
    stats["files"] += 1
    L = len(text)
    ranges = [(0, L)] + [tuple(sorted((rnd.randint(0, L), rnd.randint(0, L)))) for _ in range(3)]
    brk = [p for p, c in enumerate(text) if c in "()[]{}"]
    offs = brk + [rnd.randint(0, L) for _ in range(20)] + [L]
    cmds = [("syntax.tree", {"doc": 1}), ("syntax.tree", {"doc": 1})]
    cmds += [("syntax.highlight", {"doc": 1, "start": a, "end": b}) for a, b in ranges]
    cmds += [("syntax.match-bracket", {"doc": 1, "offset": o}) for o in offs]
    cmds += [("doc.info", {"doc": 1})]
    rs = doc_session(text, cmds, path="f.rs")
    tree = rs[1]["result"]; tree2 = rs[2]["result"]
    if tree["mode"] != "full" or tree2["mode"] != "unchanged" or {**tree2, "mode": "full"} != tree:
        stats["mode_bad"] += 1
    if tree["has_error"]:
        stats["has_error"] += 1
        examples.append(("has_error", f.split("/src/", 1)[1], tree["errors"][:3]))
    root = sx.parse_sexp(tree["sexp"])
    model = sx.highlight_model(text, root)
    full = rs[3]["result"]["spans"]
    if not sx.check_partition(full, 0, L): stats["partition_bad"] += 1
    got = sx.spans_to_kinds(full, 0, L)
    stats["hl_chars"] += L
    bad = [(p, got[p], model[p]) for p in range(L) if p < len(got) and got[p] != model[p]]
    if bad:
        stats["hl_kind_mismatch_files"] += 1
        for p, g, m in bad:
            kind_conf[(g, m)] = kind_conf.get((g, m), 0) + 1
        p = bad[0][0]
        examples.append(("hl", f.split("/src/", 1)[1], p, text[max(0, p - 15):p + 15], bad[0][1], bad[0][2], len(bad)))
    for (a, b), r in zip(ranges, rs[3:3 + len(ranges)]):
        sp = r["result"]["spans"]
        if not sx.check_partition(sp, a, b): stats["partition_bad"] += 1
        if sp != sx.kinds_to_spans(got, a, b): stats["clip_bad"] += 1
    bm = sx.brackets_model(text, root)
    for o, r in zip(offs, rs[3 + len(ranges):3 + len(ranges) + len(offs)]):
        stats["br_checks"] += 1
        exp = {"open": bm[o][0], "close": bm[o][1]} if o in bm else None
        if r["result"]["bracket"] != exp:
            stats["br_mismatch"] += 1
            if sum(1 for e in examples if e[0] == "br") < 8:
                examples.append(("br", f.split("/src/", 1)[1], o, text[max(0, o - 15):o + 15], r["result"], exp))
    info = rs[-1]["result"]
    if info["dirty"] or info["can_undo"]:
        stats["mode_bad"] += 1
print(f"seed={seed_} " + " ".join(f"{k}={v}" for k, v in stats.items()))
print("kind confusions (got, model): count", json.dumps({f"{g}/{m}": c for (g, m), c in sorted(kind_conf.items())}))
for e in examples[:40]:
    print(json.dumps(e, ensure_ascii=False)[:400])
