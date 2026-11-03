# Review: tray spike Result and tray facts

Reviewer: wakeshot-reviewer, 2026-11-03. Scope: item 5 of "Before Haik signs off Phase 0" in
`docs/reviews/p0-delta.md`. Card `docs/tasks/p0-spike-tray.md` (Result and Log); `docs/research.md` "Tray, menu and
notifications", "Tray menus, synthetic input and the theme switch", and "The tray spike takes no power request".
Evidence read: `spikes/tray/out/`, `spikes/tray/src/`, `build.rs`, `Cargo.toml`, `tray-spike.exe.manifest`,
`docs/research/img/tray-*.png`, two shots in the tray worker's scratchpad, read-only registry and process reads, and a
copy of the notification database (`%LOCALAPPDATA%\Microsoft\Windows\Notifications\wpndatabase.db`, copied to my
scratchpad and queried for handler names, counts and times only, no notification content). Nothing was run, built,
clicked or written outside this file and my scratchpad.

Result: 0 blockers, 2 major, 12 minor. Most facts hold: I checked them line by line against the logs and pictures
(list near the end). The two majors are a wrong cause for the orange toast icon, and leftovers that the end check did
not look for.

## Findings

### 1. Major: the orange toast icon is not a channel slip in the spike's PNG

Where: `research.md` "Registered toast" bullet ("probably a channel order slip in the spike's PNG code, not
checked"); card Log 12:57 T4 and "Found outside scope" ("probably a channel order slip").

What the evidence says: `spikes/tray/out/toast-icon.png` (written 12:58, the file the registered `IconUri` named)
decodes to RGB (43, 124, 211) = `2b7cd3`, blue. The code path is clean: `glyph::glyph_rgba` returns straight RGBA
(`glyph.rs:32`), and `capture::write_png` writes it as `ColorType::Rgba` (`capture.rs:145-151`) with no swap. The crop
step is not the cause either: `tray-balloon-dark.png`, made by the same agent crop script, shows the blue "i"
correctly. Yet `tray-toast-registered-dark.png` shows the icon orange, which is `d37c2b`: R and B swapped. So Windows
(or a cached copy of the icon) swapped the channels, not the spike. Not checked: which. The notification database
holds no image asset for the handler, and `wpnidm` has no file from today.

Why it matters: Phase 5 will ship a toast icon. With the wrong cause on record, nobody tests it, and the real icon
could show in wrong colours in every toast header.

Fix: correct both texts (edits 1 and 18). Add a Phase 5 check: show a registered toast with an asymmetric test icon
(pure red), compare on screen, and try a fresh `IconUri` path to rule out a cache.

### 2. Major: leftovers the end check did not look for; uninstall needs more than deleting the keys

Where: card Log 13:04 end check ("No toast keys", "end check OK"); `spikes/tray/README.md` "Leftovers on the PC"
("removed on quit or with `--cleanup`"); card Implications ("register the app id per user under HKCU at start
(remove it on uninstall)").

Found on the machine now (reads only):

- `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\NotifyIconGeneratedAumid_17618738229921704208`,
  values `LastNotificationAddedTime` (12:57:54 today, the T4 balloon) and `PeriodicNotificationCount` 2 (probably the
  01:33 and 12:57 balloons). Windows made it for the balloon; the number is the spike's `NotifyIconSettings` id. Quit
  and `--cleanup` do not remove it, and the README does not list it.
- Notification database: handlers `Wakeshot.TraySpike` (2 toasts, 01:33:39 and 12:58:07) and
  `Wakeshot.TraySpike.Unregistered` (2 toasts, 01:33:36 and 12:58:20), expiring 2026-11-05 and 2026-11-06, plus handler
  `NotifyIconGeneratedAumid_17618738229921704208` (created 01:33:32, the `--auto` balloon; 0 stored). So the spike's
  toasts still sit in the store after quit. Whether the group still shows in Haik's notification centre now: not
  checked on screen.
- `HKCU\Control Panel\NotifyIconSettings\17618738229921704208` (release exe, `UID` 1) and `...\2295659972384642272`
  (the `NIF_GUID` icon, `IconGuid` {5E1F3A2C-...}, path in an old scratchpad). The README lists these as harmless;
  they likely show as entries in Settings > Taskbar > Other system tray icons (not checked on screen).
- The `guidmove\tray-spike.exe` copy still exists in scratchpad `1828f2de-...`.

Clean (checked): no `tray-spike`, power or lid spike process; no `AppUserModelId\Wakeshot*` key; no
`Notifications\Settings\Wakeshot*` key; `IsPromoted` absent; no holder `pwsh`; `powercfg /requests` all None.

Why it matters:

- Harmless, but the card says "end check OK" while traces remain. Haik should know before sign-off.
- The "+1 notifications" in the notification centre (research says "not checked") is most likely the 01:33 `--auto`
  toast. It survived that run's key deletion and showed again once the app id was registered at 12:58. So deleting
  `AppUserModelId\<aumid>` and `Notifications\Settings\<aumid>` does not remove stored toasts. A Wakeshot uninstall that
  only deletes the keys would leave its toasts in the store.
- New fact for Phase 5: a balloon is filed under `NotifyIconGeneratedAumid_<NotifyIconSettings id>`, so its
  notification on/off switch follows the icon's settings entry (the exe path, for a `uID` icon). Balloons are not
  stored (0 rows).

Fix: record the leftovers in the card and README. Implication: on uninstall, call
`ToastNotificationManager.History.Clear(aumid)` before deleting the keys (not tested). Haik may clear the "Wakeshot
tray spike" group by hand, or let it expire on 2026-11-06. Deleting the `NotifyIconGeneratedAumid_...` key and the two
`NotifyIconSettings` keys is optional and needs Haik's OK (Explorer owns them).

### 3. Minor: "so it reached the notification centre" is contradicted by Win+N

Where: `research.md` "Toast" bullet, "Unregistered app id: `Show()` succeeds and `History.GetHistory(aumid)` counts 1,
so it reached the notification centre."

The Win+N check at 12:58 (same section) found no entry for the unregistered toast, and the shots at 12:58:22 and
12:58:24 show no banner (I looked at `125822-T4-toast-unreg-a.png`). The database does hold it (handler
`Wakeshot.TraySpike.Unregistered`, 2 rows, banner and toast settings 1). So it was stored, not shown. Phase 1 must not
treat `Show()` success or a history count as proof that the user saw anything.

Also small: the code calls `GetHistoryWithId`, not `GetHistory` (`toast.rs:115`).

### 4. Minor: the dark menu recipe in Implications is less than what was tested

Where: card Implications, "Dark menus: call `SetPreferredAppMode(AllowDark)` before the first menu ... With
`AllowDark`, no `FlushMenuThemes` is needed on a live switch."; `research.md` "Live theme switch through Settings".

The spike did more: at start it called `SetPreferredAppMode(AllowDark)`, then `RefreshImmersiveColorPolicyState`
(104) and `FlushMenuThemes` (136) (`main.rs:231-232`, `theme.rs:88-95`). It also built a new `HMENU` for every open
(`build_menu`, `main.rs:651`). Neither part was tested without. If Phase 1 drops the start-up refresh or keeps one
menu for the whole run, the switch result may not hold. Fix: write the tested sequence into the implication (edits
10 and 16).

### 5. Minor: the stale value in the first broadcast is partly a spike artifact

Where: `research.md` "Live theme switch through Settings", "(`SystemUsesLightTheme` or `ShouldAppsUseDarkMode` still
old)"; card Result item 3 and the `ImmersiveColorSet` implication.

`SystemUsesLightTheme` still 0 in the first Light broadcast is real (registry read, log at 9750184 ms). But
`ShouldAppsUseDarkMode` reads uxtheme's cached policy, and the spike never calls `RefreshImmersiveColorPolicyState`
before reading it in the broadcast path (`log_theme` calls `ux().apps_dark()` only). In the Dark switch the registry
was already right at the first broadcast; only the cached value lagged. The design advice (wait for the broadcasts,
then compare) still holds. Phase 1 should not read `ShouldAppsUseDarkMode` without a refresh, and should pick the tray
glyph from the registry.

### 6. Minor: `SetForegroundWindow` returns TRUE for menus opened by code too

Where: `research.md` "Menu details", "returns TRUE after a real right-click and FALSE when the menu is opened by code."

The interactive log shows `SetForegroundWindow=true` for the code-opened `switch3-*` and `switch4-*` menus (log at
9753822, 9755691, 9783915 and 9785768 ms), and false for `switch1`/`switch2` and the 01:33 and 01:35 runs. Windows'
foreground rules decide (likely: the spike had the last input, from the agent's clicks a minute before), not who
opened the menu. A code-opened menu (hotkey, toast click) may then not close on an outside click (KB135788). Fix the
sentence (edit 5).

### 7. Minor: `Shell_NotifyIconGetRect` gave the same numbers before and after `IsPromoted`

Where: `research.md` "Explorer applies `IsPromoted` live", "with `Shell_NotifyIconGetRect`, which then returned the
icon's own rect instead of the chevron's."

The card Log (10:19 and 10:20, T2) gives RECT 3378,2088 to 3426,2160 both times: the chevron moved one slot left and
the icon took its place. Only the screenshots tell the two apart (`tray-icon-add1-dark-taskbar.png` chevron,
`s0-taskbar-promoted.png` white circle; I checked both). So the rect alone cannot tell "hidden" from "pinned", and
Phase 5's first-run pin tip cannot use it to detect a hidden icon.

### 8. Minor: stale exe size in the Result and in research.md

Where: card Result item 6 "x64 exe 504 KB"; `research.md` "Size" "504 KB".

The x64 release exe that ran every interactive test is 519,168 bytes (read now; the card's own 03:54 Log line says
so). 504 KB was before the token code. The ARM64 exe on disk (498,176 bytes, machine `0xAA64`, checked) is still the
01:43 build, from before the token code.

### 9. Minor: some memory numbers are not in any saved log

Where: `research.md` "Memory" ("9.7 to 10 MB, private 1.5 to 1.7 MB, 133 handles; threads drop from 4 to 1"); card
Result item 6 ("idle 1.5 MB private"); card Log 01:3x ("`--idle 60`: idle working set 9.7 MB, private 1.5 MB").

The logs show idle 10,288 KB / 1,604 KB (`log-auto.txt`) and 10,380 KB / 1,704 KB (`log-interactive.txt`). The
`--idle 60` log's only reading is 19,356 KB / 3,252 KB, taken after Haik opened the menu, so not idle. 9.7 MB, 1.5 MB,
133 handles and the thread count must come from outside reads that were not saved. No design effect.

### 10. Minor: "picked by `SystemUsesLightTheme`, not `AppsUseLightTheme`" was not tested apart

Where: card Implications, "Two icon sets, picked by `SystemUsesLightTheme` (taskbar), not `AppsUseLightTheme`."

"Choose your mode" Light and Dark change both values together (log at 9750298 and 9780400 ms). The code picks by
`SystemUsesLightTheme` (`main.rs:422`), so the test shows the glyph follows the switch, not which value is the right
one. The choice matches Windows' meaning (the taskbar follows the Windows mode). Fix: note that Custom mode (Windows
dark, apps light) was not tested (edit 15).

### 11. Minor: "Windows keys a uID icon's pinned state by exe path" is inferred

Where: card Implications, "Keep a stable install path: Windows keys a uID icon's pinned state by exe path."

The evidence is the key's values (`ExecutablePath`, `UID`), not a move test. Only the `NIF_GUID` icon was moved. The
advice is sound; mark it inferred (edit 14). It matters for the installer choice: a versioned install folder would
reset the pin on every update if this holds.

### 12. Minor: the hidden window never got `WM_DPICHANGED`

Where: `research.md` "`LoadIconMetric` follows the system DPI" and the matching implication.

The log shows `WM_DISPLAYCHANGE` (0x7e) nine times, each followed by an icon reload, and no 0x2e0 at all, although the
primary display changed from the 144 dpi DELL to the 120 dpi laptop screen during the run. So Phase 5 cannot wait for
`WM_DPICHANGED` on the hidden window; reload on `WM_DISPLAYCHANGE` and on `TaskbarCreated`. Worth one line in the
facts (edit 11).

### 13. Minor: evidence that lives only in a temp scratchpad, and a log clock that drifts

- The `IsPromoted` live fact, the unregistered toast "no banner", the Win+N result and the tooltip shots rest on
  files in `...\06511897-...\scratchpad`, a temp folder that may be cleaned. Fix: copy two non-private crops into
  `docs/research/img/`: the taskbar strip from `s0-taskbar-before.png` and `s0-taskbar-promoted.png`, and the
  bottom-right corner of `125822-T4-toast-unreg-a.png` without the terminal text.
- The spike log stamps only ms since start (`log.rs:57`, `Instant`). By 11:37 that clock ran 16 s ahead of the wall
  clock (picture file times and the notification database agree; at 10:27 it was still in step). So "start + ms" times
  after 10:27 are about 16 s late (for example 13:02:04 for the Light switch, really 13:01:48). Cause not checked
  (likely the sleep during Haik's battery steps, or a clock sync). Phase 1 logs should stamp wall time on every line.

### 14. Minor: wording and pointer nits

- Card Result item 3, "one also came twice with no change": the log has two separate single broadcasts with no
  change (about 11:37 and 12:04). Edit 12.
- The registered toast header "Wakeshot tray spike" cannot prove the `DisplayName` was used: `FileDescription`,
  `ProductName` and the window title are the same string (`build.rs:20,23`, `main.rs:250`). The icon proves the
  registration was read. Phase 5: use a different `DisplayName` once to see which one Windows shows.
- `NIN_BALLOONTIMEOUT` arrives twice at the same ms in both runs (`log-auto.txt` 15696 ms, `log-interactive.txt`
  9522219 ms). Not in the facts; Phase 1 must not count balloon events.
- `docs/product.md:139` cites `research.md:288-292`, which now holds "Add and update" and the start of "First add lands
  in the overflow". Line numbers shift while `research.md` is edited; cite the bullet name.
- Sound: the tray card's implication says "ship a short WAV as a resource with `SND_SYSTEM`"; the `p0-delta.md`
  lid-cue table says "no `SND_SYSTEM`" for its keep-a-sound option. Neither is decided; the delta review owns sound.
  Reconcile when the lid-cue pick is made.

## Checked and holding

- Token line: `elevated false, integrity Medium (0x2000)`, PID 11944.
- First add in the overflow: rect 3378,2088 to 3426,2160; `tray-icon-add1-dark-taskbar.png` and
  `tray-icon-first-add-overflow.png` show the chevron.
- `IsPromoted` live: written 10:20:13 per the card; `s0-taskbar-promoted.png` (10:20:16) shows the white circle on the
  taskbar right of the chevron. `IsPromoted` is absent now.
- Explorer restart: `TaskbarCreated received on main window` at 459705 ms; nothing on the message-only window (the code
  would log it, `main.rs:1095-1096`); `NIM_ADD (TaskbarCreated, attempt 2): true`; `tray-icon-add2-dark-taskbar.png`
  (10:27:16) shows the white circle.
- Version 4 events: 0x204, 0x205, then `WM_CONTEXTMENU` with the point in `wParam` (`main.rs:1035-1037`, log).
- Dark menus: ordinals 135, 136, 104, 132 resolve; `SetPreferredAppMode` returned 0, 1, 3, 2 (`log-auto.txt`,
  `log-menus.txt`). Pictures `forcelight*`, `forcedark*`, `allowdark-system-dark`, `live-dark-*`, `agent-dark-*`,
  `switch3-*`, `switch4-*` show arrow, check, radio bullet, grey status line, separators and round corners.
- Live switch: two broadcasts per switch about 0.1 s apart (9750182/9750295 and 9780307/9780398 ms); icon id 103, then
  101; `switch3-light-noflush` light and `switch4-dark-noflush` dark with no flush since 12:04 (only the settle timer
  flushes, after the no-flush capture). `switch1`/`switch2` are both dark, from the 11:37 and 12:04 broadcasts.
- Balloon: `NIN_BALLOONSHOW`, then `NIN_BALLOONTIMEOUT` 6.6 s later (6577 ms); the picture shows the header, the tray
  icon and the blue "i".
- Registered toast: `Dismissed reason=ToastDismissalReason(2)` 6.5 s after `Show()`; `Setting=NotificationSetting(0)`
  for both ids at Medium; `0x80070490` for both in the 01:33 run.
- Quit: `NIM_DELETE: true`; unregister codes 0, 0, 2, 2 as the card says.
- Haik's `tray-menu-live-dark-01/02/03` match the backup by SHA-256; eight agent captures sit in `tray-live-agent`.
- Manifest: Common Controls 6, PMv2, `asInvoker`; no `SetProcessDpiAwareness*` call in `src`, so PMv2 comes from the
  manifest. comctl32 v6 `6.0.26100.9711` in WinSxS and `System32\comctl32.dll` 5.82 (read now).
- No power request: `src` has no `PowerSetRequest`, `PowerCreateRequest`, `PowerClearRequest` or
  `SetThreadExecutionState`. Logs open in append mode (`log.rs:37-41`). Screenshots overwrite (the card handled that).
- `NIF_GUID` move: one `NotifyIconSettings` entry with `IconGuid` {5E1F3A2C-7B9D-4C1E-9A6F-2D8B4E0C7A13} and the
  `guidmove` path.
- Sound scheme `SystemNotification` = `C:\WINDOWS\media\Windows Background.wav` (read now).
- Size facts: LTO, 1 codegen unit, abort, strip (`Cargo.toml`); four icons of 8 frames are about 45 KB each by the
  `build.rs` layout.

## Edits for the orchestrator

`research.md` is being edited, so match on the text, not the line. Each old text is quoted exactly as it stands now,
with its line breaks and leading spaces.

### `docs/research.md`

1. Registered toast bullet. Old:

   ```
   (blue): probably a channel order slip in the spike's PNG code, not checked.
   ```

   New:

   ```
   (blue). The PNG on disk is blue (43, 124, 211) and the spike writes straight RGBA, so Windows or a cached copy
   swapped R and B; which one was not checked. Phase 5 checks its real toast icon on screen.
   ```

2. Toast bullet, unregistered app id. Old:

   ```
   counts 1, so it reached the notification
       centre.
   ```

   New:

   ```
   counts 1, so Windows stored it (the notification database holds it), but the notification centre does not show it
       (Win+N, below).
   ```

3. Notification centre bullet. Old:

   ```
   Which one the "+1" is: not checked.
   ```

   New:

   ```
   The notification database holds two toasts for the registered id (01:33:39 and 12:58:07), so the "+1" is most
   likely the 01:33 toast, which survived that run's key deletion.
   ```

4. "Windows writes `Notifications\Settings\<aumid>` only for the registered app id" bullet. Old:

   ```
     `AppUserModelId` key (`unregister` log: 0 for the registered id, 2 = not found for the unregistered one).
   ```

   New:

   ```
     `AppUserModelId` key (`unregister` log: 0 for the registered id, 2 = not found for the unregistered one). Not
     removed by quit: the balloon's `Notifications\Settings\NotifyIconGeneratedAumid_<NotifyIconSettings id>` key and the
     stored toasts in the notification database (they expire after 3 days). So an uninstall must clear the toast
     history, not only delete the keys.
   ```

5. Menu details. Old:

   ```
   returns TRUE after a real right-click and FALSE
     when the menu is opened by code.
   ```

   New:

   ```
   returned TRUE after every right-click on the icon. For
     menus opened by code it returned FALSE at 01:33, 01:35, 11:37 and 12:04 and TRUE at 13:01 and 13:02: Windows'
     foreground rules decide, not who opened the menu.
   ```

6. Explorer applies `IsPromoted` live. Old:

   ```
   Checked with taskbar screenshots before and after and with `Shell_NotifyIconGetRect`, which
     then returned the icon's own rect instead of the chevron's.
   ```

   New:

   ```
   Checked with taskbar screenshots before and after. `Shell_NotifyIconGetRect` gave the same
     RECT both times: the chevron moved one slot left and the icon took its place, so the rect alone cannot tell
     hidden from pinned.
   ```

7. Size. Old:

   ```
   `strip`: 504 KB; 420 KB without the PNG
   ```

   New:

   ```
   `strip`: 504 KB at first, 519,168 bytes after the token code (03:54); 420 KB without the PNG
   ```

8. Memory. Old:

   ```
   9.7 to 10 MB, private 1.5 to 1.7 MB, 133 handles; threads drop from 4 to 1 after about 30 s.
   ```

   New:

   ```
   10.0 to 10.4 MB, private 1.6 to 1.7 MB in the logs (9.7 MB, 1.5 MB, 133 handles and threads dropping from 4 to 1
     after about 30 s are from outside reads that were not saved).
   ```

9. Live theme switch. Old:

   ```
   (`SystemUsesLightTheme` or `ShouldAppsUseDarkMode` still old); in the second
   ```

   New:

   ```
   (`SystemUsesLightTheme` still 0 in the Light switch; `ShouldAppsUseDarkMode` still old in both, but the spike
     reads it without calling `RefreshImmersiveColorPolicyState` first); in the second
   ```

10. Live theme switch. Old:

    ```
    With `SetPreferredAppMode(AllowDark)`
      set at start, the next menu already had the new theme
    ```

    New:

    ```
    With `SetPreferredAppMode(AllowDark)`,
      `RefreshImmersiveColorPolicyState` and `FlushMenuThemes` once at start and a new menu built for each open, the
      next menu already had the new theme
    ```

11. "`LoadIconMetric` follows the system DPI" bullet. Old:

    ```
    How sharp it looks was not judged.
    ```

    New:

    ```
    How sharp it looks was not judged. The hidden window got `WM_DISPLAYCHANGE` nine times and never
      `WM_DPICHANGED`.
    ```

### `docs/tasks/p0-spike-tray.md`

12. Result item 3. Old: `one also came twice with no change.` New: `twice a single one came with no theme change (about
    11:37 and 12:04).`
13. Result item 6. Old:

    ```
    6. x64 exe 504 KB, idle 1.5 MB private, about 10 MB working set;
    ```

    New:

    ```
    6. x64 exe 519,168 bytes (504 KB before the token code), idle 1.6 to 1.7 MB private, about 10 MB working set;
    ```

14. Implications. Old:

    ```
    - Keep a stable install path: Windows keys a uID icon's pinned state by exe path.
    ```

    New:

    ```
    - Keep a stable install path: Windows most likely keys a uID icon's pinned state by exe path (inferred from the
      key's `ExecutablePath` and `UID`; a uID icon was not moved in a test).
    ```

15. Implications. Old:

    ```
    - Two icon sets, picked by `SystemUsesLightTheme` (taskbar), not `AppsUseLightTheme`.
    ```

    New:

    ```
    - Two icon sets, picked by `SystemUsesLightTheme` (taskbar), not `AppsUseLightTheme` (the test changed both
      together; Custom mode was not tested).
    ```

16. Implications. Old:

    ```
    - Dark menus: call `SetPreferredAppMode(AllowDark)` before the first menu,
    ```

    New:

    ```
    - Dark menus: call `SetPreferredAppMode(AllowDark)`, then `RefreshImmersiveColorPolicyState` and
      `FlushMenuThemes`, before the first menu, and build the menu at each open (the tested sequence),
    ```

17. Implications. Old:

    ```
    register the app id per user under HKCU at start (remove it on uninstall)
    ```

    New:

    ```
    register the app id per user under HKCU at start (on uninstall, clear its toast history with
      `ToastNotificationManager.History.Clear(aumid)`, then remove the keys; the keys alone leave stored toasts; Clear
      not tested)
    ```

18. Found outside scope. Old:

    ```
    - The spike's toast icon PNG shows orange for RGB 2b7cd3 (probably a channel order slip). Throwaway code, not fixed.
    ```

    New:

    ```
    - The toast header shows the spike's toast icon orange, but `out/toast-icon.png` is blue (43, 124, 211) and the
      code writes straight RGBA, so the spike is not the cause. Windows or a cached copy swapped R and B (not checked).
      Phase 5 checks its real toast icon on screen.
    ```

19. Found outside scope: add one line for the leftovers in finding 2 (balloon key, stored toasts until 2026-11-06, two
    `NotifyIconSettings` keys, the `guidmove` exe), and let Haik decide whether to clear them. Leave the Log lines as
    they are: they record what was thought at the time.

### Other files (optional)

- `spikes/tray/README.md` "Leftovers on the PC": add the `NotifyIconGeneratedAumid_<id>` key and the stored toasts.
- `docs/product.md:139`: replace `` `research.md:288-292` `` with `` `research.md`, "First add lands in the overflow" ``.
