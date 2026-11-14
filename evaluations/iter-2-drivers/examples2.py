"""Re-run every published example and refusal example of `kiln-cli commands --schema`, on disk
(--root) and in memory (--memory-seed); validate results against result_schema; compare kinds.
Also: unknown key to every command; every refusal kind seen vs the command's `refusals`.
usage: python3 -I examples2.py"""
import sys, os, json, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *
import jsonschema

d = json.loads(subprocess.run(["timeout", "20", KILN, "commands", "--schema"], capture_output=True).stdout)
n = ok = 0; bad = []; valid = invalid = 0; rn = rok = 0; memdiff = 0


def files_of(e):
    out = []
    for f in e["files"]:
        if isinstance(f, list):
            out.append((f[0], f[1]))
        else:
            out.append((f["path"], f["text"] if "text" in f else bytes.fromhex(f["hex"])))
    return out


def go(c, e, i, tag):
    if c["name"].startswith("buffer."):
        rc, out, err = exe(c["name"], e["params"], "--text", e["initial_text"], "--memory")
        if rc == 0:
            return {"ok": True, "result": json.loads(out)}, None
        return {"ok": False, "error": json.loads(err.splitlines()[0])}, None
    root = fresh(f"ex-{tag}-{c['name']}-{i}")
    fs = files_of(e)
    for p, t in fs:
        write(root, p, t)
    lines = [req(s[0], id=f"s{j}", **s[1]) for j, s in enumerate(e["setup"])] + [req(c["name"], id="x", **e["params"])]
    rc, rs, err = runj(lines, root=root, name=f"ex-{tag}.jsonl", timeout=600)
    rc2, rs2, err2 = mem(lines, fs, name=f"ex-{tag}-m.jsonl", timeout=600)
    setup_ok = all(r["ok"] for r in rs[:-1])
    return rs[-1], (rs == rs2, setup_ok)


for c in d["commands"]:
    for i, e in enumerate(c["examples"]):
        n += 1
        last, cmp_ = go(c, e, i, "ok")
        res = last.get("result")
        if cmp_ and not cmp_[0]:
            memdiff += 1; bad.append(("memdiff", c["name"], i))
        if cmp_ and not cmp_[1]:
            bad.append(("setup-failed", c["name"], i))
        if res == e["result"]:
            ok += 1
        else:
            bad.append((c["name"], i, last, e["result"]))
        if res is not None:
            try:
                jsonschema.validate(res, c["result_schema"]); valid += 1
            except Exception as ex:
                invalid += 1; bad.append(("schema", c["name"], str(ex)[:200]))
    for i, e in enumerate(c["refusal_examples"]):
        rn += 1
        last, cmp_ = go(c, e, i, "rx")
        kind = (last.get("error") or {}).get("kind")
        if cmp_ and not cmp_[0]:
            memdiff += 1; bad.append(("memdiff-rx", c["name"], i))
        if cmp_ and not cmp_[1]:
            bad.append(("setup-failed-rx", c["name"], i))
        if kind == e["kind"] and last["error"]["code"] == "rejected":
            rok += 1
        else:
            bad.append(("rx", c["name"], i, e["kind"], last))
        if e["kind"] not in c["refusals"]:
            bad.append(("rx-kind-not-declared", c["name"], e["kind"]))
    missing = [k for k in c["refusals"] if k not in {e["kind"] for e in c["refusal_examples"]}]
    if missing:
        print("declared-without-example", c["name"], missing)
print(f"examples={n} reproduced={ok} result_schema_valid={valid} invalid={invalid}")
print(f"refusal_examples={rn} reproduced={rok} memory_vs_disk_differences={memdiff}")
for b in bad:
    print(json.dumps(b, ensure_ascii=False)[:600])
declared = {k for c in d["commands"] for k in c["refusals"]}
published = {k["kind"] for k in d["error_kinds"]["rejected"]}
print("published-not-declared", sorted(published - declared), "declared-not-published", sorted(declared - published))

root = fresh("unknown")
write(root, "a.txt", "abc\n")
lines = [req("doc.open", id="open", path="a.txt")]
for c in d["commands"]:
    lines.append(req(c["name"], id=c["name"], doc=1, zzz=1))
lines.append(req("doc.text", id="text", doc=1))
lines.append(req("cursor.get", id="cur", doc=1))
rc, rs, err = runj(lines, root=root)
kinds = {r["id"]: (r.get("error") or {}).get("code") for r in rs[1:-2]}
print("unknown-key codes:", sorted(set(map(str, kinds.values()))), "commands:", len(kinds))
print("after:", json.dumps(rs[-2]["result"]), json.dumps(rs[-1]["result"]))
