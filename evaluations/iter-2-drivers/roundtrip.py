"""undo.roundtrip / undo.redo / ws.dirty-tracking regression: random edit sessions, undo all, redo all.
usage: python3 -I roundtrip.py <seed> <sessions> <edits>"""
import sys, os, random, shutil
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import Ctl, WORK
seed, ns, ne = map(int, sys.argv[1:4]); rnd = random.Random(seed)
root = os.path.join(WORK, f"rt-{seed}")
if os.path.isdir(root): shutil.rmtree(root)
os.makedirs(root)
A = ["a", "é", "😀", "\r\n", "\n", "\r", "中", "xy"]
ok = bad = 0
for s in range(ns):
    init = "".join(rnd.choice(A) for _ in range(rnd.randint(0, 15)))
    open(os.path.join(root, f"{s}.txt"), "w", newline="").write(init)
    c = Ctl(root); d = c.ok("doc.open", path=f"{s}.txt")["doc"]; t = init
    for _ in range(ne):
        op = rnd.choice(["ins", "del", "rep", "norm"])
        if op == "ins":
            o = rnd.randint(0, len(t)); x = rnd.choice(A); c.ok("doc.insert", doc=d, offset=o, text=x)
        elif op == "del":
            a = rnd.randint(0, len(t)); b = rnd.randint(a, len(t)); r = c.ok("doc.delete", doc=d, start=a, end=b)
            assert r["removed"] == t[a:b]
        elif op == "rep":
            a = rnd.randint(0, len(t)); b = rnd.randint(a, len(t)); c.ok("doc.replace", doc=d, start=a, end=b, text=rnd.choice(A))
        else:
            c.ok("eol.normalize", doc=d, to=rnd.choice(["lf", "crlf", "cr"]))
        t = c.ok("doc.text", doc=d)["text"]
    final = t; n_undo = 0
    while c.ok("doc.undo", doc=d)["undone"]: n_undo += 1
    i1 = c.ok("doc.info", doc=d); t0 = c.ok("doc.text", doc=d)["text"]
    n_redo = 0
    while c.ok("doc.redo", doc=d)["redone"]: n_redo += 1
    t1 = c.ok("doc.text", doc=d)["text"]; i2 = c.ok("doc.info", doc=d)
    c.ok("doc.save", doc=d); i3 = c.ok("doc.info", doc=d)
    good = (t0 == init and not i1["dirty"] and not i1["can_undo"] and t1 == final and n_undo == n_redo
            and i2["dirty"] == (final != init) and not i3["dirty"]
            and open(os.path.join(root, f"{s}.txt"), newline="").read() == final)
    ok += good; bad += not good
    c.close()
print(f"seed={seed} sessions={ns} edits={ne} ok={ok} mismatches={bad}")
