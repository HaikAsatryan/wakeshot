# Core state machine, part 2: power settings in the episode

Phase: 2. Status: ready. Needs Haik: no. Question 10 (Phase 1 file, "New questions") is answered: the default
(restore and requests only while another session is on the PC's screen), Haik 2026-11-03 18:09
(`docs/phases/02-06-signoff.md` item 11); build it. Agent: `wakeshot-worker`. Size: about 150K tokens. Split trigger
(review m13: no margin): if the worker passes about 120K before the session end and console part, it stops at a clean
point and that part (session end, fast user switching, console left and back) becomes its own card, run before
`p2-core-lid`.
Real laptop: no (pure core code and tests). Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-core-settings\target`.
Runs after: `p2-core-holds`. Parallel with: `p2-ownership-props`, and any card of another track.

## Goal

Everything the core decides about power settings: when to `Apply` and `Restore`, the journal mirror, released keys,
plan switches, retries, the user's own actions, session end and fast user switching, with the notices that explain
them. The IO side runs the commands; this card tests with hand-fed `Done` outcomes.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/02-core.md` (track notes 3, 5, 7, 8, 9, 15).
- `docs/product.md` "User settings safety", "Power behavior" (what is written and when).
- `docs/architecture.md` section 3 (episode flow: hold start, while holding, hold end, restore retry, user actions;
  `ApplyError`; `Outcome`), section 4 (processes, the settings mutex, end of session, `wakeshot restore`), section 5
  "Power settings" and "Ownership" (what the IO side does with `Apply`, `Restore`, `ReadPlan`), section 8 (settings
  rows).
- `docs/phases/01-design-skeleton.md` Questions 10 to 12; `docs/phases/03-windows-power.md` notes 5 to 7, 10 (what
  the IO side does with unknown keys, released-key entries, a corrupt journal, a failed entry read).
- `docs/reviews/p1-design-power-verify.md` minors 4, 6, 9; `p1-contracts-domain.md` m4, m5, m7.
- `crates/wakeshot-core/src/` contract files; Result of `p2-core-holds` (seams, `Sim`).

## Owns

`crates/wakeshot-core/src/core.rs` (bodies), `state.rs`, `sim.rs`, `scenarios/settings.rs`, and `src/core/`
submodules. Nothing else.

## Deliverables

1. Apply at hold start (and at `SettingsOwner`, `ConsoleBack`, `EndCancelled`, after a plan switch) only while: this
   tray owns settings, its session is on the console, not stepped back, not at session end, and some key is not
   released. Writes: lid AC, lid DC, sleep DC to 0, less the released keys. No plan values yet: `ReadPlan` first
   (track note 15).
2. `ReadPlan` on every `Tick` and every 10 s (`Deadline::Mono`) while anything is owned. `Event::ActivePlan(id)`
   (contract C8, posted by `p3-system-events` within milliseconds of a switch; review M2): when `id` differs from the
   last `Plan`'s active id and anything is owned or a hold counts, send `ReadPlan` at once; never act on it alone (the
   `Plan` answer drives item 3). Test `active_plan_event_reads_plan`; mutant "ignore `ActivePlan`" must fail it.
3. `Plan(Ok)`: live values; a new active id is a plan switch, handled first: `Restore { plan: Some(old) }`, then
   `Apply` to the new plan. `Plan(Err)`: releases nothing.
4. Journal mirror: a copy of the `journal` in `Done(Applied)` and `Done(Restored)`, less the keys `UserChanged` names.
5. `UserChanged`: the key released on every plan for the episode (power verify minor 9); `Notify(UserChanged)`.
6. `Done(Applied)`: `SafetyNet` or `Write`: requests only for the episode, `Notify(RequestsOnly)` once; access denied
   (5 or 1260): for this run; `PlanChanged`: wait for the next plan read.
7. Holding ends: `Requests` off and `Restore { plan: None }` at once.
8. Retry of a failed restore on each `Tick`: while the core applies, only entries on other plans (one `Restore { plan:
   Some(p) }` each); the active plan's entries for released keys go with the next `Apply` there (track note 3);
   otherwise `Restore { plan: None }`. Never an entry a running hold uses (power verify minor 4).
   `Notify(RestoreFailed)` once per failure streak. Leftover journal from `Started`: `Restore { why: Leftover }` on
   each tick until it succeeds (track note 9).
9. `Event::JournalCorrupt`: `Notify(JournalCorrupt)`.
10. `Menu(Restore)` (menu or `restore-request.json`): `Restore { plan: None, why: UserAsked }`; all three keys released
    for the episode; requests kept; `Notify(RestoredByUser)`; after its `Done`: `FinishRestoreRequest` with the fresh
    `StateFile`, not debounced (track note 5).
11. `Menu(Quit)`: requests off, `Restore { why: Quit }`, `DeleteHold(keep-awake.json)`, `Quit`.
12. `Session(Ending)`: `Restore { why: SessionEnd }`; no `Apply` until `EndCancelled` (power verify minor 6).
    `Ended`: nothing more.
13. `Session(ConsoleLeft)`: `Restore { why: ConsoleLeft }`, the system request only, no display request;
    `ConsoleBack`: the display request if asked, `Apply` as at a hold start (Question 10: the default, Haik's answer).
14. `owner: false`: requests only; `SettingsOwner`: apply as at a hold start.
15. `Log` lines for each release and each requests-only switch.
16. `scenarios/settings.rs`, written before the code, one named test per item above and these cases: an apply in
    flight when the hold ends; a plan switch while an apply is in flight; `UserChanged` then a plan switch: the key
    stays released on the new plan; a restore that fails twice then works (one warning); a hold that starts while a
    restore fails (the retry skips the active plan's held entries); session end during a hold, then cancelled; console
    left and back during a hold; Restore my power settings, then a plan switch: nothing written until work ends; Quit
    with Keep awake on.
    Added by the orchestrator (`p2-ownership-restore` Found outside scope 3): a restore whose journal load fails
    reports `Outcome::Restored` with `result: Err` and `journal: []`, since nothing is known. Never read that empty
    list as "nothing owned": on `Restored` with `Err`, keep retrying while a restore is wanted, whatever `journal`
    holds. One scenario test for it.

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-core scenarios::settings` and `cargo test --locked -p wakeshot-core`: all pass
  (including `scenarios::holds`); list the new test names.
- `rustfmt --edition 2024 --check <owned files>`: no diff. `cargo clippy --locked -p wakeshot-core --all-targets
  --target x86_64-pc-windows-msvc -- -D warnings` and the same with `--target aarch64-pc-windows-msvc`: exit 0.
- No clock read, no I/O, no new dependency (as in `p2-core-holds`).
- Mutants, each made by hand, run, then reverted (record the failing test): key `released` by plan; let the retry
  take the active plan's entries while holding; apply after `Session(Ending)`. Each must fail a test. Mutants run
  only in a scratch copy of the workspace, never in the repo (`docs/phases/02-core.md` note 20).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.
- Only the owned files changed.

## Out of scope

- How the IO side runs `Apply`, `Restore`, `ReadPlan` (the ownership routines and the power track). Lid, lock, floor,
  countdown (`p2-core-lid`). Contract files, `lib.rs`, `Cargo.toml`, `docs/architecture.md`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p2-plan-core`.
- 2026-11-03 20:39, orchestrator, from `p2-ownership-apply`'s "Found outside scope" items 3 and 4:
  - An apply or check whose journal load fails or finds a corrupt file also reports `journal: []` (same rule as the
    restore above). Never read it as "nothing owned"; `p3-journal`'s lock timeout is the same case.
  - Orchestrator decision: after an apply fails with `SafetyNet(JOURNAL_CORRUPT)`, the core sends one
    `Restore { plan: None }`, so the routine sets the file aside and `JournalCorrupt` reaches the user, instead of
    every later apply failing. Once per corrupt file, not per apply. The power safety review checks it; the
    `architecture.md` line comes with the docs pass.

## Result

What was done, files changed, checks and outcomes, open items.
