#!/usr/bin/env python3
"""Retrospective of cut 2 (R-304): overlap of `touches` among the platform batch T-071..T-079
and the drafts of the planned feature items, read at commit c45a17f (git show). Read-only.
Usage (repository root): python3 -I retrospectives/scripts/iter2s2c2_touches.py
"""
import re
import subprocess
from collections import defaultdict

SHA = "c45a17f9b6142f422f0ca2eaf8487da3975e22b4"
BATCH = ["T-071", "T-079", "T-072", "T-073", "T-074", "T-075", "T-076", "T-077", "T-078"]
FEAT = ["T-022", "T-024", "T-025", "T-026", "T-027", "T-028", "T-031", "T-062", "T-063"]


def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=True).stdout


files = git("ls-tree", "-r", "--name-only", SHA, "tasks").split()


def task_file(t):
    return next(f for f in files if f.startswith(f"tasks/{t}-"))


def front(t):
    txt = git("show", f"{SHA}:{task_file(t)}")
    m = re.match(r"---\n(.*?)\n---", txt, re.S)
    fm = m.group(1)
    tl = re.search(r"^touches:.*?\n((?:  - .*\n)*)", fm + "\n", re.M)
    paths = [re.sub(r"\s+#.*", "", l[4:]).strip() for l in tl.group(1).splitlines()] if tl else []
    st = re.search(r"^state:\s*(\S+)", fm, re.M).group(1)
    tier = re.search(r"^tier:\s*(\S+)", fm, re.M).group(1)
    dep = re.search(r"^depends_on:\s*(\[[^\]]*\])", fm, re.M).group(1)
    bud = re.search(r"^budget_tokens:\s*(\d+)", fm, re.M).group(1)
    return dict(paths=paths, state=st, tier=tier, dep=dep, budget=int(bud), text=txt)


def overlap(a, b):
    a, b = a.rstrip("/"), b.rstrip("/")
    return a == b or a.startswith(b + "/") or b.startswith(a + "/")


def report(group, name):
    print(f"\n## {name}")
    fr = {t: front(t) for t in group}
    for t in group:
        f = fr[t]
        print(f"  {t} state {f['state']} tier {f['tier']} budget {f['budget']:,} depends_on {f['dep']} touches {len(f['paths'])}")
    pairs = []
    for i, a in enumerate(group):
        for b in group[i + 1:]:
            shared = sorted({p for p in fr[a]["paths"] for q in fr[b]["paths"] if overlap(p, q)})
            if shared:
                pairs.append((a, b, shared))
    print(f"  pairs with overlapping touches: {len(pairs)} of {len(group)*(len(group)-1)//2}")
    for a, b, s in pairs:
        print(f"    {a} x {b}: {len(s)}: {', '.join(s[:6])}{' …' if len(s) > 6 else ''}")
    cnt = defaultdict(set)
    for t in group:
        for p in fr[t]["paths"]:
            cnt[p].add(t)
    multi = {p: sorted(v) for p, v in cnt.items() if len(v) > 1}
    print(f"  paths listed by more than one task: {len(multi)}")
    for p, v in sorted(multi.items()):
        print(f"    {p}: {v}")
    gp = [p for t in group for p in fr[t]["paths"] if p.startswith((".github", "xtask/")) or p == "Cargo.toml"]
    print(f"  gate-path entries (.github, xtask/, Cargo.toml) across the group: {len(gp)}")
    return fr


b = report(BATCH, "platform batch")
for t in BATCH:
    txt = b[t]["text"]
    print(f"  {t}: mentions of 'ci/T-': {len(re.findall(r'ci/T-', txt))}; 'CI-REQUEST': {len(re.findall('CI-REQUEST', txt))}; "
          f"criteria 'Observed only through CI' present: {'Observed only through CI' in txt or 'through CI' in txt}")
f = report(FEAT, "feature drafts of the 17 planned items")
for t in FEAT:
    last = git("log", "-1", "--format=%cI", SHA, "--", task_file(t)).strip()
    txt = f[t]["text"]
    print(f"  {t} last edited {last}; mentions v1.4/§8.1/platform: {bool(re.search(r'v1\.4|§8\.1|platform', txt))}; "
          f"mentions T-055..T-059: {sorted(set(re.findall(r'T-05[5-9]', txt)))}")
