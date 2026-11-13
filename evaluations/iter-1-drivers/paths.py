"""Path confinement probes (docs/protocol/paths.md) through `kiln-cli control`.
usage: python3 -I paths.py   (builds fixtures under work/pc/, prints every request/response)"""
import sys, os, json, shutil, socket, stat, hashlib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import Ctl, WORK, exe

P = os.path.join(WORK, "pc")
if os.path.isdir(P):
    shutil.rmtree(P)
R = os.path.join(P, "root")
O = os.path.join(P, "out")
os.makedirs(R); os.makedirs(O); os.makedirs(os.path.join(P, "rootx"))
os.makedirs(os.path.join(R, "deep", "inner")); os.makedirs(os.path.join(R, "sub"))
os.makedirs(os.path.join(R, "out"))  # decoy inside the root


def w(p, s):
    with open(p, "w", newline="") as f:
        f.write(s)


w(os.path.join(R, "f.txt"), "inside\n")
w(os.path.join(R, "g.txt"), "g\n")
w(os.path.join(R, "h.txt"), "h\n")
w(os.path.join(R, "out", "secret.txt"), "DECOY INSIDE\n")
w(os.path.join(O, "secret.txt"), "SECRET OUTSIDE\n")
w(os.path.join(P, "rootx", "s.txt"), "SIBLING\n")
L = lambda name, target: os.symlink(target, os.path.join(R, name))
L("rel_out", "../out/secret.txt")
L("abs_out", os.path.join(O, "secret.txt"))
L("dir_out", "../out")
L("chain1", "chain2"); L("chain2", "../out/secret.txt")
L("up", "deep/inner")
L("dotdot_after_dirlink", "up/../../../out/secret.txt")       # kernel: outside; text: ../../out
L("textually_inside", "dir_out/../root/../out/secret.txt")     # kernel: outside; text: out/secret.txt (decoy)
L("textually_outside", "dir_out/../root/f.txt")                # kernel: inside (R/f.txt)
L("dang_out", "../out/nonexistent.txt")
L("dang_abs", "/nonexistent-dir-kiln/x")
L("dang_in", "sub/new.txt")
L("back_in", "../root/f.txt")
L("back_missing", "../root/missing.txt")
L("via_missing_dir", "../nodir/../root/f.txt")
L("sibling", "../rootx/s.txt")
L("self", ".")
L("parent", "..")
L("loop1", "loop2"); L("loop2", "loop1")
L("slash_file", "f.txt/")
L("to_file", "f.txt")
L("to_root_abs", R)
# 40 and 41 links in a chain inside the root
for n, tgt in ((40, "f.txt"), (41, "f.txt")):
    prev = tgt
    for i in range(n):
        name = f"c{n}_{i}"
        os.symlink(prev, os.path.join(R, name)); prev = name
os.mkfifo(os.path.join(R, "fifo"))
L("fifo_link", "fifo")
os.mkfifo(os.path.join(O, "fifo_out"))
L("fifo_out_link", "../out/fifo_out")
s = socket.socket(socket.AF_UNIX); s.bind(os.path.join(R, "sock"))
os.mknod(os.path.join(R, "nulldev"), stat.S_IFCHR | 0o666, os.makedev(1, 3))
L("devnull", "/dev/null")
L("devzero_in_dir", "nulldev")
os.link(os.path.join(O, "secret.txt"), os.path.join(R, "hard"))


def snapshot():
    h = {}
    for base in (P,):
        for dp, dn, fn in os.walk(base):
            for n in fn + dn:
                p = os.path.join(dp, n)
                st = os.lstat(p)
                if stat.S_ISREG(st.st_mode):
                    h[p] = hashlib.sha256(open(p, "rb").read()).hexdigest()[:12]
                elif stat.S_ISLNK(st.st_mode):
                    h[p] = "->" + os.readlink(p)
                else:
                    h[p] = oct(st.st_mode)
    return h


before = snapshot()
c = Ctl(R)
log = []


def q(cmd, **p):
    r = c.req(cmd, **p)
    log.append((cmd, p, r))
    print(json.dumps([cmd, p]), "=>", json.dumps(r, ensure_ascii=False))
    return r


opens = ["rel_out", "abs_out", "dir_out/secret.txt", "dir_out/fifo_out", "dir_out", "chain1",
         "dotdot_after_dirlink", "textually_inside", "textually_outside", "dang_out", "dang_abs",
         "dang_in", "back_in", "back_missing", "via_missing_dir", "sibling", "self/f.txt",
         "parent/root/f.txt", "parent/out/secret.txt", "parent/rootx/s.txt", "parent", "self",
         "loop1", "slash_file", "to_file", "to_file/x", "f.txt/x", "nonexist/x", "to_root_abs/f.txt",
         "c40_39", "c41_40", "fifo", "fifo_link", "fifo_out_link", "sock", "nulldev", "devnull",
         "devzero_in_dir", "deep", "hard", "../f.txt", "./f.txt", "a//b", "/etc/passwd", "a\\b",
         "x\u0000y", "", "f.txt/", "dir_out/../f.txt"]
for p in opens:
    q("doc.open", path=p)
mid = snapshot()
print("UNCHANGED_AFTER_OPENS", mid == before)
for k in sorted(set(before) | set(mid)):
    if before.get(k) != mid.get(k):
        print("  diff", k, before.get(k), mid.get(k))
c.close()
s.close()
