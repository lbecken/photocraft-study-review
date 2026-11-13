"""Permission-based probes run as uid 0 WITHOUT CAP_DAC_OVERRIDE / CAP_DAC_READ_SEARCH (capsh --drop),
so owner permission bits apply: an unreadable (0300) root, unreadable files and directories.
usage: python3 -I unreadable.py"""
import sys, os, json, shutil, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import KILN, WORK

P = os.path.join(WORK, "ur2")
if os.path.isdir(P):
    for dp, dn, fn in os.walk(P):
        os.chmod(dp, 0o700)
    shutil.rmtree(P)
R = os.path.join(P, "root")
os.makedirs(R)


def w(p, s):
    with open(p, "w") as f:
        f.write(s)


class Ctl:
    def __init__(self, root):
        cmd = f"exec timeout 600 {KILN} control --root {root}"
        self.p = subprocess.Popen(["capsh", "--drop=cap_dac_override,cap_dac_read_search", "--", "-c", cmd],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self.n = 0

    def q(self, cmd, **params):
        self.n += 1
        self.p.stdin.write((json.dumps({"id": self.n, "command": cmd, "params": params}) + "\n").encode())
        self.p.stdin.flush()
        r = json.loads(self.p.stdout.readline())
        print(json.dumps([cmd, params]), "=>", json.dumps(r).replace(P, "$P"))
        return r

    def close(self):
        self.p.stdin.close(); return self.p.wait(timeout=30)


w(os.path.join(R, "a.txt"), "orig a\n")
w(os.path.join(R, "noread.txt"), "secret\n"); os.chmod(os.path.join(R, "noread.txt"), 0o000)
os.makedirs(os.path.join(R, "nox")); w(os.path.join(R, "nox", "f.txt"), "f\n"); os.chmod(os.path.join(R, "nox"), 0o600)
os.makedirs(os.path.join(R, "ro")); w(os.path.join(R, "ro", "g.txt"), "g\n"); os.chmod(os.path.join(R, "ro"), 0o500)
os.chmod(R, 0o300)
st = os.stat(R)
print("root inode", st.st_ino, "mode", oct(st.st_mode))
c = Ctl(R)
print("--- basic operation in an unreadable (0300) root")
c.q("doc.open", path="a.txt"); c.q("doc.insert", doc=1, offset=0, text="X"); c.q("doc.save", doc=1)
c.q("doc.open", path="noread.txt"); c.q("doc.open", path="nox/f.txt"); c.q("doc.open", path="missing.txt")
c.q("doc.open", path="ro/g.txt"); c.q("doc.insert", doc=2, offset=0, text="Y"); c.q("doc.save", doc=2)
print("--- root renamed away, new directory created at the same path")
os.rename(R, R + ".old"); os.makedirs(R); w(os.path.join(R, "a.txt"), "impostor\n")
print("new root inode", os.stat(R).st_ino)
c.q("doc.insert", doc=1, offset=0, text="Z"); c.q("doc.save", doc=1); c.q("doc.open", path="a.txt")
print("impostor a.txt:", open(os.path.join(R, "a.txt")).read().strip())
shutil.rmtree(R); os.rename(R + ".old", R)
c.q("doc.save", doc=1)
print("--- root removed and recreated (inode may be reused: documented residual limit)")
os.chmod(R, 0o700)
saved = {n: open(os.path.join(R, n)).read() for n in ["a.txt"]}
for n in os.listdir(R):
    p = os.path.join(R, n)
    if os.path.isdir(p):
        for dp, dn, fn in os.walk(p):
            os.chmod(dp, 0o700)
        shutil.rmtree(p)
    else:
        os.remove(p)
os.rmdir(R); os.makedirs(R); w(os.path.join(R, "a.txt"), "replacement\n"); os.chmod(R, 0o300)
print("recreated root inode", os.stat(R).st_ino, "(was", st.st_ino, ")")
c.q("doc.insert", doc=1, offset=0, text="W"); c.q("doc.save", doc=1)
print("replacement a.txt now:", open(os.path.join(R, "a.txt")).read().strip())
print("rc", c.close())
for dp, dn, fn in os.walk(P):
    os.chmod(dp, 0o700)
