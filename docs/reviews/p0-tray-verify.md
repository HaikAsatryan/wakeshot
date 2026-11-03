# Verify: tray review fixes

Reviewer: wakeshot-reviewer, 2026-11-03. Scope: card `docs/tasks/p1-tray-review-fixes.md` against
`docs/reviews/p0-tray.md` ("Edits for the orchestrator"). Read: `docs/research.md` (tray section only),
`docs/tasks/p0-spike-tray.md`, `spikes/tray/README.md`, `docs/product.md`, `docs/tasks/p1-tray-menu-look.md`,
`docs/tasks/p1-tray-menu-research.md`, `spikes/tray/src/` (`main.rs`, `theme.rs`, `toast.rs`, `glyph.rs`,
`capture.rs`), `spikes/tray/out/` logs, exe file sizes and times. Read-only checks: decoded `out/toast-icon.png` with
a stdlib script in my scratchpad; `Test-Path` on the leftover keys and files. Nothing written but this file.

Result: 0 blockers, 0 major, 5 minor. All 19 numbered edits and both optional edits are in place. The 5 changed edits
keep the review's intent and are backed by the files.

## Edits in place

- `research.md` edits 1 to 11: all found (lines 300-302, 318-320, 332-334, 348-356, 369-374, 389-392, 396-404,
  407-408 at the time of reading). The link to `docs/research/tray-menu-look.md` is at line 280-281.
- `p0-spike-tray.md` edits 12 to 18: Result items 3 and 6 (lines 296, 303), Implications (323-338), Found outside
  scope (146-148). Edit 19: lines 149-153. Log lines left as they were, as the review asked.
- `spikes/tray/README.md` "Leftovers on the PC": balloon key and stored toasts added (lines 59-63).
- `product.md:136-138`: the menu follows the Windows mode, with the reason. `product.md:141`: pointer now names the
  bullet "First add lands in the overflow".
- `p1-tray-menu-look.md`: "Decided" 1 to 4, owner-drawn test items (deliverable 5), leftover removal (deliverable 6)
  with full key names and path, "Runs after" `p1-lid-sound` and `p1-tray-review-fixes`. All four leftovers exist now
  (checked with `Test-Path`), so deliverable 6 names real targets.

## The 5 changed edits

- 1 (Phase 5 method added to `research.md`): keeps intent. The review's own Fix line asks for the pure red test icon
  and the fresh `IconUri` path. Checked: `toast-icon.png` is 64x64 RGBA, every pixel (43, 124, 211) at varying alpha;
  `main.rs:840` passes `[0x2b, 0x7c, 0xd3]` to `glyph_rgba`; `capture.rs:148` writes `ColorType::Rgba`; the only
  `swap(0, 2)` (`capture.rs:134`) is in the screenshot path, not the icon path. `architecture.md:432` and its test
  table row 13 also carry the Phase 5 check.
- 2 (`GetHistory` to `GetHistoryWithId`): correct, `toast.rs:115`.
- 7 ("03:54" to "built 03:41"): correct. The x64 release exe is 519,168 bytes, last written 03:41:04. The ARM64 exe
  is 498,176 bytes (486 KB), last written 01:43:20, so "the 01:43 build from before the token code" holds.
- 10 ("without another `FlushMenuThemes`"): right, since the new text adds a flush at start. See minor 2 for one
  gap that the review's text already had.
- 19 (decision recorded instead of "Haik decides"): backed by `p1-tray-menu-research.md:72-74`, decision 4, taken
  under Haik's delegation of design calls (line 63).

Spot-checked the review facts behind edits 5 and 8: `SetForegroundWindow=false` at 4675236 to 6315311 ms (11:37 and
12:04 from the 10:19:34 start) and in `log-auto`/`log-menus`; `true` for every right-click and for the code-opened
menus at 9753822 to 9785768 ms (13:01 to 13:02). Idle memory in the logs: 10,288 KB / 1,604 KB and 10,380 KB /
1,704 KB. All match.

## Findings

### 1. Minor: the card's dark menu implication still picks the app mode

Where: `docs/tasks/p0-spike-tray.md:328-330` ("Dark menus: call `SetPreferredAppMode(AllowDark)` ... With
`AllowDark`, no `FlushMenuThemes` is needed on a live switch").

What breaks: `product.md:136` now says the menu follows `SystemUsesLightTheme`. `AllowDark` follows the app mode
(`research.md:312`). A reader of the card's Implications (which `p1-tray-menu-look` lists under "Read first") gets
the opposite rule. `p1-tray-menu-look.md:32-34` has the right note, so the risk is low. Also: that `AllowDark`
follows the app mode and not the Windows mode was never tested apart; both values were equal in every logged read.

Fix: append to the implication: "Superseded for the colour choice: `product.md` follows the Windows mode, so a
classic menu uses `ForceDark` or `ForceLight` from `SystemUsesLightTheme` plus `FlushMenuThemes`."

### 2. Minor: "`FlushMenuThemes` once at start" is not the whole tested sequence

Where: `research.md:372-374` (edit 10) and `p0-spike-tray.md:328-329` (edit 16, "the tested sequence").

What the code did: `ux().flush()` calls `RefreshImmersiveColorPolicyState` then `FlushMenuThemes` (`theme.rs:88-95`).
The interactive spike called it at start (`main.rs:232`) and also at every settle, right after the no-flush capture
(`main.rs:999-1000`). So before `switch3` there were flushes at the 11:37 and 12:04 settles, and before `switch4`
there was one at the `switch3` settle, made while Windows was light. "Without another `FlushMenuThemes`" after the
switch is still true. The review's own new text had the same gap.

Fix: "once at start (and after each earlier settle, after its no-flush capture)". No design effect expected.

### 3. Minor: size bullet now reads as if 420 KB belongs to the current build

Where: `research.md:348-351`. "519,168 bytes after the token code (built 03:41); 420 KB without the PNG encoder.
About 198 KB of that is resources ... So the code ... is about 220 KB."

What breaks: 420 KB and the 220 KB code share were measured on the first (504 KB) build. Placed after the new
number, they read as the current build's split.

Fix: "420 KB without the PNG encoder (first build)".

### 4. Minor: the memory bullet's other numbers do not match the logs

Where: `research.md:355-356`, next to edit 8. "After opening menus: working set 16 to 18 MB, private 2.0 to 3.3 MB.
After WinRT toasts: working set 25 to 28 MB, private 3.9 MB."

The logs: after menus 16,304 to 19,356 KB working set (`log-idle.txt:20` 19,356 KB after Haik's right-click); after
toasts private 3,176 to 3,920 KB (`log-interactive.txt:136,144` 3,560 and 3,176 KB). Not from the fix card, but the
same bullet edit 8 corrected.

Fix: "16 to 19 MB" and "private 3.2 to 3.9 MB".

### 5. Minor: small stale notes in the cards

- `p1-tray-review-fixes.md:53-54` lists `product.md:91` as citing `research.md:485`. `product.md:91` now cites the
  bullet "Limits: Kernel-Power 125", and `product.md` has no `research.md:<line>` pointer left. That half of the note
  is stale.
- `p1-tray-menu-look.md` deliverable 7 (finding 13 crops, from the 15:08 copy of the old scratchpad) has no Log line
  saying who added it and when.
- `p0-spike-tray.md:287` and `:313` (Result) still say "end check OK" and "toast keys gone" with no pointer to the
  leftovers in Found outside scope (`:149-153`). Optional: add "(leftovers it missed: Found outside scope)".

## Checked and holding around the edits

- `research.md:294-295` (first add returns the chevron rect) agrees with edit 6 (same rect after `IsPromoted`).
- `research.md:383-384` (synthetic right-click, `SetForegroundWindow` TRUE) agrees with edit 5.
- `README.md:38` says the interactive mode opens the menu by itself after a theme switch, which matches edit 5 calling
  the `switch3`/`switch4` menus code-opened.
- "Expire after 3 days" fits the review's expiry dates (2026-11-05 and 2026-11-06) once read in UTC.
- Edit 3's "+1 is most likely the 01:33 toast" and edit 2's "stored, not shown" do not contradict the notification
  centre bullet or the unregistered toast line at `research.md:393`.

Verdict: fixed. The 5 minor items are optional cleanups; none blocks `p1-tray-menu-look`.
