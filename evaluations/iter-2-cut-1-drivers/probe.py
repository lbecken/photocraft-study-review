"""Ad-hoc probe: probe.py <files-json> <paths-json> <script-jsonl-file> [size]
Prints compact: for every snapshot the Editor/Title/Message/Documents; other lines raw."""
import json, sys, os
sys.path.insert(0, os.path.dirname(__file__))
import kl
files = json.loads(sys.argv[1]); paths = json.loads(sys.argv[2])
script = open(sys.argv[3], encoding="utf-8").read()
size = sys.argv[4] if len(sys.argv) > 4 else None
root = kl.fresh("probe"); kl.write_files(root, files)
log = os.path.join(root + ".log")
rc, out, err, _ = kl.kscript(script, root=root, paths=paths, size=size, log=log)
print("rc", rc, "stderr", err.decode()[:500])
for l in out.decode().splitlines():
    r = json.loads(l)
    if r.get("ok") and "root" in r.get("result", {}):
        print("SNAP", r["id"], "frames", r["result"]["frames"])
        for n, d in kl.walk(r["result"]["root"]):
            if n.get("label") in ("Editor", "Title", "Message", "Documents", "Path") or n.get("role") in ("scroll_bar",):
                m = {k: v for k, v in n.items() if k != "children"}
                if n.get("label") == "Documents":
                    m["entries"] = [c.get("value") for c in n["children"]]
                print("  ", json.dumps(m, ensure_ascii=False))
    else:
        print(l[:600])
print("LOG"); print(open(log).read()[-3000:])
