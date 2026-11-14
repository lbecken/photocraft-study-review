"""nav.workspace-symbols vs a model: walk = project.tree (limit 1,000,000) file entries ending in
.rs; text = open document's current text, else decoded file; outline of each text from a
separate session's nav.outline; match/rank/limit/skipped per nav.md.
usage: python3 -I wsym.py <seed> <nfiles> <queries>"""
import sys, os, json, random, glob, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

seed_, nf, nq = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
rnd = random.Random(seed_)
allf = sorted(f for f in glob.glob(os.path.expanduser("~/.cargo/registry/src/*/*/**/*.rs"), recursive=True)
              if os.path.getsize(f) <= 8000)
root = fresh(f"ws-{seed_}")
picked = rnd.sample(allf, nf)
names = {}
for i, f in enumerate(picked):
    d = rnd.choice(["", "src/", "src/a/", "lib/", "ign/", "target/", "x/target/"])
    rel = f"{d}f{i}.rs"
    data = open(f, "rb").read()
    enc = rnd.random()
    if enc < 0.08: data = b"\xef\xbb\xbf" + data
    elif enc < 0.14: data = b"\xff\xfe" + data.decode("utf-8", "replace").encode("utf-16-le")
    elif enc < 0.18: data = b"\xfe\xff" + data.decode("utf-8", "replace").encode("utf-16-be")
    elif enc < 0.22: data = data + b"\n// \xff\n"
    write(root, rel, data)
write(root, ".gitignore", "ign/\n")
write(root, "notes.txt", "fn foo_in_txt() {}")
write(root, "UPPER.RS", "fn upper_rs() {}")
write(root, ".rs", "fn dot_rs_only() {}")
os.makedirs(os.path.join(root, "dir.rs")); write(root, "dir.rs/inner.rs", "struct InnerOfDirRs;")
os.symlink("f0.rs", os.path.join(root, "link.rs"))
os.mkfifo(os.path.join(root, "pipe.rs"))
write(root, "big.rs", "fn big_one() {}\n" + "a;" * 524288)  # 1,048,592 chars > limit
write(root, "edge.rs", "fn " + "e" * 3 + "() {}\n" + "a;" * ((1048576 - 13) // 2) + "a")  # exactly at the limit?
GENV = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", HOME="/nonexistent")

# 1) learn the walk and each file's outline in a separate session
rc, rs, err = runj([req("project.tree", id="t", limit=1000000)], root=root)
entries = rs[0]["result"]["entries"]
rsfiles = [e["path"] for e in entries if e["kind"] == "file" and e["path"].endswith(".rs")]
lines = []
for i, p in enumerate(rsfiles):
    lines += [req("doc.open", id=f"o{i}", path=p), req("nav.outline", id=f"n{i}", doc=i + 1 - sum(0 for _ in [])), req("doc.text", id=f"x{i}", doc=i + 1)]
# doc ids: a failed doc.open does not use an id, so map ids after the run
rc, rs, err = runj(lines, root=root, timeout=600)
by = {r["id"]: r for r in rs}
# rerun with correct ids
lines = []; nid = 0; idmap = {}
for i, p in enumerate(rsfiles):
    o = by[f"o{i}"]
    lines.append(req("doc.open", id=f"o{i}", path=p))
    if o["ok"]:
        nid += 1; idmap[p] = nid
        lines += [req("nav.outline", id=f"n{i}", doc=nid), req("doc.info", id=f"i{i}", doc=nid)]
rc, rs, err = runj(lines, root=root, timeout=600)
by = {r["id"]: r for r in rs}
outl = {}; skipped_exp = []
for i, p in enumerate(rsfiles):
    o = by[f"o{i}"]
    if not o["ok"]:
        skipped_exp.append({"path": p, "reason": o["error"]["kind"]}); continue
    n = by[f"n{i}"]
    if not n["ok"]:
        skipped_exp.append({"path": p, "reason": n["error"]["kind"]}); continue
    outl[p] = n["result"]["items"]
texts = {}
lines = []
for i, p in enumerate(rsfiles):
    if p in idmap:
        pass


def linecol(text, off):
    line = 0; ls = 0; i = 0
    while i < off:
        c = text[i]
        if c == "\r" and text[i + 1:i + 2] == "\n":
            if i + 1 >= off: break
            i += 2; line += 1; ls = i; continue
        if c in "\r\n":
            i += 1; line += 1; ls = i; continue
        i += 1
    return line, off - ls


# texts for line/col: fetch through doc.text
lines = []
for i, p in enumerate(rsfiles):
    if p in idmap and p in outl:
        lines.append(req("doc.open", id=f"o{i}", path=p))
nid = 0; ids = {}
for i, p in enumerate(rsfiles):
    if p in idmap and p in outl:
        nid += 1; ids[p] = nid
lines += [req("doc.text", id=f"x{ids[p]}", doc=ids[p]) for p in ids]
rc, rs, err = runj(lines, root=root, timeout=600)
by = {r["id"]: r for r in rs}
for p in ids:
    texts[p] = by[f"x{ids[p]}"]["result"]["text"]

QS = ["new", "a", "e", "FROM", "test", "parse", "eee", "impl", "Debug", "fmt", "x", "big_one", "upper", "dot", "inner", "foo",
      "ß", "Σ", "_", "fn", "from_str", "Error", "default", "::", "<", " for "]
checks = bad = 0
for qi in range(nq):
    q = rnd.choice(QS)
    lim = rnd.choice([None, 1, 3, 10, 200, 10000])
    ql = q.lower()
    ranked = []
    for wi, p in enumerate(rsfiles):
        if p not in outl: continue
        for it in outl[p]:
            nm = it["name"].lower()
            if ql in nm:
                rank = 0 if nm == ql else (1 if nm.startswith(ql) else 2)
                ln, col = linecol(texts[p], it["name_start"])
                ranked.append((rank, wi, it["start"], {"path": p, "kind": it["kind"], "name": it["name"], "start": it["start"], "end": it["end"],
                                                       "name_start": it["name_start"], "name_end": it["name_end"], "line": ln, "col": col}))
    ranked.sort(key=lambda x: (x[0], x[1], x[2]))
    L = lim or 200
    exp = {"items": [x[3] for x in ranked[:L]], "truncated": len(ranked) > L, "skipped": skipped_exp, "walk_truncated": False, "too_deep": 0}
    p = {"query": q}
    if lim: p["limit"] = lim
    rc, rs, err = runj([req("nav.workspace-symbols", id=1, **p)], root=root, timeout=600)
    got = rs[0]["result"]
    checks += 1
    if got != exp:
        bad += 1
        if bad <= 4:
            print("MISMATCH q=", q, "limit", lim, "got", len(got["items"]), "exp", len(exp["items"]))
            for k in ("truncated", "skipped", "walk_truncated", "too_deep"):
                if got[k] != exp[k]: print("  ", k, json.dumps(got[k])[:300], "| exp", json.dumps(exp[k])[:300])
            d = next((i for i in range(min(len(got["items"]), len(exp["items"]))) if got["items"][i] != exp["items"][i]), None)
            if d is not None: print("   first diff", d, json.dumps(got["items"][d]), json.dumps(exp["items"][d]))
print(f"seed={seed_} rsfiles={len(rsfiles)} searched={len(outl)} skipped={json.dumps(skipped_exp)} queries={checks} mismatches={bad}")
