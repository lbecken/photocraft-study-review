"""Shared helpers for the R-302 drivers (on top of kl.py, copied from the cut-1 drivers).

Environment: KILN_BIN, KILN_CLI, KILN_WORK (see work/env.sh). Every process runs under timeout.
"""
import json, os, sys, subprocess
sys.path.insert(0, os.path.dirname(__file__))
import kl  # noqa: E402

COMMANDS = None


def mutating_names():
    global COMMANDS
    if COMMANDS is None:
        rc, out, err = kl.run([kl.CLI, "commands", "--schema"], t=30)
        d = json.loads(out)
        COMMANDS = {c["name"]: c.get("mutating") for c in d["commands"]}
    return {n for n, m in COMMANDS.items() if m}


class Session:
    """A `kiln script` run: reqs is a list of dicts/strings; returns responses by id, the log."""

    def __init__(self, reqs, root=None, paths=(), size=None, memory=False, t=600, name="s", extra=()):
        self.reqs = reqs
        sdir = kl.fresh("log-" + name)
        self.logp = os.path.join(sdir, "log.jsonl")
        self.rc, out, self.err, self.script = kl.kscript(reqs, root=root, paths=paths, size=size, log=self.logp,
                                                          memory=memory, t=t, name=name, extra=extra)
        self.out = out
        self.lines = out.decode("utf-8", "replace").splitlines()
        self.resp = [json.loads(l) for l in self.lines if l.strip()]
        self.by = {}
        for r in self.resp:
            self.by.setdefault(r.get("id"), r)
        try:
            self.log = [json.loads(l) for l in open(self.logp, encoding="utf-8") if l.strip()]
        except FileNotFoundError:
            self.log = None

    def __getitem__(self, i):
        return self.by[i]

    def snap(self, i):
        return self.by[i]["result"]["root"]

    def logcmds(self):
        return [(e["command"], e.get("params", {})) for e in self.log]

    def mut_log(self):
        m = mutating_names()
        return [(c, p) for c, p in self.logcmds() if c in m]


def nodes(root):
    return [n for n, d in kl.walk(root)]


def find(root, **kw):
    r = []
    for n in nodes(root):
        ok = True
        for k, v in kw.items():
            if k == "label_contains":
                ok = ok and v in n.get("label", "")
            elif k == "value_contains":
                ok = ok and v in n.get("value", "")
            else:
                ok = ok and n.get(k) == v
        if ok:
            r.append(n)
    return r


def one(root, **kw):
    r = find(root, **kw)
    assert len(r) == 1, (kw, len(r))
    return r[0]


def maybe(root, **kw):
    r = find(root, **kw)
    return r[0] if len(r) == 1 else None


def rows(n):
    return ["".join(x["text"] for x in row) for row in n.get("rows", [])]


def tree_items(root):
    return [n for n in nodes(root) if n.get("role") == "tree_item" or
            (n.get("role") == "label" and "label" not in n and n.get("value", "").startswith("truncated"))]


def tabs_view(root):
    """Sequence after each tab in depth-first order: [(label, toggled, modified, has_close, has_discard)]."""
    out = []
    cur = None
    for n in nodes(root):
        r = n.get("role")
        if r == "tab":
            cur = [n.get("label"), n.get("toggled"), False, False, False, n]
            out.append(cur)
        elif cur is not None:
            if r == "label" and n.get("value") == "modified" and "label" not in n:
                cur[2] = True
            elif r == "button" and n.get("label") == "Close " + cur[0]:
                cur[3] = True
            elif r == "button" and n.get("label") == "Discard " + cur[0]:
                cur[4] = True
    return out


def status(root):
    d = {}
    for lab in ("Line", "Column", "Encoding", "Line ending"):
        f = find(root, role="label", label=lab)
        d[lab] = f[0] if len(f) == 1 else (None if not f else "AMBIG")
    return d


def message(root):
    m = find(root, label="Message")
    return m[0].get("value", "") if m else None


def title(root):
    t = find(root, label="Title")
    return t[0].get("value") if t else None


def editor(root):
    e = find(root, role="multiline_text_input", label="Editor")
    return e[0] if e else None


def cli_run_log(log_entries, root, t=600):
    text = "".join(json.dumps(e, ensure_ascii=False, sort_keys=True) + "\n" for e in log_entries)
    return kl.cli_run(text, root, t=t)
