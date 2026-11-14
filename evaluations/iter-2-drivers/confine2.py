"""ws.path-confinement changes of this stage: root that cannot be opened for reading is refused at
start (run/control/exec); re-open of an already-open path is checked against the root and the
path's location. Run without DAC capabilities (nodac.sh) where permissions matter.
usage: python3 -I confine2.py"""
import sys, os, json, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

N = [os.path.join(os.path.dirname(os.path.abspath(__file__)), "nodac.sh")]


def sh(args, stdin=b""):
    p = subprocess.run(N + ["timeout", "30"] + args, input=stdin, capture_output=True)
    return p.returncode, p.stdout.decode()[:200], p.stderr.decode().strip().replace(WORK, "$W")[:300]


print("=== 1. root modes at start (no DAC)")
base = fresh("c2-modes")
for mode in (0o700, 0o500, 0o311, 0o300, 0o100, 0o000, 0o400):
    r = os.path.join(base, f"r{mode:o}")
    os.makedirs(r); write(r, "a.txt", "x\n"); os.chmod(r, mode)
    script = os.path.join(WORK, "c2-open.jsonl"); open(script, "w").write('{"id":1,"command":"doc.open","params":{"path":"a.txt"}}\n')
    out = [sh([KILN, "run", script, "--root", r]), sh([KILN, "control", "--root", r], b'{"id":1,"command":"doc.list"}\n'),
           sh([KILN, "exec", "doc.list", "{}", "--root", r])]
    print(f"mode {mode:o}:", json.dumps(out))
    os.chmod(r, 0o700)
# relative root as given, a root symlink to an unreadable dir, a dir above the root not searchable
r = os.path.join(base, "r300"); os.chmod(r, 0o300)
os.symlink("r300", os.path.join(base, "lnk300"))
print("link to 0300 dir:", sh([KILN, "exec", "doc.list", "{}", "--root", os.path.join(base, "lnk300")]))
p = subprocess.run(N + ["bash", "-c", f"cd {base} && timeout 30 {KILN} exec doc.list '{{}}' --root r300"], capture_output=True)
print("relative root:", p.returncode, p.stderr.decode().strip())
os.chmod(r, 0o700)
up = os.path.join(base, "up"); os.makedirs(os.path.join(up, "root")); write(os.path.join(up, "root"), "a.txt", "x")
os.chmod(up, 0o600)
print("parent not searchable:", sh([KILN, "exec", "doc.list", "{}", "--root", os.path.join(up, "root")]))
os.chmod(up, 0o700)

print("=== 2. root made unreadable / unsearchable after start")
r = fresh("c2-after")
write(r, "a.txt", "a\n"); write(r, "b.txt", "b\n"); write(r, "s/c.rs", "fn c() {}")
c = Ctl(r, prefix=N)
print(c.req("doc.open", path="a.txt")["ok"])
os.chmod(r, 0o300)
for cmd, p in [("doc.open", {"path": "b.txt"}), ("doc.insert", {"doc": 1, "offset": 0, "text": "X"}), ("doc.save", {"doc": 1}),
               ("doc.open", {"path": "a.txt"}), ("project.tree", {}), ("project.tree", {"path": "s"}), ("nav.workspace-symbols", {"query": "c"})]:
    print("  0300:", cmd, json.dumps(p), "->", json.dumps(c.req(cmd, **p))[:250])
os.chmod(r, 0o600)
for cmd, p in [("doc.open", {"path": "s/c.rs"}), ("doc.open", {"path": "a.txt"}), ("doc.save", {"doc": 1}), ("project.tree", {})]:
    print("  0600:", cmd, json.dumps(p), "->", json.dumps(c.req(cmd, **p))[:250])
os.chmod(r, 0o700)
print("  a.txt on disk:", open(os.path.join(r, "a.txt")).read().strip(), "rc", c.close())

print("=== 3. re-open of an already-open path")
P = fresh("c2-reopen"); out = fresh("c2-reopen-out")
write(out, "o.txt", "outside\n")
r = os.path.join(P)
for n in ["del", "dir", "fifo", "inlink", "outlink", "noread", "nodecode", "deep", "nosearch/f", "chain", "parentout/f", "same"]:
    write(r, n + (".txt" if "/" not in n else ".txt"), n + "\n")
write(r, "target.txt", "t\n")
c = Ctl(r, prefix=N)
opened = {}
for n in ["del", "dir", "fifo", "inlink", "outlink", "noread", "nodecode", "deep", "nosearch/f", "chain", "parentout/f", "same"]:
    p = n + ".txt"; opened[p] = c.req("doc.open", path=p)["result"]["doc"]
c.req("doc.insert", doc=opened["same.txt"], offset=0, text="EDIT ")
os.unlink(os.path.join(r, "del.txt"))
os.unlink(os.path.join(r, "dir.txt")); os.mkdir(os.path.join(r, "dir.txt"))
os.unlink(os.path.join(r, "fifo.txt")); os.mkfifo(os.path.join(r, "fifo.txt"))
os.unlink(os.path.join(r, "inlink.txt")); os.symlink("target.txt", os.path.join(r, "inlink.txt"))
os.unlink(os.path.join(r, "outlink.txt")); os.symlink("../c2-reopen-out/o.txt", os.path.join(r, "outlink.txt"))
os.chmod(os.path.join(r, "noread.txt"), 0)
open(os.path.join(r, "nodecode.txt"), "wb").write(b"\xff\xfe\xfd")
os.unlink(os.path.join(r, "deep.txt"))
prev = "target.txt"
for i in range(41):
    nm = f"l{i}"; os.symlink(prev, os.path.join(r, nm)); prev = nm
os.symlink(prev, os.path.join(r, "deep.txt"))
os.chmod(os.path.join(r, "nosearch"), 0o600)
os.unlink(os.path.join(r, "chain.txt")); os.symlink("../c2-reopen-out/../c2-reopen/target.txt", os.path.join(r, "chain.txt"))
os.rename(os.path.join(r, "parentout"), os.path.join(out, "parentout")); os.symlink("../c2-reopen-out/parentout", os.path.join(r, "parentout"))
for p in opened:
    print("  reopen", p, "->", json.dumps(c.req("doc.open", path=p))[:230])
print("  text of same.txt:", c.req("doc.text", doc=opened["same.txt"])["result"]["text"].strip())
print("  outside file:", open(os.path.join(out, "o.txt")).read().strip())
# save side of the same cases
for p in ["outlink.txt", "deep.txt", "nosearch/f.txt", "chain.txt"]:
    print("  save", p, "->", json.dumps(c.req("doc.save", doc=opened[p]))[:200])
os.chmod(os.path.join(r, "nosearch"), 0o700); os.chmod(os.path.join(r, "noread.txt"), 0o600)
# root moved away then removed: reopen refused
os.rename(r, r + ".moved")
print("  root moved: reopen same.txt ->", json.dumps(c.req("doc.open", path="same.txt"))[:200])
os.rename(r + ".moved", r)
print("  root back: reopen same.txt ->", json.dumps(c.req("doc.open", path="same.txt"))[:200])
print("  rc", c.close())

print("=== 4. a directory above the root made unsearchable after start")
up = fresh("c2-up"); r = os.path.join(up, "root"); os.makedirs(r); write(r, "a.txt", "a\n"); write(r, "b.txt", "b\n")
c = Ctl(r, prefix=N)
print(c.req("doc.open", path="a.txt")["ok"], c.req("doc.insert", doc=1, offset=0, text="Q")["ok"])
os.chmod(up, 0o600)
for cmd, p in [("doc.open", {"path": "a.txt"}), ("doc.open", {"path": "b.txt"}), ("doc.save", {"doc": 1}), ("project.tree", {}), ("nav.workspace-symbols", {"query": "a"})]:
    print("  ", cmd, json.dumps(p), "->", json.dumps(c.req(cmd, **p))[:220])
os.chmod(up, 0o700)
print("   after restoring:", json.dumps(c.req("doc.save", doc=1))[:200], open(os.path.join(r, "a.txt")).read().strip(), "rc", c.close())
