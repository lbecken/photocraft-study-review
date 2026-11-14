"""Common helpers: drive `kiln-cli control` / `run` / `exec` (public interfaces only)."""
import json, os, subprocess

KILN = os.environ["KILN_BIN"]  # path of the kiln-cli binary under test (the evaluator used a debug build of a clean clone)
WORK = os.environ["KILN_EVAL_WORK"]  # an empty scratch directory for fixtures


class Ctl:
    def __init__(self, root, timeout=None):
        self.p = subprocess.Popen(["timeout", "600", KILN, "control", "--root", root],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE)
        self.n = 0

    def raw(self, line):
        self.p.stdin.write(line.encode() + b"\n")
        self.p.stdin.flush()
        out = self.p.stdout.readline()
        return json.loads(out)

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
        rc = self.p.wait(timeout=60)
        return rc


def run(script_lines, root=None, extra=(), timeout=60):
    path = os.path.join(WORK, "tmp-script.jsonl")
    with open(path, "w") as f:
        for l in script_lines:
            f.write((l if isinstance(l, str) else json.dumps(l)) + "\n")
    args = ["timeout", str(timeout), KILN, "run", path] + (["--root", root] if root else []) + list(extra)
    p = subprocess.run(args, capture_output=True)
    return p.returncode, p.stdout.decode(), p.stderr.decode()


def exe(name, params, *extra, timeout=30):
    p = subprocess.run(["timeout", str(timeout), KILN, "exec", name,
                        params if isinstance(params, str) else json.dumps(params)] + list(extra),
                       capture_output=True)
    return p.returncode, p.stdout.decode(), p.stderr.decode()
