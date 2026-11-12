# photocraft-study-review

An experiment: take the findings of the PhotoCraft forensic study
(<https://github.com/lbecken/photocraft-study>), turn them into a **reviewed and improved
methodology for building software with AI agents**, and test that methodology by having
agents build a real product with it.

- **The methodology** lives in `methodology/` and is versioned. It changes only at the end of
  an iteration, and every change cites the measured problem that motivated it.
- **The proof-of-concept product** is **Kiln**, a small desktop code editor for Rust projects,
  written in Rust (`crates/`, `apps/`). See `docs/CHARTER.md`.
- **The experiment record** is in `tasks/`, `metrics/`, `retrospectives/` and `docs/decisions.md`.

Start with `STATE.md`, then `docs/CHARTER.md`, then `methodology/METHODOLOGY.md`.
