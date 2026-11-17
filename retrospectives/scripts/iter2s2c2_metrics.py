#!/usr/bin/env python3
"""Retrospective numbers for iteration 2, stage 2, from cut 1 to cut 2 (run R-304).

Reads the records as they were at commit c45a17f (D-217) via `git show`, so that later
writes by other runs do not move the figures: metrics/tasks.jsonl, metrics/queue.jsonl,
reviews/*.md front matter, docs/decisions.md. Read-only; python3 standard library only.

Usage (from the repository root):
    python3 -I retrospectives/scripts/iter2s2c2_metrics.py [section ...]
Sections: completeness tokens overhead budgets joins queue reviews arms rework time lead
          gates decisions acks helpers all (default).

Conventions (stated in retrospectives/iter-2-stage-2-cut-2.md):
- Period runs: R-254 .. R-303 (the brief's range).
- Product tasks of cut 2: T-057, T-058, T-059. Platform tasks: T-069, T-070, T-054.
  Publish: T-084 (no run).
- Overhead: run records with task `iter-2` in R-254 .. R-303.
- Window: D-171's commit 30917a5 (2026-11-15T16:15:17Z) to D-216's commit 995b176
  (2026-11-16T02:50:49Z).
"""
import json
import re
import statistics
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime

SHA = "c45a17f9b6142f422f0ca2eaf8487da3975e22b4"
LO, HI = 254, 303
CUT2 = ["T-057", "T-058", "T-059"]
PLAT = ["T-069", "T-070", "T-054"]
TASKS = CUT2 + PLAT
ITEMS = {"T-057": "ui.project-panel", "T-058": "ui.tabs", "T-059": "ui.status-bar"}
BUILDER = ("implementer", "test-author", "reviewer")
START = "2026-11-15T16:15:17Z"   # 30917a5, D-171
END = "2026-11-16T02:50:49Z"     # 995b176, D-216
CEIL = {"low": 150000, "medium": 800000, "high": 1100000}

OVERHEAD_KIND = {
    "R-254": "retrospective to cut 1", "R-255": "platform plan (ADR-0009)", "R-257": "methodology edit v1.4",
    "R-261": "spec check", "R-262": "spec check", "R-270": "spec check", "R-276": "spec check",
    "R-284": "spec check", "R-285": "spec check", "R-295": "spec check",
    "R-271": "CI reading", "R-282": "CI reading", "R-292": "CI reading", "R-293": "CI reading",
    "R-294": "CI reading", "R-299": "CI reading", "R-301": "CI reading", "R-303": "CI reading",
    "R-302": "evaluation at cut 2",
}


def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True, check=True).stdout


def show(path):
    return git("show", f"{SHA}:{path}")


def ts(s):
    return datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ")


def front(txt):
    m = re.match(r"---\n(.*?)\n---", txt, re.S)
    out = {}
    if m:
        for line in m.group(1).splitlines():
            if ":" in line and not line.startswith(" "):
                k, v = line.split(":", 1)
                out[k.strip()] = v.split("#")[0].strip()
    return out


def nums(s):
    return {k: int(v) for k, v in re.findall(r"(\w+): (\d+)", s or "")}


recs = [json.loads(l) for l in show("metrics/tasks.jsonl").splitlines() if l.strip()]
queue = [json.loads(l) for l in show("metrics/queue.jsonl").splitlines() if l.strip()]
runs = [r for r in recs if r["kind"] == "run"]
prun = [r for r in runs if LO <= int(r["run"][2:]) <= HI]
trans = [r for r in recs if r["kind"] == "transition"]
joins = [r for r in recs if r["kind"] == "join"]
jdef = [r for r in recs if r["kind"] == "join_defects"]
files = git("ls-tree", "-r", "--name-only", SHA).split()
reviews = {}
for p in files:
    m = re.match(r"reviews/(T-\d+-r\d+)\.md$", p)
    if m:
        reviews[m.group(1)] = front(show(p))
decisions = show("docs/decisions.md")


def T(r):
    return r["tokens"] or 0


def tok(task, pred=lambda r: True):
    return sum(T(r) for r in runs if r["task"] == task and pred(r))


def run(rid):
    return next(r for r in runs if r["run"] == rid)


def section(n):
    print(f"\n## {n}")


def completeness():
    section("completeness")
    ids = {int(r["run"][2:]) for r in runs}
    print(f"run records R-{LO}..R-{HI}: {len(prun)}; missing ids: {[f'R-{n}' for n in range(LO, HI + 1) if n not in ids]}")
    print("by role:", dict(Counter(r["role"] for r in prun)))
    print("by task:", dict(Counter(r["task"] for r in prun)))
    print("by outcome:", dict(Counter(r["outcome"] for r in prun)))
    print("tokens null (unknown):", [(r["run"], r["task"], r["role"]) for r in prun if r["tokens"] is None])
    print("missing_context not none:", [(r["run"], r["missing_context"]) for r in prun if r["missing_context"] != "none"])
    print("wall_s < 60:", [(r["run"], r["role"], r["wall_s"], T(r)) for r in prun if r["wall_s"] < 60])
    bad = [(r["run"], r["wall_s"]) for r in prun
           if abs(int((ts(r["ended_at"]) - ts(r["started_at"])).total_seconds()) - r["wall_s"]) > 1]
    print("wall_s differs from ended-started by > 1 s:", bad)
    print("tier field of task runs:", sorted({(r["task"], r["tier"]) for r in prun if r["task"] != "iter-2"}))
    corpus = "".join(show(p) for p in files if re.match(r"(tasks|reviews|evaluations|retrospectives)/.*\.md$|docs/decisions\.md$|handoffs/", p))
    print("period runs whose id appears in no task/review/evaluation/retrospective/decision/handoff file:",
          [r["run"] for r in prun if r["run"] not in corpus])
    for t in TASKS + ["T-084"]:
        tr = [x for x in trans if x["task"] == t]
        print(f"  {t}: runs {sum(1 for r in runs if r['task'] == t)}, transitions {len(tr)}, joins {sum(1 for j in joins if j['task'] == t)}, "
              f"queue records {sum(1 for q in queue if q['task'] == t)}")
        print("     ", " ".join(f"{x['from']}>{x['to']}@{x['at'][5:16]}" for x in tr))
    print("join_defects records in the period:", [j for j in jdef if j["at"] >= START])


def tokens():
    section("tokens")
    tot = sum(T(r) for r in prun)
    print(f"R-{LO}..R-{HI}: {tot:,} tokens over {len(prun)} runs ({sum(1 for r in prun if r['tokens'] is None)} unknown)")
    for t in TASKS:
        br, n = defaultdict(int), Counter()
        for r in runs:
            if r["task"] == t:
                br[r["role"]] += T(r)
                n[r["role"]] += 1
        unk = [r["run"] for r in runs if r["task"] == t and r["tokens"] is None]
        print(f"  {t} {tok(t):>9,} runs {sum(n.values())} unknown {unk} " + ", ".join(f"{k} {v:,} ({n[k]})" for k, v in br.items()))
    c2 = sum(tok(t) for t in CUT2)
    pl = sum(tok(t) for t in PLAT)
    ov = sum(T(r) for r in prun if r["task"] == "iter-2")
    print(f"cut-2 product tasks {c2:,}; platform tasks {pl:,}; iter-2 overhead {ov:,} ({ov/tot:.1%}); sum {c2+pl+ov:,}")
    byrole = defaultdict(int)
    for r in prun:
        byrole[r["role"]] += T(r)
    print("by role (all period runs):", {k: f"{v:,}" for k, v in byrole.items()})
    print(f"per new verified item (3): cut-2 task tokens {c2/3:,.0f}; with all overhead and platform work {tot/3:,.0f}")
    for t in CUT2:
        print(f"  {ITEMS[t]}: {tok(t):,}")
    allr = sum(T(r) for r in runs)
    print(f"all run records at {SHA[:7]}: {allr:,} over {len(runs)} runs; unknown {sum(1 for r in runs if r['tokens'] is None)}")
    print(f"all run records up to R-253: {sum(T(r) for r in runs if int(r['run'][2:]) <= 253):,}")


def overhead():
    section("overhead by kind (iter-2 runs in the period)")
    kinds = defaultdict(list)
    for r in prun:
        if r["task"] == "iter-2":
            kinds[OVERHEAD_KIND.get(r["run"], "?")].append(r)
    tot = sum(T(r) for r in prun)
    ov = sum(T(r) for r in prun if r["task"] == "iter-2")
    for k, rs in sorted(kinds.items(), key=lambda kv: -sum(T(r) for r in kv[1])):
        s = sum(T(r) for r in rs)
        print(f"  {k:28} {s:>9,} ({s/tot:.1%} of all, {s/ov:.1%} of overhead) runs {[(r['run'], T(r)) for r in rs]}")
    pre = sum(T(run(x)) for x in ("R-254", "R-255", "R-257"))
    print(f"overhead without R-254, R-255, R-257 (retrospective, platform plan, methodology edit): {ov-pre:,} of {tot-pre:,} = {(ov-pre)/(tot-pre):.1%}")
    print(f"D-216 states 2,487,330 overhead and 6,178,453 in all; recomputed: overhead R-254..R-302 = "
          f"{sum(T(r) for r in prun if r['task']=='iter-2' and int(r['run'][2:])<=302):,}; all R-254..R-302 = "
          f"{sum(T(r) for r in prun if int(r['run'][2:])<=302):,}")


def helpers():
    section("CI reading")
    for rid in ("R-271", "R-282", "R-292", "R-293", "R-294", "R-299", "R-301", "R-303"):
        r = run(rid)
        print(f"  {rid} {T(r):>8,} wall {r['wall_s']:>4} s tool_calls {r['tool_calls']:>3} {r['started_at'][5:16]} {r['note'][:90]}")
    s = sum(T(run(x)) for x in ("R-271", "R-282", "R-292", "R-293", "R-294", "R-299", "R-301", "R-303"))
    print(f"CI reading total {s:,}; logs by helper runs {s - T(run('R-294')):,}; artefacts read by the resumed planner R-294 {T(run('R-294')):,}")
    print(f"R-293 (logs of run 43775682214) {T(run('R-293')):,} against R-294 (artefacts of the same run) {T(run('R-294')):,}: ratio {T(run('R-293'))/T(run('R-294')):.1f}")


def budgets():
    section("budgets and Extend rows")
    for t in TASKS:
        tier = next(r["tier"] for r in runs if r["task"] == t)
        b = CEIL[tier]
        rv1 = reviews.get(f"{t}-r1")
        at = tok(t, lambda r: rv1 and r["ended_at"] <= run(rv1["run"])["ended_at"]) if rv1 else None
        print(f"  {t} {tier} ceiling {b:,}; at r1 end {at:,} ({at/b:.1%}); total {tok(t):,} ({tok(t)/b:.2f}x)")
    rows = re.findall(r"\| (D-\d+) \| [^|]*\| Extend (T-\d+) to (\d+)", decisions)
    print("Extend rows (all):", rows[-6:])
    for d, t, n in rows:
        if t in TASKS:
            print(f"  {d} {t} to {int(n):,}: final {tok(t):,} = {tok(t)/int(n):.1%} of the extension")
    t69 = [r for r in runs if r["task"] == "T-069"]
    print("T-069 runs:", [(r["run"], r["role"], T(r), r["ended_at"][11:19]) for r in t69])
    t58 = [r for r in runs if r["task"] == "T-058"]
    print("T-058 runs:", [(r["run"], r["role"], T(r), r["started_at"][11:19]) for r in t58])


def joins_s():
    section("joins")
    pj = [j for j in joins if j["at"] >= START]
    for j in pj:
        print(f"  {j['task']} {j['at'][5:19]} acc {j['acc_total']} failbase {j['acc_fail_base']} kind {j['base_fail_kind']} "
              f"fail_at_join {j['acc_fail_first_join']} acc_defects {j['acc_defects']} guards {j.get('guard_total')} "
              f"gate {j['gate_green']} result {j['result']}")
    print("joins per task:", dict(Counter(j["task"] for j in pj)), "total", len(pj))
    print("results:", dict(Counter(j["result"] for j in pj)))
    first = {}
    for j in pj:
        first.setdefault(j["task"], j)
    print("first join per task joined:", {t: j["result"] for t, j in first.items()})
    print("acc_defects null on", sum(1 for j in pj if j["acc_defects"] is None), "of", len(pj))


def queue_s():
    section("queue")
    pq = [q for q in queue if q["started_at"] >= START]
    for q in pq:
        print(f"  {q['task']} attempt {q['attempt']} {q['started_at'][5:19]} {q['result']} step {q['failed_step']} gate_s {q['gate_seconds']} xtask {q['gate_xtask']}")
    tasks_q = sorted({q["task"] for q in pq})
    f1 = [t for t in tasks_q if any(q["task"] == t and q["attempt"] == 1 and q["result"] == "merged" for q in pq)]
    print(f"first attempt merged (records): {len(f1)} of {len(tasks_q)}: {f1}")
    gs = [q["gate_seconds"] for q in pq if q["result"] == "merged"]
    print("merged gate seconds:", gs, "median", statistics.median(gs))
    pre = [q for q in pq if q["started_at"] < "2026-11-16T01:28:32Z" and q["result"] == "merged"]
    post = [q for q in pq if q["started_at"] > "2026-11-16T01:28:32Z" and q["result"] == "merged"]
    print("before T-054's merge:", [(q["task"], q["gate_seconds"], q["gate_xtask"]) for q in pre])
    print("T-054 and after:", [(q["task"], q["gate_seconds"], q["gate_xtask"]) for q in pq if q["task"] in ("T-054",)],
          [(q["task"], q["gate_seconds"], q["gate_xtask"]) for q in post])
    m = [q["gate_seconds"] for q in pq if q["result"] == "merged" and q["gate_xtask"] == "main"]
    print("merged with main's xtask only: median", statistics.median(m), m)
    c1 = [1074.9, 548.3, 560.7, 720.4]
    print("cut-1 period merged gate seconds (retro-2c1):", c1, "median", statistics.median(c1))


def reviews_s():
    section("reviews")
    tot = defaultdict(Counter)
    for k, v in sorted(reviews.items()):
        if k[:5] not in TASKS:
            continue
        f, m = nums(v.get("findings")), nums(v.get("mutants"))
        rnd = int(v["round"])
        cost = T(run(v["run"]))
        print(f"  {k} {v['run']} {v['verdict']:15} {f} doc {v.get('doc_sentences')} mutants {m} tokens {cost:,}")
        if k[:5] in CUT2 + ["T-070", "T-054"]:
            tot[rnd].update(f)
            tot[rnd]["doc_sentences"] += int(v.get("doc_sentences") or 0)
            tot[rnd]["tokens"] += cost
            for kk, vv in m.items():
                tot[rnd]["m_" + kk] += vv
    for rnd, c in sorted(tot.items()):
        live = c["m_introduced"] - c["m_equivalent"]
        print(f"  round {rnd} (T-057, T-058, T-059, T-070, T-054): {dict(c)}; acc kill {c['m_killed_by_acceptance']}/{live} = "
              f"{c['m_killed_by_acceptance']/live:.0%}; any {c['m_killed']}/{live} = {c['m_killed']/live:.0%}")


def arms():
    section("arms: T-058 (independent) vs T-057, T-059 (implementer-first)")
    for t in CUT2:
        br = defaultdict(int)
        for r in runs:
            if r["task"] == t:
                br[r["role"]] += T(r)
        r1 = reviews[f"{t}-r1"]
        m = nums(r1["mutants"])
        live = m["introduced"] - m["equivalent"]
        nj = sum(1 for j in joins if j["task"] == t)
        nr = max(int(v["round"]) for k, v in reviews.items() if k[:5] == t)
        print(f"  {t} arm {r1['arm']}: tokens {tok(t):,} {dict(br)}; joins {nj}; rounds {nr}; r1 findings {r1['findings']}; "
              f"r1 mutants live {live}, by acceptance {m['killed_by_acceptance']} ({m['killed_by_acceptance']/live:.0%}), "
              f"procedure {m['killed_by_procedure']}, survived {m['survived']}")
    # building runs before the first join (build cost) vs after
    for t in CUT2:
        j1 = next(j for j in joins if j["task"] == t)["at"]
        pre = sum(T(r) for r in runs if r["task"] == t and r["ended_at"] <= j1)
        print(f"  {t} tokens up to the first join {pre:,} (runs {[r['run'] for r in runs if r['task']==t and r['ended_at']<=j1]})")


def rework():
    section("rework after round 1 (runs starting at or after the round-1 review run ended)")
    s = 0
    for t in TASKS:
        rv = reviews.get(f"{t}-r1")
        if not rv:
            continue
        e1 = run(rv["run"])["ended_at"]
        rr = [r for r in runs if r["task"] == t and r["started_at"] >= e1]
        a = sum(T(r) for r in rr)
        s += a
        print(f"  {t} {a:>8,} ({a/tok(t):.1%}) {[(r['run'], r['role'], T(r)) for r in rr]}")
    allp = sum(tok(t) for t in TASKS)
    print(f"total post-round-1 {s:,} = {s/allp:.1%} of {allp:,} (T-069: low, its only review run is r1)")


def time_s():
    section("time")
    a, b = ts(START), ts(END)
    print(f"window {START} .. {END}: {(b-a).total_seconds()/3600:.2f} h")
    st = [r for r in prun if ts(r["ended_at"]) > a and ts(r["started_at"]) < b]
    iv = sorted((max(ts(r["started_at"]), a), min(ts(r["ended_at"]), b)) for r in st)
    gaps, cur = [], a
    for s_, e in iv:
        if s_ > cur:
            gaps.append((cur, s_))
        cur = max(cur, e)
    if b > cur:
        gaps.append((cur, b))
    idle = sum((y - x).total_seconds() for x, y in gaps)
    print(f"time with no run active (by the records): {idle/3600:.2f} h ({idle/(b-a).total_seconds():.0%}) in {len(gaps)} gaps")
    print("gaps >= 10 min:", [(x.strftime('%H:%M:%S'), y.strftime('%H:%M:%S'), round((y-x).total_seconds()/60, 1)) for x, y in gaps if (y - x).total_seconds() >= 600])
    stop_a, stop_b = ts("2026-11-15T20:35:00Z"), ts("2026-11-15T21:11:33Z")
    over = [r["run"] for r in st if ts(r["started_at"]) < stop_a and ts(r["ended_at"]) > stop_b]
    print(f"runs whose record spans the usage-limit stop (about 20:35Z to the restart 21:11:33Z, D-190): {over}")
    print(f"  the stop is {(stop_b-stop_a).total_seconds()/60:.0f} min; with it counted as idle: {(idle+(stop_b-stop_a).total_seconds())/3600:.2f} h")
    bl = [r for r in st if r["role"] in BUILDER]
    pts = sorted([(ts(r["started_at"]), 1) for r in bl] + [(ts(r["ended_at"]), -1) for r in bl], key=lambda x: (x[0], x[1]))
    c, mx, area, last = 0, 0, Counter(), None
    for t, d in pts:
        if last is not None:
            area[c] += (t - last).total_seconds()
        c += d
        mx = max(mx, c)
        last = t
    print("max builder/reviewer runs open at once:", mx, "hours at each level:", {k: round(v / 3600, 2) for k, v in sorted(area.items())})
    impl_test = [r for r in st if r["role"] in ("implementer", "test-author")]
    pts = sorted([(ts(r["started_at"]), 1) for r in impl_test] + [(ts(r["ended_at"]), -1) for r in impl_test], key=lambda x: (x[0], x[1]))
    c, mx2 = 0, 0
    for t, d in pts:
        c += d
        mx2 = max(mx2, c)
    print("max implementer+test-author runs open at once:", mx2)


def lead():
    section("lead (ready -> merged)")
    for t in TASKS + ["T-084"]:
        rd = [x["at"] for x in trans if x["task"] == t and x["to"] == "ready"]
        mg = [q["ended_at"] for q in queue if q["task"] == t and q["result"] == "merged"]
        if rd and mg:
            print(f"  {t} ready {rd[0][5:16]} merged {mg[-1][5:16]} lead {(ts(mg[-1])-ts(rd[0])).total_seconds()/3600:.2f} h; "
                  f"sum of run walls {sum(r['wall_s'] for r in runs if r['task']==t)/3600:.2f} h")
        else:
            print(f"  {t} ready {rd} merged {mg}")


def gates():
    section("gate attempts (sum of run gate_attempts) per task")
    print({t: sum(r["gate_attempts"] for r in runs if r["task"] == t) for t in TASKS})


def decisions_s():
    section("decisions")
    ids = sorted({int(x) for x in re.findall(r"^\| D-(\d+) \|", decisions, re.M)})
    print("rows D-171..:", [i for i in ids if i >= 171], "missing in 1..max:", [i for i in range(1, max(ids) + 1) if i not in ids])
    cited = set()
    for p in files:
        if re.match(r"(tasks|reviews|evaluations|handoffs|docs/adr|incidents)/.*\.md$|STATE\.md$", p):
            cited |= {int(x) for x in re.findall(r"D-(\d{3})", show(p))}
    print("cited D-NNN with no row:", sorted(c for c in cited if c not in ids))


def acks():
    section("acknowledgement of rulings (word match in the run's log entry or review)")
    texts = {t: show(next(p for p in files if p.startswith(f"tasks/{t}-"))) for t in TASKS}
    for r in prun:
        if r["task"] not in TASKS or r["role"] not in BUILDER:
            continue
        txt = texts[r["task"]]
        m = re.search(r"^- [^\n]*" + re.escape(r["run"]) + r"[^\n]*(\n(?!- 20|<!--|###).*)*", txt, re.M)
        e = m.group(0) if m else ""
        ack = bool(re.search(r"acknowledg|accept", e, re.I))
        print(f"  {r['run']} {r['task']} {r['role']:11} entry {'yes' if m else 'NO'} ack-word {'yes' if ack else 'no'}")


def checks():
    section("checks of figures in the director's record")
    # D-196: "a first review round has cost 200,000 to 260,000 on the last three UI tasks"
    r1 = [(k, reviews[k]["run"], T(run(reviews[k]["run"]))) for k in ("T-056-r1", "T-065-r1", "T-057-r1")]
    print("D-196 claim 200,000-260,000; round-1 reviews of the three UI tasks before T-058:", r1)
    # D-203: 855,501 for T-058 after R-288, R-289
    print("D-203 855,501 recomputed:", f"{sum(T(run(x)) for x in ('R-279', 'R-280', 'R-283', 'R-288', 'R-289')):,}")
    print("D-188 653,988 recomputed:", f"{T(run('R-265')) + T(run('R-269')):,}")
    print("D-178 344,183 recomputed:", f"{sum(T(run(x)) for x in ('R-256', 'R-258', 'R-259')):,}")
    a, b = ts("2026-11-14T04:34:26Z"), ts("2026-11-15T18:02:59Z")
    print(f"INC-001 red_from 47135bc push 04:34:26Z to green_at 18:02:59Z: {(b-a).total_seconds()/60:,.0f} min (D-179: about 2,249)")
    # intervals with 4 builder runs open
    bl = [r for r in prun if r["role"] in BUILDER]
    ev = sorted([(ts(r["started_at"]), 1, r["run"]) for r in bl] + [(ts(r["ended_at"]), -1, r["run"]) for r in bl], key=lambda x: (x[0], x[1]))
    c, op, since = 0, set(), None
    for t, d, rid in ev:
        if d == 1:
            op.add(rid)
        else:
            op.discard(rid)
        c += d
        if c >= 4 and since is None:
            since, who = t, sorted(op)
        if c < 4 and since is not None:
            print(f"  4 builder/reviewer runs open {since.strftime('%H:%M:%S')}..{t.strftime('%H:%M:%S')} ({(t-since).total_seconds():.0f} s): {who}")
            since = None
    imp = [r for r in prun if r["role"] in ("implementer", "test-author")]
    ev = sorted([(ts(r["started_at"]), 1, r["run"]) for r in imp] + [(ts(r["ended_at"]), -1, r["run"]) for r in imp], key=lambda x: (x[0], x[1]))
    c, op, since = 0, set(), None
    for t, d, rid in ev:
        (op.add if d == 1 else op.discard)(rid)
        c += d
        if c >= 4 and since is None:
            since, who = t, sorted(op)
        if c < 4 and since is not None:
            print(f"  4 implementer/test-author runs open {since.strftime('%H:%M:%S')}..{t.strftime('%H:%M:%S')} ({(t-since).total_seconds():.0f} s): {who}")
            since = None


SECTIONS = {"checks": checks, "completeness": completeness, "tokens": tokens, "overhead": overhead, "helpers": helpers,
            "budgets": budgets, "joins": joins_s, "queue": queue_s, "reviews": reviews_s, "arms": arms,
            "rework": rework, "time": time_s, "lead": lead, "gates": gates, "decisions": decisions_s,
            "acks": acks}

if __name__ == "__main__":
    want = sys.argv[1:] or ["all"]
    for n, f in SECTIONS.items():
        if "all" in want or n in want:
            f()
