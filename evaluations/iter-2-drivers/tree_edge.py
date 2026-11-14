"""project.tree hand-made cases: documented differences from git, forms outside the guarantee,
special .gitignore files, path parameter rules, limit/truncated, skipped names, FIFOs.
Each case prints Kiln's file+symlink list next to git's.
usage: python3 -I tree_edge.py"""
import sys, os, json, socket, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

GENV = dict(os.environ, GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", HOME="/nonexistent")


def git_list(root):
    subprocess.run(["git", "init", "-q", root], env=GENV, check=True, capture_output=True)
    out = subprocess.run(["timeout", "10", "git", "-C", root, "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
                         env=GENV, capture_output=True).stdout
    subprocess.run(["rm", "-rf", os.path.join(root, ".git")])
    return sorted(x.decode("utf-8", "backslashreplace") for x in out.split(b"\0") if x and not x.startswith(b"target/"))


def tree(root, **p):
    rc, rs, err = runj([req("project.tree", id=1, **p)], root=root)
    return rs[0]


def cmp_case(name, files, extra=None):
    root = fresh("te-" + name)
    for p, t in files.items():
        write(root, p, t)
    if extra:
        extra(root)
    r = tree(root)
    if not r["ok"]:
        print(name, "KILN-ERROR", json.dumps(r)); return
    res = r["result"]
    k = sorted(e["path"] for e in res["entries"] if e["kind"] in ("file", "symlink"))
    g = git_list(root)
    same = k == g
    print(f"{name}: same={same} ignored={res['ignored']} skipped={res['skipped']}"
          + ("" if same else f" git-only={sorted(set(g)-set(k))} kiln-only={sorted(set(k)-set(g))}"))


# documented differences
cmp_case("braces", {".gitignore": "{a,b}\n", "a": "", "b": "", "{a,b}": "", "c": ""})
cmp_case("trail-tab", {".gitignore": "a\t\n", "a": "", "a\t": ""})
cmp_case("trail-vt", {".gitignore": "a\x0b\n", "a": "", "a\x0b": ""})
cmp_case("trail-nbsp", {".gitignore": "a\u00a0\n", "a": "", "a\u00a0": ""})
cmp_case("trail-space", {".gitignore": "a \n", "a": "", "a ": ""})
cmp_case("trail-escaped-space", {".gitignore": "a\\ \n", "a": "", "a ": ""})
cmp_case("posix-class", {".gitignore": "f[[:digit:]]\n", "f1": "", "fa": ""})
cmp_case("triple-star", {".gitignore": "***/z\n", "z": "", "p/z": "", "q": ""})
cmp_case("nested-git", {"inner/.git/HEAD": "x", "inner/f": "", "g": ""})
# forms outside the guarantee
cmp_case("mid-slash", {".gitignore": "a/b\n", "a/b": "", "x/a/b": "", "a/c": ""})
cmp_case("question", {".gitignore": "f?\n", "f1": "", "f": "", "f12": ""})
cmp_case("charclass", {".gitignore": "f[ab]\nf[!c]x\n", "fa": "", "fc": "", "fcx": "", "fdx": ""})
cmp_case("range", {".gitignore": "f[a-c]\n", "fa": "", "fb": "", "fd": ""})
cmp_case("comment", {".gitignore": "#a\n\\#b\n", "#a": "", "#b": "", "a": ""})
cmp_case("escape-bang", {".gitignore": "\\!x\n", "!x": "", "x": ""})
cmp_case("mid-dstar", {".gitignore": "a/**/b\n", "a/b": "", "a/x/b": "", "a/x/y/b": "", "b": ""})
cmp_case("reinclude-under-ignored", {".gitignore": "d/\n!d/f\n", "d/f": "", "d/g": ""})
cmp_case("reinclude-dstar", {".gitignore": "d/*\n!d/f\n", "d/f": "", "d/g": ""})
cmp_case("neg-dir", {".gitignore": "*\n!*/\n!*.rs\n", "a/b.rs": "", "a/c.txt": "", "x.rs": "", "y": ""})
cmp_case("deeper-overrides", {".gitignore": "*.log\n", "s/.gitignore": "!keep.log\n", "s/keep.log": "", "s/x.log": "", "keep.log": ""})
cmp_case("anchored-sub", {"s/.gitignore": "/a\n", "s/a": "", "s/t/a": "", "a": ""})
cmp_case("star-slash", {".gitignore": "*/x\n", "a/x": "", "a/b/x": "", "x": ""})
cmp_case("dstar-end-dir", {".gitignore": "e/**\n", "e": "", "s/e/f": "", "e2/f": ""})
cmp_case("bom", {".gitignore": "\ufeffa\n", "a": "", "b": ""})
cmp_case("crlf", {".gitignore": "a\r\nb\r\n", "a": "", "b": "", "c": ""})
cmp_case("invalid-utf8-line", {".gitignore": b"\xff\xfe\na\n", "a": "", "b": ""})
cmp_case("ignored-gitignore", {".gitignore": ".gitignore\na\n", "a": "", "b": ""})
cmp_case("bad-pattern", {".gitignore": "[\na\n", "a": "", "[": "", "b": ""})
cmp_case("trailing-backslash", {".gitignore": "a\\\nb\n", "a": "", "b": ""})
cmp_case("dot-dir", {".gitignore": ".*\n!.gitignore\n", ".x": "", ".y/z": "", "w": ""})
cmp_case("target-file", {"target": "", "x/target/y": ""})
cmp_case("case", {".gitignore": "A\n", "a": "", "A": ""})
cmp_case("only-slash", {".gitignore": "/\n*/\n", "a/b": "", "c": ""})
cmp_case("leading-space", {".gitignore": " a\n", " a": "", "a": ""})
cmp_case("dstar-alone", {".gitignore": "**\n!keep\n", "keep": "", "d/keep": "", "x": ""})
cmp_case("slash-dstar-slash", {".gitignore": "a/**/\n", "a/b/c": "", "a/d": ""})
cmp_case("utf8-name", {".gitignore": "é*\n", "éa": "", "ea": "", "É": ""})

# special .gitignore entries
def gi_link(root):
    write(root, "real", "a\n"); os.symlink("real", os.path.join(root, ".gitignore"))
cmp_case("gitignore-symlink", {"a": "", "b": ""}, gi_link)
def gi_fifo(root):
    os.mkfifo(os.path.join(root, ".gitignore"))
cmp_case("gitignore-fifo", {"a": ""}, gi_fifo)
def gi_dir(root):
    os.makedirs(os.path.join(root, ".gitignore")); write(root, ".gitignore/x", "a\n")
cmp_case("gitignore-dir", {"a": ""}, gi_dir)

# kinds, special files, skipped names
root = fresh("te-kinds")
write(root, "f", "x"); os.makedirs(os.path.join(root, "d")); write(root, "d/.git", "gitdir: x")
os.mkfifo(os.path.join(root, "fifo"))
s = socket.socket(socket.AF_UNIX); _cwd = os.getcwd(); os.chdir(root); s.bind("sock"); os.chdir(_cwd)
os.symlink("/nonexistent", os.path.join(root, "dangling")); os.symlink("d", os.path.join(root, "dlink"))
os.symlink("/", os.path.join(root, "rootlink"))
subprocess.run(["mknod", os.path.join(root, "null"), "c", "1", "3"])
open(os.path.join(root.encode(), b"bad\xffname"), "w").close()
write(root, "back\\slash", "")
os.makedirs(os.path.join(root.encode(), b"baddir\xfe")); open(os.path.join(root.encode(), b"baddir\xfe", b"inner"), "w").close()
os.symlink(".", os.path.join(root, "target"))
os.makedirs(os.path.join(root, "x/target")); write(root, "x/target/k", "")
r = tree(root)
print("kinds:", json.dumps(r, ensure_ascii=False))
r = tree(root, path="dlink")
print("path=dlink:", json.dumps(r))
r = tree(root, path="rootlink")
print("path=rootlink:", json.dumps(r))
r = tree(root, path="fifo")
print("path=fifo:", json.dumps(r))
r = tree(root, path="f")
print("path=f:", json.dumps(r))
r = tree(root, path="target")
print("path=target (link to .):", json.dumps(r)[:300])
s.close()

# path parameter rules
root = fresh("te-path")
write(root, ".gitignore", "ign/\nsub/x\n/target\n")
write(root, "ign/a", ""); write(root, "target/t", ""); write(root, "sub/x", ""); write(root, "sub/y", "")
write(root, "sub/deep/.gitignore", "z\n"); write(root, "sub/deep/z", ""); write(root, "sub/deep/w", "")
os.symlink(".", os.path.join(root, "self"))
os.makedirs(os.path.join(root, "..", "te-path-out"), exist_ok=True)
os.symlink("../te-path-out", os.path.join(root, "out"))
os.symlink("../te-path", os.path.join(root, "loop_back"))
for p in ["ign", "target", "sub", "sub/deep", "self", "self/sub", ".git", "out", "loop_back", "loop_back/sub",
          "missing", "sub/x", "../x", "", "/abs", "sub/", "a//b", "self/self/sub", "out/../te-path/sub"]:
    r = tree(root, path=p)
    print(f"path={p!r}:", json.dumps(r)[:400])
os.makedirs(os.path.join(root, ".git")); write(root, ".git/k", "")
print("path=.git:", json.dumps(tree(root, path=".git"))[:300])

# limit / truncated
root = fresh("te-limit")
for i in range(5):
    write(root, f"f{i}", "")
write(root, ".gitignore", "f0\n")
for lim in [1, 4, 5, 6]:
    r = tree(root, limit=lim)["result"]
    print(f"limit={lim}: n={len(r['entries'])} truncated={r['truncated']} ignored={r['ignored']}")
for bad in [0, 1000001, None, -1, 1.5, "3"]:
    r = tree(root, limit=bad)
    print(f"limit={bad!r}:", r.get("error", {}).get("code"), r.get("error", {}).get("kind"))
print("path=null:", json.dumps(tree(root, path=None))[:200])
print("unknown key:", json.dumps(tree(root, zz=1))[:200])
r = tree(root, limit=1000000)["result"]
print("limit=1000000:", len(r["entries"]))
