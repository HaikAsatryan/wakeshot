# Word every text the user sees

Phase: 5. Status: ready. Needs Haik: one read of the text list, not blocking (batched for the plan sign-off or
`p5-haik-check`). Agent: `wakeshot-worker`. Size: about 80K tokens.
Real laptop: no (pure core code). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p5-text\target`.
Runs after: the contracts change card (it creates `text.rs` with the signatures and stub wording). Parallel with:
`p2-present` (it calls these functions; it never writes words), `p5-theme`, `p5-icon-art`, Part A of `p5-sounds` and
`p5-notices`, and any card of another track.

## Goal

Every word Wakeshot shows, in one file (decision 1: English only, no translation layer): the menu items, the status
line, the tooltip pieces, and the title, body and behavior of every notice. Plain, short, calm English that tells the
user what happened to their laptop and what, if anything, to do.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 3 and 5);
  `docs/phases/02-core.md` track notes 7, 8, 11, 13, 14 and 16 (what the core decides; no clock times in text).
- `docs/product.md`: "The job" (the user must always understand what happens), UX (menu sketch, Keep awake choices,
  tooltip names what holds, first run, "update Wakeshot", restore fails), the sound lines, "Battery floor", the
  closed-lid notice.
- `docs/architecture.md` section 3: `Notice` variants and the episode flow lines that raise each; section 8 table (the
  failures that notify).
- `crates/wakeshot-core/src/text.rs` (the signatures, which are contract: never change a signature or a type's
  shape; a missing one goes to the Result as a request), `view.rs` (`Notice`, `MenuCommand`, `KeepAwake`,
  `IconState`; the `&` access key rule in the `MenuItem` doc), `holds.rs` (`HoldKind`), `settings.rs` (`SettingKey`).

## Owns

`crates/wakeshot-core/src/text.rs` (bodies and tests only). Nothing else in `crates/`.

## Deliverables

1. Menu texts: Automatic; the Keep awake submenu and its choices (Off, 30 minutes, 1 hour, 2 hours, 4 hours, Until I
   turn it off, Keep screen on); Restore my power settings; Open log; Quit. Each with an `&` access key, unique within
   its menu level. Keep the product sketch's words unless a reason in the Result says otherwise.
2. Status and tooltip pieces: a hold group (an agent's label with its session count, a `wakeshot run` program, a hold
   file's reason or a fallback, Keep awake with what is left), the "and N more" piece, the status line and the
   tooltip for holding, idle, off, stepped back and requests only, and the dry-run mark. User-given text (hold
   reasons) has `&` doubled so the menu shows it as is. No clock times: the core has no UTC offset (Phase 2 note 14);
   use "25 min left".
3. Notices: for every `Notice` variant in `view.rs` (fourteen: the first eleven, `JournalCorrupt`, `IdleLocked`, and
   `Updated` from `p1-close-contracts`): title, body, `warning`, `silent`, `click`, through one exhaustive `match` so
   a new variant does not compile until it has words.
   - `silent` is true when a chime plays with the notice (the battery floor step back), so two sounds never overlap.
   - `click`: first run opens the taskbar settings page; a failed restore and a corrupt journal open the log (it
     holds the old values); the rest do nothing.
   - First run says how to keep the icon in view: click the notice, then turn on Wakeshot under "Other system tray
     icons" (track note 3.6).
   - "Update Wakeshot" names the agent; the closed-lid report says how long the PC ran with the lid closed and what
     held it; a user change names the setting in the user's words (lid action, sleep on battery).
   - `Updated { version }`: Wakeshot updated itself, with the version, as in ship Question 3's sample "Wakeshot
     updated to 1.1.0" (Haik's yes, 2026-11-03 18:09). No sound, so `silent` is true (the `NoticeText` doc says so);
     not a warning; the click does nothing.
4. Limits, checked by tests: balloon title at most 63 UTF-16 units and body at most 255 (`szInfoTitle[64]`,
   `szInfo[256]`); every tooltip piece short enough that `p2-present` can fit holding, two groups and "and N more" in
   127 units (`szTip[128]`). Where Microsoft documents the sizes, cite the page in research; else mark "not verified".
5. Unit tests, written first: `every_notice_has_words_within_balloon_limits` (one sample of every variant),
   `menu_access_keys_are_unique_per_level`, `user_text_ampersand_is_doubled`, `tooltip_fits_for_any_holds` (a
   `proptest` over group lists), `durations_read_plainly` (table: under a minute, 45 min, 1 h, 2 h 15 min, days),
   `counts_read_plainly` (1 session, 2 sessions), `no_dashes_emojis_or_non_english_marks` (scans every produced string
   for U+2013, U+2014 and emoji ranges), `wording_snapshot` (a table of the exact strings, so every wording change
   is a visible diff in review).
6. In the Result: the full text list (menu, status, tooltip samples, each notice), for Haik's one read.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core text`: all pass; list the names in the Result.
- `cargo test --locked -p wakeshot-core`: all pass (CI also runs it on Linux).
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets` with `-D
  warnings` for both targets: clean in the owned files.
- No signature or type shape in `text.rs` changed (compare with the contracts card's version).
- Only `text.rs` changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Which items the menu has, their order, checks and enabled state, the icon state, cutting the tooltip (`p2-present`).
- When a notice is raised (core track). Drawing and showing (`p5-menu-view`, `p5-notices`). Log lines (each module
  that logs; integration reads them for plain English). CLI messages (Phase 4).

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-03 20:12, orchestrator, from `p2-present`'s "Found outside scope": the planned `&&` doubling would show
  `&&` in the tooltip and the CLI (double only in menu text); `hold_group` must word a fallback for an empty label.

## Result

What was done, files changed, checks and outcomes, open items. Include the full text list for Haik.
