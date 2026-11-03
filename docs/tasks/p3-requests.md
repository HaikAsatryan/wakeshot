# Power requests: set, clear, retake

Phase: 3. Status: done. Needs Haik: no. Real laptop: no (one test holds a SYSTEM request for under a second; no
setting changes). Agent: `wakeshot-worker`. Size: about 60K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-requests\target`.

## Goal

Fill `power::requests`: the one request object per tray that runs `Command::Requests` and `Command::Retake`, with a
dry-run form and a fake.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1, 2, 8).
- `docs/architecture.md`: section 5 "Power requests"; section 3 (`Command::Requests`, `Retake`, `Outcome::Requests`).
- `docs/product.md`: "Power behavior", first two points.
- `docs/research.md`: "Lid and sleep spike", the "Resume" and "SystemExecutionState" points.
- `spikes/lid-sleep/src/power.rs`: `Hold::request`, `retake`. Port by reading and rewriting.

## Deliverables

`crates/wakeshot-windows/src/power/requests.rs`:

- Trait `RequestApi { set(system, display) -> Result<(), WinError>; retake() -> Result<(), WinError> }`; `retake`
  answers as `Outcome::Requests` (review m7).
- `PowerRequests`: one `PowerCreateRequest` with a simple-string reason "Wakeshot: keeping the PC awake for running
  work", created on first use; `system` sets or clears SYSTEM and EXECUTION together, `display` sets or clears
  DISPLAY; only changes what differs. `retake`: for each type that is on, clear then set, same handle (the tested
  form). A failure part way: clear what this call set, return the error. Drop clears everything and closes the handle.
- `DryRunRequests` (logs each change through `Log`, calls nothing) and `FakeRequests` (behind `test-support`; records
  calls, scripted failures).

## Acceptance checks

Tests in `power::requests::tests`:

- With a recording backend: `set_changes_only_what_differs`, `display_only_when_asked`, `retake_clears_then_sets_each_on_type`,
  `retake_with_nothing_on_does_nothing`, `failure_part_way_rolls_back`, `drop_clears_all`.
- `request_sets_and_clears` (not ignored: under a second, nothing persistent; no `real_` prefix, since `real_` means
  ignored, review M9): create, set SYSTEM, retake, clear; every call returns `Ok`. Never judge by `powercfg
  /requests` (section 5).

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows power::requests::`. Record each outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- When requests go on or off (core), resume detection (`p3-system-events`), the battery re-take check
  (`p3-haik-checks`).
- Any file but `power/requests.rs`.

## Questions for Haik

None.

## Found outside scope

- `testing.rs` (contracts card) does not re-export `FakeRequests` and `RequestCall`; they are reachable as
  `wakeshot_windows::power::requests::{FakeRequests, RequestCall}` with `test-support`. Add a `pub use` there if the
  house rule is one place for every fake.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03 20:30: started by `wakeshot-worker` (bundle with `p3-session`, this card first).
- 2026-11-03 20:35 (about): read the card's sources and the Microsoft docs of `PowerCreateRequest` and `PowerSetRequest`
  (`PowerSetRequest` adds one to a count per type). Design: a private `Calls` seam (create, set, clear, close) under
  one shared rule engine; `PowerRequests` runs it on Win32, `DryRunRequests` on a logger, the tests on a recorder.
- 2026-11-03 20:38: `requests.rs` written; 13 tests pass, the real one included. rustfmt (2024) applied to the owned
  file only. clippy clean on both targets, with and without `--all-targets`.
- 2026-11-03 20:40: research note "Power request checks" added. Card done.

## Result

Done 2026-11-03.

What was built, in `crates/wakeshot-windows/src/power/requests.rs` (the only code file changed):

- `pub trait RequestApi { fn set(&mut self, system: bool, display: bool) -> Result<(), WinError>; fn retake(&mut self)
  -> Result<(), WinError>; }`. The runner answers both with `Outcome::Requests(result.map_err(Failure::from))`.
- `PowerRequests::new()` (also `Default`; `Send + Sync`): one `PowerCreateRequest` with the simple-string reason
  "Wakeshot: keeping the PC awake for running work", made at the first call that sets a type. The handle is an
  `OwnedHandle`; the reason text lives as long as it.
- `DryRunRequests::new(log: Arc<dyn Log>)`: same rules, logs each call it would make (`power` area, Info,
  "dry run: PowerSetRequest SYSTEM"), makes none, always `Ok`.
- `FakeRequests { pub calls: Vec<RequestCall>, pub results: VecDeque<Result<(), WinError>> }` and
  `RequestCall::{Set { system, display }, Retake}`, behind `test-support`: each call takes the next result, `Ok` once
  empty.

Design choices (mine, as principal designer):

- One private rule engine over a private `Calls` seam (create, set, clear, close) runs the real calls, the dry-run
  logger and the test recorder, so the dry-run log and the tests show exactly the calls a real run makes. Change it
  when a caller needs the low-level calls itself.
- Each type is set at most once: `PowerSetRequest` adds to a count (Microsoft docs), so a second set would need a
  second clear.
- Order: clears before sets, SYSTEM, EXECUTION, DISPLAY. Clears first, so on a failure the PC holds less, never more:
  the core ignores a failed release (`core.rs`, `done`). Every clear is tried even after one fails, and a failed clear
  skips the sets of that call. A failed set clears what the call set. A type whose clear failed still counts as on,
  so the next call or the drop clears it again. A failed create is tried again at the next call. Change it when a
  test shows `PowerClearRequest` failing on a live handle.
- `retake` is the spike's tested form: clear every on type, then set them again, same handle (H4). A failed retake
  leaves nothing it set: the PC then sleeps as usual and the core shows `CannotHold`.
- Drop: clears each on type, ignores a failed clear (nobody is left to act on it), then closes the handle.

Checks, all run 2026-11-03 with the card's build folder:

- `rustfmt --edition 2024 --check crates/wakeshot-windows/src/power/requests.rs`: clean.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo test --locked -p wakeshot-windows power::requests::`: 13 passed, 0 ignored. The card's seven:
  `set_changes_only_what_differs`, `display_only_when_asked`, `retake_clears_then_sets_each_on_type`,
  `retake_with_nothing_on_does_nothing`, `failure_part_way_rolls_back`, `drop_clears_all`, `request_sets_and_clears`
  (real: create, set SYSTEM, retake, clear; every call `Ok`; well under a second). Six more: `clears_run_before_sets`,
  `failed_clear_keeps_the_type_on`, `failed_create_is_tried_again`, `dry_run_logs_each_call_and_makes_none`,
  `fake_records_calls_and_scripted_failures`, `win32_errors_keep_their_code`.
- The orchestrator's own SYSTEM request was not touched: the real test only uses its own handle.

Docs: `docs/research.md` "Power request checks" (the count rule, the close, the real test; the reason text copy and
whether a close alone ends requests are marked not verified).

Open items: the `testing.rs` re-export under "Found outside scope". Build folder shared with `p3-session` (orchestrator
brief); deleted at the end of that card.
