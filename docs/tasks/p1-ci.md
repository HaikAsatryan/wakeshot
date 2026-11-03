# Add CI on GitHub Actions

Phase: 1. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: `p1-cargo-workspace`. Parallel with: `p1-repo-files`, `p1-tray-menu-look` (none of them writes
`.github/`). The GitHub run itself happens after the orchestrator's first push.

## Goal

One workflow that checks every push and pull request: fmt, clippy, tests and release builds on x64 and on a real
ARM64 runner, plus the core alone on Linux (`docs/architecture.md` section 10).

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work".
- `docs/architecture.md` sections 2, 10 and 11.
- `docs/research.md`: "Distribution" (GitHub Actions bullet), "Toolchain" and "Phase 1 design checks" (action
  versions, ARM64 image).
- `docs/research/distribution.md` section 4 (runners).
- The workspace this card checks: root `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `crates/`.
- Current GitHub docs for workflow syntax and each action's README. Check the newest release of each action again
  (`gh api repos/<owner>/<repo>/releases/latest`); do not trust the versions above if newer ones exist.

## Deliverables

1. `.github/workflows/ci.yml`:
   - Triggers: `push` and `pull_request`. `permissions: contents: read`. `concurrency` per ref with
     `cancel-in-progress: true`. `timeout-minutes` on each job.
   - Job `windows`, matrix `windows-latest` (x64, `x86_64-pc-windows-msvc`) and `windows-11-arm` (ARM64,
     `aarch64-pc-windows-msvc`): checkout; install the toolchain from `rust-toolchain.toml` (rustup is on both images;
     if it is missing on one, install it in a step); cache; `cargo fmt --all --check` (x64 only); `cargo clippy
     --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo build --release
     --workspace --locked`; upload the two release exes as an artifact kept 7 days.
   - Job `core-linux` on `ubuntu-latest`: `cargo test -p wakeshot-core --locked`. It shows the core has no OS code.
   - Actions from `actions/` by major tag. Any other action pinned by full commit SHA with the version in a comment.
     Prefer no third-party action where a `run:` line does the job.
2. Nothing else: no release workflow, no Dependabot, no branch rules (Phase 6 or later).

## Acceptance checks

- The YAML parses (a parser available offline, or `actionlint` if it is already installed; do not download tools).
  If neither exists, say so in the Log; the first GitHub run is then the parse check.
- Every `uses:` line is `actions/<name>@vN` or `<owner>/<repo>@<40 hex> # vX.Y.Z`; a grep shows it.
- Each `run:` command of the Windows job, run locally on x64, passes (fmt, clippy, test, release build), and the
  clippy and build commands pass for `--target aarch64-pc-windows-msvc` too. Record the outcomes.
- After the orchestrator's first push (not this card's work): every job is green. The orchestrator records the run
  URL in the phase file; a red job becomes a fix card.

## Out of scope

- Git and GitHub: no push, no repo settings, no secrets. Workers never run git.
- Cargo files and code, except a fix this card needs to make a check pass; name any such fix in the Result.
- Velopack, signing, releases (Phase 6).

## Questions for Haik

Each with the worker's default.

None.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 10, CI bullet (owner `p1-design-verify-fixes`): it does not name two things the
  workflow now does. Suggested words: "each cargo command passes `--target <triple>`; a step reads the release exes'
  imports with `dumpbin /dependents` and fails on `VCRUNTIME`, `MSVCP`, `ucrtbase` or `api-ms-win-crt-*`; CI never
  sets `RUSTFLAGS`".
- `docs/research.md` "Phase 1 design checks", ARM64 image bullet: it cites the old copy in
  `actions/partner-runner-images` (image 20260105, Rust 1.92) and marks the MSVC ARM64 tools as not verified. The new
  "CI workflow" section answers it (current readmes in `actions/runner-images`). Left as is: research takes additions
  only.
- The scratchpad folder is shared by every agent of the orchestrator's session. `scratchpad\target` was made by
  another card, and a card that builds there while another does gets slowed by cargo's lock and mixes outputs. Cards
  could say "build into `scratchpad\<card>\target`".
- `push` plus `pull_request` runs CI twice for a pull request from a branch of the same repo (two refs, so the
  concurrency group does not merge them). Free on a public repo; change to `push: branches: [main]` when it gets noisy.

## Log

- 2026-11-03: card written by `p1-detailed-design`.
- 2026-11-03 15:51: updated by `p1-design-review-fixes`: size and parallel lines (`p1-lid-sound` is done).
- 2026-11-03 16:14: started by `wakeshot-worker`. Owns only `.github/` and this card.
- 2026-11-03 16:18: checked with `gh api`: newest releases checkout v7.0.1, upload-artifact v7.0.2, cache v6.1.0
  (major tags v7, v7, v6 point at those commits). Runner labels from docs.github.com. All three images ship rustup
  1.29.x; both Windows ARM64 images have VC.Tools.ARM64 and SDK 10.0.26100. No `actionlint` or PyYAML here; the
  orchestrator allows getting `actionlint` without admin, so I take the release exe with its checksum.
- 2026-11-03 16:20: wrote `.github/workflows/ci.yml`. `actionlint` 1.7.12 (SHA-256 matches the release list): 0
  errors; a broken copy proves it checks runner labels and `upload-artifact@v7` inputs. `rustup toolchain install
  --no-self-update` on an empty scratch rustup home installs 1.99.1 with 6 components in 10 s (scratch home deleted).
  Next: the local cargo runs.
- 2026-11-03 16:23: my first x64 run used `scratchpad\target`, which another card had made (the scratchpad is shared
  by the orchestrator's agents; cargo locks its build folder, so nothing broke). Moved all my files to
  `scratchpad\p1-ci\` and reran everything into `scratchpad\p1-ci\target`: fmt, clippy, test, release build and the
  CRT step pass for x64; clippy, release build and the CRT step pass for ARM64; `cargo test -p wakeshot-core` passes
  (on Windows). A control build with `RUSTFLAGS=-C debuginfo=0` makes the CRT step fail as it should. `uses:` grep: 5
  lines, all `actions/<name>@vN`.
- 2026-11-03 16:25: facts added to `docs/research.md`, new section "CI workflow". Deleted my scratch build folders.
  Done.

## Result

Status: done. The GitHub run itself waits for the orchestrator's first push.

**What was done.** `.github/workflows/ci.yml`:

- On `push` and `pull_request`; `permissions: contents: read`; one concurrency group per workflow and ref with
  `cancel-in-progress: true`; `CARGO_INCREMENTAL: 0`; a comment that CI must never set `RUSTFLAGS`.
- Job `windows` (30 min), matrix `windows-latest` + `x86_64-pc-windows-msvc` and `windows-11-arm` +
  `aarch64-pc-windows-msvc`, `fail-fast: false`: checkout (no saved credentials); `rustup toolchain install
  --no-self-update` (reads `rust-toolchain.toml`); `actions/cache` for the cargo registry, git db and `target/`, keyed
  on OS, CPU, job and the hash of `rust-toolchain.toml` and `Cargo.lock`; `cargo fmt --all --check` (x64 only);
  clippy, test and release build, each with `--locked --target <triple>`; a CRT step: `vswhere` finds `dumpbin`, and
  the step fails if either exe imports `VCRUNTIME`, `MSVCP`, `ucrtbase` or `api-ms-win-crt-*`; upload of
  `wakeshot.exe` and `wakeshot-tray.exe` as `wakeshot-<triple>`, kept 7 days, error if missing.
- Job `core-linux` (15 min) on `ubuntu-latest`: checkout, toolchain, cache, `cargo test -p wakeshot-core --locked`.
- Only `actions/checkout@v7`, `actions/cache@v6`, `actions/upload-artifact@v7` (newest releases v7.0.1, v6.1.0,
  v7.0.2). No third-party action.

**Choices.** `--target` on each command: fixed output paths for the CRT step and upload, and the CPU is explicit.
`actions/cache` instead of `Swatinem/rust-cache`: first-party; it saves only when the key changes, so the cached
`target/` is refreshed only when `Cargo.lock` or the toolchain changes. Switch to `Swatinem/rust-cache` v2.9.3 (pinned
by SHA) when CI gets slow. CI does not need `rust-analyzer`: no step uses it, so it stays out of the pin, where it
would add a download to every job; locally it is `rustup component add rust-analyzer --toolchain 1.99.1`.

**Files changed.** `.github/workflows/ci.yml` (new), `docs/research.md` (new section "CI workflow", additions only),
this card. No Cargo file or code changed.

**Checks.**

- YAML parse: `actionlint` 1.7.12 (release exe, checksum checked): 0 errors. A broken copy is flagged, so its label
  and `upload-artifact@v7` input checks are live. It does not check pwsh scripts. Pass.
- `uses:` lines: 5, all `actions/<name>@vN`; a script finds 0 lines that match neither allowed form. Pass.
- Local x64, the job's exact commands into `scratchpad\p1-ci\target`: fmt, clippy, test (5 tests), release build and
  the CRT step all exit 0. Pass.
- Local `--target aarch64-pc-windows-msvc`: clippy, release build and the CRT step exit 0. Pass.
- `cargo test -p wakeshot-core --locked`: passes on Windows. On Linux: not run here.
- CRT step against a `RUSTFLAGS=-C debuginfo=0` control build: fails, naming `VCRUNTIME140.dll` and six
  `api-ms-win-crt-*` DLLs. Pass.
- Toolchain step on an empty scratch rustup home: installs 1.99.1 with the 6 pinned components in 10 s. Pass.
- Every job green on GitHub: open, for the orchestrator after the first push.

**Open items, not verified until the first run.** The `dumpbin` lookup on the runners' VS installs. Which image
`windows-11-arm` gives today (the VS 2026 move, issue `actions/runner-images#14602`, is still open). That `~/.cargo`
is the cargo home on the Ubuntu runner (only the cache hit rate depends on it). `windows-11-arm` on a private repo:
not checked; the repo is public from the first push.
