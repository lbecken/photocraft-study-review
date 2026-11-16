"""Real window (`kiln --script` under xvfb-run) vs headless `kiln script` on the same script.

usage: wincmp2.py <screen WxH> <limit> <kind> [--size-headless WxH] [--no-root]
  kind: `tabs` takes the sessions tabs_fuzz.py left in fx/ (initial fixture `tz<seed>-<si>-*`,
  script `script-tz<seed>-<si>-*`, no start paths); `status` takes status_fuzz.py's
  (`sz<seed>-<si>-*`, start paths d0.*, d1.*); `panel` takes panel_fuzz.py's (`pz…`).
  For each pair: two copies of the initial fixture; the window on one, `kiln script` on the
  other; compares stdout bytes, exit codes and files; reports window stderr and files under HOME.
env: KILN_BIN, KILN_WORK, XLIBS.
"""
import glob, os, re, subprocess, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl

screen, limit, kind = sys.argv[1], int(sys.argv[2]), sys.argv[3]
hsize = sys.argv[sys.argv.index("--size-headless") + 1] if "--size-headless" in sys.argv else None
noroot = "--no-root" in sys.argv
seeds = sys.argv[sys.argv.index("--seeds") + 1].split(",") if "--seeds" in sys.argv else None
XLIBS = os.environ["XLIBS"]
pre = {"tabs": "tz", "status": "sz", "panel": "pz"}[kind]

pairs = []
for sd in sorted(glob.glob(os.path.join(kl.WORK, "fx", f"script-{pre}[0-9]*"))):
    m = re.match(rf"script-({pre}(\d+)-\d+)-\d+-\d+$", os.path.basename(sd))
    if not m or (seeds and m.group(2) not in seeds):
        continue
    inits = [d for d in glob.glob(os.path.join(kl.WORK, "fx", f"{m.group(1)}-*")) if os.path.isdir(d)]
    if len(inits) != 1:
        continue
    pairs.append((inits[0], os.path.join(sd, "s.jsonl")))
pairs = pairs[:limit]
n = dout = drc = dfiles = serr = homef = lines = 0
for init, script in pairs:
    paths = sorted(f for f in os.listdir(init) if re.match(r"d\d\.", f)) if kind == "status" else []
    w = kl.copytree(init, "w2w")
    h = kl.copytree(init, "w2h")
    home = kl.fresh("w2home")
    errf = w + ".stderr"
    inner = 'exec "$KILN_BIN" ' + ("" if noroot else f"--root '{w}' ") + " ".join(f"'{p}'" for p in paths) + f" --script '{script}' 2>'{errf}'"
    env = dict(os.environ)
    env.pop("WAYLAND_DISPLAY", None)
    env.update(LD_LIBRARY_PATH=XLIBS + "/usr/lib/x86_64-linux-gnu", MESA_SHADER_CACHE_DISABLE="true", HOME=home)
    p = subprocess.run(["timeout", "600", "xvfb-run", "-a", "-s", f"-screen 0 {screen}x24", "sh", "-c", inner],
                       capture_output=True, env=env, cwd=w if noroot else None)
    argv = [kl.BIN, "script", script] + ([] if noroot else ["--root", h]) + paths + (["--size", hsize] if hsize else [])
    q = subprocess.run(["timeout", "600"] + argv, capture_output=True, cwd=h if noroot else None)
    n += 1
    lines += q.stdout.count(b"\n")
    if p.stdout != q.stdout:
        dout += 1
        a, b = p.stdout.splitlines(), q.stdout.splitlines()
        i = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), min(len(a), len(b)))
        print("OUT DIFF", script, "line", i, "of", len(a), len(b), (a[i][:200] if i < len(a) else None), "|", (b[i][:200] if i < len(b) else None))
    if p.returncode != q.returncode:
        drc += 1
        print("RC DIFF", script, p.returncode, q.returncode)
    if kl.read_tree(w) != kl.read_tree(h):
        dfiles += 1
        print("FILES DIFF", script)
    e = open(errf, "rb").read() if os.path.exists(errf) else b"(no stderr file)"
    if e:
        serr += 1
        print("STDERR", script, e[:300])
    hf = [os.path.join(dp, f) for dp, dn, fn in os.walk(home) for f in fn]
    if hf:
        homef += 1
        print("HOME FILES", hf[:5])
print(f"wincmp2 kind={kind} seeds={seeds} screen={screen} headless_size={hsize} noroot={noroot} scripts={n} response_lines={lines} "
      f"stdout_differ={dout} rc_differ={drc} files_differ={dfiles} window_stderr_nonempty={serr} home_written={homef}")
