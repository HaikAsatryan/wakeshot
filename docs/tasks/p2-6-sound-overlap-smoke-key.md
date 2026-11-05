# One chime at a lid close plus a floor unplug; the smoke test checks the notice key; ARM64 checks with clang

Phase: 2 to 6 (core and ship fixes). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K
tokens. Written by the orchestrator, 2026-11-05, from `p2-6-floor-grace` "Found outside scope" 4 and
`p2-6-docs-pass` "Found outside scope" 1 and 2. Haik's answer (2026-11-05): fix both now.
Runs after: `p6-screen-reruns` and `p6-notice-app-icon` Part B (that session builds from the tree and owns
`packaging/` meanwhile). Part C also after Haik's clang install (`p6-velopack-dependency` "Steps for Haik").
Parallel with: nothing (Part C runs the whole-workspace checks).
Real laptop: no (unit and property tests; `smoke-test.ps1` is run against a local pack in a temp install only if the
script itself allows it on a dev laptop, else CI's release dry run checks it).
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p2-6-sound-overlap-smoke-key\target`.

## Goal

Three small fixes before the first release: a single sound where two overlap today, a CI check that catches a hook
that stops registering the notice app id, and the ARM64 checks on this laptop once clang is in.

## Read first

- `docs/tasks/p2-6-floor-grace.md`: "Found outside scope" 4, Result.
- `crates/wakeshot-core/src/core/lid.rs`: `settle_window_lock`, `settle_source_lock`; the props that hold
  `lid_lock_sent`.
- `docs/tasks/p2-6-docs-pass.md`: "Found outside scope" 1 and 2.
- `packaging/smoke-test.ps1`, `packaging/e2e.ps1` (how it checks the key and the PNG), `packaging/README.md` "What
  the smoke test checks".
- `docs/tasks/p6-velopack-dependency.md`: step 0 and the ARM64 items.

## Deliverables

### Part A: one chime (core)

- `crates/wakeshot-core/src/core/lid.rs`: a lid close in the unknown journal window plus an unplug under the floor
  plays one sound, not the lid chime and the floor chime in the same pass. Pass the step back's state to
  `settle_window_lock` as `settle_source_lock` gets it. The lock still goes out before the restore. Which sound stays:
  the floor chime, as `settle_source_lock` does for an unplug under the floor (`docs/product.md` sounds list).
- A scenario test for that case, and the same `lid_lock_sent` rule in the props.

### Part B: the smoke test checks the notice app id

- `packaging/smoke-test.ps1`: after install, the key
  `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot` with `DisplayName` and `IconUri`, and
  `current\wakeshot-notice.png`; after uninstall, the key gone; the key in its refusal list of traces. Port the checks
  from `e2e.ps1`.
- `packaging/README.md`: "What the smoke test checks" names them; line 81 wrapped to 120 characters.

### Part C: ARM64 with clang (`p6-velopack-dependency` step 0, after Haik)

- If `vswhere -all -products * -find 'VC\Tools\Llvm\**\clang.exe'` still finds nothing, skip Part C and say so.
- Else: the two checks of step 0 and `clang --version` in `docs/research.md` (additions only), then the card's ARM64
  items: clippy and the release build of the workspace for `aarch64-pc-windows-msvc`. Record the outcome in the
  `p6-velopack-dependency` Result and set its Status line to done without the open clang item if they pass.

## Acceptance checks

- `cargo fmt --all --check`; clippy with and without `--all-targets`, `-D warnings`, on x64, and on ARM64 if Part C
  runs; `cargo test --workspace --locked`; core props at 10000 cases, three runs.
- A hand mutant: revert the Part A fix and see the new scenario test fail.
- `smoke-test.ps1` parses (`[System.Management.Automation.Language.Parser]::ParseFile`), and passes on a local pack
  if the script allows a run here; else say CI's release dry run is its first real run.

## Out of scope

- Any other sound, lid or floor rule. `release.yml`, `ci.yml`.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. `docs/architecture.md` lines 368 to 369 (the lock a lid close in the unknown window did not get): "It comes, with
   the chime, once the journal is known ..." is now wrong for one case. Suggested words after "with the chime": "(with
   the battery floor sound instead when a floor step back starts in the same pass, as for the lock after an unplug)".
   Not a file of this card. `docs/product.md`'s sounds list already covers it ("under the battery floor the lock comes
   with the battery floor sound instead").
2. By reading only, not tested: a lid close handled in the same core pass in which a floor step back first shows
   still plays two sounds: `take_lid` locks with the lid chime before the settle pass, then `settle_lid` plays the
   floor sound. It needs the `Lid` event to land after the 30 s grace ends but before the timer's own pass (a posted
   message runs before `WM_TIMER`), so a close within milliseconds of the grace's end. Not seen in four runs of
   `no_step_plays_two_sounds` at 10000 cases. Out of scope ("any other sound, lid or floor rule"); one overlap at most.
3. `docs/tasks/p6-velopack-dependency.md`: its "Steps for Haik" still say "Not done yet" (only its Result and Status
   were this card's). Its "Needs another card" item for the orchestrator is now due: a `CLAUDE.md` fact that ARM64
   builds of the tray need `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\Llvm\x64\bin` on
   that cargo command's PATH (`ring` calls `clang` by name; checked here).
4. `smoke-test.ps1` does not check the installed shortcut's `System.AppUserModel.ID` (`e2e.ps1` step 4 does). The key
   only helps when the id the tray runs under is `velopack.HaikAsatryan.Wakeshot`; a Velopack change of the id (an
   upgrade, or a `shortcutAumid` in `pack.ps1`) would pass every smoke check and lose the header again. Not ported:
   the card names the key and the PNG; whether `Shell.Application` `ExtendedProperty` works on a GitHub runner is not
   verified. Port it with the next `velopack` version change.
5. `packaging/README.md` says "Do not run `smoke-test.ps1` on a PC where Wakeshot is or was installed". This card ran
   it here (Result, decision 5), as the card and the orchestrator allowed when the script finds no trace. The hazard
   the sentence names (a journal in the data folder) is what the refusal checks. A wording such as "Run it only where
   it finds no trace; it refuses otherwise" would match practice; left for the orchestrator or Haik.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started; Part A first (lid.rs), then B, then C. Orchestrator: clang found by vswhere.
- 2026-11-05: Part A code in: `settle_lid` works out `step_back_shows` first and passes `!step_back_shows` to
  `settle_window_lock` as the chime, which locks through `closed_lid_lock` (own log line without the chime). New
  scenario `a_window_lock_due_with_a_floor_step_back_plays_only_the_floor_sound` (docked, closed in the window, a hold
  after it, unplug at 19 %): passes. Hand mutant (chime `true`): only that test fails, 2 plays for 1. Fix back in.
- 2026-11-05: props: `lid_lock_sent` already counts a new `Lock` with the floor sound as a lid lock, so its rule
  covers the window lock as is; its doc now names it. New property `no_step_plays_two_sounds` (`case` and `lid_case`):
  passes at 10000 cases, and also on the mutant: the mixes do not reach the window-plus-unplug chain, so the scenario
  is what pins the fix; the property guards any other two sounds in one step.
- 2026-11-05: Part A checks: fmt clean (after `cargo fmt -p wakeshot-core` on the new test); clippy core with and
  without `--all-targets` clean; `cargo test -p wakeshot-core --locked` 706 + 11 + 25 passed; `PROPTEST_CASES=10000`,
  release, `scenarios::props` and `lid_events_in_any_order`: 39 passed, three runs. Part B next.
- 2026-11-05: Part B code in: `smoke-test.ps1` (names from e2e.ps1, `Get-RegText`, the trace, the install checks, the
  key gone after uninstall); README refusal list and "What the smoke test checks". ParseFile: 0 errors. Read-only: no
  trace of Wakeshot on this laptop (all eight false), no Wakeshot process, so the script allows a run. Built x64
  release, packed 0.1.0 into scratch `pack-x64`, exported `HKCU\Environment` to scratch as a backup. Running the smoke
  test now (installs into `%LocalAppData%\HaikAsatryan.Wakeshot`, writes a Run value; its uninstall takes both back).
- 2026-11-05: smoke test on the local pack (elevated shell, as on a CI runner): 28 PASS, 0 FAIL, exit 0, the four new
  checks among them. After: all eight traces false, no Wakeshot process, no `HaikAsatryan.*` folder, no
  StartupApproved value; the script's own check says the user Path has its old bytes. Part C next.
- 2026-11-05: Part C. `Get-Command clang`: none; vswhere: two hits (`Llvm\ARM64\bin`, `Llvm\x64\bin`), exit 0; x64
  `clang --version`: 22.1.3. With `Llvm\x64\bin` on that command's PATH only: ARM64 workspace clippy with and without
  `--all-targets` exit 0 (80 s, 3 s), ARM64 release build exit 0 (68 s); tray 2,404,352, CLI 532,480; CRT check
  passed, header ARM64. x64: fmt, clippy both ways, `cargo test --workspace --locked` (1,423 passed, 0 failed, 55
  ignored) all exit 0. Research section "Sound overlap, smoke test key and ARM64 checks" added.
- 2026-11-05: `p6-velopack-dependency` Status and Result updated; build and pack folders deleted; Result written.

## Result

Done, 2026-11-05. Parts A, B and C all ran.

**Part A, one sound.** `settle_lid` now works out `step_back_shows` before `settle_window_lock` and passes
`!step_back_shows` as its chime, as `settle_source_lock` gets it; `settle_window_lock` locks through
`closed_lid_lock(now, chime, out)` with its own log line when the chime is left out. The `Lock` still goes out before
the step back's `Restore`. With the lid closed in the unknown journal window and an unplug under the floor, the pass
at the grace's end now plays only the battery floor sound. Scenario
`a_window_lock_due_with_a_floor_step_back_plays_only_the_floor_sound` (docked, the window ends, a hold writes
Wakeshot's battery Do nothing, unplug at 19 %): one `Lock` at 30 s to the millisecond, before the restore, one sound,
the step back notice. Props: `lid_lock_sent`'s rule already counts a new `Lock` with the floor sound, so it covers this
lock unchanged; its doc now names it. New property `no_step_plays_two_sounds`.

**Part B, the smoke test key.** `smoke-test.ps1`: the notice app id key in the refusal traces; after install,
`DisplayName` "Wakeshot" and `IconUri` naming `current\wakeshot-notice.png`, both `REG_EXPAND_SZ`, and that file
byte-equal to `assets/wakeshot-notice.png`; after uninstall, the key gone. `packaging/README.md`: the refusal list,
"What the smoke test checks" (old line 81 rewrapped to 120), and why the key check matters.

**Part C, ARM64 with clang.** vswhere finds clang 22.1.3 in the Build Tools. ARM64 workspace clippy (both forms) and
release build pass with `...\VC\Tools\Llvm\x64\bin` on that command's PATH; the ARM64 exes import no Visual C++
runtime. `p6-velopack-dependency`: Status done, Result has the ARM64 outcome, its clang open item closed.

**Decisions** (mine, as principal designer)

1. The floor sound stays, per the card and `docs/product.md`'s sounds list.
2. `no_step_plays_two_sounds` checks the card's goal on every generated world, though its mixes do not reach this
   chain (it passed on the mutant). Change it when a design wants two cues in one step.
3. `IconUri` is compared case-insensitively (`Test-SamePath`), where `e2e.ps1` uses `-ceq`: Windows paths ignore case,
   and on a CI runner the hook's path (from Velopack's launch) and `GetFolderPath` may differ in case (not verified
   either way). Change it when a case difference ever matters.
4. Types are read with .NET `GetValueKind` (`Get-RegText`), not a raw-bytes helper as in `e2e.ps1`: checked to give 2
   for a `REG_EXPAND_SZ` value here. Change it when exact bytes of these values matter.
5. The smoke test ran on this laptop: the script found none of its eight traces and no Wakeshot process, so it allowed
   the run, which the card and the orchestrator asked for in that case. Elevated, as on a CI runner. Every trace was
   gone afterwards. Change it when the script refuses here; then CI's release dry run is its first real run.

**Files changed**: `crates/wakeshot-core/src/core/lid.rs`, `crates/wakeshot-core/src/scenarios/lid.rs`,
`crates/wakeshot-core/src/scenarios/props.rs`, `packaging/smoke-test.ps1`, `packaging/README.md`, `docs/research.md`
(new section at the end), `docs/tasks/p6-velopack-dependency.md` (Status line, Result), this card.

**Checks**

- `cargo fmt --all --check`: exit 0.
- x64: `cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` and the same
  without `--all-targets`: exit 0. ARM64, the same two (LLVM folder on the PATH): exit 0.
- `cargo test --workspace --locked`: exit 0, 1,423 passed, 0 failed, 55 ignored.
- `cargo build --release --workspace --locked`: x64 exit 0; ARM64 exit 0 (tray 2,404,352 bytes, CLI 532,480). CRT
  check on the ARM64 exes: passed.
- Core props, `PROPTEST_CASES=10000`, release, `scenarios::props` and `lid_events_in_any_order`: 39 passed, three runs.
- Hand mutant (the window lock's chime back to `true`): the new scenario fails (2 plays for 1), every other test
  passes; fix restored and the suite rerun.
- `smoke-test.ps1`: `ParseFile` 0 errors; on a local 0.1.0 x64 pack: 28 passed, 0 failed, exit 0.

**Open items**: Found outside scope 1 to 5. The build and pack folders are deleted.
