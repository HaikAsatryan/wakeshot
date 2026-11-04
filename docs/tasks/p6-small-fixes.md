# Small fixes: a release guard for the CLI run tests, one shared taskbar DPI, three doc lines

Phase: 6. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 40K tokens. Written by the
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

1. `docs/architecture.md` has two prose lines over 120 characters (lines 396 and 415: the lid lock rules in section
   3). Not this card's lines; a docs pass can rewrap them.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 07:02: worker started; Status in progress.
- 2026-11-04 07:04: all five items edited; checks next. Item 1 survey: of `crates/*/tests/`, only
  `wakeshot-cli/tests/run.rs` lacked the guard. `recovery.rs` (every exe test calls `release_build`), `smoke.rs`
  (one test, inline check), `e2e_dry_run.rs` and `e2e_real.rs` (every test calls `e2e::skip`) have it; `exe.rs` needs
  none (help and `Bad` return before any data folder use, `wakeshot-cli/src/main.rs`); `wakeshot-core/tests/*` start
  no exe.
- 2026-11-04 07:04: decided by the worker, for Haik's review: in `run.rs` a `release_build()` like `recovery.rs`
  (five tests use it, so a function, not five inline blocks), called by the five tests that start the exe.
  `ctrl_c_helper` and `sleeper` get none: the helper runs only when its parent test sets its variable, and the parent
  skips first. Change it when the helper is ever run by hand with that variable set.
- 2026-11-04 07:04: decided by the worker, for Haik's review: `taskbar_dpi` stays in `tray/icon.rs` as `pub(crate)`
  (the taskbar is the tray icon's home; `notify` already named `tray::icon` as its source), with the two doc comments
  merged; `notify.rs` imports it and drops its copy and five imports. Change it when a third user outside `tray`
  appears: then a small shared DPI module fits better.
- 2026-11-04 07:04: checked before the doc lines: `Cargo.lock` holds 160 `[[package]]` entries, `idna_adapter` 1.0.0,
  no `icu_*` package; root `Cargo.toml` pins `idna_adapter = "=1.0.0"`, only `wakeshot-tray` names it;
  `hooks::on_uninstall` calls `AutoStart::remove_approval` right after the Run value.
- 2026-11-04 07:08: all checks pass (see Result); build folder and the card's scratch folder deleted. Status done.

## Result

Done. Files changed:

| File | Change |
|---|---|
| `crates/wakeshot-cli/tests/run.rs` | `release_build()` (the `recovery.rs` form): the five tests that start `wakeshot.exe` return at once in a release build, with a "skipped" line on stderr; module doc says so |
| `crates/wakeshot-windows/src/tray/icon.rs` | `taskbar_dpi` is `pub(crate)`; its doc merges both files' reasons and names `notify` as the second user |
| `crates/wakeshot-windows/src/notify.rs` | its copy of `taskbar_dpi` removed, imported from `tray::icon`; five unused imports dropped. No behavior change: the two bodies were the same |
| `crates/wakeshot-windows/src/store.rs` | module doc names the Run value's `StartupApproved` switch value |
| `docs/architecture.md` | section 6: `store::AutoStart` also removes the switch value at uninstall; section 11: new `idna_adapter` row, `velopack` row "160 since `p6-tray-size`" |

No other test file under `crates/*/tests/` lacked the guard (Log 07:04).

Checks (x64 unless named, debug, `--locked`, own build folder):

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace -- -D warnings` with and without `--all-targets`: pass.
- ARM64 clippy for `wakeshot-windows` and `wakeshot-cli`, with and without `--all-targets`: pass.
- `cargo test --workspace --locked` (debug, once): pass. CLI `run.rs` 5 passed, 2 ignored; core 603 and 11 and 25;
  windows 552 (21 ignored); tray unit 8, `recovery.rs` 5 (2 ignored); the e2e and smoke files all ignored, as before.
  No tray was started. The release skip path itself was not run: the card forbids `--release` tests.
- Edited files: LF only, no em dash, new lines at most 120 characters (table rows excepted, as the rest of the
  table).
- Build folder deleted.

Open items: none. Two decisions for Haik's review are in the Log (07:04).
