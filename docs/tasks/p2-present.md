# Tray view, menu model and state file from the core's state

Phase: 2. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-present\target`.
Runs after: the contracts card (it creates `text.rs` with the string signatures). Parallel with:
`p2-ownership-restore`, `p2-policy-holds`, `p2-policy-power`, and any card of another track, including the tray
track's wording of `text.rs`.

## Goal

Build what the user sees as data: `TrayView` (icon state, tooltip, menu) and `StateFile`, from a plain snapshot of the
core's state. The structure is this card's; every word comes from `text.rs`, which the tray track words.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 13, 14, 16).
- `docs/product.md` UX: menu sketch, Automatic, Keep awake, icon states, "The tooltip and the menu name what holds".
- `docs/architecture.md` section 2 `view`, "Boundary traits" (`MenuView` draws as given); section 3 decisions table
  rows 0, 33, 34, 35; section 6 `state.json`; section 4 CLI (what `status` and `restore` read).
- `crates/wakeshot-core/src/`: `view.rs`, `text.rs`, `files.rs` (`StateFile`), `holds.rs` (`HoldKind`).

## Owns

`crates/wakeshot-core/src/present.rs`. Nothing else. A string `text.rs` lacks goes to the Result as a request.

## Deliverables

1. A snapshot type the core cards fill (Automatic; the counted holds with kind and label; the Keep awake choice and
   Keep screen on; Keep awake time left, from the hold's `expires` and `Now`; a "requests only" flag for the episode;
   stepped back; the overridden keys; the tray's pid and start), and one function from it to `(TrayView, StateFile)`.
   The last two fields feed the tray track's `text::Status { requests_only }` and `keep_awake_group(left)` ("25 min
   left") (review m7). Tests build their expected strings through `text::`, so wording changes in `p5-text` and
   `p5-haik-check` never break them.
2. Icon state: Holding when any hold counts and requests are on; Off when Automatic is off and nothing holds; Idle
   otherwise (stepped back included, track note 16).
3. Tooltip and first menu line name what holds: agent sessions grouped by label with a count, hold files by their
   reason (or a fallback), Keep awake; via `text::hold_group`, `text::status_line`, `text::tooltip`. The tooltip fits
   Windows' limit (`NOTIFYICONDATAW.szTip`, 128 UTF-16 units with the end mark, Microsoft docs; not tested here):
   cut whole groups and say how many more.
4. Menu in the product sketch order: status line (a `Label`), separator, Automatic (checked), Keep awake submenu (Off,
   30 minutes, 1 hour, 2 hours, 4 hours, Until I turn it off, the current one checked; Keep screen on, checked, enabled
   only while Keep awake is on), separator, Restore my power settings, Open log, Quit. Each `Action` carries its
   `MenuCommand`. Nothing the product rules out (decision 39).
5. `StateFile`: `holding` equals the tooltip groups' labels, `overridden` the known journal keys, `stepped_back`,
   Automatic, the tray's pid and start.
6. Unit tests, written first, one named test per case: each icon state; zero, one and many holds; two Claude sessions
   grouped; a long reason; a tooltip over the limit; each Keep awake choice checked; Keep screen on disabled while
   Keep awake is Off; the menu's order and commands; `StateFile` fields for each case; a snapshot round trip through
   `serde_json` for `StateFile`.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core present`: all pass; list the test names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass. `rustfmt --edition 2024 --check <owned files>`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target x86_64-pc-windows-msvc -- -D warnings` and the same
  with `--target aarch64-pc-windows-msvc`: exit 0.
- No string literal a user can see in `present.rs` (a grep for `"` outside tests and doc comments finds none, or each
  hit is explained in the Result).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only `present.rs` changed.

## Out of scope

- Words (`text.rs`, tray track), drawing (`MenuView`, tray track), notices' text. When to rebuild the view and the
  `state.json` debounce (`p2-core-holds`). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.

## Result

What was done, files changed, checks and outcomes, open items. Include the snapshot type and function for the core
cards.
