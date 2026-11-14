"""Fuzz single-line .gitignore patterns outside the guaranteed forms against git, on a fixed
tree; print every pattern where Kiln's file+symlink list differs from git's.
usage: python3 -I tree_fuzz.py <seed> <n>"""
import sys, os, json, random, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

seed_, n = int(sys.argv[1]), int(sys.argv[2])
rnd = random.Random(seed_)
GENV = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", HOME="/nonexistent")
TOK = ["a", "b", "*", "**", "?", "/", "[", "]", "!", "\\", ".", "x", "[a-b]", "[!a]", "-", " ", "^", "{", "}", ","]
FILES = ["a", "b", "ab", "a.x", "x/a", "x/b", "x/y/a", "c/x", "c/b/x", "d/a", "[a]", "*", "?", "a b", "!a",
         "-", "^", ".a", "x.a", "{a}", "a,b", "aa/b", "ba"]
root = fresh("tfz")
for f in FILES:
    write(root, f, "")
os.symlink("x", os.path.join(root, "lx"))
subprocess.run(["git", "init", "-q", root], env=GENV, check=True)
diffs = {}
for i in range(n):
    pat = "".join(rnd.choice(TOK) for _ in range(rnd.randint(1, 4)))
    if "\n" in pat or pat.startswith("#"):
        continue
    write(root, ".gitignore", pat + "\n")
    g = subprocess.run(["timeout", "10", "git", "-C", root, "ls-files", "-z", "--others", "--exclude-standard"],
                       env=GENV, capture_output=True).stdout.decode().split("\0")
    g = sorted(x for x in g if x)
    rc, rs, err = runj([req("project.tree", id=1)], root=root)
    k = sorted(e["path"] for e in rs[0]["result"]["entries"] if e["kind"] in ("file", "symlink"))
    if k != g:
        diffs[pat] = (sorted(set(g) - set(k)), sorted(set(k) - set(g)))
print(f"seed={seed_} patterns={n} differing={len(diffs)}")
for p, (go, ko) in sorted(diffs.items()):
    print(json.dumps(p), "git-only", go, "kiln-only", ko)
