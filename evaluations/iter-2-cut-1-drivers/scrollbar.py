"""Is there a `scroll_bar` node for an overflowing scroll area (ui.md, The shell)? And does a click on a
Documents entry change the current document?"""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl
files = {f"f{i:03d}.rs": f"fn f{i}() {{}}\n" for i in range(40)}
files["big.rs"] = "".join(f"fn f{i}() {{}}\n" for i in range(300))
reqs = [{"id": i, "command": "doc.open", "params": {"path": f"f{i:03d}.rs"}} for i in range(40)]
reqs += [{"id": "o", "command": "doc.open", "params": {"path": "big.rs"}}, {"id": "f", "ui": "frames", "count": 2}, {"id": "s0", "ui": "snapshot"},
         {"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 3, "col": 2}}, {"id": "s1", "ui": "snapshot"},
         {"id": "sc", "ui": "scroll", "target": {"label": "Documents"}, "direction": "down", "steps": 1}, {"id": "s2", "ui": "snapshot"},
         {"id": "ce", "ui": "click", "target": {"role": "label", "value": "f005.rs"}}, {"id": "s3", "ui": "snapshot"},
         {"id": "ce2", "ui": "click", "target": {"role": "label", "value": "f005.rs"}, "button": "secondary"}, {"id": "s4", "ui": "snapshot"}]
root = kl.fresh("sb"); kl.write_files(root, files)
rc, out, err, _ = kl.kscript(reqs, root=root, log=root + ".log")
res = {r["id"]: r for r in kl.parse(out)}
for k in ("s0", "s1", "s2", "s3", "s4"):
    s = res[k]
    roles = {}
    for n, _ in kl.walk(s["result"]["root"]):
        roles[n["role"]] = roles.get(n["role"], 0) + 1
    print(k, "roles", roles, "title", (kl.one(s, label="Title") or {}).get("value"))
for k in ("c", "sc", "ce", "ce2"):
    print(k, res[k])
log = [json.loads(l) for l in open(root + ".log")]
print("log after the click on the entry:", [(l["command"], l["params"]) for l in log][-8:])
