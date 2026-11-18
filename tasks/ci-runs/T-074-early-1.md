# CI run 44307677646 (platforms.yml) on ci/candidate @ cec40579598ee6a2e033d0ccb434c7e2373e2217 — T-074 early look 1

Record by hand; method as `tasks/ci-runs/README.md`. **DRAFT: written while the run was in progress (first check about 13:05Z); to be extended.**

## Run header
- id 44307677646, run_number 29, attempt 1, event push, title "CI early look (not for merge): T-074 joined head with T-072's joined …"
- branch `ci/candidate`; commit `cec40579598ee6a2e033d0ccb434c7e2373e2217`; created_at 2026-11-18T12:49:57Z
- first check (updated_at 13:04:59Z): status in_progress.

## Job table (first check, about 13:05Z)
| job | id | conclusion | note |
|---|---|---|---|
| gate-linux | 134813362754 | in progress | started 12:50:00Z; Gate failure (12:50:18Z-13:05:24Z); Core crates running |
| gate-macos | 134813362960 | in progress | started 12:55:01Z; Gate running |
| gate-windows | 134813362421 | in progress | started 12:49:59Z; Gate failure (12:50:28Z-12:58:47Z); Core crates running |
| window-linux | 134813362613 | success | 12:50:00Z-12:51:40Z = 1m40s |
| window-probe (macos-15) | 134813362978 | cancelled | 12:49:58Z-13:04:59Z = 15m01s; the job listing shows no steps |
| window-probe (windows-2025) | 134813362953 | failure | 12:50:00Z-12:52:15Z = 2m15s; failing step Window probe (12:52:09Z-12:52:11Z) |
