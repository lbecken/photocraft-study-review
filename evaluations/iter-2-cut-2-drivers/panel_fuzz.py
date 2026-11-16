"""ui.project-panel: seeded random sessions against a naive model of the tree and of the panel.

usage: panel_fuzz.py <seed> <sessions> <steps>
env: KILN_BIN, KILN_CLI, KILN_WORK.

Each session: a random tree of at most 18 entries (names from a hostile pool: spaces, combining
marks, a joiner, a ligature, CJK, emoji, right-to-left text, prefixes of each other, hidden names,
`*.log` ignored by a root .gitignore, a root `target/`, a `.git/`, a symlink, a FIFO). My naive
model lists a directory with os.listdir sorted by UTF-8 bytes, skipping `.git`, the root `target`
and `*.log`. Steps: clicks on `Project` and on random shown entries (dirs toggle, others open),
with random button/modifiers/`at`. After each step: snapshot and doc.list. Checks per step: the
shown entries (label, value, toggled) = the model's preorder of shown entries; `Project` toggled;
Title and doc.list after an open; Message after a refused open (FIFO, link); the log's
project.tree/doc.open requests = the model's; the log replayed on a fresh copy gives the same files;
idle frames after every action leave the log byte-identical.
"""
import json, os, random, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl, k2
from k2 import Session, find, one, rows, tree_items, message, title

POOL = ["a", "a b", "a.rs", "ab", "a-b", "b", ".hidden", "é", "x‍y", "ﬁ", "中文", "😀", "שלום", "zz",
        "B", "a b c", "Z.txt", "k.log", "1", "_"]
PROJ = {"role": "button", "label": "Project"}


def sortkey(n):
    return n.encode("utf-8")


def gen_tree(rng, root):
    budget = [rng.randint(3, 18)]
    kinds = {}

    def mk(d, rel, depth):
        names = rng.sample(POOL, rng.randint(1, 5))
        for nm in names:
            if budget[0] <= 0:
                return
            budget[0] -= 1
            p = os.path.join(d, nm)
            r = (rel + "/" + nm) if rel else nm
            if depth < 3 and rng.random() < 0.35:
                os.makedirs(p, exist_ok=True)
                kinds[r] = "dir"
                mk(p, r, depth + 1)
            else:
                if os.path.exists(p):
                    continue
                with open(p, "w") as f:
                    f.write(f"text of {r}\n")
                kinds[r] = "file"
    mk(root, "", 0)
    with open(os.path.join(root, ".gitignore"), "w") as f:
        f.write("*.log\n")
    os.makedirs(os.path.join(root, "target"), exist_ok=True)
    open(os.path.join(root, "target", "t"), "w").close()
    os.makedirs(os.path.join(root, ".git"), exist_ok=True)
    if rng.random() < 0.5:
        os.symlink("..", os.path.join(root, "up"))
    if rng.random() < 0.3:
        os.mkfifo(os.path.join(root, "fifo"))


def listing(root, rel):
    d = os.path.join(root, rel) if rel else root
    out = []
    for nm in sorted(os.listdir(d), key=sortkey):
        if nm == ".git" or (not rel and nm == "target") or nm.endswith(".log"):
            continue
        full = os.path.join(d, nm)
        r = (rel + "/" + nm) if rel else nm
        if os.path.islink(full):
            k = "symlink"
        elif os.path.isdir(full):
            k = "dir"
        elif os.path.isfile(full):
            k = "file"
        else:
            k = "other"
        out.append((r, nm, k))
    return out


class Model:
    def __init__(self, root):
        self.root = root
        self.open_ = False
        self.expanded = set()
        self.reads = []
        self.docs = []
        self.msg = ""
        self.cur = None

    def shown(self):
        if not self.open_:
            return []
        out = []

        def rec(rel):
            for r, nm, k in listing(self.root, rel):
                out.append((r, nm, (r in self.expanded) if k == "dir" else None, k))
                if k == "dir" and r in self.expanded:
                    rec(r)
        rec("")
        return out


def session(seed, si, steps):
    rng = random.Random(seed * 104729 + si)
    init = kl.fresh(f"pz{seed}-{si}")
    gen_tree(rng, init)
    m = Model(init)
    reqs = []
    exp = []
    for st in range(steps):
        sh = m.shown()
        if not m.open_ or not sh or rng.random() < 0.12:
            reqs.append({"id": f"a{st}", "ui": "click", "target": PROJ})
            if m.open_:
                m.open_ = False
                m.expanded = set()
            else:
                m.open_ = True
                m.reads.append(("project.tree", {}))
        else:
            r, nm, tog, k = rng.choice(sh)
            req = {"id": f"a{st}", "ui": "click", "target": {"role": "tree_item", "label": r}}
            x = rng.random()
            if x < 0.15:
                req["button"] = "secondary"
            elif x < 0.3:
                req["modifiers"] = rng.sample(["ctrl", "shift", "alt"], rng.randint(1, 2))
            elif x < 0.4:
                req["at"] = {"row": 0, "col": 0}
            reqs.append(req)
            if k == "dir":
                if tog:
                    m.expanded = {e for e in m.expanded if e != r and not e.startswith(r + "/")}
                else:
                    m.expanded.add(r)
                    m.reads.append(("project.tree", {"path": r}))
            elif k == "file":
                m.reads.append(("doc.open", {"path": r}))
                if r not in m.docs:
                    m.docs.append(r)
                m.cur = r
                m.msg = ""
            elif k == "symlink":  # `up` -> .. : outside_root
                m.msg = "outside_root"
            else:
                m.msg = "not_a_file"
        reqs += [{"id": f"s{st}", "ui": "snapshot"}, {"id": f"l{st}", "command": "doc.list"}]
        exp.append((m.shown(), m.open_, list(m.docs), m.cur, m.msg))
    return init, reqs, exp, m


def check(seed, si, steps):
    init, reqs, exp, m = session(seed, si, steps)
    root = kl.copytree(init, "pzui")
    s = Session(reqs, root=root, name=f"pz{seed}-{si}")
    mism = []
    for st, (sh, op, docs, cur, msg) in enumerate(exp):
        a = s[f"a{st}"]
        if not a["ok"]:
            mism.append((st, "refused", a))
            continue
        snap = s.snap(f"s{st}")
        got = [(n.get("label"), n.get("value"), n.get("toggled")) for n in tree_items(snap)]
        want = [(r, nm, tog) for r, nm, tog, k in sh]
        if got != want:
            mism.append((st, "shown", got[:8], want[:8]))
        if one(snap, **PROJ).get("toggled") is not op:
            mism.append((st, "toggle", one(snap, **PROJ).get("toggled"), op))
        if op != bool(find(snap, role="tree", label="Project files")):
            mism.append((st, "tree presence"))
        dl = [d["path"] for d in s[f"l{st}"]["result"]["docs"]]
        if dl != docs:
            mism.append((st, "docs", dl, docs))
        if cur and not (title(snap) or "").startswith(cur + " — "):
            mism.append((st, "title", title(snap), cur))
        mv = message(snap) or ""
        if (msg and not mv.startswith(msg)) or (not msg and mv):
            mism.append((st, "message", mv, msg))
        for n in tree_items(snap):
            if not n.get("offscreen") and rows(n) and rows(n) != [n.get("value")] and "ש" not in n.get("value", ""):
                mism.append((st, "rows", n.get("label"), rows(n)))
    got_reads = [(c, p) for c, p in s.logcmds() if c in ("project.tree", "doc.open")]
    if got_reads != m.reads:
        i = next((i for i, (x, y) in enumerate(zip(got_reads, m.reads)) if x != y), min(len(got_reads), len(m.reads)))
        mism.append(("reads", i, got_reads[i:i + 2], m.reads[i:i + 2]))
    rep = kl.copytree(init, "pzrep")
    rc, out, err = k2.cli_run_log(s.log, rep)
    if rc != 0 or kl.read_tree(rep) != kl.read_tree(root):
        mism.append(("replay", rc, err[:200]))
    reqs2 = []
    for r in reqs:
        reqs2.append(r)
        if "ui" in r and r["ui"] != "snapshot" and s.by[r["id"]].get("ok"):
            reqs2.append({"id": "idle-" + r["id"], "ui": "frames", "count": 20})
    s2 = Session(reqs2, root=kl.copytree(init, "pzidle"), name=f"pzi{seed}-{si}")
    if open(s2.logp, "rb").read() != open(s.logp, "rb").read():
        mism.append(("idle frames changed the log",))
    return mism, steps


if __name__ == "__main__":
    seed, n, steps = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    tot = mm = 0
    for si in range(n):
        x, k = check(seed, si, steps)
        tot += k
        mm += len(x)
        for y in x[:5]:
            print(f"seed {seed} session {si}:", json.dumps(y, ensure_ascii=False, default=str)[:400])
    print(f"panel_fuzz seed={seed} sessions={n} steps={tot} mismatches={mm}")
