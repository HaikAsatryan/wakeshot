# Small fixes: a release guard for the CLI run tests, one shared taskbar DPI, three doc lines

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 40K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p6-tray-size` "Found outside scope" 2 and 3,
`p6-uninstall-startup-approval` "Found outside scope" 2 and 4, and the orchestrators' small items list.
Runs after: `p6-tray-size` (done). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-small-fixes\target`.

## Goal

Close small items that no card owns, each in the leanest way.

## Deliverables

1. `crates/wakeshot-cli/tests/run.rs`: under `cargo test --release` its runs used the real `%LocalAppData%\Wakeshot`,
   because a release exe ignores `WAKESHOT_DATA_DIR` (`p6-tray-size` Log 06:58). Add the same `cfg!(debug_assertions)`
   skip that `crates/wakeshot-tray/tests/smoke.rs`, `recovery.rs` and `tests/e2e/mod.rs` use. Check that no other
   test file under `crates/*/tests/` that starts an exe with `WAKESHOT_DATA_DIR` lacks it (`tests/exe.rs` needs none:
   help and argument errors only); add it where one does.
2. One `taskbar_dpi`: `crates/wakeshot-windows/src/tray/icon.rs` and `crates/wakeshot-windows/src/notify.rs` each have
   the same private `fn taskbar_dpi() -> u32`. Keep one (`pub(crate)` in the file that fits best) and call it from
   the other. No behavior change.
3. `docs/architecture.md` section 11 (Dependencies): a row for the direct dependency `idna_adapter` (`=1.0.0`, tray
   only, pinned in the root `[workspace.dependencies]`, never named in code: it picks the no-Unicode IDNA back end
   for velopack's `url`; `p6-tray-size`), and the `velopack` row's `Cargo.lock` package count (now 160).
4. `docs/architecture.md` section 6, "Registry values Wakeshot writes": one clause that `store::AutoStart` also
   removes the Run value's `StartupApproved` switch value at uninstall (`p6-uninstall-startup-approval`).
5. `crates/wakeshot-windows/src/store.rs` module doc (line 1 area): name the switch value beside the Run and RunOnce
   values, if the doc lists them.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target x86_64-pc-windows-msvc -- -D warnings` with
  and without `--all-targets`; ARM64 for `wakeshot-windows` and `wakeshot-cli` (the tray's ARM64 needs clang: CI
  only); `cargo test --workspace --locked` once at the end (debug only: never `--release` for tests).
- Build folder deleted at the end.

## Out of scope

- Any behavior change; `README.md`; phase files.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result

What was done, files changed, checks and outcomes, open items.
