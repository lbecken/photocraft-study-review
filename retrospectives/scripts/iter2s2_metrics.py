#!/usr/bin/env python3
"""Retrospective numbers for the second stage of iteration 2 up to cut 1 (run R-254).

Reads metrics/tasks.jsonl, metrics/queue.jsonl, reviews/*.md front matter, tasks/*.md,
docs/decisions.md, docs/scorecard.md. Read-only; python3 standard library only.

Usage (from the repository root):
    python3 -I retrospectives/scripts/iter2s2_metrics.py [section ...]
Sections: completeness tokens budgets joins queue reviews rework time lead gates
          estimates decisions acks speccheck all (default).

Conventions fixed here (stated in retrospectives/iter-2-stage-2-cut-1.md):
- Stage runs: R-223 .. R-253 (the brief's range). R-223 (the retrospective of stage 1) and
  R-224 (the v1.3 edit) precede the plan of stage 2 (R-225); they are shown separately.
- Stage tasks: T-055, T-056, T-065 (product, reviewed) and T-067 (the publish, no run).
- Overhead: run records with task `iter-2` in R-223 .. R-253.
- Window of the stage: D-147's commit (2026-11-14T19:06:27Z) to T-067's merge
  (2026-11-15T05:49:01Z, queue record), the moment D-170 records.
"""
import json
import pathlib
import re
import statistics
import sys
from collections import Counter, defaultdict
from datetime import datetime

ROOT = pathlib.Path(__file__).resolve().parents[2]
LO, HI = 223, 253
PRODUCT = ["T-055", "T-056", "T-065"]
STAGE = PRODUCT + ["T-067"]
ITEMS = {"T-055": 0, "T-056": 2, "T-065": 1}
BUILDER = ("implementer", "test-author", "reviewer")
START = "2026-11-14T19:06:27Z"   # commit 9d5f6d6, D-147
END = "2026-11-15T05:49:01Z"     # T-067 merged (queue.jsonl)


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


recs = load("metrics/tasks.jsonl")
queue = load("metrics/queue.jsonl")
runs = [r for r in recs if r["kind"] == "run"]
srun = [r for r in runs if LO <= int(r["run"][2:]) <= HI]
trans = [r for r in recs if r["kind"] == "transition"]
joins = [r for r in recs if r["kind"] == "join"]
jdef = [r for r in recs if r["kind"] == "join_defects"]
meta = {}
for p in sorted((ROOT / "tasks").glob("T-*.md")):
    f = front(p)
    f["_path"] = p
    meta[f.get("id", p.stem[:5])] = f
reviews = {p.stem: front(p) for p in sorted((ROOT / "reviews").glob("T-*-r*.md"))}
decisions = (ROOT / "docs" / "decisions.md").read_text()


def tok(task, pred=lambda r: True):
    return sum(r["tokens"] for r in runs if r["task"] == task and pred(r))


def section(n):
    print(f"\n## {n}")


def run(rid):
    return next(r for r in runs if r["run"] == rid)


# ------------------------------------------------------------------ completeness
def completeness():
    section("completeness")
    ids = {int(r["run"][2:]) for r in runs}
    print(f"run records R-{LO}..R-{HI}:", len(srun), "missing ids:",
          [f"R-{n}" for n in range(LO, HI + 1) if n not in ids])
    print("by role:", dict(Counter(r["role"] for r in srun)))
    print("by task:", dict(Counter(r["task"] for r in srun)))
    print("by outcome:", dict(Counter(r["outcome"] for r in srun)))
    print("iteration field:", dict(Counter(r["iteration"] for r in srun)))
    print("tier field of stage-task runs:", [(r["run"], r["task"], r["tier"]) for r in srun if r["task"] in STAGE])
    print("missing_context not none:", [(r["run"], r["missing_context"]) for r in srun if r["missing_context"] != "none"])
    print("runs with start == end or wall_s < 60:", [(r["run"], r["wall_s"]) for r in srun if r["started_at"] == r["ended_at"] or r["wall_s"] < 60])
    bad = [(r["run"], r["wall_s"], int((ts(r["ended_at"]) - ts(r["started_at"])).total_seconds()))
           for r in srun if abs(int((ts(r["ended_at"]) - ts(r["started_at"])).total_seconds()) - r["wall_s"]) > 1]
    print("runs whose wall_s differs from ended-started by > 1 s:", bad)
    corpus = "".join(p.read_text() for p in list((ROOT / "tasks").glob("T-*.md")) +
                     list((ROOT / "reviews").glob("*.md")) + list((ROOT / "evaluations").glob("*.md")) +
                     list((ROOT / "retrospectives").glob("*.md")) + [ROOT / "docs" / "decisions.md"])
    print("stage runs whose id appears in no task/review/evaluation/retrospective/decision file:",
          [r["run"] for r in srun if r["run"] not in corpus])
    for t in STAGE:
        tr = [x for x in trans if x["task"] == t]
        print(f"  {t}: runs {sum(1 for r in runs if r['task']==t)}, transitions {len(tr)}, "
              f"joins {sum(1 for j in joins if j['task']==t)}, join_defects {sum(1 for j in jdef if j['task']==t)}, "
              f"queue records {sum(1 for q in queue if q['task']==t)}, state {meta[t].get('state')}, tier {meta[t].get('tier')}")
    print("join_defects records:", [(j["task"], j["at"], j["acc_defects"]) for j in jdef if j["task"] in STAGE])
    print("transition chain of each stage task:")
    for t in STAGE:
        print("  ", t, " ".join(f"{x['from']}>{x['to']}@{x['at'][11:19]}" for x in trans if x["task"] == t))


# ------------------------------------------------------------------ tokens
def tokens():
    section("tokens")
    tot = sum(r["tokens"] for r in srun)
    over = [r for r in srun if r["task"] == "iter-2"]
    print(f"R-{LO}..R-{HI} tokens: {tot:,} over {len(srun)} runs")
    print("overhead runs:", [(r["run"], r["role"], r["tokens"], r.get("note", "")[:40]) for r in over])
    ov = sum(r["tokens"] for r in over)
    pre = [r for r in over if r["run"] in ("R-223", "R-224")]
    print(f"overhead {ov:,} ({ov/tot:.1%}); of which R-223+R-224 {sum(r['tokens'] for r in pre):,};"
          f" overhead without them {ov-sum(r['tokens'] for r in pre):,} ({(ov-sum(r['tokens'] for r in pre))/(tot-sum(r['tokens'] for r in pre)):.1%} of {tot-sum(r['tokens'] for r in pre):,})")
    sc = [r for r in over if r.get("note", "").startswith("spec check")]
    print(f"spec checks: {[(r['run'], r['tokens']) for r in sc]} sum {sum(r['tokens'] for r in sc):,}")
    prod = sum(tok(t) for t in PRODUCT)
    print(f"product tasks: {prod:,}")
    for t in PRODUCT:
        br = defaultdict(int)
        n = Counter()
        for r in runs:
            if r["task"] == t:
                br[r["role"]] += r["tokens"]
                n[r["role"]] += 1
        print(f"  {t} {tok(t):>9,} runs {sum(n.values())} " + ", ".join(f"{k} {v:,} ({n[k]})" for k, v in br.items()))
    byrole = defaultdict(int)
    for r in srun:
        byrole[r["role"]] += r["tokens"]
    print("by role (R-223..R-253):", {k: f"{v:,}" for k, v in byrole.items()})
    items = sum(ITEMS.values())
    print(f"per new verified item ({items}): product {prod/items:,.0f}; with overhead from R-225 "
          f"{(tot - sum(r['tokens'] for r in pre))/items:,.0f}; all R-223..R-253 {tot/items:,.0f}")
    # review rounds 1 vs later, confirmations
    for t in PRODUCT:
        rv = sorted((k for k in reviews if k[:5] == t))
        print(f"  {t} review runs:", [(k, reviews[k]["run"], run(reviews[k]["run"])["tokens"]) for k in rv])
    print("confirmation runs not in a review file:", [r["run"] for r in srun if r["role"] == "reviewer"
          and r["run"] not in {v["run"] for v in reviews.values()}])
    print(f"all run records in tasks.jsonl: {sum(r['tokens'] for r in runs):,} ({len(runs)} runs)")


# ------------------------------------------------------------------ budgets
def budgets():
    section("budgets and the Extend row")
    for t in PRODUCT:
        b = int(meta[t].get("budget_tokens", "0").split()[0])
        rv1 = run(reviews[f"{t}-r1"]["run"])
        at = tok(t, lambda r: r["ended_at"] <= rv1["ended_at"])
        aft = tok(t, lambda r: r["started_at"] >= rv1["ended_at"])
        print(f"  {t} ceiling {b:,} at r1 end {at:,} ({at/b:.1%}); after r1 {aft:,}; total {tok(t):,} ({tok(t)/b:.1%})")
    rows = re.findall(r"\| (D-\d+) \| [^|]*\| Extend (T-\d+) to (\d+)", decisions)
    print("Extend rows in decisions.md (all):", rows)
    t56 = [r for r in runs if r["task"] == "T-056"]
    cap = 1300000
    after = [r for r in t56 if r["started_at"] >= "2026-11-15T00:31:26Z"]
    print(f"T-056 runs after R-241: {[(r['run'], r['tokens']) for r in after]} sum {sum(r['tokens'] for r in after):,}"
          f" (D-159 estimate 220,000); total {tok('T-056'):,} = {tok('T-056')/cap:.1%} of cap, {tok('T-056')/1100000:.1%} of ceiling")
    t56r = [r for r in t56 if r["run"] in ("R-239", "R-240")]
    print(f"T-056 at D-158 (R-239+R-240): {sum(r['tokens'] for r in t56r):,} = {sum(r['tokens'] for r in t56r)/800000:.1%} of 800,000")


# ------------------------------------------------------------------ joins
def joins_s():
    section("joins")
    for j in joins:
        if j["task"] in STAGE:
            print(f"  {j['task']} {j['at'][5:19]} acc {j['acc_total']} failbase {j['acc_fail_base']} kind {j['base_fail_kind']}"
                  f" fail_at_join {j['acc_fail_first_join']} acc_defects {j['acc_defects']} guards {j.get('guard_total')}"
                  f" gate {j['gate_green']} result {j['result']} joined {j['joined_sha'][:7]}")
    per = Counter(j["task"] for j in joins if j["task"] in STAGE)
    print("joins per task:", dict(per), "total", sum(per.values()))
    print("results:", dict(Counter(j["result"] for j in joins if j["task"] in STAGE)))
    print("join_defects:", [(j["task"], j["at"], j["acc_defects"]) for j in jdef if j["task"] in STAGE])
    allj = [j for j in joins]
    print(f"all joins in tasks.jsonl: {len(allj)}; with acc_fail_first_join > 0: "
          f"{[(j['task'], j['at'][:10], j['acc_fail_first_join']) for j in allj if (j.get('acc_fail_first_join') or 0) > 0]}")


# ------------------------------------------------------------------ queue
def queue_s():
    section("queue")
    q2 = [q for q in queue if q["task"] in STAGE]
    for q in q2:
        print(f"  {q['task']} attempt {q['attempt']} {q['started_at'][5:19]}..{q['ended_at'][11:19]} {q['result']}"
              f" step {q['failed_step']} gate_s {q['gate_seconds']} xtask {q['gate_xtask']}")
    gs = [q["gate_seconds"] for q in q2 if q["result"] == "merged"]
    print("merged gate seconds:", gs, "median", statistics.median(gs))
    print("first attempt merged:", sum(1 for q in q2 if q["attempt"] == 1 and q["result"] == "merged"), "of", len({q["task"] for q in q2}))
    s1 = [q["gate_seconds"] for q in queue if q["task"] in
          ["T-039", "T-042", "T-019", "T-020", "T-021", "T-036", "T-023", "T-037", "T-038", "T-049"]]
    print("stage-1 merged gate seconds median", statistics.median(s1), "max", max(s1))


# ------------------------------------------------------------------ reviews
def reviews_s():
    section("reviews")
    tot = defaultdict(Counter)
    for k, v in sorted(reviews.items()):
        if k[:5] not in PRODUCT:
            continue
        f, m = nums(v["findings"]), nums(v["mutants"])
        rnd = int(v["round"])
        cost = run(v["run"])["tokens"]
        print(f"  {k} {v['verdict']:15} {f} doc_sentences {v.get('doc_sentences')} mutants {m} tokens {cost:,}")
        tot[rnd].update(f)
        tot[rnd]["doc_sentences"] += int(v.get("doc_sentences") or 0)
        tot[rnd]["tokens"] += cost
        if not (rnd == 3 or (k == "T-055-r3")):
            for kk, vv in m.items():
                tot[rnd]["m_" + kk] += vv
    for rnd, c in sorted(tot.items()):
        live = c["m_introduced"] - c["m_equivalent"]
        kr = f" kill by acc {c['m_killed_by_acceptance']}/{live} = {c['m_killed_by_acceptance']/live:.0%}, any {c['m_killed']}/{live}" if live else ""
        print(f"  round {rnd}: {dict(c)}{kr}")
    print("  (round-3 mutants excluded: T-055-r3 carries round 2's over, T-056-r3 zeros)")


def rework():
    section("rework after round 1 (runs starting at or after the round-1 review run ended)")
    s = 0
    for t in PRODUCT:
        e1 = run(reviews[f"{t}-r1"]["run"])["ended_at"]
        rr = [r for r in runs if r["task"] == t and r["started_at"] >= e1]
        a = sum(r["tokens"] for r in rr)
        s += a
        print(f"  {t} {a:>8,} ({a/tok(t):.1%}) {[(r['run'], r['role'], r['tokens']) for r in rr]}")
        pre = [r for r in runs if r["task"] == t and r["ended_at"] <= e1 and r["role"] != "reviewer"
               and r["run"] not in ("R-226", "R-227", "R-239", "R-240", "R-247", "R-248")]
        print(f"     rework before round 1 (after the first build runs): {[(r['run'], r['tokens']) for r in pre]}")
    allp = sum(tok(t) for t in PRODUCT)
    print(f"total post-round-1 {s:,} = {s/allp:.1%} of {allp:,}")


# ------------------------------------------------------------------ time
def time_s():
    section("time")
    a, b = ts(START), ts(END)
    print(f"window {START} .. {END}: {(b-a).total_seconds()/3600:.2f} h")
    st = [r for r in srun if ts(r["ended_at"]) > a and ts(r["started_at"]) < b]
    iv = sorted((max(ts(r["started_at"]), a), min(ts(r["ended_at"]), b)) for r in st)
    gaps, cur = [], a
    for s_, e in iv:
        if s_ > cur:
            gaps.append((cur, s_))
        cur = max(cur, e)
    if b > cur:
        gaps.append((cur, b))
    idle = sum((y - x).total_seconds() for x, y in gaps)
    print(f"time with no run active: {idle/3600:.2f} h ({idle/(b-a).total_seconds():.0%}) in {len(gaps)} gaps")
    print("gaps >= 10 min:", [(x.strftime('%H:%M:%S'), y.strftime('%H:%M:%S'), round((y-x).total_seconds()/60, 1)) for x, y in gaps if (y-x).total_seconds() >= 600])
    print("all gaps (min):", [round((y-x).total_seconds()/60, 1) for x, y in gaps])
    bl = [r for r in st if r["role"] in BUILDER]
    pts = sorted([(ts(r["started_at"]), 1) for r in bl] + [(ts(r["ended_at"]), -1) for r in bl], key=lambda x: (x[0], x[1]))
    c, mx, area, last = 0, 0, Counter(), None
    for t, d in pts:
        if last is not None:
            area[c] += (t - last).total_seconds()
        c += d
        mx = max(mx, c)
        last = t
    print("max builder runs open at once:", mx, "hours at each level:", {k: round(v / 3600, 2) for k, v in sorted(area.items())})
    for t in PRODUCT:
        rr = [r for r in runs if r["task"] == t]
        rd = [x["at"] for x in trans if x["task"] == t and x["to"] == "ready"][0]
        mg = [q["ended_at"] for q in queue if q["task"] == t and q["result"] == "merged"][-1]
        busy = sum(r["wall_s"] for r in rr)
        print(f"  {t} ready {rd[11:19]} merged {mg[5:19]} lead {(ts(mg)-ts(rd)).total_seconds()/3600:.2f} h; sum of run walls {busy/3600:.2f} h")
    for t in PRODUCT:
        rv1 = run(reviews[f"{t}-r1"]["run"])
        jn = [j for j in joins if j["task"] == t]
        print(f"  {t} join->r1 start gaps: first join {jn[0]['at'][11:19]} r1 start {rv1['started_at'][11:19]}")
    print("director spans between runs of the queue phase:")
    for t, a1, b1 in (("T-055", "2026-11-14T21:23:21Z", "2026-11-14T22:07:15Z"),
                      ("T-056", "2026-11-15T01:18:00Z", "2026-11-15T01:49:01Z"),
                      ("T-065", "2026-11-15T04:23:41Z", "2026-11-15T04:33:59Z")):
        print(f"  {t} round-2 review end {a1[11:19]} -> merge {b1[11:19]}: {(ts(b1)-ts(a1)).total_seconds()/60:.1f} min")


def lead():
    section("lead (ready -> merged) with stage-1 comparison")
    for t in PRODUCT + ["T-067"]:
        rd = [x["at"] for x in trans if x["task"] == t and x["to"] == "ready"][0]
        mg = [q["ended_at"] for q in queue if q["task"] == t and q["result"] == "merged"][-1]
        print(f"  {t} {(ts(mg)-ts(rd)).total_seconds()/3600:.2f} h")


def gates():
    section("gate attempts (sum of run gate_attempts) per task")
    print({t: sum(r["gate_attempts"] for r in runs if r["task"] == t) for t in PRODUCT})


def estimates():
    section("ADR-0007 estimates against spend")
    est = {"T-055 build": 810888, "T-056 build": 810888, "T-056 spec check": 326683, "evaluation A": 404836}
    act = {"T-055 build": tok("T-055"), "T-056+T-065 build": tok("T-056") + tok("T-065"),
           "spec checks (R-237, R-238, R-246)": sum(run(r)["tokens"] for r in ("R-237", "R-238", "R-246")),
           "evaluation A (R-253)": run("R-253")["tokens"]}
    print("estimate rows (ADR-0007):", est, "sum", f"{sum(est.values()):,}")
    print("actual:", {k: f"{v:,}" for k, v in act.items()}, "sum", f"{sum(act.values()):,}")
    print(f"ADR-0007 cut-1 cumulative 5,490,713 includes rows 1-4 (3,137,418) deferred by D-147; rows 5-6 + eval = {5490713-3137418:,}")
    plan = sum(run(r)["tokens"] for r in ("R-225",))
    print(f"plan run R-225 {plan:,}; methodology edit R-224 {run('R-224')['tokens']:,}; retro R-223 {run('R-223')['tokens']:,}")


def decisions_s():
    section("decisions")
    ids = sorted({int(x) for x in re.findall(r"^\| D-(\d+) \|", decisions, re.M)})
    print("rows D-143..:", [i for i in ids if i >= 143])
    print("ids missing in 1..max:", [i for i in range(1, max(ids) + 1) if i not in ids])
    cited = set()
    for p in list((ROOT / "tasks").glob("*.md")) + list((ROOT / "reviews").glob("*.md")) + \
            list((ROOT / "evaluations").glob("*.md")) + [ROOT / "STATE.md"] + list((ROOT / "handoffs").glob("*.md")) + \
            list((ROOT / "docs" / "adr").glob("*.md")):
        cited |= {int(x) for x in re.findall(r"D-(\d{3})", p.read_text())}
    print("cited D-NNN with no row:", sorted(c for c in cited if c not in ids))


def acks():
    section("acknowledgement of rulings in stage builder runs (word match in the run's log entry or review)")
    for r in srun:
        if r["task"] not in PRODUCT or r["role"] not in BUILDER:
            continue
        txt = meta[r["task"]]["_path"].read_text()
        m = re.search(r"^- [^\n]*" + re.escape(r["run"]) + r" [^\n]*(\n(?!- 20|<!--|###).*)*", txt, re.M)
        e = m.group(0) if m else ""
        for k, v in reviews.items():
            if v.get("run") == r["run"]:
                e += (ROOT / "reviews" / f"{k}.md").read_text()
        ack = bool(re.search(r"acknowledg|accept", e, re.I))
        print(f"  {r['run']} {r['task']} {r['role']:11} entry {'yes' if m else 'NO'} ack-word {'yes' if ack else 'no'}")


def speccheck():
    section("spec checks")
    for rid in ("R-237", "R-238", "R-246"):
        r = run(rid)
        print(f"  {rid} {r['tokens']:,} wall {r['wall_s']} s tool_calls {r['tool_calls']} note: {r['note'][:120]}")
    s1 = [2286780 / 14, 326683]
    print(f"stage-1 spec checks (retro-2s1): 2,286,780 over 14 runs, 326,683 per task; mean per run {s1[0]:,.0f}")


SECTIONS = {"completeness": completeness, "tokens": tokens, "budgets": budgets, "joins": joins_s,
            "queue": queue_s, "reviews": reviews_s, "rework": rework, "time": time_s, "lead": lead,
            "gates": gates, "estimates": estimates, "decisions": decisions_s, "acks": acks,
            "speccheck": speccheck}

if __name__ == "__main__":
    want = sys.argv[1:] or ["all"]
    for n, f in SECTIONS.items():
        if "all" in want or n in want:
            f()
