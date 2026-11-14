"""Root identity probes (docs/protocol/paths.md, "The root") via `kiln-cli control`.
usage: python3 -I rootid.py"""
import sys, os, json, shutil, stat
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import Ctl, WORK

P = os.path.join(WORK, "pc3")
if os.path.isdir(P):
    shutil.rmtree(P)


def w(p, s):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", newline="") as f:
        f.write(s)


def rd(p):
    return open(p).read().strip() if os.path.exists(p) else None


def tmps():
    return [os.path.relpath(os.path.join(dp, f), P) for dp, dn, fn in os.walk(P) for f in fn if f.startswith(".kiln-tmp")]


def session(c):
    def q(cmd, **p):
        r = c.req(cmd, **p)
        s = json.dumps(r, ensure_ascii=False)
        print(json.dumps([cmd, p]), "=>", s.replace(P, "$P"))
        return r
    return q


print("=== A: root directory renamed away, another directory put in its place, then moved back")
R = os.path.join(P, "A", "root")
w(os.path.join(R, "a.txt"), "orig\n")
c = Ctl(R); q = session(c)
q("doc.open", path="a.txt"); q("doc.insert", doc=1, offset=0, text="X")
os.rename(R, R + ".old"); w(os.path.join(R, "a.txt"), "impostor\n"); w(os.path.join(R, "b.txt"), "impostor b\n")
q("doc.save", doc=1); q("doc.open", path="b.txt"); q("doc.open", path="a.txt")
print("impostor a:", rd(os.path.join(R, "a.txt")), "orig a:", rd(os.path.join(R + ".old", "a.txt")), "tmp:", tmps())
shutil.rmtree(R)  # nothing in its place
q("doc.save", doc=1); q("doc.open", path="a.txt")
os.rename(R + ".old", R)
q("doc.save", doc=1); print("a after moving back:", rd(os.path.join(R, "a.txt")))
q("doc.open", path="a.txt")
print("rc", c.close())

print("=== B: --root is a symlink; re-pointed to a copy, then back")
B = os.path.join(P, "B"); R1 = os.path.join(B, "r1"); R2 = os.path.join(B, "r2"); LK = os.path.join(B, "link")
w(os.path.join(R1, "a.txt"), "r1\n"); w(os.path.join(R2, "a.txt"), "r2\n")
os.symlink("r1", LK)
c = Ctl(LK); q = session(c)
q("doc.open", path="a.txt"); q("doc.insert", doc=1, offset=0, text="X")
os.remove(LK); os.symlink("r2", LK)
q("doc.save", doc=1); q("doc.open", path="a.txt")
print("r2/a:", rd(os.path.join(R2, "a.txt")), "r1/a:", rd(os.path.join(R1, "a.txt")))
os.remove(LK); os.symlink(R1, LK)  # absolute link to the same directory
q("doc.save", doc=1); print("r1/a:", rd(os.path.join(R1, "a.txt")))
print("rc", c.close())

print("=== C: a directory above the root replaced by a symlink to the same directory, then to a copy")
C = os.path.join(P, "C"); MID = os.path.join(C, "mid"); RC = os.path.join(MID, "root")
w(os.path.join(RC, "a.txt"), "c\n")
c = Ctl(RC); q = session(c)
q("doc.open", path="a.txt"); q("doc.insert", doc=1, offset=0, text="X")
os.rename(MID, MID + ".real"); os.symlink("mid.real", MID)
q("doc.save", doc=1); print("mid.real/root/a:", rd(os.path.join(MID + ".real", "root", "a.txt")))
shutil.copytree(MID + ".real", os.path.join(C, "copy"), symlinks=True)
os.remove(MID); os.symlink("copy", MID)
q("doc.insert", doc=1, offset=0, text="Y"); q("doc.save", doc=1); q("doc.open", path="a.txt")
print("copy/root/a:", rd(os.path.join(C, "copy", "root", "a.txt")), "real:", rd(os.path.join(MID + ".real", "root", "a.txt")))
print("rc", c.close())

print("=== D: root replaced by a symlink to a directory outside")
D = os.path.join(P, "D"); RD = os.path.join(D, "root"); OD = os.path.join(D, "out")
w(os.path.join(RD, "a.txt"), "d\n"); w(os.path.join(OD, "a.txt"), "OUT\n")
c = Ctl(RD); q = session(c)
q("doc.open", path="a.txt"); q("doc.insert", doc=1, offset=0, text="X")
os.rename(RD, RD + ".old"); os.symlink("out", RD)
q("doc.save", doc=1); q("doc.open", path="a.txt"); q("doc.list")
print("out/a:", rd(os.path.join(OD, "a.txt")), "tmp:", tmps())
print("rc", c.close())

print("=== E: root renamed (moved) while the session runs: does the session follow the directory?")
E = os.path.join(P, "E"); RE = os.path.join(E, "root")
w(os.path.join(RE, "a.txt"), "e\n")
c = Ctl(RE); q = session(c)
q("doc.open", path="a.txt"); os.rename(RE, RE + ".moved")
q("doc.open", path="a.txt"); q("doc.text", doc=1)
print("rc", c.close())
