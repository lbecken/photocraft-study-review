"""Helpers for iteration-2 drivers (public interfaces only). Env defaults point at the clone."""
import json, os, subprocess, shutil
BASE = os.environ["KILN_EVAL_BASE"]  # the scratch directory that holds clone/ and work/ (the evaluator had its own path here; the director replaced it at publication)
os.environ.setdefault("KILN_BIN", BASE + "/clone/target/debug/kiln-cli")
os.environ.setdefault("KILN_EVAL_WORK", BASE + "/work")
KILN = os.environ["KILN_BIN"]
WORK = os.environ["KILN_EVAL_WORK"]


def fresh(name):
    """An empty fixture directory under WORK/fx/<name> (name is a fixed literal)."""
    d = os.path.join(WORK, "fx", name)
    assert d.startswith(WORK + "/fx/") and ".." not in name
    if os.path.lexists(d):
        subprocess.run(["chmod", "-R", "u+rwx", d], capture_output=True)
        shutil.rmtree(d)
    os.makedirs(d)
    return d


def write(root, rel, data):
    p = os.path.join(root, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "wb") as f:
        f.write(data if isinstance(data, bytes) else data.encode())


def req(command, id=None, **params):
    o = {"command": command, "params": params}
    if id is not None:
        o["id"] = id
    return o


def run(lines, root=None, extra=(), timeout=120, name="tmp-script.jsonl", prefix=()):
    path = os.path.join(WORK, name)
    with open(path, "w") as f:
        for l in lines:
            f.write((l if isinstance(l, str) else json.dumps(l)) + "\n")
    args = list(prefix) + ["timeout", str(timeout), KILN, "run", path] + (["--root", root] if root else []) + list(extra)
    p = subprocess.run(args, capture_output=True)
    return p.returncode, p.stdout.decode(), p.stderr.decode()


def runj(lines, root=None, **kw):
    rc, out, err = run(lines, root=root, **kw)
    return rc, [json.loads(l) for l in out.splitlines()], err


def seed(files, name="seed.json"):
    path = os.path.join(WORK, name)
    fs = []
    for p, t in files:
        if isinstance(t, bytes):
            fs.append({"path": p, "hex": t.hex()})
        else:
            fs.append({"path": p, "text": t})
    json.dump({"files": fs}, open(path, "w"))
    return path


def mem(lines, files, **kw):
    return runj(lines, extra=["--memory-seed", seed(files)], **kw)


def exe(name, params, *extra, timeout=60, prefix=()):
    p = subprocess.run(list(prefix) + ["timeout", str(timeout), KILN, "exec", name,
                        params if isinstance(params, str) else json.dumps(params)] + list(extra),
                       capture_output=True)
    return p.returncode, p.stdout.decode(), p.stderr.decode()


class Ctl:
    def __init__(self, root, prefix=()):
        self.p = subprocess.Popen(list(prefix) + ["timeout", "900", KILN, "control", "--root", root],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.n = 0

    def raw(self, line):
        self.p.stdin.write(line.encode() + b"\n"); self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def req(self, command, **params):
        self.n += 1
        r = self.raw(json.dumps({"id": self.n, "command": command, "params": params}))
        assert r["id"] == self.n, r
        return r

    def ok(self, command, **params):
        r = self.req(command, **params)
        if not r["ok"]:
            raise RuntimeError(f"{command} {params} -> {r}")
        return r["result"]

    def close(self):
        self.p.stdin.close()
        return self.p.wait(timeout=60)


def doc_session(text, cmds, path="a.rs", mem_fs=True):
    """open `text` as doc 1 in a memory session, run cmds (list of (command, params)); return responses."""
    lines = [req("doc.open", id=0, path=path)] + [req(c, id=i + 1, **p) for i, (c, p) in enumerate(cmds)]
    rc, rs, err = mem(lines, [(path, text)])
    return rs
