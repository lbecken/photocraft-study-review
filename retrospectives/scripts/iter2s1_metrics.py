#!/usr/bin/env python3
"""Retrospective numbers for the first stage of iteration 2 (run R-223).

Reads metrics/tasks.jsonl, metrics/queue.jsonl, reviews/*.md front matter, tasks/*.md and
docs/decisions.md, docs/scorecard.md. Read-only; python3 standard library only.

Usage (from the repository root):
    python3 -I retrospectives/scripts/iter2s1_metrics.py [section ...]
Sections: completeness, tokens, budgets, extends, lead, queue, joins, reviews, rework, arms,
time, scorecard, decisions, acks, time2, ceilings, gates, all (default).

Conventions fixed here (stated in retrospectives/iter-2-stage-1.md):
- Stage-1 tasks: the ten with a `merged` queue record after D-091 and `iteration: 2`:
  T-039 T-042 T-019 T-020 T-021 T-036 T-023 T-037 T-038 T-049. Product tasks: the seven with a
  review (T-019 T-020 T-021 T-023 T-036 T-037 T-038); low-tier: T-039 T-042 T-049.
- Overhead: run records with task `iter-2` (planner runs, spec checks, evaluator).
- Iteration-1 comparison figures are taken from retrospectives/iter-1.md (its commands), or
  recomputed here from the same files where said (section `arms`).
"""
import json
import pathlib
import re
import statistics
import sys
from collections import Counter, defaultdict
from datetime import datetime

ROOT = pathlib.Path(__file__).resolve().parents[2]
STAGE = ["T-039", "T-042", "T-019", "T-020", "T-021", "T-036", "T-023", "T-037", "T-038", "T-049"]
PRODUCT = ["T-019", "T-020", "T-021", "T-023", "T-036", "T-037", "T-038"]
ITEM_TASKS = {"T-019": 1, "T-020": 4, "T-021": 4, "T-023": 4}
BUILDER = ("implementer", "test-author", "reviewer")
IT1_MEDIUM = {"T-010": "independent", "T-018": "independent", "T-011": "implementer"}


def load(p):
    return [json.loads(l) for l in (ROOT / p).read_text().splitlines() if l.strip()]


def ts(s):
    return datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ")


def front(path):
    m = re.match(r"---\n(.*?)\n---", path.read_text(), re.S)
    out = {}
    if m:
        for line in m.group(1).splitlines():
            if ":" in line and not line.startswith(" "):
                k, v = line.split(":", 1)
                out[k.strip()] = v.split("#")[0].strip()
    return out


def nums(s):
    return {k: int(v) for k, v in re.findall(r"(\w+): (\d+)", s or "")}


def pct(xs, q):
    xs = sorted(xs)
    k = (len(xs) - 1) * q
    lo, hi = int(k), min(int(k) + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)


def desc(xs):
    if not xs:
        return "n=0"
    return (f"n={len(xs)} median {statistics.median(xs):,.0f} p75 {pct(xs, .75):,.0f} "
            f"min {min(xs):,} max {max(xs):,} sum {sum(xs):,}")


recs = load("metrics/tasks.jsonl")
queue = load("metrics/queue.jsonl")
runs = [r for r in recs if r["kind"] == "run"]
runs2 = [r for r in runs if r["iteration"] == 2]
trans = [r for r in recs if r["kind"] == "transition"]
joins = [r for r in recs if r["kind"] == "join"]
meta = {}
for p in sorted((ROOT / "tasks").glob("T-*.md")):
    f = front(p)
    f["_path"] = p
    meta[f.get("id", p.stem[:5])] = f
reviews = {p.stem: front(p) for p in sorted((ROOT / "reviews").glob("T-*-r*.md"))}
decisions = (ROOT / "docs" / "decisions.md").read_text()
tier = {t: m.get("tier", "?").split()[0] for t, m in meta.items()}


def tok(task, pred=lambda r: True):
    return sum(r["tokens"] for r in runs if r["task"] == task and pred(r))


def section(n):
    print(f"\n## {n}")


def review_end(task, rnd):
    rv = reviews.get(f"{task}-r{rnd}")
    if not rv:
        return None
    rr = [r for r in runs if r["run"] == rv["run"]]
    return rr[0]["ended_at"] if rr else None


# ------------------------------------------------------------------ completeness
def completeness():
    section("completeness")
    print("iteration-2 run records:", len(runs2), "by role:", dict(Counter(r["role"] for r in runs2)))
    print("by outcome:", dict(Counter(r["outcome"] for r in runs2)))
    print("by task:", dict(Counter(r["task"] for r in runs2)))
    ids = sorted({int(r["run"][2:]) for r in runs})
    lo, hi = 154, 222
    missing = [f"R-{n:03d}" for n in range(lo, hi + 1) if n not in ids]
    print(f"run ids R-{lo}..R-{hi} with no run record anywhere:", missing)
    other = [(r["run"], r["iteration"], r["role"]) for r in runs
             if lo <= int(r["run"][2:]) <= hi and r["iteration"] != 2]
    print("run ids in range booked to another iteration:", other)
    print("missing_context != none:", [(r["run"], r["missing_context"]) for r in runs2
                                       if r["missing_context"] != "none"])
    odd = [(r["run"], r["wall_s"], r["tokens"]) for r in runs2 if r["wall_s"] < 60 and r["tokens"] > 50000]
    print("runs with wall_s < 60 and tokens > 50,000 (booking suspicious):", odd)
    # each run id mentioned in some task, review or evaluation file
    corpus = "".join(p.read_text() for p in list((ROOT / "tasks").glob("T-*.md")) +
                     list((ROOT / "reviews").glob("*.md")) + [ROOT / "evaluations" / "iter-2.md"])
    nolog = [r["run"] for r in runs2 if r["run"] not in corpus]
    print("iteration-2 runs whose id appears in no task, review or evaluation file:", nolog)
    # tasks without a run record
    for t in STAGE:
        n = sum(1 for r in runs if r["task"] == t)
        if n == 0:
            print("stage task without any run record:", t)
    # transitions
    print("iteration-2 transitions:", dict(Counter(x["to"] for x in trans if x["iteration"] == 2)))
    print("tasks with a transition to evaluated:", sorted({x["task"] for x in trans if x["to"] == "evaluated"}))
    print("states in front matter:", {t: meta[t].get("state") for t in STAGE})
    # join records with acc_defects filled
    j2 = [j for j in joins if j["task"] in STAGE]
    print("stage joins:", len(j2), "with acc_defects not null:", sum(j["acc_defects"] is not None for j in j2))
    print("revert_check records in stage:", sum(1 for r in recs if r["kind"] == "revert_check" and r.get("at", "") >= "2026-11-13T21:24:05Z"))


# ------------------------------------------------------------------ tokens
def tokens():
    section("tokens")
    tot = sum(r["tokens"] for r in runs2)
    over = [r for r in runs2 if r["task"] == "iter-2"]
    print(f"iteration-2 booked tokens: {tot:,} over {len(runs2)} runs")
    print(f"on stage tasks: {sum(tok(t) for t in STAGE):,}; on iter-2 overhead: {sum(r['tokens'] for r in over):,}"
          f" ({sum(r['tokens'] for r in over)/tot:.1%})")
    sc = [r for r in over if r.get("note", "").startswith("spec check")]
    pl = [r for r in over if r["role"] == "planner" and not r.get("note", "").startswith("spec check")]
    ev = [r for r in over if r["role"] == "evaluator"]
    print(f"  spec checks: {len(sc)} runs {sum(r['tokens'] for r in sc):,}; other planner: {len(pl)} runs "
          f"{sum(r['tokens'] for r in pl):,} {[r['run'] for r in pl]}; evaluator: {len(ev)} runs {sum(r['tokens'] for r in ev):,}")
    print("by role (all iteration 2):")
    byrole = defaultdict(int)
    for r in runs2:
        byrole[r["role"]] += r["tokens"]
    print("  ", {k: f"{v:,}" for k, v in byrole.items()})
    print("per stage task (booked; by role):")
    for t in STAGE:
        br = defaultdict(int)
        for r in runs:
            if r["task"] == t:
                br[r["role"]] += r["tokens"]
        print(f"  {t} {tier[t]:6} {tok(t):>9,} runs={sum(1 for r in runs if r['task']==t)} {dict(br)}")
    # spec checks by task (from note)
    per = defaultdict(int)
    for r in sc:
        m = re.search(r"T-\d{3}", r["note"])
        per[m.group(0) if m else "?"] += r["tokens"]
    print("spec-check tokens by task named in the note:", {k: f"{v:,}" for k, v in per.items()})
    items = 13
    it = sum(tok(t) for t in ITEM_TASKS)
    print(f"item tasks T-019/20/21/23: {it:,} for 13 items = {it/items:,.0f} per item; "
          f"+ their spec checks {sum(per[t] for t in ITEM_TASKS):,} = {(it+sum(per[t] for t in ITEM_TASKS))/items:,.0f}")
    print(f"all iteration-2 tokens per new verified item: {tot/items:,.0f}")
    prod = [tok(t) for t in PRODUCT]
    print("product tasks:", desc(prod))
    for tr in ("high", "medium", "low"):
        xs = [tok(t) for t in STAGE if tier[t] == tr]
        print(f"  {tr}: {desc(xs)}")
    print(f"tokens per merged task, without overhead: {sum(tok(t) for t in STAGE)/len(STAGE):,.0f}; "
          f"with: {tot/len(STAGE):,.0f}; product only with overhead: {tot/len(PRODUCT):,.0f}")
    # builder/review run p75 for ceilings
    for tr in ("high", "medium", "low"):
        xs = [tok(t) for t in STAGE if tier[t] == tr]
        if xs:
            print(f"  {tr} p75 of stage tasks: {pct(xs, .75):,.0f}")


# ------------------------------------------------------------------ budgets
def budgets():
    section("budgets")
    for t in STAGE:
        b = int(meta[t].get("budget_tokens", "0") or 0)
        s = tok(t)
        print(f"  {t} {tier[t]:6} budget {b:>9,} spent {s:>9,} ratio {s/b if b else 0:.2f}")


# ------------------------------------------------------------------ extends
def extends():
    section("extends (75 % rule)")
    rows = {m.group(2): (m.group(1), int(m.group(3))) for m in
            re.finditer(r"\| (D-\d+) \| [^|]*\| Extend (T-\d+) to (\d+)", decisions)}
    print("Extend rows:", rows)
    for t in PRODUCT:
        b = int(meta[t].get("budget_tokens", "0") or 0)
        e1 = review_end(t, 1)
        at = tok(t, lambda r: r["ended_at"] <= e1)
        after = tok(t, lambda r: r["started_at"] >= e1)
        tot = tok(t)
        row = rows.get(t)
        print(f"  {t} ceiling {b:,} at r1 end {at:,} ({at/b:.1%}) rework after r1 {after:,} total {tot:,}"
              f" row {row} {'(total/N %.1f%%)' % (100*tot/row[1]) if row else ''}"
              f" total/ceiling {tot/b:.1%}")


# ------------------------------------------------------------------ lead / queue
def lead():
    section("lead time ready -> merged, and first builder dispatch -> merged")
    for t in STAGE:
        rd = [x["at"] for x in trans if x["task"] == t and x["to"] == "ready"]
        mg = [q["ended_at"] for q in queue if q["task"] == t and q["result"] == "merged"]
        b = [r["started_at"] for r in runs if r["task"] == t and r["role"] in BUILDER]
        if rd and mg:
            h = (ts(mg[-1]) - ts(rd[0])).total_seconds() / 3600
            h2 = (ts(mg[-1]) - ts(min(b))).total_seconds() / 3600 if b else float("nan")
            print(f"  {t} {tier[t]:6} ready {rd[0][5:16]} merged {mg[-1][5:16]} lead {h:.2f} h; dispatch->merged {h2:.2f} h")


def queue_s():
    section("queue")
    q2 = [q for q in queue if q["task"] in STAGE]
    print("stage queue records:", len(q2), "results:", dict(Counter(q["result"] for q in q2)))
    print("first attempt merged:", sum(1 for q in q2 if q["attempt"] == 1 and q["result"] == "merged"), "of", len({q['task'] for q in q2}))
    gs = [q["gate_seconds"] for q in q2]
    print("gate seconds:", [f"{q['task']}:{q['gate_seconds']}" for q in q2], "median", statistics.median(gs))
    print("refusal records in queue.jsonl (result not merged/conflict/gate):", [q for q in q2 if q["result"] not in ("merged",)])


def joins_s():
    section("joins")
    for j in joins:
        if j["task"] in STAGE:
            print(f"  {j['task']} {j['at'][5:16]} acc {j['acc_total']} failbase {j['acc_fail_base']} kind {j['base_fail_kind']}"
                  f" firstjoinfail {j['acc_fail_first_join']} acc_defects {j['acc_defects']} guards {j['guard_total']}"
                  f" gate {j['gate_green']} result {j['result']}")
    per = Counter(j["task"] for j in joins if j["task"] in STAGE)
    print("joins per task:", dict(per), "total", sum(per.values()))
    print("first-join results:", {t: [j["result"] for j in joins if j["task"] == t][0] for t in per})
    print("base_fail_kind over stage joins:", dict(Counter(j["base_fail_kind"] for j in joins if j["task"] in STAGE)))


# ------------------------------------------------------------------ reviews
def reviews_s():
    section("reviews")
    tot = Counter()
    mut = defaultdict(Counter)
    for k, v in reviews.items():
        t = k[:5]
        if t not in PRODUCT:
            continue
        f, m = nums(v["findings"]), nums(v["mutants"])
        rnd = int(v["round"])
        rr = [r for r in runs if r["run"] == v["run"]]
        cost = rr[0]["tokens"] if rr else None
        print(f"  {k} {v['tier']:6} {v['arm']:11} {v['verdict']:15} {f} mutants {m} follow_ups {v.get('follow_ups')} tokens {cost:,}")
        tot.update({f"r{rnd}_{c}": n for c, n in f.items()})
        mut[rnd].update(m)
        tot[f"r{rnd}_tokens"] += cost or 0
        tot[f"r{rnd}_n"] += 1
    print("totals:", dict(tot))
    for rnd, m in mut.items():
        live = m["introduced"] - m["equivalent"]
        print(f"  round {rnd} mutants {dict(m)} kill rate by acceptance {m['killed_by_acceptance']}/{live}"
              f" = {m['killed_by_acceptance']/live:.0%}; any test {m['killed']}/{live}")
    print("round-1 verdicts:", Counter(v["verdict"] for k, v in reviews.items() if k[:5] in PRODUCT and v["round"] == "1"))


def rework():
    section("rework after round 1 (runs that start at or after the round-1 review run ended)")
    s = 0
    for t in PRODUCT:
        e1 = review_end(t, 1)
        rr = [r for r in runs if r["task"] == t and r["started_at"] >= e1]
        a = sum(r["tokens"] for r in rr)
        s += a
        print(f"  {t} {a:>8,} ({a/tok(t):.1%} of task) runs {[(r['run'], r['role'], r['tokens']) for r in rr]}")
    allp = sum(tok(t) for t in PRODUCT)
    print(f"total post-round-1 {s:,} = {s/allp:.1%} of product-task tokens {allp:,}")


def arms():
    section("medium-tier arms (stage 1 and iteration 1)")
    arm = {"T-020": "independent", "T-021": "implementer", "T-023": "implementer", **IT1_MEDIUM}
    for t, a in arm.items():
        br = defaultdict(int)
        for r in runs:
            if r["task"] == t:
                br[r["role"]] += r["tokens"]
        rvs = {k: v for k, v in reviews.items() if k[:5] == t}
        f = Counter()
        for v in rvs.values():
            f.update(nums(v["findings"]))
        r1 = rvs.get(f"{t}-r1")
        m1 = nums(r1["mutants"]) if r1 else {}
        live = m1.get("introduced", 0) - m1.get("equivalent", 0)
        js = [j for j in joins if j["task"] == t]
        print(f"  {t} {a:11} tokens {tok(t):,} {dict(br)} rounds {len(rvs)} findings(all rounds) {dict(f)}"
              f" r1 kill-by-acc {m1.get('killed_by_acceptance')}/{live} joins {len(js)} first-join {js[0]['result'] if js else None}"
              f" first-join failing acc {[j['acc_fail_first_join'] for j in js]}")


# ------------------------------------------------------------------ time
def time_s():
    section("time, concurrency, idle")
    st = [r for r in runs2]
    t0 = min(ts(r["started_at"]) for r in st)
    t1 = max(ts(r["ended_at"]) for r in st)
    q_end = max(ts(q["ended_at"]) for q in queue if q["task"] in STAGE)
    print("first iteration-2 run start", t0, "last run end", t1, "last stage merge", q_end)
    print(f"span first run start -> last merge: {(q_end - t0).total_seconds()/3600:.2f} h")
    ev = []
    for r in st:
        ev.append((ts(r["started_at"]), 1, r))
        ev.append((ts(r["ended_at"]), -1, r))
    # idle: gaps where no iteration-2 run is active, up to last merge
    iv = sorted((ts(r["started_at"]), ts(r["ended_at"])) for r in st)
    gaps, cur = [], iv[0][1]
    for s, e in iv[1:]:
        if s > cur:
            gaps.append((cur, s))
        cur = max(cur, e)
    if q_end > cur:
        gaps.append((cur, q_end))
    big = [(a.strftime("%m-%dT%H:%M"), b.strftime("%H:%M"), round((b - a).total_seconds() / 60)) for a, b in gaps if (b - a).total_seconds() >= 600]
    print(f"gaps with no iteration-2 run active: {len(gaps)} total {sum((b-a).total_seconds() for a,b in gaps)/3600:.2f} h; >=10 min: {big}")
    # builder concurrency
    bl = [r for r in st if r["role"] in BUILDER]
    pts = sorted([(ts(r["started_at"]), 1) for r in bl] + [(ts(r["ended_at"]), -1) for r in bl], key=lambda x: (x[0], x[1]))
    c, mx, area, last = 0, 0, Counter(), None
    for t, d in pts:
        if last is not None:
            area[c] += (t - last).total_seconds()
        c += d
        mx = max(mx, c)
        last = t
    print("max builder runs open at once (booked start/end):", mx,
          "hours at each level:", {k: round(v / 3600, 2) for k, v in sorted(area.items())})
    allp = sorted([(ts(r["started_at"]), 1) for r in st] + [(ts(r["ended_at"]), -1) for r in st], key=lambda x: (x[0], x[1]))
    c, mx2 = 0, 0
    for t, d in allp:
        c += d
        mx2 = max(mx2, c)
    print("max runs of any role open at once:", mx2)
    print("long wall runs (> 1 h):", [(r["run"], r["task"], r["role"], r["wall_s"], r["tokens"]) for r in st if r["wall_s"] > 3600])


# ------------------------------------------------------------------ scorecard
def scorecard():
    section("scorecard")
    txt = (ROOT / "docs" / "scorecard.md").read_text()
    m = re.search(r"\d+ items: [^\n]*", txt)
    print("docs/scorecard.md:", m.group(0) if m else "?")
    st = Counter(re.findall(r"\| (verified|implemented|planned|failing) \|", txt))
    print("status cells:", dict(st))
    pl = re.findall(r"\| `?([a-z]+\.[a-z0-9.-]+)`? \|[^\n]*\| planned \|", txt)
    print("planned items:", len(pl), Counter(p.split(".")[0] for p in pl))


# ------------------------------------------------------------------ decisions
def decisions_s():
    section("decisions")
    ids = [int(x) for x in re.findall(r"^\| D-(\d+) \|", decisions, re.M)]
    print("rows D-089..last:", len([i for i in ids if i >= 89]), "last", max(ids))
    print("ids missing in sequence:", [i for i in range(89, max(ids) + 1) if i not in ids])
    print("D-104 referenced in tasks/reviews:",
          sum(p.read_text().count("D-104") for p in list((ROOT / "tasks").glob("*.md")) + list((ROOT / "reviews").glob("*.md"))))


# ------------------------------------------------------------------ acknowledgements (v1.2 change 4)
def entry(text, run):
    lines = text.splitlines()
    out, on = [], False
    for ln in lines:
        if re.match(r"^- 20\d\d-\d\d-\d\d[^·]* · " + run + r" ", ln):
            on = True
            out.append(ln)
            continue
        if on:
            if re.match(r"^(- 20\d\d-|<!--|#)", ln):
                on = False
                continue
            out.append(ln)
    return "\n".join(out)


def acks():
    section("ruling acknowledgements in builder log entries (task files; reviews for reviewers)")
    res = Counter()
    for r in runs2:
        if r["task"] not in PRODUCT + ["T-039", "T-042"] or r["role"] not in BUILDER:
            continue
        txt = meta[r["task"]]["_path"].read_text()
        e = entry(txt, r["run"])
        if r["role"] == "reviewer":
            for k, v in reviews.items():
                if v.get("run") == r["run"]:
                    e += (ROOT / "reviews" / f"{k}.md").read_text()
        has_entry = bool(e.strip())
        ack = bool(re.search(r"acknowledg|accept|ruling[s]? [^.]{0,80}appl", e, re.I))
        res[(r["role"], has_entry, ack)] += 1
        if not ack:
            print("  no 'acknowledg' in entry:", r["run"], r["task"], r["role"], "entry found" if has_entry else "NO ENTRY")
    print("(role, entry found, acknowledgement word):", dict(res))


# ------------------------------------------------------------------ time net of stops
STOPS = [("2026-11-13T18:31:00Z", "2026-11-13T21:12:00Z", "D-092 usage limit"),
         ("2026-11-13T23:54:00Z", "2026-11-14T02:10:00Z", "D-112 usage limit + restart 02:11"),
         ("2026-11-14T06:16:00Z", "2026-11-14T07:10:00Z", "D-129 usage limit + restart 07:11"),
         ("2026-11-14T12:05:00Z", "2026-11-14T18:15:00Z", "D-143 usage limit (after close) + restart 18:15")]


def time2():
    section("time net of recorded usage-limit stops")
    v12, close = ts("2026-11-13T21:24:05Z"), ts("2026-11-14T12:03:39Z")
    print(f"v1.2 in force -> stage close: {(close - v12).total_seconds()/3600:.2f} h")
    for a, b, n in STOPS:
        print(f"  stop {n}: {(ts(b)-ts(a)).total_seconds()/3600:.2f} h")
    inside = sum(max(0, (min(ts(b), close) - max(ts(a), v12)).total_seconds()) for a, b, _ in STOPS)
    print(f"stops inside v1.2..close: {inside/3600:.2f} h; available {((close-v12).total_seconds()-inside)/3600:.2f} h")
    # minutes with >=1 builder run active, excluding stops, inside v1.2..close
    st = [(max(ts(r["started_at"]), v12), min(ts(r["ended_at"]), close)) for r in runs2]
    st = [(a, b) for a, b in st if b > a]
    def minus_stops(a, b):
        segs = [(a, b)]
        for sa, sb, _ in STOPS:
            sa, sb = ts(sa), ts(sb)
            new = []
            for x, y in segs:
                if sb <= x or sa >= y:
                    new.append((x, y))
                else:
                    if x < sa: new.append((x, sa))
                    if sb < y: new.append((sb, y))
            segs = new
        return segs
    iv = sorted(s for a, b in st for s in minus_stops(a, b))
    cov, cur_s, cur_e = 0, None, None
    for a, b in iv:
        if cur_e is None or a > cur_e:
            if cur_e is not None: cov += (cur_e - cur_s).total_seconds()
            cur_s, cur_e = a, b
        else:
            cur_e = max(cur_e, b)
    cov += (cur_e - cur_s).total_seconds()
    avail = (close - v12).total_seconds() - inside
    print(f"time with >=1 iteration-2 run active (net of stops): {cov/3600:.2f} h of {avail/3600:.2f} h available;"
          f" none running {(avail-cov)/3600:.2f} h ({(avail-cov)/avail:.0%})")


# ------------------------------------------------------------------ ceiling recalibration
IT1 = ["T-006", "T-008", "T-010", "T-011", "T-013", "T-014", "T-015", "T-016", "T-017", "T-018", "T-032", "T-035"]


def ceilings():
    section("ceilings: p75 of builder+review tokens per merged task, stage 1 and pooled with iteration 1")
    for tr in ("high", "medium", "low"):
        s1 = [tok(t, lambda r: r["role"] in BUILDER) for t in STAGE if tier[t] == tr and tok(t) > 0]
        i1 = [tok(t, lambda r: r["role"] in BUILDER) for t in IT1 if tier[t] == tr]
        pool = s1 + i1
        f = lambda xs: f"n={len(xs)} p75 {pct(xs, .75):,.0f} max {max(xs):,}" if xs else "n=0"
        print(f"  {tr}: stage {f(s1)}; iteration 1 {f(i1)}; pooled {f(pool)}")


def gates():
    section("gate attempts per task (sum of run gate_attempts)")
    g = {t: sum(r["gate_attempts"] for r in runs if r["task"] == t) for t in STAGE if t != "T-049"}
    print(g, "median", statistics.median(g.values()), "sum", sum(g.values()))


SECTIONS = {"completeness": completeness, "tokens": tokens, "budgets": budgets, "extends": extends,
            "lead": lead, "queue": queue_s, "joins": joins_s, "reviews": reviews_s, "rework": rework,
            "arms": arms, "time": time_s, "scorecard": scorecard, "decisions": decisions_s, "acks": acks, "time2": time2, "ceilings": ceilings, "gates": gates}

if __name__ == "__main__":
    want = sys.argv[1:] or ["all"]
    for n, f in SECTIONS.items():
        if "all" in want or n in want:
            f()
