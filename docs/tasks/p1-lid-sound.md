# Make Wakeshot's sounds audible, starting with lid close

Phase: 1. Status: done. Needs Haik: yes, about 3 minutes at the end: close the lid
twice and say what he heard. Agent: `wakeshot-worker`.
Runs after: Phase 0 sign-off. Parallel with: any Phase 1 card that does not use the screen, sound or power settings.

## Goal

Haik wants sounds wherever they give a useful hint (2026-11-03). The lid-close sound is the most important one, and
Phase 0 never showed it was heard: `PlaySound` returned true in D, E and I, but in D the default device was the DELL
monitor (no speakers), and in I, the only clean run, Haik heard nothing. Find why, and find a way that is heard every
time the laptop keeps running with the lid closed.

## Read first

- `docs/research.md`: the "Sound" lines (tray spike and lid spike results), the "Sound at lid close: Microsoft's rule"
  line, and the volume and audio device lines.
- `docs/reviews/p0-delta.md` M2 and section 4.
- `docs/tasks/p0-spike-lid-sleep.md` Result (D, E, I) and `spikes/lid-sleep/out/lid-20261103-114033.log` around the
  close.
- `docs/product.md`: lines on sound and notifications.

## Deliverables

1. Why I was silent. Check each, with a test where an agent can (no lid close needed for most): the System Sounds
   level in the volume mixer; `SND_SYSTEM` routing; whether the lock 2 ms after the sound cut it; whether Windows
   stops audio at the lid event or the standby entry (Microsoft's audio rule for screen off and explicit entry); whether
   the closed lid mutes or muffles the speakers on this laptop. A WASAPI loopback capture on the speaker endpoint
   shows whether audio really played, without a human ear.
2. Which device to play on. The default device can be a monitor with no speakers or muted headphones. Options: the
   default device only, or a fallback to the laptop speakers. Pick one with the reason.
3. A working method in the lid spike (`spikes/lid-sleep/`): own short WAV as a resource or a system sound, played at
   the right moment (for example before the lock, or with a short delay), on the picked device. Shown by loopback
   capture, then by Haik's ear.
4. A short list of the other moments where a sound would help (for example work ended and the PC will sleep, battery
   floor reached, restore failed), each with a default yes or no. Phase 5 builds them; this card only proposes.
5. Results in `docs/research.md` with how they were checked.

## Acceptance checks

- Loopback capture shows our sound playing at a lid close, with the lid spike's lock on.
- Haik hears it at two lid closes: one with the DELL connected (if it is the default device), one on the laptop
  alone.
- Haik's volume, default device and sound scheme are the same after the card as before (read and record both).

## Out of scope

- The sound design itself (Phase 5). Changing Haik's sound scheme or default device for good.

## Questions for Haik

Each with the worker's default.

1. Between 14:56 and 15:00 the agent played five short test sounds with the lid open (the soft Windows sound and a
   two-note chime). Did you hear them, and was the chime clear? Default: assume yes; the lid-close steps below
   settle it anyway. Answer (relayed 15:29): "Maybe I heard ... but do not remember already. At least once I
   heard." The closes settled it.

## Steps for Haik (two lid closes, about 3 minutes)

Done 15:16 to 15:20; agent steps after them done 15:30 (Log).

Before you start: the laptop is on battery with the DELL cable out (as at 15:02). Nothing else needs to be closed.
Each run makes a copy of your power plan with lid action Do nothing and puts your plan back at the end (as in the
morning runs). Each run locks the PC at the close.

Close 1, laptop alone:

1. Keep the DELL cable out. In File Explorer open `C:\Users\haika\code\wakeshot\spikes\lid-sleep` and double-click
   `sound-close-1.cmd`. A console window opens. If it says "refusing: elevated token", stop and tell the
   orchestrator.
2. Wait for the line `>>> READY: close the lid now <<<` (about 2 s).
3. Close the lid and listen. At the close you may hear the soft Windows sound (the morning one); about 3 s later our
   two-note chime should play.
4. About 10 s after the close, open the lid and sign in.
5. Wait until the window shows `DONE: plan restored` (about 20 s after the lid opened), then press a key to close
   it.
6. Note: Windows sound at the close: heard or not. Chime about 3 s later: heard or not, and clear or faint.

Close 2, DELL connected:

7. Plug in the DELL cable. Wait 30 s.
8. Double-click `sound-close-2.cmd` in the same folder. Wait for `>>> READY: close the lid now <<<`.
9. Close the lid and listen. Our chime should play at the close, from the laptop. About 3 s later the Windows sound
   goes to the default device; if that is the DELL, you will most likely hear nothing then.
10. About 10 s after the close, open the lid and sign in (on whichever screen shows the sign-in).
11. Wait for `DONE: plan restored`, then press a key to close the window. You may leave the DELL plugged in.
12. Note: chime at the close: heard or not, clear or faint. Anything about 3 s later: heard or not.

Tell the orchestrator your notes from steps 6 and 12, the answer to question 1, and whether the DELL is still
plugged in.

Agent steps after Haik's closes (next worker run): read the two newest `spikes/lid-sleep/out/lid-*.log` files
(`[listen]`, `[lid]`, `[audio]`, `[before]`/`[after]` lines); run `<scratch>\snap.ps1 after` and compare with
`<scratch>\snap-before.txt`. With the DELL plugged in, Windows makes it the default device by itself (seen at 10:21:
DELL `{...ca5b6419...}` default for all roles at 24.0%, Speakers 67.3%); compare against that state, or against the
BEFORE line if the DELL is out. Then fill the lid-close results in `docs/research.md` ("Lid-close sound") and this
card's Result.

## Found outside scope

Problems seen but not fixed.

- `spikes/tray/src/sound.rs` plays both its sounds with `PlaySound(... | SND_SYSTEM)`, so they go to the default
  device and are silent when that is the DELL (same cause as run D). Phase 5 should use this card's device rule for
  every sound.
- `docs/research.md`, "Morning results", Sound bullet ends with "Not checked: the System Sounds level ..., whether
  the lock 2 ms later cut the sound". The first is now answered in "Lid-close sound"; the bullet can point there once
  the lid-close results are in.

## Log

- 2026-11-03: card written by the orchestrator from Haik's correction ("I want sounds whenever it will hint us and
  will be good").
- 2026-11-03 14:37: worker started (Phase 0 signed off 14:35).
- 2026-11-03 14:39: screen holder `<scratch>\sound-screen-holder.ps1` (copy of the tray card's, reason "Wakeshot lid
  sound card agent steps: screen holder") started, PID 19924. The laptop is on battery, so it holds nothing and ends
  by itself after 60 s on battery. (`<scratch>` = this worker's scratchpad
  `C:\Users\haika\AppData\Local\Temp\claude\C--Users-haika-code-wakeshot\4fa582c2-fd10-483c-8e4d-a373ec22347e\scratchpad`.)
- 2026-11-03 14:40 BEFORE (`<scratch>\snap.ps1`, reads only, saved as `<scratch>\snap-before.txt`): on battery 95%,
  DELL not connected. Default playback for all three roles: Speakers (Realtek)
  `{0.0.0.00000000}.{fb152119-7d6e-4352-a9bf-882ae54afa03}`, 67.3%, not muted; it is the only active render endpoint.
  Sound scheme `.Default`; SystemNotification plays `C:\WINDOWS\media\Windows Background.wav`. Balanced active and
  the only plan; LIDACTION 1/1, STANDBYIDLE 10800/600, VIDEOIDLE 300/180, CONSOLELOCK 1/1.
- 2026-11-03 14:55: new `spikes/lid-sleep/src/sound.rs` (device pick, own chime through WASAPI, loopback listener,
  audio read), modes `audio-read` and `sound-test`, `lid` options `--sound chime|alias`, `--alias-after S`,
  `--listen`. fmt, clippy `-D warnings` clean; 40 tests pass (4 new). `audio-read` (reads only,
  `out/audio-read-20261103-145551.log`): the System Sounds session on the Speakers is at 100%, not muted. Registry
  form factors: both DELL endpoints 9 (display audio), Speakers 1, "Realtek HD Audio 2nd output" 3 (headphones).
- 2026-11-03 14:56 `sound-test` (elevated agent, lid open; `out/sound-test-20261103-145607.log`): the Phase 0 alias
  reached the Speakers 757 ms after `PlaySound` returned (8 ms), 1210 ms long, loopback peak -1.2 dBFS. Our chime:
  stream running 287 ms after the call, on the Speakers (pick Default), engine took all 750 ms of it, loopback 503 ms
  above -50 dBFS, peak -0.4 dBFS. Two sounds played.
- 2026-11-03 14:57 dry run `limited lid --simulate-close-after 3 --no-lock --work 5 --max-minutes 1 --listen --sound
  chime --alias-after 2` (Medium; `out/lid-limited-20261103-145749.log`): chime and alias both seen in loopback; the
  chime took 572 ms to start, past the 500 ms wait before the lock. Plan copy made and restored; the 13 `after` lines
  equal the `before` lines. Two sounds played.
- 2026-11-03 14:59: added a warm-up at start, step times in the chime line, the wait before the lock raised to 1 s,
  and `--chime-after S`. Dry run with the chime only (`out/lid-limited-20261103-145950.log`): device pick 5 ms,
  stream ready 482 ms (all in `IAudioClient::Initialize`), lock would wait 490 ms; plan restored. One sound played.
- 2026-11-03 15:01: screen holder 19924 had ended by itself at 14:40:59 (on battery 60 s); `powercfg /requests`
  empty. Launch files `spikes/lid-sleep/sound-close-1.cmd` and `sound-close-2.cmd` written. Facts so far in
  `docs/research.md`, section "Lid-close sound".
- 2026-11-03 15:02 AFTER agent steps (`<scratch>\snap-after-agent.txt`): identical to BEFORE except the time
  (battery 92%). No journal left; Balanced the only plan. Status set to blocked for Haik's two closes.
- 2026-11-03 15:16 to 15:20: Haik did both closes (relayed by the orchestrator at 15:29): "Both close 1 and close 2
  tests work but I guess in close 1 first was windows sound then your custom and in close 2 order was reverse." DELL
  unplugged after close 2 and out since.
- 2026-11-03 15:30 agent steps (reads only, no sound, no screen use): read `out/lid-20261103-151653.log` and
  `out/lid-20261103-151907.log` (results in `docs/research.md`, "Lid-close sound"); each run's 13 `after` lines equal
  its `before` lines. AFTER (`<scratch>\snap-after.txt`, 15:30:27, DELL out): identical to BEFORE except the time
  (battery 91%). No journal in either spike, no spike process, `powercfg /requests` empty. Status done.

## Result

Status: done.

1. Why I was silent (`docs/research.md`, "Lid-close sound"). Close 1 repeated I exactly (alias, lock 0.2 ms
   later, laptop alone on battery). This time loopback showed the full sound on the Speakers and Haik heard it, so
   neither the lock nor the closed lid cuts or mutes sound here, and I's silence did not repeat. Its cause stays
   unknown; most likely it was not noticed (inference). Ruled out by test:
   - Not the System Sounds slider: 100%, not muted.
   - Not the sound file: `Windows Background.wav` is as loud as the toast sound and louder than the device-remove
     sound Haik most likely heard in D with the lid closed.
   - `SND_SYSTEM` plays on the default device (doc plus loopback). That explains D (default was the speakerless
     DELL), not I (default was the Speakers).
   - Any sound here starts 0.3 to 0.8 s after the call, so the lock comes first; that does no harm (close 1).
   - No lid setting in the audio driver keys.
   - Haik's "Windows sound" after the chime in close 2 was most likely the device-remove sound when he unplugged the
     DELL (Speakers' loopback, 15:19:40, about 20 s after the lid opened); the alias control itself went to the
     speakerless DELL (inference).
2. Device: the default device, unless its form factor is display audio (HDMI or DisplayPort); then the speakers.
   Reason: the sound confirms a physical act at the laptop, a screen's audio often has no speakers (the DELL has
   none, yet Windows makes it the default on plug-in), and headphones or a headset stay as they are, so someone
   wearing them gets no sound out loud. Default only would stay silent in Haik's normal docked case (run D).
3. Method in the lid spike: own chime (two tones, 1 kHz and 1.6 kHz, 0.75 s with 150 ms lead silence, peak -6 dBFS)
   built in the exe, played through WASAPI shared mode on the picked endpoint, on its own thread; the lock waits
   until the stream runs (at most 1 s). (Superseded: close 1 showed the lock does not cut sound, so the design locks
   at once and plays the chime in parallel; `docs/architecture.md`.) In loopback and heard at both closes: close 1 laptop alone, 3 s after the
   close with the session locked; close 2 with the DELL as default, picked the Speakers, at the close before the
   lock. The close-1 timing differs from the product (chime at the close) so that the same close could repeat I.
4. Other moments for a sound (Phase 5 builds them; default in brackets):
   - Lid closed while Wakeshot holds: the chime of this card [yes].
   - Work ended with the lid closed, hibernate in a moment: a short "going to sleep" sound right before it [yes].
   - Battery floor reached: Wakeshot steps back and may hibernate [yes].
   - Restore of power settings failed: the warning notification is enough [no extra sound].
   - A hold starts or ends with the lid open (agent starts or finishes): too frequent, the icon shows it [no].
   - Keep awake timer ran out: the notification is enough [no].
   - Lid opened after a closed-lid hold, AC lost or back while holding, a setting we override changed by the user,
     "update Wakeshot": notification or icon only [no].
   - Note for Phase 5: Windows toasts play their own sound unless set silent, and toasts are hidden on the lock
     screen by default (not tested here), so the lid-close cue cannot be a toast sound.
5. Facts: `docs/research.md`, new section "Lid-close sound".

Files: new `spikes/lid-sleep/src/sound.rs`, `spikes/lid-sleep/sound-close-1.cmd`, `sound-close-2.cmd`; changed
`spikes/lid-sleep/src/main.rs` (modes `audio-read`, `sound-test`), `src/modes.rs` (`lid` options `--sound`,
`--alias-after`, `--chime-after`, `--listen`; the old `play_sound` moved to `sound::play_alias`), `Cargo.toml`
(windows features), `README.md`; `docs/research.md`; this card.

Checks: `cargo fmt --check` clean, `cargo clippy --release --all-targets -- -D warnings` clean, `cargo test` 40
passed (4 new: device rule, chime shape, peak and dB, sound segments). Acceptance:

- Loopback shows our sound at a lid close with the lock on: pass (both closes).
- Haik hears it at two lid closes, one with the DELL as default, one laptop alone: pass (his words above).
- Volume, default device, sound scheme the same after as before: pass. BEFORE 14:40 and AFTER 15:30, both with the
  DELL out: Speakers default for all roles, 67.3%, not muted; scheme `.Default`; SystemNotification
  `Windows Background.wav`; power values and plan list identical. Nothing in this card writes volume, device or
  scheme. (The DELL read 26.0% at 15:19; it was 24.0% at 10:21; not changed by this card.)

Open items:

- Loopback shows what reached the endpoint, not the speaker; Haik's ear covered that at both closes. Whether the
  150 ms lead silence is needed (codec waking late) was not tested.
- A loopback capture keeps the audio engine busy on that endpoint, so the lid runs are not quite the product case.
- The 0.3 to 0.6 s stream start is all in `IAudioClient::Initialize`. If Phase 5 wants the sound right at the close,
  test keeping an initialized stream ready during a hold.
- Whether Wakeshot honors the "No sounds" scheme or needs its own sound switch: a Phase 5 product question.
