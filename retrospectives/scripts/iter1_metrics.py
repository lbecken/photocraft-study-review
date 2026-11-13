#!/usr/bin/env python3
"""Iteration-1 retrospective numbers (R-153).

Reads metrics/tasks.jsonl, metrics/queue.jsonl, reviews/*.md and tasks/spec-checks/*.md front
matter, tasks/*.md, docs/decisions.md and docs/scorecard.md, and prints every table the
retrospective cites. Read-only; python3 standard library only.

Usage (from the repository root):
    python3 -I retrospectives/scripts/iter1_metrics.py [section ...]
Sections: completeness, provenance, tokens, tiers, lead, queue, gates, joins, reviews, arms,
rework, specchecks, rulings, decisions, concurrency, idle, scorecard, projection, all (default).

Conventions fixed here (and stated in retrospectives/iter-1.md):
- Iteration-1 merged tasks are the twelve with `iteration: 1` in front matter and a `merged`
  queue record: T-006 T-008 T-010 T-011 T-013 T-014 T-015 T-016 T-017 T-018 T-032 T-035.
  T-012 (front matter `iteration: 0`, merged 02:56Z before methodology v1.1) is reported
  separately because decision D-089 lists it among the thirteen.
- Two spec-check runs were never booked (D-055): R-067 (T-016 and T-017, 294,896 tokens) and
  R-069 (T-015 re-check and T-032, 59,214 tokens). They are added by hand where said, R-067
  split evenly between T-016 and T-017, R-069 booked wholly to T-032 (its T-015 part is a
  re-check of a task that was already over budget; the split is not recorded).
"""
import json
import pathlib
import re
import statistics
import sys
from collections import defaultdict, Counter
from datetime import datetime, timedelta

ROOT = pathlib.Path(__file__).resolve().parents[2]
IT1 = ["T-006", "T-008", "T-010", "T-011", "T-013", "T-014", "T-015", "T-016", "T-017",
       "T-018", "T-032", "T-035"]
UNBOOKED = {"R-067": (294896, {"T-016": 147448, "T-017": 147448}),
            "R-069": (59214, {"T-032": 59214})}
BUILDER_ROLES = ("implementer", "test-author", "reviewer")


def load_jsonl(p):
    return [json.loads(l) for l in (ROOT / p).read_text().splitlines() if l.strip()]


def ts(s):
    return datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ")


def front(path):
    txt = path.read_text()
    m = re.match(r"---\n(.*?)\n---", txt, re.S)
    out = {}
    if not m:
        return out
    for line in m.group(1).splitlines():
        if ":" in line and not line.startswith(" "):
            k, v = line.split(":", 1)
            out[k.strip()] = v.split("#")[0].strip()
    return out


def pct(xs, q):
    xs = sorted(xs)
    if not xs:
        return None
    k = (len(xs) - 1) * q
    lo, hi = int(k), min(int(k) + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)


def desc(xs):
    if not xs:
        return "n=0"
    return (f"n={len(xs)} median {statistics.median(xs):,.2f} p75 {pct(xs, .75):,.2f} "
            f"min {min(xs):,.2f} max {max(xs):,.2f} sum {sum(xs):,.2f}")


recs = load_jsonl("metrics/tasks.jsonl")
queue = load_jsonl("metrics/queue.jsonl")
runs = [r for r in recs if r["kind"] == "run"]
runs1 = [r for r in runs if r["iteration"] == 1]
trans = [r for r in recs if r["kind"] == "transition"]
joins = [r for r in recs if r["kind"] == "join"]
reverts = [r for r in recs if r["kind"] == "revert_check"]
meta = {}
for p in sorted((ROOT / "tasks").glob("T-*.md")):
    f = front(p)
    f["_path"] = p
    meta[f["id"]] = f
reviews = {p.stem: front(p) for p in sorted((ROOT / "reviews").glob("T-*-r*.md"))}
reviews1 = {k: v for k, v in reviews.items() if k[:5] in IT1}
specchecks = {p.stem: front(p) for p in sorted((ROOT / "tasks" / "spec-checks").glob("T-*.md"))}
decisions = (ROOT / "docs" / "decisions.md").read_text()
merged_at = {q["task"]: q["ended_at"] for q in queue if q["result"] == "merged"}
tier = {t: m.get("tier", "?").split()[0] for t, m in meta.items()}


def section(name):
    print(f"\n## {name}")


def nums(s):
    return {k: int(v) for k, v in re.findall(r"(\w+): (\d+)", s or "")}


def task_tokens(t, hand=False):
    s = sum(r["tokens"] for r in runs if r["task"] == t)
    if hand:
        s += sum(v[1].get(t, 0) for v in UNBOOKED.values())
    return s


def extend_rows():
    out = {}
    for m in re.finditer(r"\| (D-\d+) \| [^|]*\| Extend (T-\d+) to (\d+)", decisions):
        out[m.group(2)] = (m.group(1), int(m.group(3)))
    return out


# ---------------------------------------------------------------- completeness
def completeness():
    section("completeness")
    print("records, all iterations:", dict(Counter(r["kind"] for r in recs)), "queue:", len(queue))
    print("records booked to iteration 1:",
          dict(Counter(r["kind"] for r in recs if r.get("iteration") == 1)))
    print("join records after 03:00Z on 2026-11-13 (iteration 1):",
          sum(1 for j in joins if j["at"] >= "2026-11-13T03:00:00Z"),
          "revert checks:", sum(1 for j in reverts if j["at"] >= "2026-11-13T03:00:00Z"))
    ids = sorted({int(r["run"][2:]) for r in runs})
    lo = 49
    missing = [f"R-{n:03d}" for n in range(lo, max(ids) + 1) if n not in ids]
    print(f"run ids R-{lo:03d}..R-{max(ids):03d} with no run record ({len(missing)}):", missing)
    distinct1 = {r["run"] for r in runs1}
    print("iteration-1 run records:", len(runs1), "distinct run ids:", len(distinct1))
    split = defaultdict(list)
    for r in runs1:
        split[r["run"]].append(r["task"])
    print("run ids split over several task records:", {k: v for k, v in split.items() if len(v) > 1})
    print("iteration-1 runs by role:", dict(Counter(r["role"] for r in runs1)))
    print("iteration-1 runs by outcome:", dict(Counter(r["outcome"] for r in runs1)))
    print("blocked runs:", [(r["run"], r["task"], r["role"], r["ended_at"][11:16], r["tokens"])
                            for r in runs1 if r["outcome"] == "blocked"])
    print("role: director run records (all iterations):", sum(r["role"] == "director" for r in runs))
    # log entries
    corpus = ""
    for p in list((ROOT / "tasks").glob("T-*.md")) + list((ROOT / "reviews").glob("*.md")) + \
            list((ROOT / "tasks" / "spec-checks").glob("*.md")):
        corpus += p.read_text()
    nolog = sorted(r for r in distinct1 if not re.search(re.escape(r) + r"\b", corpus))
    print("iteration-1 run ids with no mention in tasks/, tasks/spec-checks/ or reviews/:", nolog)
    # transitions
    skip = [(t["task"], t["at"][11:]) for t in trans
            if t["from"] == "in_progress" and t["to"] == "tests_joined" and t["at"] > "2026-11-13T03"]
    print("iteration-1 transitions in_progress->tests_joined (skip self_verified):", len(skip), skip)
    t013 = merged_at["T-013"]
    print("  of which after T-013 merged (", t013, "):", sum(1 for s in skip if "2026-11-13T" + s[1] > t013))
    ready = {}
    for t in trans:
        if t["to"] == "ready":
            ready.setdefault(t["task"], t["at"])
    early = [(r["run"], r["task"], r["role"], r["started_at"][11:], ready.get(r["task"], "")[11:])
             for r in runs1 if r["role"] in ("implementer", "test-author") and r["task"] in ready
             and r["started_at"] < ready[r["task"]]]
    print("builder runs starting before their task's ready transition:", early)
    for k, fm in sorted(reviews1.items()):
        miss = [x for x in ("findings", "mutants", "reviewed_sha", "verdict") if x not in fm]
        if miss:
            print("review missing front-matter fields:", k, miss)
    print("iteration-1 review files:", len(reviews1), "all have findings and mutants:",
          all("findings" in v and "mutants" in v for v in reviews1.values()))


def provenance():
    section("provenance of iteration-1 token and time figures (from run notes)")
    pats = {
        "difference of running totals (delta / minus / 'agent total')": r"delta over|minus|agent total",
        "last context size from a transcript": r"last context size",
        "start time entered by the director": r"start (time )?entered by the director",
        "times from the agent transcript": r"times from the agent transcript",
        "even split of one run over several task records": r"split evenly",
        "segments folded into one record": r"segments",
    }
    for label, p in pats.items():
        sel = sorted({r["run"] for r in runs1 if re.search(p, r.get("note", ""))})
        print(f"{label}: {len(sel)} {sel}")
    rnd = [(r["run"], r["tokens"]) for r in runs1 if r["tokens"] % 1000 == 0]
    print("iteration-1 run records whose tokens are a multiple of 1,000:", rnd)
    ph = [(r["run"], r["tool_calls"]) for r in runs1 if "placeholder" in r.get("note", "")]
    print("records whose note says a field is a placeholder:", ph)
    whole = [r["run"] for r in runs1 if r["started_at"].endswith(":00Z") and r["ended_at"].endswith(":00Z")]
    print("iteration-1 runs with both timestamps on a whole minute:", len(whole), whole)


# ---------------------------------------------------------------- tokens
def tokens():
    section("tokens per iteration-1 merged task (booked; + hand = with unbooked R-067/R-069)")
    ext = extend_rows()
    roles = ["planner", "implementer", "test-author", "reviewer", "evaluator", "retrospective-analyst"]
    print("task   tier    budget    " + " ".join(f"{x[:11]:>11}" for x in roles) +
          "      booked   x budget   +hand   extend row")
    for t in IT1 + ["T-012"]:
        per = defaultdict(int)
        for r in runs:
            if r["task"] == t:
                per[r["role"]] += r["tokens"]
        b = int(meta[t]["budget_tokens"].split()[0])
        tot = sum(per.values())
        e = ext.get(t, ("-", "-"))
        print(f"{t}  {tier[t]:<6} {b:>8}   " + " ".join(f"{per[x]:>11}" for x in roles) +
              f"  {tot:>10}   {tot / b:>6.2f}  {task_tokens(t, True):>9}   {e[0]} {e[1]}")
    over = [t for t in IT1 if task_tokens(t) > int(meta[t]["budget_tokens"].split()[0])]
    print("iteration-1 merged tasks over budget (booked):", len(over), over,
          "without an Extend row:", [t for t in over if t not in ext])
    over_h = [t for t in IT1 if task_tokens(t, True) > int(meta[t]["budget_tokens"].split()[0])]
    print("... with unbooked spec checks added by hand:", over_h)
    s_it1 = sum(r["tokens"] for r in runs1)
    print("all iteration-1 run records:", f"{s_it1:,}", "+ unbooked R-067/R-069:",
          f"{s_it1 + sum(v[0] for v in UNBOOKED.values()):,}")
    by = defaultdict(int)
    for r in runs1:
        cat = ("iter-1 overhead (planner)" if r["task"] == "iter-1" else
               "merged task" if r["task"] in IT1 else "draft task (planner shares)")
        by[cat] += r["tokens"]
    print("iteration-1 tokens by where they are booked:", dict(by))
    roles_all = defaultdict(int)
    for r in runs1:
        roles_all[r["role"]] += r["tokens"]
    print("iteration-1 tokens by role:", dict(roles_all))
    spec = [r for r in runs1 if r["role"] == "reviewer" and re.search(r"spec (re-)?check", r.get("note", ""))]
    print("booked spec-check runs:", [(r["run"], r["task"], r["tokens"]) for r in spec],
          "sum", sum(r["tokens"] for r in spec), "+ unbooked", sum(v[0] for v in UNBOOKED.values()))


def tiers():
    section("tokens per merged task by tier (iteration 1)")
    for tr in ("high", "medium", "low"):
        ts_ = [t for t in IT1 if tier[t] == tr]
        print(tr, ts_)
        print("  booked:", desc([task_tokens(t) for t in ts_]))
        print("  +hand :", desc([task_tokens(t, True) for t in ts_]))
    cats = {
        "product tasks with scorecard items (T-008 T-010 T-011 T-018)": ["T-008", "T-010", "T-011", "T-018"],
        "contract/library tasks with no scorecard item (T-015 T-016 T-017 T-032)": ["T-015", "T-016", "T-017", "T-032"],
        "process, tooling and documentation (T-006 T-013 T-014 T-035)": ["T-006", "T-013", "T-014", "T-035"],
    }
    total = sum(r["tokens"] for r in runs1) + sum(v[0] for v in UNBOOKED.values())
    for k, v in cats.items():
        s = sum(task_tokens(t, True) for t in v)
        print(f"{k}: {s:,} ({100 * s / total:.1f} % of {total:,})")
    ov = sum(r["tokens"] for r in runs1 if r["task"] == "iter-1")
    dr = sum(r["tokens"] for r in runs1 if r["task"] not in IT1 and r["task"] != "iter-1")
    print(f"iter-1 planner overhead: {ov:,} ({100 * ov / total:.1f} %); planner shares booked to "
          f"draft tasks: {dr:,} ({100 * dr / total:.1f} %)")


# ---------------------------------------------------------------- lead time
def lead():
    section("lead time and dispatch-to-merge wall time (hours), iteration-1 merged tasks")
    rows = []
    for t in IT1:
        m = ts(merged_at[t])
        rd = min((x["at"] for x in trans if x["task"] == t and x["to"] == "ready"), default=None)
        fb = min((r["started_at"] for r in runs if r["task"] == t and r["role"] in ("implementer", "test-author")),
                 default=None)
        h = lambda a: round((m - ts(a)).total_seconds() / 3600, 2) if a else None
        rows.append((t, tier[t], h(rd), h(fb)))
        print(t, tier[t], "ready->merged", h(rd), "first builder run->merged", h(fb))
    for tr in ("high", "medium", "low"):
        print(tr, "ready->merged:", desc([r[2] for r in rows if r[1] == tr]))
        print(tr, "first builder->merged:", desc([r[3] for r in rows if r[1] == tr and r[3] is not None]))


def queue_():
    section("queue")
    q1 = [q for q in queue if q["task"] in IT1]
    for q in q1:
        print(q["task"], "attempt", q["attempt"], q["result"], q["failed_step"], "gate_s", q["gate_seconds"],
              q["started_at"][11:])
    tasks = sorted({q["task"] for q in q1})
    first = [t for t in tasks if any(q["task"] == t and q["attempt"] == 1 and q["result"] == "merged" for q in q1)]
    print("first-pass by record:", len(first), "/", len(tasks))
    print("gate seconds per queue run:", desc([q["gate_seconds"] for q in q1 if q["result"] == "merged"]))
    print("queue precondition refusals are not recorded in queue.jsonl; decisions name two: "
          "T-015 (D-050, abbreviated reviewed_sha), T-032 (D-058, approval row)")


def gates():
    section("gate attempts (sum of run gate_attempts) and runs per task")
    for t in IT1:
        rr = [r for r in runs if r["task"] == t]
        print(t, tier[t], "runs", len({r["run"] for r in rr}),
              "by role", dict(Counter(r["role"] for r in rr)),
              "gate attempts", sum(r["gate_attempts"] for r in rr),
              "joins", sum(j["task"] == t for j in joins),
              "review rounds", max([int(k.split("-r")[1]) for k in reviews if k.startswith(t + "-r")] or [0]))
    print("gate attempts per task:", desc([sum(r["gate_attempts"] for r in runs if r["task"] == t) for t in IT1]))


def joins_():
    section("joins (iteration 1)")
    j1 = [j for j in joins if j["task"] in IT1]
    for j in j1:
        print(j["task"], j["at"][11:], j["result"], "acc", j["acc_total"], "fail_base", j["acc_fail_base"],
              j["base_fail_kind"], "fail_first_join", j["acc_fail_first_join"], "acc_defects", j["acc_defects"],
              "impl_green", j["impl_tests_green"], "gate", j.get("gate_green"))
    print("joins:", len(j1), "by result:", dict(Counter(j["result"] for j in j1)))
    print("joins with acc_fail_first_join > 0:", [(j["task"], j["at"][11:], j["acc_fail_first_join"], j["acc_defects"])
                                                 for j in j1 if j["acc_fail_first_join"]])
    print("joins with acc_defects set:", [j for j in j1 if j["acc_defects"] is not None])
    print("join_defects records:", sum(1 for r in recs if r["kind"] == "join_defects"))
    print("revert checks:", [(r["task"], r["at"][11:], r["result_kind"], r["acc_fail"], r["acc_total"])
                             for r in reverts if r["task"] in IT1])


# ---------------------------------------------------------------- reviews
def reviews_():
    section("review rounds (iteration 1): findings and mutants from front matter; tokens from the run record")
    run_tok = {}
    for r in runs:
        run_tok[r["run"]] = run_tok.get(r["run"], 0) + r["tokens"]
    rows = []
    for k, fm in sorted(reviews1.items()):
        f = nums(fm.get("findings"))
        mu = nums(fm.get("mutants"))
        rid = fm["run"].split()[0]
        tok = run_tok.get(rid)
        rnd = int(k.split("-r")[1])
        rows.append((k, rnd, fm.get("tier"), fm.get("arm"), fm["verdict"], tok, f, mu))
        print(f"{k:<10} {rid} {fm.get('tier'):<6} {fm.get('arm'):<11} {fm['verdict']:<16} tokens {tok!s:>7} {f} {mu}")
    print("T-032-r2 also covers addendum run R-106 (14,844 tokens)")
    for rnd in (1, 2, 3):
        sel = [r for r in rows if r[1] == rnd]
        s = defaultdict(int)
        m = defaultdict(int)
        for r in sel:
            for kk, vv in r[6].items():
                s[kk] += vv
            for kk, vv in r[7].items():
                m[kk] += vv
        live = m["introduced"] - m["equivalent"]
        kr = f"{m['killed_by_acceptance']}/{live} = {100 * m['killed_by_acceptance'] / live:.0f} %" if live else "n/a"
        print(f"round {rnd}: n={len(sel)} findings {dict(s)} mutants {dict(m)} acceptance kill rate {kr} "
              f"tokens {[r[5] for r in sel]} sum {sum(r[5] for r in sel)}")
    for tr in ("high", "medium"):
        sel = [r for r in rows if r[2] == tr]
        s = defaultdict(int)
        m = defaultdict(int)
        for r in sel:
            for kk, vv in r[6].items():
                s[kk] += vv
            for kk, vv in r[7].items():
                m[kk] += vv
        live = m["introduced"] - m["equivalent"]
        print(f"tier {tr}: reviews {len(sel)} findings {dict(s)} mutants {dict(m)} "
              f"acceptance kill rate {m['killed_by_acceptance']}/{live}")
    tasks = sorted({r[0][:5] for r in rows})
    print("rounds per reviewed task:", {t: max(r[1] for r in rows if r[0][:5] == t) for t in tasks})
    print("round-2+ reviews whose verdict is request-changes:",
          [r[0] for r in rows if r[1] >= 2 and r[4] == "request-changes"])


def arms():
    section("medium-tier arms")
    for t in ("T-010", "T-011", "T-018"):
        rr = [r for r in runs if r["task"] == t]
        per = defaultdict(int)
        for r in rr:
            per[r["role"]] += r["tokens"]
        rv = {k: v for k, v in reviews.items() if k.startswith(t)}
        f = defaultdict(int)
        for k, fm in rv.items():
            for kk, vv in nums(fm.get("findings")).items():
                f[kk] += vv
        r1 = nums(reviews[t + "-r1"].get("mutants"))
        allm = defaultdict(int)
        for fm in rv.values():
            for kk, vv in nums(fm.get("mutants")).items():
                allm[kk] += vv
        jj = [j for j in joins if j["task"] == t]
        print(t, "arm", reviews[t + "-r1"].get("arm"), "tokens", sum(per.values()), dict(per),
              "| rounds", len(rv), "findings", dict(f),
              "| r1 mutants", r1, "| all-round survived", allm["survived"],
              "| joins", len(jj), "acc_fail_first_join", [j["acc_fail_first_join"] for j in jj],
              "acc_defects", [j["acc_defects"] for j in jj],
              "| scorecard items", meta[t].get("scorecard_items"))


def rework():
    section("tokens booked to runs that ended after the round-1 review run ended (post-r1 rework)")
    total = 0
    for k, fm in sorted(reviews1.items()):
        if not k.endswith("-r1"):
            continue
        t = k[:5]
        end = max(r["ended_at"] for r in runs if r["run"] == fm["run"].split()[0])
        after = defaultdict(int)
        for r in runs:
            if r["task"] == t and r["ended_at"] > end:
                after[r["role"]] += r["tokens"]
        s = sum(after.values())
        total += s
        print(t, "after r1:", s, dict(after), f"= {100 * s / task_tokens(t):.0f} % of the task")
    print("total post-r1 rework:", total, f"of merged-task tokens {sum(task_tokens(t) for t in IT1)}")


def specchecks_():
    section("pre-dispatch spec checks")
    tok = {r["run"]: r["tokens"] for r in runs}
    tok.update({k: v[0] for k, v in UNBOOKED.items()})
    for k, fm in specchecks.items():
        print(k, "run", fm.get("run"), "tokens", tok.get(fm.get("run")), "verdict", fm.get("verdict"),
              "findings", fm.get("findings"), "rechecks", fm.get("rechecks", "-"))
    print("R-115 (T-008 re-check)", tok.get("R-115"), "| R-069 also re-checked T-015 (spec-checks/T-015.md 'Round 2')")
    allruns = ["R-062", "R-067", "R-069", "R-101", "R-115"]
    print("spec-check tokens, all five runs:", sum(tok[r] for r in allruns))


def rulings():
    section("director rulings per task file (section before '## Log')")
    pat_a = re.compile(r"^\s*- \*{0,2}([a-z]\d{1,2})\b", re.M)          # q1, v4, s8, w1, x7, ...
    pat_b = re.compile(r"^  ([a-z]\d?)\. ", re.M)                        # T-013 a.-q., T-032 r1.-r6.
    pat_c = re.compile(r"^\s*- (D-\d+) \(", re.M)                         # planning answers D-1 ...
    pat_d = re.compile(r"^\s*- Director\b", re.M)                         # unnumbered director lines
    pat_e = re.compile(r"^  (\d)\. ", re.M)                               # T-015 D-046 points 1-3
    for t in IT1:
        body = meta[t]["_path"].read_text().split("\n## Log")[0]
        a = pat_a.findall(body)
        b = pat_b.findall(body)
        c = pat_c.findall(body)
        d = pat_d.findall(body)
        e = pat_e.findall(body) if t == "T-015" else []
        print(f"{t}: lettered {len(a) + len(b)} {a + b} | planning answers {len(c)} {c} | "
              f"unnumbered 'Director' lines {len(d)} | numbered points {e}")


def decisions_():
    section("decision rows from D-036 on")
    rows = re.findall(r"^\| (D-\d+) \|[^|]*\|(.*)$", decisions, re.M)
    rows = [(i, txt) for i, txt in rows if int(i[2:]) >= 36]
    print("rows:", len(rows))
    rul = [(i, bool(re.search(r"`[0-9a-f]{7,40}`", txt))) for i, txt in rows if re.search(r"[Rr]uling", txt)]
    print("rows that mention a ruling:", len(rul), "of which cite any commit id:", [i for i, s in rul if s])
    appr = [(i, txt.strip()[:40]) for i, txt in rows if re.search(r"Approve T-\d+", txt)]
    print("rows containing 'Approve T-NNN':", appr)
    cor = [i for i, txt in rows if re.search(r"(correct|replace|revers|withdraw|mistake|wrong|slip|miss)", txt, re.I)]
    print("rows with a correction word (correct/replace/revers/withdraw/mistake/wrong/slip/miss):", len(cor), cor)
    cnt = re.findall(r"(\w+) director ruling", decisions)
    print("ordinal counts of corrected director rulings named in rows:", cnt)


# ---------------------------------------------------------------- time
def intervals(roles=None, exclude=("R-056",)):
    seen = {}
    for r in runs1:
        if r["run"] in exclude or (roles and r["role"] not in roles):
            continue
        seen[r["run"]] = (ts(r["started_at"]), ts(r["ended_at"]), r["role"], r["task"])
    return seen


def concurrency():
    section("concurrency from recorded start/end times (R-056 excluded: five segments with idle gaps)")
    for label, roles in (("all roles", None), ("implementer/test-author/reviewer", BUILDER_ROLES),
                         ("implementer/test-author", ("implementer", "test-author"))):
        iv = intervals(roles)
        events = sorted({v[0] for v in iv.values()})
        best = (0, None, [])
        over = []
        hist = Counter()
        for t in events:
            live = sorted(k for k, v in iv.items() if v[0] <= t < v[1])
            hist[len(live)] += 1
            if len(live) > best[0]:
                best = (len(live), t, live)
            if len(live) > 3:
                over.append((t.strftime("%H:%M:%S"), live))
        busy = sum((v[1] - v[0]).total_seconds() for v in iv.values())
        print(f"{label}: runs {len(iv)}, max live {best[0]} at {best[1]} {best[2]}; "
              f"sum of run wall {busy / 3600:.2f} h")
        print("   instants (run starts) with > 3 live:", len(over), over[:8])
    # time-weighted average
    iv = intervals(BUILDER_ROLES)
    start = min(v[0] for v in iv.values())
    end = max(v[1] for v in iv.values())
    t = start
    weighted = 0
    step = timedelta(seconds=10)
    zero = 0
    while t < end:
        n = sum(1 for v in iv.values() if v[0] <= t < v[1])
        weighted += n
        zero += n == 0
        t += step
    steps = (end - start) / step
    print(f"builder-role runs {start:%H:%M}-{end:%H:%M}: time-weighted mean live {weighted / steps:.2f}; "
          f"share of time with none live {100 * zero / steps:.0f} %")


def idle():
    section("periods of 20 min or more with no iteration-1 run live (R-056 included)")
    iv = intervals(None, exclude=())
    pts = sorted(iv.values())
    cur_end = pts[0][1]
    gaps = []
    for s, e, *_ in pts[1:]:
        if s > cur_end and (s - cur_end) >= timedelta(minutes=20):
            gaps.append((cur_end, s))
        cur_end = max(cur_end, e)
    for a, b in gaps:
        print(f"{a:%H:%M:%S} -> {b:%H:%M:%S}  {(b - a).total_seconds() / 3600:.2f} h")
    print("note: R-056 (planner, 03:30-06:31) is one record over five segments; its span hides the 04:15-06:10 stop")
    first = min(ts(r["started_at"]) for r in runs1)
    last = ts(merged_at["T-018"])
    d035 = ts("2026-11-13T03:11:05Z")   # commit e1f1a2b, D-035 (git log)
    print(f"span first iteration-1 run {first:%H:%M:%S} -> T-018 merged {last:%H:%M:%S}: "
          f"{(last - first).total_seconds() / 3600:.2f} h; from D-035 commit: {(last - d035).total_seconds() / 3600:.2f} h")
    print(f"merged tasks per hour (12 iteration-1 tasks): {12 / ((last - d035).total_seconds() / 3600):.2f}")


# ---------------------------------------------------------------- scorecard and projection
def scorecard():
    section("scorecard (docs/scorecard.md on main)")
    txt = (ROOT / "docs" / "scorecard.md").read_text()
    st = re.findall(r"^\| `([^`]+)` \|.*?\| (verified|implemented|planned|failing) \|", txt, re.M)
    c = Counter(s for _, s in st)
    print("items", len(st), dict(c), f"verified share {100 * c['verified'] / len(st):.1f} %",
          f"60 % needs {-(-60 * len(st) // 100)} items")
    status = dict(st)
    claimed = defaultdict(list)
    for t, m in meta.items():
        for it in re.findall(r"[a-z]+\.[a-z0-9.-]+", m.get("scorecard_items", "")):
            claimed[it].append(t)
    merged1 = [it for t in IT1 for it in re.findall(r"[a-z]+\.[a-z0-9.-]+", meta[t].get("scorecard_items", ""))]
    print("items claimed by iteration-1 merged tasks:", merged1, [status.get(i) for i in merged1])
    drafts = [t for t, m in meta.items() if m.get("state", "").startswith("draft")]
    print("draft tasks:", len(drafts), drafts)
    planned = [i for i, s in status.items() if s == "planned"]
    by_draft = [i for i in planned if any(meta[t].get("state", "").startswith("draft") for t in claimed.get(i, []))]
    print("planned items claimed by a draft:", len(by_draft))
    print("planned items claimed by no task:", [i for i in planned if i not in by_draft])


def projection():
    section("eighteen drafts against iteration-1 costs")
    drafts = sorted(t for t, m in meta.items() if m.get("state", "").startswith("draft"))
    bt = Counter(tier[t] for t in drafts)
    print("drafts by tier:", dict(bt), "sum of budgets:", sum(int(meta[t]["budget_tokens"].split()[0]) for t in drafts))
    for tr in ("high", "medium"):
        xs = [task_tokens(t, True) for t in IT1 if tier[t] == tr]
        print(tr, "iteration-1 merged (with hand-added checks): median", statistics.median(xs),
              "mean", round(statistics.mean(xs)), "n", len(xs))
    hi = [task_tokens(t, True) for t in IT1 if tier[t] == "high"]
    me = [task_tokens(t, True) for t in IT1 if tier[t] == "medium"]
    for lab, f in (("median", statistics.median), ("mean", statistics.mean)):
        est = bt["high"] * f(hi) + bt["medium"] * f(me)
        print(f"18 drafts at the iteration-1 {lab}: {est:,.0f} tokens")
    total1 = sum(r["tokens"] for r in runs1) + sum(v[0] for v in UNBOOKED.values())
    tasks1 = sum(task_tokens(t, True) for t in IT1)
    print(f"iteration-1 total {total1:,}; on merged tasks {tasks1:,}; overhead factor {total1 / tasks1:.2f}")


def budgets():
    section("budget recalibration inputs: builder+review tokens (planner shares and spec checks removed)")
    spec_runs = {"R-062", "R-101", "R-115"}
    rows = {}
    for t in IT1:
        b = sum(r["tokens"] for r in runs if r["task"] == t and r["role"] != "planner" and r["run"] not in spec_runs)
        rows[t] = b
        print(t, tier[t], "booked", task_tokens(t), "builder+review", b,
              "ratio to v1.1 ceiling", round(b / int(meta[t]["budget_tokens"].split()[0]), 2))
    for tr in ("high", "medium", "low"):
        xs = [rows[t] for t in IT1 if tier[t] == tr and rows[t] > 0]
        p = pct(xs, .75)
        print(tr, desc(xs), "p75 rounded up to 50k:", -(-int(p) // 50000) * 50000)
    blocked = [r for r in runs1 if r["outcome"] == "blocked"]
    print("blocked runs tokens:", sum(r["tokens"] for r in blocked),
          "of which usage-limit stops (R-122 excluded):", sum(r["tokens"] for r in blocked if r["run"] != "R-122"))
    relaunch = ["R-091", "R-092", "R-093", "R-132", "R-133", "R-134"]
    print("fresh agents that continued blocked work:", relaunch,
          sum(r["tokens"] for r in runs1 if r["run"] in relaunch))
    print("T-010 pre-code stop (R-107 111,572 + R-108 114,823 from the run notes):", 111572 + 114823)


SECTIONS = {"completeness": completeness, "provenance": provenance, "tokens": tokens, "tiers": tiers,
            "lead": lead, "queue": queue_, "gates": gates, "joins": joins_, "reviews": reviews_,
            "arms": arms, "rework": rework, "specchecks": specchecks_, "rulings": rulings,
            "decisions": decisions_, "concurrency": concurrency, "idle": idle,
            "scorecard": scorecard, "projection": projection, "budgets": budgets}

if __name__ == "__main__":
    want = sys.argv[1:] or ["all"]
    for name, fn in SECTIONS.items():
        if "all" in want or name in want:
            fn()
