# Make the three chimes and the player that plays them

Phase: 5. Status: blocked: waiting for the sound slot (Part A done). Needs Haik: his ears in `p5-haik-check`. "Do the chimes honor the Windows No sounds
scheme?" (`p5-plan-tray-ux`, Questions) is answered: yes, the default (Haik 2026-11-03 18:09). Agent:
`wakeshot-worker`. Size: about 120K tokens.
Real laptop: **sound** in Part B (one screen or sound card at a time, across all tracks). Build folder:
`CARGO_TARGET_DIR=<scratchpad>\p5-sounds\target`.
Runs after: the contracts change card (`SoundPlayer: Send + Sync` with `play_and_wait`, the event sink). Parallel with
(Part A): any card that does not own `sound.rs`; Part B only with cards that use no screen, sound or power settings.

## Goal

Wakeshot's three chimes (lid closed while holding, going to sleep, battery floor), made in code, and the player that
plays them on the right device without ever blocking the caller, plus a waiting form for the hibernate thread.

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work"; `docs/phases/05-tray-ux.md` (track notes 2 to 5).
- `docs/product.md`: the sound lines under "Power behavior" (which moments, the device rule).
- `docs/architecture.md` section 2 (`SoundPlayer` and the device rule), section 4 "Tray threads" (Sound row), section
  5 "Lock and hibernate" (the going-to-sleep sound and how long the hibernate thread waits), section 8 ("Sound fails:
  log only").
- `docs/research.md`, "Lid-close sound" (all of it: start delay, levels, form factors, loopback as an agent check).
- `docs/tasks/p1-lid-sound.md` Result and open items; `docs/reviews/p1-contracts-domain.md` M5.
- `spikes/lid-sleep/src/sound.rs`: `pick`, `chime`, `play_chime_inner`, the form factor reads, the listener (for the
  Part B check only). Port by reading and rewriting.
- `crates/wakeshot-windows/src/sound.rs` (`SoundPlayer` and its doc), the event sink; `crates/wakeshot-core/src/view.rs`
  (`Cue`), `command.rs` (`Outcome::Sound`).

## Owns

`crates/wakeshot-windows/src/sound.rs` and any child module files it declares under `crates/wakeshot-windows/src/sound/`
(with `FakeSoundPlayer` if the contracts card moved it there; its public API stays as the contracts card left it);
`crates/wakeshot-windows/examples/ux_sounds.rs`.

## Deliverables

`FakeSoundPlayer` already lives in `sound.rs` (the contracts card put it there, re-exported from `testing.rs`); its API
and the `SoundPlayer` trait are contract: add, never change (`p1-contracts-split` edit 7).

Part A (no laptop):

1. Chimes as PCM made in code, each with a short lead silence (the tested 150 ms), soft fades in and out (no click),
   peak at -6 dBFS, under 1.2 s, and told apart by pitch direction and rhythm:
   - `LidClosedHolding`: the tested two-tone rising chime (1 kHz then 1.6 kHz, 0.75 s), which Haik heard at two closes;
   - `GoingToSleep`: falling, softer;
   - `BatteryFloor`: low, two short pulses.
   Final pitches and lengths are yours; Haik judges them by ear in `p5-haik-check`.
2. Device rule, ported: the default render device, unless its form factor is display audio (`DigitalAudioDisplayDevice`,
   9); then the active endpoint with form factor Speakers (1); else the default. Headphones and headsets stay.
3. `WasapiPlayer::new(sink: Arc<dyn EventSink>)`, `Send + Sync`:
   - `play(cue)` returns at once; one short thread per sound (COM initialized on it) opens a shared-mode stream on the
     picked endpoint in its mix format, plays the chime, and posts `Event::Done(Outcome::Sound(..))` through the sink.
   - `play_and_wait(cue)` blocks until the chime has played out or 3 s have passed (the wait the design leaves to this
     track; the Phase 0 start delay was up to 0.8 s), for the hibernate thread only.
   - The "No sounds" scheme, per Haik's answer (honor it): read the current scheme name
     (`HKCU\AppEvents\Schemes`; which value means No sounds is from Microsoft docs, or read on this PC, or "not
     verified"), and when it is No sounds play nothing and report it as skipped (the contracts card's sound outcome
     type), so the log says why.
   - A stream layer behind a small private trait, so tests drive `play` and `play_and_wait` without audio.
4. Unit tests, written first: `chimes_have_lead_silence_fades_and_peak`, `chimes_differ_in_pitch_direction` (count zero
   crossings per segment), `chimes_fit_the_wait_limit`, `device_rule_picks_speakers_for_display_audio` (table: display
   default with speakers, display default with no speakers, headphones default, speakers default),
   `no_sounds_scheme_is_detected` (pure, from the scheme name), `play_never_blocks` (a fake stream that takes 1 s;
   `play` returns within 50 ms), `play_and_wait_stops_at_the_limit` (a fake stream that never ends),
   `result_is_posted_as_done_sound` (the fake sink), `player_is_send_and_sync` (compile-time).
5. Example `ux_sounds`: plays one cue (`--cue lid|sleep|floor`), all three 3 s apart (`--all`), or one with
   `play_and_wait` (`--wait`), and prints the picked endpoint's name and form factor and each result with the wall
   time.

Part B (sound slot; set Status `blocked: waiting for the sound slot` after Part A and stop):

6. Reads first (reads only, like `p1-lid-sound`'s snapshot): default render device per role, its volume and mute,
   form factors, the sound scheme. No change to any of them.
7. Play each cue with `ux_sounds` while a loopback capture runs on the picked endpoint (a scratch tool ported from the
   lid spike's listener, outside the repo; no loopback code in `sound.rs`). Record per cue: endpoint, time from call to
   a running stream, length and peak seen in loopback. `--wait`: returns after the chime, under 3 s.
8. Reads after: equal to the reads before. No process of the card left.
9. `docs/research.md`: a new section "Tray UX: chimes" (shapes, start delays, the scheme rule and its source).

## Acceptance checks

Run from the repo root with the build folder above. Record each outcome.

- `cargo test --locked -p wakeshot-windows sound`: all pass; list the names in the Result.
- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with `-D
  warnings` for both targets; `cargo build --release --locked -p wakeshot-windows` for both targets: each clean in the
  owned files.
- Part B: each cue seen in loopback on the picked endpoint, full length; the reads after equal the reads before.
- Only the owned files and the new `docs/research.md` section changed.
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- When a chime plays (core track); the hibernate thread that calls `play_and_wait` (Phase 3); the lock at lid close.
- Keeping a stream ready during a hold (track note 3.7). Any change to Haik's volume, device or scheme. Checklist 16
  (a real lid close; Phase 3 or integration, after this card). Haik's ears (`p5-haik-check`).

## Questions for Haik

Each with the worker's default.

None. The open calls are decided by the worker and listed for Haik's review in the Result.

## Needs another card

- `p6-int-tray-mode`: build one player, `Arc::new(WasapiPlayer::new(sink.clone(), log.clone()))`, and hand the same
  `Arc<dyn SoundPlayer>` to the loop runner (`LoopRunners::play` calls `player.play(cue)`) and to the hibernate thread
  (`run_hibernate(.., &*sound, ..)`). No file edit needed; the card's text (Deliverable 3) said `new(sink)`.

## Found outside scope

Problems seen but not fixed.

1. `spikes/lid-sleep/src/sound.rs` `prop_string` never clears the `PROPVARIANT` it reads (windows-rs 0.62 has no
   `Drop` for it), so each name read leaks a little. Spike only; the ported code clears it.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.
- 2026-11-03 21:26, orchestrator, from `p3-hibernate`'s Result: keep the "going to sleep" sound short; a lid opened
  while it plays still hibernates (the hibernate checks now run before the sound).
- 2026-11-04 01:42, worker: Part A started. Reading list first.
- 2026-11-04, before 01:54 (clock not read), worker: read done. Plan: `sound.rs` keeps the trait and fake, adds
  `sound/chime.rs` (PCM in code), `sound/device.rs` (device rule and endpoint reads), `sound/player.rs`
  (`WasapiPlayer`, the private `Audio` trait, scheme read, WASAPI stream). The core says a sound's runner logs it (`core.rs` "Their runners log them"), so
  the player takes a `Log` too: `WasapiPlayer::new(sink, log)`. Scheme read on this PC: `HKCU\AppEvents\Schemes`
  default `.Default`; `Names\.None` is `@mmres.dll,-801`, which loads as "No Sounds". No Microsoft page found.
- 2026-11-04 01:54, worker: `sound/chime.rs`, `sound/device.rs`, `sound/player.rs` written, `sound.rs` declares
  them. `cargo test --locked -p wakeshot-windows sound`: 18 passed (15 new in `sound::`). Next: `ux_sounds`, then
  fmt, clippy and release builds.
- 2026-11-04 01:59, worker: `examples/ux_sounds.rs` written. Owned files formatted with `rustfmt --edition 2024`.
  One clippy hit (`type_complexity` in my device test table), fixed. All Part A checks pass (Result). New section
  "Tray UX: chimes" in `docs/research.md` (Part A facts). No hit in other cards' files. Part A done; Status set to
  `blocked: waiting for the sound slot`. Build folder kept for Part B.
- Part B notes for whoever runs it: build `ux_sounds` into the same build folder (`cargo build --locked -p
  wakeshot-windows --example ux_sounds`); port the spike's `Listener` (`spikes/lid-sleep/src/sound.rs`, `listen`,
  `Segments`) into a scratch tool in the scratchpad, outside the repo, as the card says; the player's log line gives
  "stream running N ms after the call" per cue, and the example prints the picked endpoint's name and form factor.

## Result

Part A: done 2026-11-04 01:59. Part B: waiting for the sound slot.

### Part A

What was built:

- Chimes made in code (`sound/chime.rs`), rendered at the stream's own rate, mono, normalized to their peak. Each has
  150 ms lead silence, 100 ms tail silence, raised-cosine fades on every note (no click):

  | Cue | Notes | Peak | Fades in / out | Length |
  |---|---|---|---|---|
  | `LidClosedHolding` | 1046.5 Hz 160 ms, then 1568 Hz 340 ms; rising (the tested chime) | -6 dBFS | 5 / 10 ms | 750 ms |
  | `GoingToSleep` | 1568 Hz 170 ms, then 1046.5 Hz 380 ms; falling, the lid chime turned around | -9 dBFS | 15 / 30 ms | 800 ms |
  | `BatteryFloor` | two pulses, 659.25 Hz (E5) 130 ms, 100 ms gap, same pitch, soft 2nd harmonic | -6 dBFS | 5 / 15 ms | 610 ms |

- Device rule (`sound/device.rs`), ported: the console-role default; if its form factor is display audio (9), the
  first active Speakers (1) endpoint; else the default. No default: Speakers, else the first active endpoint.
- `WasapiPlayer` (`sound/player.rs`): one `wakeshot-sound` thread per sound with COM (MTA) on it. It reads the
  scheme, picks the device, opens a shared-mode stream in the endpoint's mix format, feeds and drains the buffer, and
  logs one line. `play` posts `Done(Sound(..))`. `play_and_wait` returns its result, or `Err(sound wait, 1460)` at
  3 s. The private `Audio` trait (scheme read, stream) lets tests run without audio.
- "No sounds": `HKCU\AppEvents\Schemes` default value `.None` (case ignored) means skip: `Ok(SoundEnd::Skipped)`
  and the log line "<cue> chime skipped: the sound scheme is No sounds". The source: research "Tray UX: chimes".
- Example `ux_sounds [--cue lid|sleep|floor] [--all] [--wait]`: prints the scheme, the picked endpoint's name, form
  factor and reason, the player's log lines and each result, with the wall time.

Public API (`wakeshot_windows::sound`), for `p6-int-tray-mode` and the hibernate thread:

- `WasapiPlayer::new(sink: Arc<dyn EventSink>, log: Arc<dyn Log>) -> WasapiPlayer`, `Send + Sync`, implements
  `SoundPlayer`. `play(cue)` returns at once and posts `Event::Done(Outcome::Sound(Result<SoundEnd, Failure>))`.
  `play_and_wait(cue) -> Result<SoundEnd, WinError>`: `Ok(Played)` once the engine took the whole chime,
  `Ok(Skipped)` under No sounds, `Err(WinError { call: "sound wait", code: 1460 })` after 3 s (the thread plays on).
  Both write one `sound` log line per chime (Info, or Warn on failure; a lost post is a Warn too).
- For harnesses and logs: `picked_endpoint() -> Result<(Endpoint, PickWhy), WinError>`, `current_scheme() ->
  Option<String>`, `pick(default_id, &[Endpoint]) -> Option<(usize, PickWhy)>`, `form_factor_name`,
  `Endpoint { id, name, form_factor: Option<i32> }`, `PickWhy::text()`.
- The trait and `FakeSoundPlayer` are unchanged.

Files: new `crates/wakeshot-windows/src/sound/chime.rs`, `sound/device.rs`, `sound/player.rs`,
`crates/wakeshot-windows/examples/ux_sounds.rs`; changed `crates/wakeshot-windows/src/sound.rs` (module lines and
re-exports only); `docs/research.md` (new section "Tray UX: chimes"); this card.

Checks (2026-11-04, all by 01:59, build folder `<scratchpad>\p5-sounds\target`):

- `cargo test --locked -p wakeshot-windows sound`: 18 passed, 0 failed. New: `sound::chime::tests::`
  `chimes_have_lead_silence_fades_and_peak`, `chimes_differ_in_pitch_direction`, `chimes_fit_the_wait_limit`,
  `the_lid_chime_is_the_tested_one`; `sound::device::tests::` `device_rule_picks_speakers_for_display_audio`,
  `an_unreadable_form_factor_counts_as_neither`; `sound::player::tests::` `no_sounds_scheme_is_detected`,
  `play_never_blocks`, `play_and_wait_stops_at_the_limit`, `play_and_wait_returns_once_played`,
  `result_is_posted_as_done_sound`, `a_lost_post_is_logged`, `player_is_send_and_sync`,
  `mix_formats_are_written_or_left_to_the_engine`, `samples_fill_every_channel`. Also matched and passing:
  `power::hibernate::tests::sound_failure_still_hibernates`, `sound_plays_only_when_all_checks_pass`,
  `testing::tests::fake_sound_player_posts_through_the_sink`.
- `rustfmt --edition 2024 --check` on the five owned Rust files: clean. All LF.
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four clean.
- `cargo build --release --locked -p wakeshot-windows --target <t>`, both targets: clean.
- Only owned files changed (above).

Decided by the worker, for Haik's review:

1. The player takes a log: `new(sink, log)`, not `new(sink)`. Why: the core leaves a sound's log line to its runner
   (`core.rs`, "Their runners log them"), and a lost post can only be logged by the player. Change it when the
   integration wants one place that logs every runner's result.
2. "Going to sleep" is the lid chime turned around and 3 dB softer (-9 dBFS), 0.8 s. Why: rising "stays on" against
   falling "goes off" is easy to tell apart, and it may play at night. Change it when Haik hears it as too quiet in
   `p5-haik-check`.
3. "Battery floor" is two short pulses on E5 (659 Hz) with a soft second harmonic. Why: the card asks for low, and
   small laptop speakers are weak below about 1 kHz, so the harmonic makes it carry. Change it when Haik cannot hear
   it with the lid closed.
4. The lid chime is the tested one, with its peak set to exactly -6 dBFS (it was about -6.4).
5. Log lines name the device's form factor and why it was picked, never its name. Why: a Bluetooth headset is often
   named after its owner, and logs get shared in bug reports. Change it when a support case needs the name.
6. Mix formats: float32, 16-bit and 32-bit PCM (also 24 bits in 32) are written as they are; any other format plays
   the tested 16-bit mono 48 kHz with Windows converting it. Why: the card asks for the mix format, and the fallback
   is the one heard at two lid closes.
7. Only `.None` skips. An unreadable scheme, `.Current` (a changed scheme) or a saved scheme plays. Why: only a clear
   "No sounds" should keep Wakeshot quiet.
8. At the 3 s limit `play_and_wait` returns `Err(sound wait, 1460)` and the stream thread plays on; the hibernate
   cuts it. A stream that has not drained 2 s after the chime should have ended gives up with
   `IAudioClient::GetCurrentPadding` 1460.
9. The default device is the console role, the one Windows plays system sounds on (as the spike).

Open items: Part B (loopback per cue, reads before and after, the `--wait` check, Part B facts in research).
