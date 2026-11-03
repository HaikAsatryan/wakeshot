# Tray view, menu model and state file from the core's state

Phase: 2. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
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

None.

## Found outside scope

Problems seen but not fixed.

1. `text.rs` (tray track, `p5-text`): present uses one group string in three places: the tooltip, the status line
   (a menu `Label`, where `&` is a prefix mark) and `state.json` (`wakeshot status` prints it). `p5-text` deliverable 2
   doubles `&` in hold reasons inside the group; then the tooltip and the CLI would show `&&`. Proposal: double `&`
   only in `text::status_line`, the one menu text built from groups.
2. `text.rs`: `hold_group` gets an empty label for a hold file with no reason and should word a fallback by kind.
   Its doc does not say so yet. Request to `p5-text`: say it in the doc and test it.
3. `text.rs`: `text::action` gets the command a row sends, so a checked Automatic gives `Automatic(false)` and a
   checked Keep screen on gives `KeepScreenOn(false)`. Its words must not depend on the bool (the stub's do; harmless
   until `p5-text`).
4. `p5-tray-icon`: the tray marks a dry-run tooltip with `text::dry_run` after present has fitted it to 127 units,
   so a full dry-run tooltip loses its end to the last-guard cut, in the middle of a group. Dry run only. Fix options:
   the tray drops `and N more` style groups itself, or present gets a dry-run flag (a contract change).
5. `p2-policy-holds` / `p2-core-holds`: `policy::hold_files::keep_awake().choice` is the minutes left (29 after one
   minute of a 30-minute hold). Passed as `Snapshot.keep_awake`, the menu would check no length. `Snapshot.keep_awake`
   is the length picked; see Result, "For `p2-core-holds`".
6. While this card built, `p2-policy-power`'s half-done files broke `-p wakeshot-core` builds for a few minutes
   (an import of a missing function in `policy/lock.rs`; unused variables in `lid.rs` and `lock.rs` under
   `-D warnings`; 59 `todo!()` test failures). All gone on the last run. Not a standing problem.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03: worker started; reading the card's sources.
- 2026-11-03 19:29 to 19:45: first run stopped by Haik before any code.
- 2026-11-03 ~20:00: second run started fresh (running); reading the card's sources.
- 2026-11-03: sources read. Design: `Snapshot` with every counted hold as a `Holder { kind, label, expires }`, the
  Keep awake choice as the length picked (not the time left), and a `requests_on` flag for the icon rule; `build`
  gives `(TrayView, StateFile)`. Writing tests and code in `present.rs`.
- 2026-11-03: 24 tests written first against `todo!()` bodies: all compile and all 24 fail. Implementing.
- 2026-11-03: code written. First test run blocked by another card's half-done file: `policy/lock.rs:59` imports
  `policy::ending::clamped_idle_ms`, not there yet (`p2-policy-power`). Waiting, then retrying.
- 2026-11-03: retry builds. `present` tests: 24 of 24 pass. Whole crate: 186 pass, 59 fail, all 59 in
  `p2-policy-power`'s files (`policy::ending` 12, `floor` 9, `lid` 20, `lock` 10, `requests` 8; `todo!()` bodies,
  its tests-first step). `rustfmt --edition 2024 --check present.rs`: no diff. Running clippy.
- 2026-11-03: clippy at first failed only in `policy/lid.rs` and `lock.rs` (unused variables); a warn-level run
  showed nothing in `present.rs`. Rerun minutes later: all four clippy runs exit 0, whole crate 245 of 245 pass.
  Checked the `szTip` size on Microsoft Learn; added to `docs/research.md` "Tray view checks". Build folder and
  scratch file deleted. Done.

## Result

Status: done. Only `crates/wakeshot-core/src/present.rs` and this card changed, plus one addition to
`docs/research.md` ("Tray view checks": `szTip[128]` with the null, `szInfo[256]`, `szInfoTitle[64]`, from Microsoft
Learn's `NOTIFYICONDATAW` page). No `text.rs` signature was missing.

### API for the core cards

```rust
pub const KEEP_AWAKE_CHOICES: [KeepAwake; 6]; // Off, 30, 60, 120, 240, UntilOff: the submenu's order
pub struct Holder { pub kind: HoldKind, pub label: Option<String>, pub expires_wall_ms: Option<i64> }
pub struct Snapshot {
    pub automatic: bool,
    pub holds: Vec<Holder>,        // every counted hold, any kind, any order, also while stepped back
    pub keep_awake: KeepAwake,     // the length PICKED (the row to check), not the time left
    pub screen_on: bool,           // shown only while keep_awake is not Off
    pub requests_on: bool,         // the system request is on now
    pub requests_only: bool,       // the episode holds with requests only (decision 29)
    pub stepped_back: bool,
    pub overridden: Vec<OwnedKey>, // the journal mirror's keys, any order
    pub tray: ProcessRef,          // Started.tray
}
pub fn build(snapshot: &Snapshot, now: Now) -> (TrayView, StateFile);
pub fn keep_awake_choice(expires_wall_ms: Option<i64>, now: Now) -> KeepAwake;
```

`Holder` maps one to one from `policy::hold_files::HoldAnswer` (`kind`, `label`, plus the facts' `expires_wall_ms`)
and from an agent session (`HoldKind::Agent`, `Some(label)`, `None`). `build` is pure: compare with the last answer
and send `Tray` or `WriteState` only on a change.

For `p2-core-holds`: keep the picked length in the state. A menu pick sets it. For a `keep-awake.json` the tray did not
write (the CLI, a script), call `keep_awake_choice` once when the file appears or its `expires` changes, and keep the
answer: the CLI writes now plus the length, so the shortest menu length that covers the time left gives the length
back. Set it to `Off` when the Keep awake hold ends. The tooltip's "N min left" changes once a minute: rebuild at
`policy::hold_files::keep_awake().next` (the next minute boundary), or at each safety tick (then at most 30 s late).

### What `build` does

- Icon: Holding when a hold counts and `requests_on`; Off when Automatic is off and nothing holds; Idle otherwise
  (stepped back, or a request that failed: nothing holds the PC then, decision 0).
- Groups: one per (kind, label) with a count, through `text::hold_group` (a hold file with no reason gets an empty
  label); Keep awake is one group through `text::keep_awake_group`, its minutes left rounded up and at least 1, or
  `UntilOff` with no `expires`. Order: `HoldKind` order (Agent, Run, Script, Keep awake), then label.
- Tooltip: `text::tooltip` with all groups; while over 127 UTF-16 units, drop the last whole group and add
  `text::more_groups(n)`. The status line (`text::status_line`, the menu's first row, a `Label`) gets the same shown
  groups. `state.json` `holding` gets all groups, uncut.
- `text::Status`: the icon, `stepped_back`, `requests_only`, and `screen_on` only while Keep awake is on.
- Menu: status line, separator, Automatic (checked = on, sends `Automatic(!on)`), Keep awake submenu (the six choices,
  the picked one checked, each sends `KeepAwake(choice)`; separator; Keep screen on, checked = on, sends
  `KeepScreenOn(!on)`, enabled only while Keep awake is on), separator, Restore, Open log, Quit (always enabled).
- `StateFile`: version 1, the tray's pid and start, Automatic, `holding`, `overridden` sorted with repeats dropped,
  `stepped_back`.

### Design choices (mine, as designer)

| Choice | Why | Change it when |
|---|---|---|
| `keep_awake` is the length picked | the user sees what they chose ("1 hour") next to the time left in the tooltip; a check that follows the time left would move from 1 hour to 30 minutes on its own | the core's pick state causes bugs, or Haik prefers the stateless rule (check the shortest length that covers the time left, from the hold alone) |
| A length no row has checks no row | a wrong check is worse than none; only a script can set such a length | never, unless the menu gets more lengths |
| `requests_on` in the snapshot | the card's icon rule needs it; `stepped_back` alone misses a failed request | a contract gives the request state another way |
| Status line shows the same cut groups as the tooltip | many long reasons never make the menu wide, and both say the same | the menu should list every hold (one row per group) |
| `holding` = group texts, uncut | the CLI then needs no words for counts, fallbacks and Keep awake | the CLI needs raw labels |
| A separator before Keep screen on | it is an on and off option, not a seventh length | Haik's look (`p5-haik-check`) says it reads better without |
| Restore always enabled | the user's way out; it also stops overriding for the rest of the work, which counts before any write | never |
| Keep screen on shows off and greyed while Keep awake is Off, whatever `screen_on` says | track note 16: Keep awake Off clears it | the tray track changes note 16 |

### Checks

- `cargo test --locked -p wakeshot-core present`: 24 passed, 0 failed. Tests, written first (all 24 failed against
  `todo!()` bodies): `nothing_holds_with_automatic_on_gives_idle_and_no_groups`,
  `nothing_holds_with_automatic_off_gives_off`, `one_hold_with_the_request_on_gives_holding_and_one_group`,
  `keep_awake_with_automatic_off_gives_holding`, `stepped_back_gives_idle_and_still_names_the_holds`,
  `a_hold_without_the_request_gives_idle`, `requests_only_goes_to_the_status`,
  `two_claude_sessions_make_one_group_of_two`, `many_holds_group_by_kind_then_label_whatever_the_input_order`,
  `a_hold_file_without_a_reason_gets_an_empty_label`, `a_long_reason_shows_whole`,
  `a_tooltip_over_the_limit_drops_whole_groups_and_counts_them`, `keep_awake_shows_the_minutes_left_rounded_up`,
  `keep_awake_at_its_last_moment_shows_one_minute`, `keep_awake_without_expires_shows_until_off`,
  `each_keep_awake_choice_checks_only_itself`, `a_length_the_menu_does_not_offer_checks_no_length`,
  `keep_screen_on_is_disabled_while_keep_awake_is_off`,
  `keep_screen_on_is_enabled_and_toggles_while_keep_awake_is_on`, `the_menu_follows_the_sketch_with_each_command`,
  `automatic_shows_its_state_and_sends_the_other`, `state_file_lists_overridden_keys_sorted_once`,
  `state_file_round_trips_through_serde_json`, `keep_awake_choice_for_a_file_the_tray_did_not_write`. Every expected
  string comes from `text::`; the tooltip checks find the longest cut that fits rather than fixed words, so
  `p5-text` wording changes do not break them.
- `cargo test --locked -p wakeshot-core`: 245 passed, 0 failed (an earlier run had 59 failures, all in
  `p2-policy-power`'s files while it was half done).
- `rustfmt --edition 2024 --check crates/wakeshot-core/src/present.rs`: no diff.
- `cargo clippy --locked -p wakeshot-core --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0.
- String literals: `present.rs` has no `"` character before its `#[cfg(test)]` line, comments and docs included.
- Only `present.rs` changed in `crates/`; the card and the `docs/research.md` addition are the docs.
- Build folder `<scratchpad>\p2-present` deleted.

Open items: the requests to `p5-text` and `p5-tray-icon` under "Found outside scope" 1 to 4; `p2-core-holds` keeps
the picked Keep awake length (item 5).
