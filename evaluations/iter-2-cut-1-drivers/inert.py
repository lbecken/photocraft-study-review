"""Inert checks: secondary click on Open; horizontal scroll directions on Documents and Editor."""
import json, sys, os
sys.path.insert(0, os.path.dirname(__file__))
import kl
long = "/".join(["n" * 200] * 2) + ".txt"
files = {f"f{i:02d}.txt": "x" for i in range(30)}; files[long] = "y"; files["a.rs"] = "fn a() {}\n" + "x" * 400 + "\n"
root = kl.fresh("inert"); kl.write_files(root, files)
reqs = [{"id": "f", "ui": "focus", "target": {"label": "Path"}}, {"id": "t", "ui": "type", "text": "a.rs"},
        {"id": "c2", "ui": "click", "target": {"label": "Open"}, "button": "secondary"}, {"id": "s0", "ui": "snapshot"}]
reqs += [{"id": i, "command": "doc.open", "params": {"path": p}} for i, p in enumerate(sorted(files))]
reqs += [{"id": "o", "command": "doc.open", "params": {"path": "a.rs"}}, {"id": "fr", "ui": "frames", "count": 3}, {"id": "s1", "ui": "snapshot"}]
for d in ("right", "left", "down", "up"):
    reqs += [{"id": "d" + d, "ui": "scroll", "target": {"label": "Documents"}, "direction": d, "steps": 5}, {"id": "sd" + d, "ui": "snapshot"},
             {"id": "e" + d, "ui": "scroll", "target": {"label": "Editor"}, "direction": d, "steps": 5}, {"id": "se" + d, "ui": "snapshot"}]
rc, out, err, _ = kl.kscript(reqs, root=root, log=root + ".log")
r = {x["id"]: x for x in kl.parse(out)}
log = [json.loads(l) for l in open(root + ".log")]
print("secondary click on Open with Path=a.rs:", r["c2"].get("result"), "docs:", [c.get("value") for c in kl.one(r["s0"], label="Documents")["children"]])


def sig(s):
    d = kl.one(s, label="Documents"); e = kl.one(s, label="Editor")
    ent = [c for c in d["children"] if not c.get("offscreen")]
    names = [(kl.rows_text(c) or [""])[0] for c in ent]
    return (names[0][:10], names[-1][:10], max(len(n) for n in names), [x[:6] + "…" + str(len(x)) for x in kl.rows_text(e)[:2]])


print("start", sig(r["s1"]))
for d in ("right", "left", "down", "up"):
    print(d, "Documents scroll:", r["d" + d]["result"], sig(r["sd" + d])[:3], "| Editor scroll:", r["e" + d]["result"], sig(r["se" + d])[3])
