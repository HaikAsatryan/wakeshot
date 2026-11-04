# Sleep, not hibernate, after work ends with the lid closed (and at the battery floor)

Phase: 2 to 6 (core and power). Status: ready. Needs Haik: no (his lid test comes later, in `p6-int-haik-checks`).
Agent: `wakeshot-worker`. Size: about 180K tokens. Written by the orchestrator (session `wakeshot-12`, 2026-11-04)
from Haik's answer of 2026-11-04 (`docs/product.md`, work end rule, battery floor, failed lock).
Runs after: `p2-6-record-lost` (same files) and `p3-idle-sleep-check` (its answers decide how Wakeshot sees that the
laptop slept). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-sleep-route\target`.

## Goal

Haik's rule: when Wakeshot is done, the laptop ends up where Windows would have put it; with the lid closed, that is
sleep. His words: "just make it sleep but if technically it's impossible to control then think about your best
effort but do not go to critical". On Modern Standby no app call starts sleep, so Wakeshot uses Windows' own idle
sleep, with hibernate only as the fallback and the built lock as the last step.

## Read first

- `docs/product.md`: the work end rule, battery floor, "Never lower security" (failed lock), sounds; "User settings
  safety".
- `docs/tasks/p3-idle-sleep-check.md` Result (how long idle sleep takes with the lid closed; what a desktop app sees
  at sleep and wake).
- `docs/tasks/p2-6-record-lost.md` Result (the countdown it starts after a lost journal).
- `crates/wakeshot-core/src/core/lid.rs` (work-end and floor countdowns, relock, hibernate result), `policy/lid.rs`,
  `policy/floor.rs`, `settings.rs` (`SettingKey`), `command.rs` (`Hibernate`, `HibernateEnd`), `event.rs`, `state.rs`,
  `text.rs` (`HibernateFailed`).
- `crates/wakeshot-windows/src/power/hibernate.rs`, `power/plan.rs` (`PlanSetting`, writes limited to `SettingKey`),
  `power/executor.rs`, `power/journal.rs` (key names in the file), `power/system.rs`, `app/dispatch.rs`.
- `docs/architecture.md` sections 3, 5 and 6.

## Deliverables

1. `SettingKey::SleepAc` (journal key `sleepAc`): written, journaled and restored like the other three. An older
   journal without it still loads.
2. The step after the countdown. Where the work-end countdown (5 minutes with no input, the checks, the "going to
   sleep" sound) now sends `Hibernate`, it instead sets the sleep timeout of the plan in use to 60 s on AC and on DC,
   each only where the value in force (the user's own after the restore) is longer or Never, as Wakeshot-owned
   journal entries. The same for the battery floor countdown and for the failed-lock step back.
3. Fallback. If 5 minutes after that write the laptop has not slept (use what `p3-idle-sleep-check` found to tell),
   the lid is still closed and no input came, send `Hibernate` as today (its own checks stay). If hibernate is not
   allowed or fails: the built relock and notice. The short timeout stays in force, so Windows sleeps the laptop once
   nothing holds it.
4. Restore of the short timeout: at lid open, at the first unlock after a wake (the user is there), when a hold
   starts, and at every other restore point (quit, sign-out, next start, uninstall, guard). Not at a wake alone: a
   wake with the lid still closed must sleep again. A user's own change to the timeout wins as today.
5. Words: `text.rs` notices that say "hibernate" for this path now say what happens (sleep, or hibernate as the
   fallback). Keep `text.rs` rules.
6. Tests first: core scenarios for work end, floor and failed lock (write, sleep seen, fallback, lid open, unlock
   after wake, a wake with the lid closed, a hold that starts, a user change); props updated; Windows unit tests with
   the fakes; journal round trip with `sleepAc`.
7. `docs/architecture.md` sections 3, 5 and 6: the lines that state these rules.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets`, x64 and ARM64 (the tray's ARM64: CI only);
  `cargo test --workspace --locked` (debug only); the core props at 4096 cases once.
- Mutants only with one job, a low timeout and a memory cap; stop if free memory drops under 10 GB.
- Haik's plan never written; no tray started; build folder deleted at the end.

## Out of scope

- The real lid test (`p6-int-haik-checks` gets it). `README.md`, phase files, `docs/product.md` (tell the
  orchestrator if a product line is wrong).

## Questions for Haik

None (his answer is in `docs/product.md`). Decide the rest yourself and record each choice as "decided by the
worker, for Haik's review".

## Found outside scope

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).

## Result
