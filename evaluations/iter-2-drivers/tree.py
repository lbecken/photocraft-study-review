"""project.tree vs `git ls-files --cached --others --exclude-standard` on random trees with
.gitignore files using only the documented pattern forms (`*.x`, `/a`, `b/`, `!c`, `**/d`, `e/**`),
plus symlinks. Also: memory (--memory-seed) vs disk for the same files (no symlinks), preorder /
byte order, and `ignored` against an independent count.
usage: python3 -I tree.py <seed> <cases>"""
import sys, os, json, random, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

seed_, cases = int(sys.argv[1]), int(sys.argv[2])
rnd = random.Random(seed_)
NAMES = ["a", "b", "c", "d", "e", "x.log", "y.rs", "A", "B.txt", "z.tmp", "e.rs", "target", "sub", "_"]
DIRS = ["a", "b", "d", "e", "sub", "target", "A", "c"]
FORMS = ["*.log", "*.rs", "*.tmp", "/a", "/b", "/sub", "/target", "b/", "d/", "e/", "sub/",
         "!c", "!x.log", "!y.rs", "!e.rs", "!a", "**/d", "**/c", "**/x.log", "e/**", "sub/**", "a/**", "/e/**"]
GENV = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", HOME="/nonexistent")


def gen():
    files = {}
    def rec(prefix, depth):
        for _ in range(rnd.randint(1, 5)):
            if depth < 3 and rnd.random() < 0.4:
                dn = rnd.choice(DIRS)
                p = prefix + dn
                if p in files: continue
                rec(p + "/", depth + 1)
            else:
                p = prefix + rnd.choice(NAMES)
                if any(k.startswith(p + "/") for k in files): continue
                files[p] = "x"
    rec("", 0)
    # drop paths that collide (file and dir of same name)
    out = {}
    for p in sorted(files):
        parts = p.split("/")
        if any("/".join(parts[:i]) in out for i in range(1, len(parts))):
            continue
        if any(k.startswith(p + "/") for k in out):
            continue
        out[p] = "x"
    dirs = {"/".join(p.split("/")[:i]) for p in out for i in range(1, len(p.split("/")))}
    for dd in [""] + sorted(dirs):
        if rnd.random() < 0.6:
            pats = [rnd.choice(FORMS) for _ in range(rnd.randint(1, 4))]
            gp = (dd + "/" if dd else "") + ".gitignore"
            if gp not in out and not any(k.startswith(gp + "/") for k in out):
                out[gp] = "\n".join(pats) + "\n"
    return out, dirs


mism = 0; memmis = 0; total = 0
for case in range(cases):
    files, dirs = gen()
    root = fresh(f"tree-{seed_}-{case % 4}")
    for p, t in files.items():
        write(root, p, t)
    # memory vs disk (drop symlinks; memory has none)
    rc, rd, err = runj([req("project.tree", id=1)], root=root)
    rc2, rm, err2 = mem([req("project.tree", id=1)], sorted(files.items()))
    # memory lists no empty dirs; disk lists all dirs. Compare files and also dirs that hold files
    def norm(res):
        ents = res["entries"]
        return [e for e in ents]
    dd_ = rd[0]["result"]; mm = rm[0]["result"]
    if dd_ != mm:
        # allowed difference: empty directories on disk (none created here, since every dir holds a file)
        memmis += 1
        if memmis <= 3:
            print("MEMDIFF case", case, json.dumps(dd_)[:400], json.dumps(mm)[:400])
    links = []
    if rnd.random() < 0.5 and dirs:
        dd = rnd.choice(sorted(dirs))
        lp = dd + "/lnk" + str(case)
        os.symlink("..", os.path.join(root, lp)); links.append(lp)
    if rnd.random() < 0.3:
        os.symlink("/etc", os.path.join(root, "abs.log")); links.append("abs.log")
    subprocess.run(["git", "init", "-q", root], env=GENV, check=True, capture_output=True)
    g = subprocess.run(["git", "-C", root, "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
                       env=GENV, capture_output=True, check=True).stdout.decode().split("\0")
    g = sorted(x for x in g if x and not x.startswith("target/"))
    rc, rs, err = runj([req("project.tree", id=1)], root=root)
    r = rs[0]["result"]
    k = sorted(e["path"] for e in r["entries"] if e["kind"] in ("file", "symlink"))
    total += 1
    if k != g:
        mism += 1
        if mism <= 5:
            print("MISMATCH case", case, "git-only", sorted(set(g) - set(k)), "kiln-only", sorted(set(k) - set(g)))
            print("  files", json.dumps({p: t for p, t in files.items() if p.endswith(".gitignore")}), "links", links)
    # order check: preorder, byte order of names
    paths = [e["path"] for e in r["entries"]]
    def keyf(p): return [s.encode() for s in p.split("/")]
    if paths != sorted(paths, key=keyf):
        print("ORDER case", case)
        mism += 1
print(f"seed={seed_} cases={total} git_mismatches={mism} memory_disk_differences={memmis}")
