"""Re-run every published example of `kiln-cli commands --schema` independently (files + setup +
params through `kiln-cli run` or `exec --text`), validate results against result_schema, and
send an unknown key to every command (cmd.registry).
usage: python3 -I examples.py"""
import sys, os, json, shutil, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import KILN, WORK, run, exe

d = json.loads(subprocess.run(["timeout", "20", KILN, "commands", "--schema"], capture_output=True).stdout)
try:
    import jsonschema
except ImportError:
    jsonschema = None
base = os.path.join(WORK, "ex")
if os.path.isdir(base):
    shutil.rmtree(base)
os.makedirs(base)
n = ok = 0
bad = []
valid = invalid = 0
for c in d["commands"]:
    for i, e in enumerate(c["examples"]):
        n += 1
        if c["name"].startswith("buffer."):
            rc, out, err = exe(c["name"], e["params"], "--text", e["initial_text"], "--memory")
            res = json.loads(out) if rc == 0 else None
        else:
            root = os.path.join(base, f"{c['name']}-{i}")
            os.makedirs(root)
            for p, t in e["files"]:
                fp = os.path.join(root, p); os.makedirs(os.path.dirname(fp), exist_ok=True)
                open(fp, "w", newline="").write(t)
            lines = [{"command": s[0], "params": s[1]} for s in e["setup"]] + [{"command": c["name"], "params": e["params"]}]
            rc, out, err = run(lines, root=root)
            last = json.loads(out.splitlines()[-1])
            res = last.get("result")
        if res == e["result"]:
            ok += 1
        else:
            bad.append((c["name"], i, res, e["result"]))
        if jsonschema and res is not None:
            try:
                jsonschema.validate(res, c["result_schema"]); valid += 1
            except Exception as ex:
                invalid += 1; bad.append(("schema", c["name"], str(ex)[:200]))
print(f"examples={n} reproduced={ok} result_schema_valid={valid} invalid={invalid} jsonschema={'yes' if jsonschema else 'no'}")
for b in bad:
    print(json.dumps(b, ensure_ascii=False))

# unknown key for every command, on an open document; text must not change
root = os.path.join(base, "unknown")
os.makedirs(root); open(os.path.join(root, "a.txt"), "w").write("abc\n")
lines = [{"id": "open", "command": "doc.open", "params": {"path": "a.txt"}}]
for c in d["commands"]:
    lines.append({"id": c["name"], "command": c["name"], "params": {"doc": 1, "zzz": 1}})
lines.append({"id": "text", "command": "doc.text", "params": {"doc": 1}})
lines.append({"id": "cur", "command": "cursor.get", "params": {"doc": 1}})
rc, out, err = run(lines, root=root)
rs = [json.loads(l) for l in out.splitlines()]
kinds = {r["id"]: (r.get("error") or {}).get("code") for r in rs[1:-2]}
print("unknown-key codes:", sorted(set(kinds.values())), "commands:", len(kinds))
print("after:", json.dumps(rs[-2]["result"]), json.dumps(rs[-1]["result"]))
