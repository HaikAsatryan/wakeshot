# Why the release tray exe doubled, and whether it can shrink

Phase: 6. Status: in progress. Needs Haik: no (a dependency change waits for the orchestrator's go). Agent:
`wakeshot-worker`. Size: about 50K tokens. Written by the orchestrator (session `wakeshot-12`, 2026-11-04) from the
`p6-install-e2e` rerun (sizes) and docs pass round 4 "Found outside scope" 8.
Runs after: `p6-int-update-wiring` (done). Parallel with: stop 1 runs beside any card (read only, own target folder);
stop 2, if any, changes `Cargo.toml` and `Cargo.lock` and runs alone among cards that build.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-tray-size\target`.

## Goal

The release x64 tray exe grew from 2,026,496 bytes to 3,952,640 bytes when `p6-int-update-wiring` started using the
update feed (`url`, `idna`, `icu_*` and an HTTPS stack now built). Size is a selling point (the README compares it with
rivals). Find what adds the bytes, and whether a smaller build keeps everything the app uses.

## Read first

- `docs/tasks/p6-int-update-wiring.md` (Result), `docs/tasks/p6-velopack-dependency.md` (Result), `crates/wakeshot-tray/
  src/velo.rs`, `crates/wakeshot-tray/Cargo.toml`, root `Cargo.toml` (profile, `velopack` pin).
- The `velopack` crate's features at the pinned version (its `Cargo.toml` in the cargo registry, and its docs).

## Stop 1 (read only)

1. Build release x64 in your own target folder. Measure: `cargo tree -p wakeshot-tray -e features -i <crate>` for
   `url`, `idna`, `icu_*`, the TLS and HTTP crates; and the size each adds, with `cargo bloat` if it is installed
   (do not install tools; if it is missing, compare sizes of builds with a feature turned off, in your own target
   folder, with any `Cargo.toml` change made only in a scratch copy of the workspace outside the repo).
2. Which `velopack` features (or our own code in `velo.rs`) pull each, and which of them the app needs (the GitHub
   update source over HTTPS, the local test feed in debug builds).
3. Options, each with the size it saves and what it costs (lost function, a TLS choice, risk). The release profile
   too (`opt-level`, `lto`, `codegen-units`, `panic`, `strip`), if not set already.
4. Write the findings and your pick in the card Log and Result, then end with a short report that starts "Stop 1
   ready". Change no repo file but this card.

## Stop 2 (only after the orchestrator's go)

The picked change in `Cargo.toml`, `crates/wakeshot-tray/Cargo.toml` and `Cargo.lock`, with the full checks: `cargo fmt
--all --check`; clippy both forms, both targets (the tray's ARM64 needs clang: CI only); `cargo test --workspace
--locked`; the release builds; the dry-run smoke test (`crates/wakeshot-tray/tests/smoke.rs`).

## Out of scope

- `velo.rs` logic, the update schedule, packaging scripts.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 06:13: worker started stop 1 (read only). Status in progress.

## Result

What was done, files changed, checks and outcomes, open items.
