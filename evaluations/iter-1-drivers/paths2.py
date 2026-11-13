"""Save-time confinement and root identity probes (docs/protocol/paths.md), via `kiln-cli control`.
usage: python3 -I paths2.py"""
import sys, os, json, shutil, stat, hashlib, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import Ctl, WORK

P = os.path.join(WORK, "pc2")
if os.path.isdir(P):
    shutil.rmtree(P)
R = os.path.join(P, "root"); O = os.path.join(P, "out")
os.makedirs(R); os.makedirs(O)


def w(p, s):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", newline="") as f:
        f.write(s)


def tree(base):
    out = {}
    for dp, dn, fn in os.walk(base):
        for n in fn + dn:
            p = os.path.join(dp, n); st = os.lstat(p)
            rel = os.path.relpath(p, P)
            if stat.S_ISREG(st.st_mode):
                out[rel] = open(p, "rb").read()
            elif stat.S_ISLNK(st.st_mode):
                out[rel] = "->" + os.readlink(p)
            elif stat.S_ISDIR(st.st_mode):
                out[rel] = "dir"
            else:
                out[rel] = oct(st.st_mode)
    return out


for n in ["s1", "s4", "s5", "s7", "s8t", "s9", "s11", "reopen", "fifo2"]:
    w(os.path.join(R, n + ".txt"), n + "\n")
w(os.path.join(R, "sub", "a.txt"), "suba\n")
w(os.path.join(R, "sub2", "x.txt"), "sub2x\n")
w(os.path.join(R, "sub3", "y.txt"), "sub3y\n")
w(os.path.join(O, "secret.txt"), "SECRET\n")
os.makedirs(os.path.join(O, "subtarget"))
os.symlink("s8t.txt", os.path.join(R, "s8link"))

c = Ctl(R)


def q(cmd, **p):
    r = c.req(cmd, **p)
    s = json.dumps(r, ensure_ascii=False)
    assert P not in s, "root path leaked: " + s
    print(json.dumps([cmd, p]), "=>", s)
    return r


def edit_open(path):
    d = q("doc.open", path=path)["result"]["doc"]
    q("doc.insert", doc=d, offset=0, text="EDIT ")
    return d


def tmp_left():
    return [os.path.relpath(os.path.join(dp, f), P) for dp, dn, fn in os.walk(P) for f in fn if f.startswith(".kiln-tmp")]


print("--- S1 file replaced by link to outside file")
d = edit_open("s1.txt"); os.remove(os.path.join(R, "s1.txt")); os.symlink("../out/secret.txt", os.path.join(R, "s1.txt"))
q("doc.save", doc=d); q("doc.info", doc=d)
print("secret:", open(os.path.join(O, "secret.txt")).read().strip(), "tmp:", tmp_left())

print("--- S2 parent dir replaced by link to outside dir")
d = edit_open("sub/a.txt"); os.rename(os.path.join(R, "sub"), os.path.join(P, "sub.moved")); os.symlink("../out/subtarget", os.path.join(R, "sub"))
q("doc.save", doc=d)
print("out/subtarget:", os.listdir(os.path.join(O, "subtarget")), "tmp:", tmp_left())

print("--- S3 file replaced by dangling link to outside")
d = edit_open("s11.txt"); os.remove(os.path.join(R, "s11.txt")); os.symlink("../out/new.txt", os.path.join(R, "s11.txt"))
q("doc.save", doc=d)
print("out/new.txt exists:", os.path.lexists(os.path.join(O, "new.txt")), "tmp:", tmp_left())

print("--- S4 file replaced by FIFO")
d = edit_open("s4.txt"); os.remove(os.path.join(R, "s4.txt")); os.mkfifo(os.path.join(R, "s4.txt"))
t0 = time.time(); q("doc.save", doc=d); print("elapsed %.2fs" % (time.time() - t0))
print("still fifo:", stat.S_ISFIFO(os.lstat(os.path.join(R, "s4.txt")).st_mode), "tmp:", tmp_left())

print("--- S5 file replaced by directory")
d = edit_open("s5.txt"); os.remove(os.path.join(R, "s5.txt")); os.mkdir(os.path.join(R, "s5.txt"))
q("doc.save", doc=d); print("tmp:", tmp_left())

print("--- S6 parent dir replaced by a regular file")
d = edit_open("sub2/x.txt"); shutil.rmtree(os.path.join(R, "sub2")); w(os.path.join(R, "sub2"), "now a file\n")
q("doc.save", doc=d); print("sub2 content:", open(os.path.join(R, "sub2")).read().strip(), "tmp:", tmp_left())

print("--- S7 file replaced by dangling link inside the root")
d = edit_open("s7.txt"); os.remove(os.path.join(R, "s7.txt")); os.symlink("sub3/created.txt", os.path.join(R, "s7.txt"))
q("doc.save", doc=d)
print("link kept:", os.path.islink(os.path.join(R, "s7.txt")), "created:", open(os.path.join(R, "sub3", "created.txt")).read().strip())

print("--- S8 save through an inside link")
d = edit_open("s8link"); q("doc.save", doc=d)
print("link kept:", os.path.islink(os.path.join(R, "s8link")), "target:", open(os.path.join(R, "s8t.txt")).read().strip())

print("--- S9 file removed: save recreates it")
d = edit_open("s9.txt"); os.remove(os.path.join(R, "s9.txt")); q("doc.save", doc=d)
print("s9:", open(os.path.join(R, "s9.txt")).read().strip())

print("--- S10 parent dir removed")
d = edit_open("sub3/y.txt"); os.rename(os.path.join(R, "sub3"), os.path.join(P, "sub3.moved")); q("doc.save", doc=d)
print("tmp:", tmp_left())

print("--- S12 reopen of an open path after it became a link outside")
d = edit_open("reopen.txt"); os.remove(os.path.join(R, "reopen.txt")); os.symlink("../out/secret.txt", os.path.join(R, "reopen.txt"))
q("doc.open", path="reopen.txt"); q("doc.text", doc=d)

print("--- S13 doc.close of the refused documents with discard, ids of failed opens")
q("doc.list")
c.close()
print("FINAL secret:", open(os.path.join(O, "secret.txt")).read().strip(), "out tree:", sorted(tree(O)))
