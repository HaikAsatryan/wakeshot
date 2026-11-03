# Word every text the user sees

Phase: 5. Status: done. Needs Haik: one read of the text list, not blocking (batched for the plan sign-off or
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

1. While this card ran, the power review fix round's files broke `-p wakeshot-core` for a while: 17 compile errors in
   `core.rs` and `core/lid.rs`, then 12, 6 and 2 failing scenario tests. Gone at the last run (552 passed). Not a
   standing problem.
2. The fix round now also raises `LockFailed` when a lock is not confirmed and before a step back. The `LockFailed`
   words ("Windows did not lock the PC when Wakeshot asked. Press Windows key + L to lock it yourself.") still hold;
   they do not say that a closed laptop then hibernates after 5 minutes idle. (Done 2026-11-04 by the power fix round: the words now say it; the table below has them.) Change it when the core gives that case
   its own notice.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-03 20:12, orchestrator, from `p2-present`'s "Found outside scope": the planned `&&` doubling would show
  `&&` in the tooltip and the CLI (double only in menu text); `hold_group` must word a fallback for an empty label.
- 2026-11-03 23:36, worker: started; reading the card's sources.
- 2026-11-03 23:47, worker: sources read; Microsoft pages checked (`NOTIFYICONDATAW`, `DrawText`). Picks: tooltip in
  two lines ("Wakeshot", then the status line; the Phase 0 spike showed both lines), groups plain and only
  `status_line` doubles `&`, a last-guard cut in `notice` and `dry_run`. Writing the tests first.
- 2026-11-03 23:50, worker: 10 tests written against the stub bodies (two private helpers as `todo!()`): all
  compile, 8 fail; the 2 that pass (`tooltip_fits_for_any_holds`, `no_dashes_emojis_or_non_english_marks`) pass
  because the stub's words happen to be short ASCII. Writing the bodies.
- 2026-11-03 23:54, worker: bodies written; the 10 `text` tests pass. Mutation check: a 39-unit head made
  `tooltip_fits_for_any_holds` fail, so the property bites (reverted; the regression seed file it wrote,
  `crates/wakeshot-core/proptest-regressions/text.txt`, deleted). rustfmt (edition 2024) applied to `text.rs`, check
  clean; clippy clean for both targets, with and without `--all-targets`. Full `cargo test -p wakeshot-core`: 538
  passed, 12 failed, 4 ignored; all 12 failures panic in `scenarios/lid.rs` and `scenarios/settings.rs` (the power
  review fix round's files, running now), none in `text`. Waiting, then running again.
- 2026-11-03 23:56, worker: research section "Tray words checks" added; Result written with the text list. Added a
  dry-run check to the tooltip property. A rerun first hit 17 compile errors in `core.rs` and `core/lid.rs` (the fix
  round mid-edit), then built: 10 `text` tests pass, 544 passed and 6 failed in all, the 6 in `scenarios/lid.rs` and
  `scenarios/props.rs`. rustfmt check and clippy (both targets, with and without `--all-targets`) clean. Waiting 4
  minutes for a last full run.
- 2026-11-04 00:01, worker: last full run: 552 passed, 0 failed, 2 ignored. Checks recorded; card done; build
  folder deleted.

## Result

Every word the tray shows is now in `crates/wakeshot-core/src/text.rs`: menu, hold groups, status line, tooltip,
dry-run mark and all fourteen notices, through one `match` with no wildcard. Same public items and signatures as the
contracts card's version (13 items, compared with the file as read at start); everything new is private.

Files changed: `crates/wakeshot-core/src/text.rs` (bodies, private helpers, docs, tests); `docs/research.md` (new
section "Tray words checks", added at the end); this card.

### Picks (principal designer)

1. **Tooltip in two lines**: "Wakeshot", then the status line's words. The tray holds many icons, so the name comes
   first; the Phase 0 spike showed a `\n` tooltip with both lines. Change it when `p5-tray-icon` Part B shows Windows
   11 drops the second line: then "Wakeshot: " plus the words on one line.
2. **Groups are plain text; only `status_line` doubles `&`** (`p2-present` Found outside scope 1). The tooltip and
   `state.json` show a reason as written.
3. **Short heads (at most 28 units)**, so one 60 character reason plus "and N more" fits under every status, and two
   everyday groups plus "and N more" always fit. For that, "lid open only" (requests only) replaces "screen on"
   rather than joining it: it changes what the lid does, and a screen that stays on shows itself. Change it when the
   tooltip gets more room.
4. **Counts**: an agent always shows its sessions ("1 session", "2 sessions"); a `wakeshot run` program and a hold
   file show a count only from 2 ("2 commands", "2 files"). Empty labels: "an agent", "wakeshot run", "a hold file"
   (the last two as the core's log words them).
5. **Time words**: short in the tooltip and the closed-lid report ("25 min left", "2 h 15 min", "2 days 3 h"; minutes
   dropped beside days), full words in the menu ("1 hour 30 minutes"). No clock times (Phase 2 track note 14).
6. **Access keys**: the first letter, except Keep &screen on, whose own word is the one a user looks for. Unique per
   level: top A, K, R, O, Q; submenu O, 3, 1, 2, 4, U, S.
7. **Warnings**: the step back, requests only, cannot hold, restore failed, hibernate failed, lock failed, update
   Wakeshot and corrupt journal. Info: first run, user change, restored by user, closed-lid report, idle lock,
   updated.
8. **Silent**: the step back (its chime plays), `Updated` (the card), and also the idle lock, beyond the card's rule:
   it comes when nobody has used the PC for the sleep time, and a sound for an absent user helps nobody (Phase 2 track
   note 11's reason). Change it when Haik wants a sound there.
9. **Last-guard cuts**: `notice` cuts title and body at 63 and 255 UTF-16 units on a character boundary (only a
   version, an agent label or what held can reach them); the closed-lid report fits its list to 200 with whole names
   and "and N more". `dry_run` puts its mark on the name line and cuts the end at 127 (dry run only).

### Text list for Haik (one read, `p5-haik-check`)

Menu (the `&` marks the access key):

```
Holding awake: Claude Code (2 sessions), cargo     (status line, not pickable)
--------------------------------
&Automatic
&Keep awake               >   &Off
                              &30 minutes
                              &1 hour
                              &2 hours
                              &4 hours
                              &Until I turn it off
                              --------
                              Keep &screen on
--------------------------------
&Restore my power settings
&Open log
&Quit
```

Hold groups: `Claude Code (1 session)`, `Claude Code (2 sessions)`, `an agent (1 session)` (no label); `cargo`,
`cargo (2 commands)`, `wakeshot run` (no program name); `nightly backup`, `nightly backup (2 files)`, `a hold file`,
`2 hold files` (no reason); `Keep awake (25 min left)`, `Keep awake (1 h 30 min left)`, `Keep awake (2 days 3 h
left)`, `Keep awake (until turned off)`; the cut: `and 3 more`.

Status line (the tooltip's second line; its first line is `Wakeshot`, or `Wakeshot (dry run)`):

| State | Words |
|---|---|
| Holding | Holding awake: Claude Code (2 sessions), cargo |
| Holding, Keep screen on | Holding awake, screen on: Keep awake (25 min left) |
| Holding, power requests only | Holding awake, lid open only: cargo |
| Too many to fit | Holding awake: Claude Code (2 sessions), and 2 more |
| Holds, but the request failed | Cannot hold awake: cargo |
| Battery floor (stepped back) | Battery low, not holding: Claude Code (2 sessions) |
| Automatic on, nothing holds | Not holding, the PC sleeps as usual |
| Automatic off, nothing holds | Automatic is off, the PC sleeps as usual |

Notices (W = warning icon, S = silent, click: what clicking it opens):

| Notice | Title | Body | Flags |
|---|---|---|---|
| First run | Wakeshot is running | Its icon may hide under the ^ arrow on the taskbar. To keep it in view, click this notice, then turn on Wakeshot under "Other system tray icons". | click: taskbar settings |
| User changed lid (plugged in) | Wakeshot keeps your change | You changed what closing the lid does when plugged in. Wakeshot leaves it as you set it until the current work ends. | |
| User changed lid (battery) | same | You changed what closing the lid does on battery. (rest the same) | |
| User changed battery sleep | same | You changed when the PC sleeps on battery. (rest the same) | |
| Restore my power settings | Your power settings are back | Wakeshot put back your lid and sleep settings and leaves them alone until the current work ends. It still keeps the PC awake for the work, but only while the lid is open. | |
| Battery floor | Battery low: the PC may sleep now | The battery is under 20%, so Wakeshot stopped keeping the PC awake and put back your power settings. Plug in to keep it awake again. | W, S |
| Requests only | Wakeshot cannot change power settings | Something, such as a company policy, blocked the change. Wakeshot still keeps the PC awake, but only while the lid is open. | W |
| Cannot hold | Wakeshot cannot keep the PC awake | Windows did not accept Wakeshot's request, so the PC may sleep while your work runs. | W |
| Restore failed | Could not put back your power settings | Wakeshot keeps trying. Until it works, closing the lid may not sleep the PC, and on battery it may not sleep at all. Click to see your old values in the log. | W, click: log |
| Hibernate failed | Wakeshot could not hibernate the PC | Wakeshot tried to hibernate the PC with the lid closed, but it did not work. Your power settings are back, so Windows should sleep it at your usual sleep time. | W |
| Lock failed | Wakeshot could not lock the PC | Windows did not lock the PC. Lock it with Windows key + L. With the lid closed, Wakeshot tries once more; if that fails, it stops keeping the PC awake and hibernates it after 5 minutes with no input. | W |
| Closed-lid report | The PC ran with the lid closed | The PC kept running for 2 h 15 min with the lid closed. What held it: Claude Code and cargo. | |
| Update Wakeshot | Update Wakeshot | Wakeshot cannot fully read what Claude Code is doing, most likely because Claude Code was updated. Until you update Wakeshot, it may not keep the PC awake for Claude Code as it should. | W |
| Corrupt journal | Check your lid and sleep settings | Wakeshot's record of your old power settings was damaged, so it cannot put them back. Set the lid and sleep settings again by hand; click to see the old values in the log. | W, click: log |
| Keep screen on idle lock | Wakeshot locked the PC | Keep screen on is on, and the PC had no input for your usual sleep time, so Wakeshot locked it. Your work keeps running. | S |
| Updated | Wakeshot updated to 1.1.0 | The new version is running. | S |

Closed-lid report with many holds: "... What held it: nightly backup 1, ..., nightly backup 6 and 3 more." Under a
minute: "The PC kept running for less than a minute with the lid closed."

### Checks

- `cargo test --locked -p wakeshot-core text`: 10 pass: `an_empty_label_gets_a_name_by_kind`,
  `closed_lid_report_names_what_held_it`, `counts_read_plainly`, `durations_read_plainly`,
  `every_notice_has_words_within_balloon_limits`, `menu_access_keys_are_unique_per_level`,
  `no_dashes_emojis_or_non_english_marks`, `tooltip_fits_for_any_holds` (proptest; a mutation check showed it fails on
  a 39-unit head), `user_text_ampersand_is_doubled`, `wording_snapshot`. Written first: 8 of 10 failed on the stubs.
- `cargo test --locked -p wakeshot-core`: last run 2026-11-04 about 00:01, 552 passed, 0 failed, 2 ignored. Earlier
  runs failed only in the power review fix round's `scenarios` files while it worked (12, then 6, then 2), never in
  `text`.
- `rustfmt --edition 2024 --check crates/wakeshot-core/src/text.rs`: clean.
- `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: clean.
- Signatures: the 13 public items keep their signatures and shapes; new items are private.
- Only `text.rs` changed in `crates/`. The proptest seed file my mutation check wrote
  (`crates/wakeshot-core/proptest-regressions/text.txt`) is deleted.

### Open items

- The "Other system tray icons" label on this Windows build is not verified (no screen use here); `p5-haik-check`
  looks at it when Haik clicks the first-run notice.
- `p5-notices` and `p5-menu-view` show these words; `p5-menu-paint` must draw `&` by the Win32 rule (the `MenuItem`
  doc), or the status line shows `&&`.
