# Make the three chimes and the player that plays them

Phase: 5. Status: ready. Needs Haik: his answer to "Do the chimes honor the Windows No sounds scheme?"
(`p5-plan-tray-ux`, Questions; default yes, taken if no answer has come); his ears in `p5-haik-check`. Agent:
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
   - The "No sounds" scheme, per Haik's answer: with the default, read the current scheme name
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

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03: card written by `p5-plan-tray-ux`.

## Result

What was done, files changed, checks and outcomes, open items. Include the public API as built and each chime's shape.
