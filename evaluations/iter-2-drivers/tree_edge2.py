"""project.tree: depth bound, prefix-outside rule, nested git repo, refusal order, permissions
(run the permission part under capsh without DAC capabilities), long host paths, timing.
usage: python3 -I tree_edge2.py [perm]"""
import sys, os, json, subprocess, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

CAPSH = [os.path.join(os.path.dirname(os.path.abspath(__file__)), "nodac.sh")]


def tree(root, prefix=(), **p):
    rc, rs, err = runj([req("project.tree", id=1, **p)], root=root, prefix=prefix)
    return rs[0] if rs else (rc, err)


if len(sys.argv) > 1 and sys.argv[1] == "perm":
    root = fresh("t2-perm")
    write(root, "a/f", ""); write(root, "b/.gitignore", "x\n"); write(root, "b/x", ""); write(root, "c/g", "")
    os.chmod(os.path.join(root, "a"), 0o300)
    print("unlistable dir a:", json.dumps(tree(root, prefix=CAPSH)))
    os.chmod(os.path.join(root, "a"), 0o700)
    os.chmod(os.path.join(root, "b/.gitignore"), 0o000)
    print("unreadable b/.gitignore:", json.dumps(tree(root, prefix=CAPSH)))
    print("unreadable b/.gitignore, path=c:", json.dumps(tree(root, prefix=CAPSH, path="c")))
    os.chmod(os.path.join(root, "b/.gitignore"), 0o600)
    os.chmod(os.path.join(root, "c"), 0o600)  # listable? no search permission
    print("dir c without x:", json.dumps(tree(root, prefix=CAPSH)))
    print("dir c without x, path=c:", json.dumps(tree(root, prefix=CAPSH, path="c")))
    os.chmod(os.path.join(root, "c"), 0o700)
    os.chmod(os.path.join(root, "c/g"), 0o000)
    print("unreadable file c/g (listed, not read):", json.dumps(tree(root, prefix=CAPSH)))
    os.chmod(os.path.join(root, "c/g"), 0o600)
    # root's own gitignore unreadable, listing a subdirectory
    write(root, ".gitignore", "zz\n"); os.chmod(os.path.join(root, ".gitignore"), 0o000)
    print("root .gitignore unreadable, path=c:", json.dumps(tree(root, prefix=CAPSH, path="c")))
    os.chmod(os.path.join(root, ".gitignore"), 0o600)
    # root 0311 (search, not list) and 0000
    for mode in (0o311, 0o000, 0o500, 0o100):
        os.chmod(root, mode)
        p = subprocess.run(CAPSH + ["bash", "-c", f"timeout 20 {KILN} exec project.tree {{}} --root {root}; echo rc=$?"], capture_output=True)
        print(f"root mode {oct(mode)}: out={p.stdout.decode().strip()[:200]!r} err={p.stderr.decode().strip()[:200]!r}")
        os.chmod(root, 0o700)
    sys.exit(0)

# depth bound
root = fresh("t2-deep")
p = "/".join(["d"] * 131)
write(root, p + "/f", "")
write(root, "/".join(["d"] * 127) + "/g", "")
r = tree(root)["result"]
paths = [e["path"] for e in r["entries"]]
mx = max(len(x.split("/")) for x in paths)
print("deep: entries", len(paths), "max segments", mx, "too_deep", r["too_deep"],
      "127-seg g listed", ("/".join(["d"] * 127) + "/g") in paths,
      "128-seg dir listed", "/".join(["d"] * 128) in paths)
r = tree(root, path="/".join(["d"] * 100))["result"]
mx = max(len(x.split("/")) for x in r["entries"] and [e["path"] for e in r["entries"]])
print("deep from path of 100 segs: entries", len(r["entries"]), "max segments", mx, "too_deep", r["too_deep"])
r = tree(root, path="/".join(["d"] * 128))
print("path of 128 segs:", json.dumps(r)[:300])
r = tree(root, path="/".join(["d"] * 131))
print("path of 131 segs:", json.dumps(r)[:300])

# prefix with an outside directory leading back in
root = fresh("t2-prefix")
out = fresh("t2-prefix-out")
write(root, "sub/f", ""); write(root, ".gitignore", "sub/f\n")
os.symlink("../t2-prefix", os.path.join(out, "back"))
os.symlink("../t2-prefix-out", os.path.join(root, "out"))
os.symlink("/nonexistent-dir", os.path.join(root, "dang"))
print("out/back/sub:", json.dumps(tree(root, path="out/back/sub")))
rc, rs, err = runj([req("doc.open", id=1, path="out/back/sub/f")], root=root)
print("doc.open out/back/sub/f:", json.dumps(rs[0])[:200])
print("out/missing:", json.dumps(tree(root, path="out/missing")))
print("dang:", json.dumps(tree(root, path="dang")))
print("dang/x:", json.dumps(tree(root, path="dang/x")))
print("sub/f/x:", json.dumps(tree(root, path="sub/f/x")))

# a nested git repository
root = fresh("t2-nested")
write(root, "inner/f", ""); write(root, "g", "")
GENV = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", HOME="/nonexistent")
subprocess.run(["git", "init", "-q", os.path.join(root, "inner")], env=GENV, check=True)
subprocess.run(["git", "init", "-q", root], env=GENV, check=True)
g = subprocess.run(["timeout", "10", "git", "-C", root, "ls-files", "--others", "--exclude-standard"], env=GENV, capture_output=True).stdout.decode().split()
print("nested repo: git", g, "kiln", [e["path"] for e in tree(root)["result"]["entries"]])

# long host path (4096 bytes)
root = fresh("t2-long")
seg = "x" * 200
rel = "/".join([seg] * 20)
_c = os.getcwd(); os.chdir(root)
for _ in range(20):
    os.mkdir(seg); os.chdir(seg)
open("f", "w").close(); os.chdir(_c)
print("host path length of deepest dir:", len(os.path.join(root, rel)))
r = tree(root)
print("long:", json.dumps(r)[:300] if not r.get("ok") else ("ok entries", len(r["result"]["entries"])))

# timing: 100,000 files
root = fresh("t2-many")
for i in range(100):
    os.makedirs(os.path.join(root, f"d{i:03}"))
    for j in range(1000):
        open(os.path.join(root, f"d{i:03}", f"f{j:04}.rs"), "w").close()
t = time.time(); r = tree(root, limit=1000000); el = time.time() - t
print("100,100 entries:", len(r["result"]["entries"]), "truncated", r["result"]["truncated"], f"{el:.2f}s (debug)")
t = time.time(); r = tree(root); el = time.time() - t
print("default limit:", len(r["result"]["entries"]), "truncated", r["result"]["truncated"], f"{el:.2f}s")
# (appended) a .gitignore in a directory that can be listed but not searched
