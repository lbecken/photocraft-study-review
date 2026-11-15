"""Helpers for driving `kiln script`, `kiln-cli run` and `kiln-cli control`.

Environment:
  KILN_BIN   path to the `kiln` binary
  KILN_CLI   path to the `kiln-cli` binary
  KILN_WORK  scratch directory for fixtures/outputs (under the evaluation work dir)
Every process runs under `timeout`.
"""
import json, os, subprocess, shutil, hashlib, itertools

BIN = os.environ.get("KILN_BIN")
CLI = os.environ.get("KILN_CLI")
WORK = os.environ.get("KILN_WORK")
assert BIN and CLI and WORK, "set KILN_BIN, KILN_CLI, KILN_WORK"

_counter = itertools.count()


def fresh(name):
    """A fresh empty directory under WORK (names are made unique by a counter)."""
    d = os.path.join(WORK, "fx", f"{name}-{os.getpid()}-{next(_counter)}")
    os.makedirs(d)
    return d


def write_files(root, files):
    for p, data in files.items():
        full = os.path.join(root, p)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        if isinstance(data, str):
            data = data.encode("utf-8")
        with open(full, "wb") as f:
            f.write(data)


def read_tree(root):
    out = {}
    for dp, dn, fn in os.walk(root):
        for f in fn:
            full = os.path.join(dp, f)
            rel = os.path.relpath(full, root)
            if os.path.islink(full):
                out[rel] = ("link", os.readlink(full))
            else:
                with open(full, "rb") as fh:
                    out[rel] = fh.read()
    return out


def jl(reqs):
    return "".join((r if isinstance(r, str) else json.dumps(r, ensure_ascii=False)) + "\n" for r in reqs)


def run(argv, stdin=None, t=120, env=None, cwd=None):
    p = subprocess.run(["timeout", str(t)] + argv, input=stdin, capture_output=True, env=env, cwd=cwd)
    return p.returncode, p.stdout, p.stderr


def kscript(reqs, root=None, paths=(), size=None, log=None, memory=False, extra=(), t=300, name="s"):
    """Run `kiln script` on a script built from reqs. Returns (rc, out_bytes, err_bytes, script_path)."""
    sdir = fresh("script-" + name)
    sp = os.path.join(sdir, "s.jsonl")
    with open(sp, "w", encoding="utf-8") as f:
        f.write(reqs if isinstance(reqs, str) else jl(reqs))
    argv = [BIN, "script", sp]
    if root is not None:
        argv += ["--root", root]
    if memory:
        argv += ["--memory"]
    if log:
        argv += ["--log", log]
    if size:
        argv += ["--size", size]
    argv += list(extra) + list(paths)
    rc, out, err = run(argv, t=t)
    return rc, out, err, sp


def parse(out):
    return [json.loads(l) for l in out.decode("utf-8").splitlines() if l.strip()]


def walk(node, depth=0):
    yield node, depth
    for c in node.get("children", []):
        yield from walk(c, depth + 1)


def find(snap, role=None, label=None):
    root = snap["result"]["root"] if "result" in snap else snap
    r = []
    for n, d in walk(root):
        if role is not None and n.get("role") != role:
            continue
        if label is not None and n.get("label") != label:
            continue
        r.append(n)
    return r


def one(snap, role=None, label=None):
    r = find(snap, role, label)
    return r[0] if len(r) == 1 else None


def rows_text(node):
    return ["".join(run_["text"] for run_ in row) for row in node.get("rows", [])]


def cli_run(script_text, root, log=None, t=300):
    sdir = fresh("clirun")
    sp = os.path.join(sdir, "s.jsonl")
    with open(sp, "w", encoding="utf-8") as f:
        f.write(script_text)
    argv = [CLI, "run", sp, "--root", root]
    if log:
        argv += ["--log", log]
    return run(argv, t=t)


def cli_control(text, root, t=120):
    return run([CLI, "control", "--root", root], stdin=text.encode("utf-8") if isinstance(text, str) else text, t=t)


def copytree(src, name):
    d = fresh(name)
    shutil.rmtree(d)
    shutil.copytree(src, d, symlinks=True)
    return d


def md5(b):
    return hashlib.md5(b).hexdigest()
