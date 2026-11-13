#!/usr/bin/env python3
"""Iteration-0 retrospective numbers (R-052).

Reads metrics/tasks.jsonl, metrics/queue.jsonl, reviews/*.md front matter and tasks/*.md
front matter, and prints every table the retrospective cites. Read-only.

Usage (from the repository root):  python3 -I retrospectives/scripts/iter0_metrics.py [section]
Sections: completeness, tokens, roles, rounds, reviews, joins, lead, wall, budgets, all (default).
"""
import json
import pathlib
import re
import statistics
import sys
from collections import defaultdict
from datetime import datetime

ROOT = pathlib.Path(__file__).resolve().parents[2]
FEATURE = ["T-001", "T-002", "T-003", "T-004", "T-005"]


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


tasks = load_jsonl("metrics/tasks.jsonl")
queue = load_jsonl("metrics/queue.jsonl")
runs = [r for r in tasks if r["kind"] == "run"]
trans = [r for r in tasks if r["kind"] == "transition"]
joins = [r for r in tasks if r["kind"] == "join"]
reverts = [r for r in tasks if r["kind"] == "revert_check"]
meta = {}
for p in sorted((ROOT / "tasks").glob("T-*.md")):
    f = front(p)
    meta[f["id"]] = f
reviews = {}
for p in sorted((ROOT / "reviews").glob("T-*-r*.md")):
    reviews[p.stem] = front(p)


def pct(xs, q):
    xs = sorted(xs)
    if not xs:
        return None
    k = (len(xs) - 1) * q
    lo, hi = int(k), min(int(k) + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (k - lo)


def section(name):
    print(f"\n## {name}")


def completeness():
    section("completeness")
    print("records:", {k: sum(1 for r in tasks if r['kind'] == k) for k in
                       ("run", "transition", "join", "revert_check")}, "queue:", len(queue))
    ids = {r["run"] for r in runs}
    print("distinct run ids in run records:", len(ids), "run records:", len(runs))
    nums = sorted(int(i[2:]) for i in ids)
    missing = [f"R-{n:03d}" for n in range(1, max(nums) + 1) if n not in nums]
    print("run ids R-001..R-%03d with no run record:" % max(nums), missing)
    shared = defaultdict(list)
    for r in runs:
        shared[r["run"]].append(r["task"])
    print("run ids split over several task records:",
          {k: v for k, v in shared.items() if len(v) > 1})
    print("runs with tokens == 0:", [r["run"] for r in runs if r["tokens"] == 0])
    print("runs whose tokens are a delta over an earlier resumed run:",
          [r["run"] for r in runs if "delta" in r.get("note", "")])
    rounded = [r["run"] for r in runs if r["started_at"].endswith(":00Z") and r["ended_at"].endswith(":00Z")]
    print("runs with both timestamps on a whole minute (likely hand-entered):", len(rounded), rounded)
    tr_round = [f'{t["task"]}:{t["from"]}->{t["to"]}@{t["at"]}' for t in trans if t["at"].endswith("0:00Z") or t["at"].endswith("5:00Z")]
    print("transitions on a :x0:00/:x5:00 mark:", len(tr_round))
    # transition chain consistency
    print("transition order problems (a transition whose 'from' state was entered later):")
    by = defaultdict(list)
    for t in trans:
        by[t["task"]].append(t)
    for task, ts_ in by.items():
        entered = {}
        for t in ts_:
            if t["from"] in entered and ts(entered[t["from"]]) > ts(t["at"]):
                print("  ", task, t["from"], "entered", entered[t["from"]], "but left", t["at"])
            if t["from"] != "draft" and t["from"] not in entered:
                print("  ", task, "leaves", t["from"], "with no transition into it")
            entered[t["to"]] = t["at"]
    # states recorded vs task front matter
    for task, m in meta.items():
        last = by[task][-1]["to"] if by.get(task) else "(none)"
        if last != m.get("state") and not (last == "(none)" and m.get("state") == "draft"):
            print("  front-matter state", task, m.get("state"), "vs last transition", last)
    print("tasks with no transition records:", [t for t in meta if t not in by])
    # missing ready transition for merged: merged tasks
    for r in reviews:
        fm = reviews[r]
        if "findings" not in fm:
            print("review without findings:", r)
        if "mutants" not in fm:
            print("review without mutants in front matter:", r)


def task_tokens(task, exclude_zero=False):
    return sum(r["tokens"] for r in runs if r["task"] == task)


def tokens():
    section("tokens per task (all roles, R-022 recorded as 0)")
    per = defaultdict(lambda: defaultdict(int))
    for r in runs:
        per[r["task"]][r["role"]] += r["tokens"]
    roles = ["planner", "implementer", "test-author", "reviewer", "evaluator"]
    print("task   tier   budget   " + " ".join(f"{x:>11}" for x in roles) + "       total")
    for t in sorted(per):
        tier = meta.get(t, {}).get("tier", "?")
        b = meta.get(t, {}).get("budget_tokens", "-")
        print(f"{t}  {tier:<5} {b:>7}   " + " ".join(f"{per[t][x]:>11}" for x in roles) + f"  {sum(per[t].values()):>10}")
    feat = [sum(per[t].values()) for t in FEATURE]
    print("feature tasks T-001..T-005 tokens:", feat, "sum", sum(feat),
          "median", statistics.median(feat), "p75", pct(feat, .75), "max", max(feat))
    allr = sum(r["tokens"] for r in runs)
    print("all recorded run tokens:", allr, "(of which T-000 bootstrap",
          sum(per["T-000"].values()), ", iteration-1 drafts",
          sum(sum(per[t].values()) for t in per if meta.get(t, {}).get("iteration") == "1"), ")")
    by_role = defaultdict(int)
    for t in FEATURE:
        for k, v in per[t].items():
            by_role[k] += v
    tot = sum(by_role.values())
    print("feature-task tokens by role:", {k: (v, round(100 * v / tot, 1)) for k, v in by_role.items()})


def rounds():
    section("implementation and test rounds per feature task")
    for t in FEATURE:
        rr = [r for r in runs if r["task"] == t]
        print(t, "impl runs", sum(r["role"] == "implementer" for r in rr),
              "test runs", sum(r["role"] == "test-author" for r in rr),
              "review runs", sum(r["role"] == "reviewer" for r in rr),
              "joins", sum(j["task"] == t for j in joins),
              "gate attempts", sum(r["gate_attempts"] for r in rr))


def reviews_():
    section("review rounds: tokens, findings, mutants")
    run_tok = {r["run"]: r["tokens"] for r in runs}
    rows = []
    for k, fm in reviews.items():
        f = dict(re.findall(r"(\w+): (\d+)", fm.get("findings", "")))
        mu = dict(re.findall(r"(\w+): (\d+)", fm.get("mutants", "")))
        tok = run_tok.get(fm["run"])
        rows.append((k, fm["run"], fm["verdict"], tok, f, mu))
        print(f"{k:<10} {fm['run']} {fm['verdict']:<16} tokens {tok:>7} findings {f} mutants {mu}")
    for rnd in ("1", "2", "3"):
        sel = [r for r in rows if r[0].endswith("r" + rnd)]
        toks = [r[3] for r in sel]
        fsum = defaultdict(int)
        for r in sel:
            for kk, vv in r[4].items():
                fsum[kk] += int(vv)
        print(f"round {rnd}: n={len(sel)} tokens {toks} median {statistics.median(toks)} findings {dict(fsum)}")
    tot = defaultdict(int)
    for r in rows:
        for kk, vv in r[4].items():
            tot[kk] += int(vv)
    print("all rounds findings:", dict(tot))


def joins_():
    section("joins")
    for j in joins:
        print(j["task"], j["at"], j["result"], "acc_total", j["acc_total"], "fail_base", j["acc_fail_base"],
              j["base_fail_kind"], "fail_first_join", j["acc_fail_first_join"], "acc_defects", j["acc_defects"],
              "impl_green", j["impl_tests_green"], "gate", j.get("gate_green"))
    print("joins with acc_total != acc_fail_base:",
          [(j["task"], j["at"], j["acc_total"], j["acc_fail_base"]) for j in joins if j["acc_total"] != j["acc_fail_base"]])
    print("sum acc_defects over all joins:", sum(j["acc_defects"] or 0 for j in joins),
          [(j["task"], j["at"]) for j in joins if j["acc_defects"]])
    print("revert checks:", [(r["task"], r["result_kind"], r["acc_fail"], r["acc_total"]) for r in reverts])


def lead():
    section("lead time ready -> merged, and dispatch -> merged")
    merged = {q["task"]: q["ended_at"] for q in queue if q["result"] == "merged"}
    for t, m in merged.items():
        rd = min((x["at"] for x in trans if x["task"] == t and x["to"] == "ready"), default=None)
        ip = min((x["at"] for x in trans if x["task"] == t and x["to"] == "in_progress"), default=None)
        first_run = min((r["started_at"] for r in runs if r["task"] == t and r["role"] != "planner"), default=None)
        f = lambda a: round((ts(m) - ts(a)).total_seconds() / 3600, 2) if a else None
        print(t, meta[t]["tier"], "ready->merged h", f(rd), "first non-planner run->merged h", f(first_run))
    print("queue gate seconds:", [(q["task"], q["gate_seconds"]) for q in queue])


def wall():
    section("agent wall seconds per feature task (sum of run wall_s; runs overlap)")
    for t in FEATURE:
        rr = [r for r in runs if r["task"] == t and r["role"] != "planner"]
        print(t, "sum wall_s", sum(r["wall_s"] for r in rr), "h", round(sum(r["wall_s"] for r in rr) / 3600, 2))
    first = min(ts(r["started_at"]) for r in runs if r["task"] in FEATURE)
    last = max(ts(q["ended_at"]) for q in queue if q["task"] in FEATURE)
    print("iteration span first feature run -> last feature merge h:", round((last - first).total_seconds() / 3600, 2))
    first0 = min(ts(r["started_at"]) for r in runs)
    last0 = max(ts(q["ended_at"]) for q in queue)
    print("iteration span first run (bootstrap) -> last merge h:", round((last0 - first0).total_seconds() / 3600, 2))


def budgets():
    section("budget use")
    for t in sorted(meta):
        b = meta[t].get("budget_tokens")
        if b and b.isdigit():
            u = task_tokens(t)
            print(t, meta[t]["tier"], meta[t]["state"], "used", u, "budget", b, "ratio", round(u / int(b), 2))


SECTIONS = {"completeness": completeness, "tokens": tokens, "rounds": rounds, "reviews": reviews_,
            "joins": joins_, "lead": lead, "wall": wall, "budgets": budgets}



def concurrency():
    section("concurrent agent runs (from run timestamps; several are hand-entered)")
    seen = set()
    iv = []
    for r in runs:
        if r["run"] in seen:
            continue
        seen.add(r["run"])
        iv.append((ts(r["started_at"]), ts(r["ended_at"]), r["run"], r["role"], r["task"]))
    for label, keep in (("all roles", lambda x: True),
                        ("implementer/test-author/reviewer", lambda x: x[3] in ("implementer", "test-author", "reviewer"))):
        sel = [x for x in iv if keep(x)]
        events = sorted({x[0] for x in sel})
        best = (0, None, [])
        over = []
        for t in events:
            live = [x[2] for x in sel if x[0] <= t < x[1]]
            if len(live) > best[0]:
                best = (len(live), t, live)
            if len(live) > 3:
                over.append((t.strftime("%H:%M:%S"), live))
        print(label, "max", best[0], "at", best[1], best[2])
        print("  instants with > 3 live:", over)


SECTIONS["concurrency"] = concurrency

if __name__ == "__main__":
    want = sys.argv[1:] or ["all"]
    for name, fn in SECTIONS.items():
        if "all" in want or name in want:
            fn()
