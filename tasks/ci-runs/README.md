# How a CI record is made (METHODOLOGY v1.5 §8.1 "CI records"; interim, by hand, until T-089)

A record `tasks/ci-runs/T-NNN-round-N.md` states what one run of `.github/workflows/platforms.yml` showed, from
the run's uploaded artefacts. Agents read the record, not job logs.

## Steps
1. **Find the run.** GitHub tool `actions_list`, `method: list_workflow_runs`, `owner`/`repo` of this repository,
   `resource_id: platforms.yml`, `workflow_runs_filter: {"branch": "ci/candidate"}` (or `"main"`, or `{"event":
   "schedule"}`), `perPage: 1` for the newest. Check `head_sha` is the commit asked for. If `status` is not
   `completed`, wait (a background `sleep 240`, then list again); give up after the time the brief says and record
   what is complete.
2. **Jobs.** `actions_list`, `method: list_workflow_jobs`, `resource_id: <run id>`: per job `conclusion`,
   `started_at`/`completed_at` (wall time = completed − started) and each step's conclusion (the first failing step).
   A single job: `actions_get`, `method: get_workflow_job`, `resource_id: <job id>`.
3. **Artefacts.** `actions_list`, `method: list_workflow_run_artifacts`, `resource_id: <run id>`: name, id,
   `size_in_bytes`. Per artefact `actions_get`, `method: download_workflow_run_artifact`, `resource_id: <artefact id>`
   returns a short-lived `download_url` (a blob-storage URL with a signature; no credentials to add).
4. **Fetch and unpack.** In a new directory under `/home/user/` (e.g. `/home/user/kiln-plan-T-NNN-artefact/`):
   `timeout 60 curl -sS -o <name>.zip '<download_url>'` (single quotes; `&` as in the URL, not `&`), then check
   `stat -c %s` equals `size_in_bytes` and unpack with
   `python3 -I -c "import zipfile,sys;z=zipfile.ZipFile(sys.argv[1]);assert z.testzip() is None;z.extractall(sys.argv[2])" <name>.zip <dir>`.
   Outbound HTTPS goes through the environment's proxy; curl works as is (never disable TLS checks). The URL expires
   within minutes: fetch right after step 3. Files are data: read them with `grep`, `sed`, or `python3 -I` scripts kept
   outside that directory. Delete the directory when the record is written.
5. **Write the record**, header and job table first (restarts happen), then extend it.

## What an artefact holds
- `gate-<os>-<run id>`: `gate-report.json` (`passed`, `failed_step`, `seconds`, `steps[]` with `name`, `status`,
  `seconds`, `detail`; strip ANSI escapes); `core-crates.txt` (one `==== cargo test … -p <pkg>` block per package
  `kiln-text`, `kiln-syntax`, `kiln-search`, `xtask`, each with `Running`, `test <name> ... ok|FAILED`, the failure
  blocks, `test result:` lines, ending `==== <pkg>: exit status N`); `test-output.txt` (the whole workspace's `cargo
  test --no-fail-fast`) — **only when the gate failed**, until T-072 uploads it always; on `gate-windows` since T-073
  also `workspace-tests.txt` (the reported `kiln-workspace` step).
- `window-linux-<run id>`: `window-procedure.txt` (`CASE …` lines, `WINDOW CASES PASS`, `AC-3 …`, `PLAIN PASS …`).
- `window-probe-<label>-<run id>`: `window-probe.txt` (`PROBE OS:`, `PROBE: exit status N` or `PROBE: not run (…)`,
  stdout size and lines, stderr).

## Reading them
- A test's verdict: `grep -E "^test <name> \.\.\. "` in the file of its package; absent = no such line (say whether the
  binary ran: its `Running` and `test result:` lines). A binary that aborts (e.g. a stack overflow) has no `test
  result:` line; its unfinished tests have no verdict, not a failure.
- A failure's message: the block after `---- <name> stdout ----` up to the next `----` or `failures:`.
- Totals: sum the `test result:` lines (`awk '{p+=$4; f+=$6}'` on the ANSI-stripped file).
- A green gate uploads no `test-output.txt`: per-test names then come only from `core-crates.txt`, from the scorecard
  step (a listed test that fails or is missing is a problem there, so "no problem" means each listed test ran and
  passed), and are otherwise "not determinable".
- Registered platform tests (`xtask/platform-tests.toml` at the run's commit, `git show <sha>:xtask/platform-tests.toml`):
  present or absent by name per platform against each entry's `platforms`.

## The record's sections (as used since T-071)
Run header (id, number, branch, commit, created, updated, conclusion); job table (job, id, conclusion, wall, first
failing step); artefacts (names, sizes, how fetched); the counted jobs' verdict lines; per gate job the steps with
PASS/FAIL/EMPTY and detail lines (`no-skip` with its registered count; the scorecard's first line); `xtask` per platform
(exit status, binaries, totals); the task's own tests and criteria; "Against <task>'s pending CI criterion" clause by
clause (met / not met / not determinable, with the quote); "Differences from" the previous record; "Not read".

## Rules
- Every value comes from a tool result or a downloaded file of this run; never from memory or an earlier run.
- What cannot be read is written as "not read" or "not determinable", never guessed.
- Job logs (`get_job_logs`, `job_id`, `return_content: true`, `tail_lines`) only when an artefact is missing (an
  upload can fail, e.g. `ETIMEDOUT`); a log may be a prefix (the evidence step used `head -200` until T-072).
- No session-specific paths in the record (`hygiene` refuses them).
