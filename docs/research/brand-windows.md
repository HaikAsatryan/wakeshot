# Brand research: Windows icons in 2026

For card `docs/tasks/p6-brand-research-windows.md`, written 2026-11-05. It feeds `p6-brand-directions`: facts and
short reasons only, no design work.

Marks: **verified** means read at the source (URL and date given) or measured on this laptop (how given).
**Unverified** means a secondary source, or a source not read in full. A guess says so.

## Summary

1. **Microsoft prefers no plate on Windows.** "Icons look best with a transparent background. If your app's branding
   requires your icon be plated on a background, that's okay too", at the cost of theme work (D2). All 20 Microsoft
   app icons installed on this laptop are free shapes with soft depth; none sits on a coloured square as a backdrop
   (Photos' rounded square is the photo itself). The violet rounded square is a Mac and phone habit; it is allowed,
   not native.
2. **The plate earns its keep in two places:** it keeps the gold sun readable on light backgrounds (gold on f3f3f3 is
   1.35:1; "Yellow will never pass ... on light theme until it's brown", D1), and the notice header uses one PNG for
   light and dark notices at about 16 px. A free-shape icon must solve both another way.
3. **Microsoft 365's 2025 refresh went to softer shapes, stronger gradients, "depth, and highlights"**, not glass.
   No source shows Windows' own icons moving to glass in 2025 to 2026.
4. **Tray: 8 of 12 known apps use a one-colour mark, 5 use their colour app icon** (PowerToys counted in both). The
   one-colour apps use colour only for state (badges, a red error, Windows' green charging battery), as Microsoft's
   old guide asks. Brand gold alone is 1.07:1 on the light taskbar; brand violet is 2.7 to 3.6:1.
5. **The tray icon is the brand mark without its plate** in every one-colour app surveyed; states add a badge or
   change part of the mark. Wakeshot already follows this.
6. **Pick for the tray: a one-colour glyph, with colour only while holding** (section 3). Change it when users read
   the gold as a warning, or Haik wants the tray identical to Windows' glyphs.
7. **ICO lacks 256 px.** Microsoft's minimum is 16, 24, 32, 48 and 256; Start pins at 400% and the shell's jumbo list
   use 256.
8. **Menu glyphs:** mixing system glyphs with a custom one is what Windows' Explorer menu and PowerToys do. For Keep
   awake, E917 Clock is not in Segoe MDL2 Assets (the fallback font); E823 Recent draws the same clock and is in both.
   Sun E706 is Windows' brightness sign. Lightbulb EA80 and Stopwatch E916 are in both fonts. Fluent System Icons are
   MIT; they hold clock, timer, sun, lightbulb, but no sunrise.
9. **Notices:** header = the registered `IconUri` PNG at about 16 logical px; body = the balloon icon at `SM_CXICON`
   (32 px at 100%, 48 px at 150%).

Not verified: the live taskbar on this laptop (the screen was locked, so the survey uses files and earlier
screenshots); the look of Slack, Teams, NVIDIA, LM Studio and ChatGPT tray icons; Claude desktop's Windows tray
pixels; the exact header size in logical pixels; Partner Center's current asset form for an EXE app.

## 1. App icon guidance

Sources, all **verified** (read in full at the source on 2026-11-05):

- D1: "Design guidelines for Windows app icons",
  https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-design (ms.date 2021-10-29, updated
  2026-07-14).
- D2: "Construct your Windows app's icon",
  https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-construction (ms.date 2026-07-21,
  updated 2026-08-05). The public mirror `MicrosoftDocs/windows-dev-docs` still holds an older copy (ms.date
  2022-06-02) with the same key lines.
- D3: "Icons (Design basics)", https://learn.microsoft.com/en-us/windows/win32/uxguide/vis-icons (Windows 7 era
  guide, updated 2025-04-15; the page itself says it "has not been updated for newer versions of Windows").
- D4: "App screenshots, images, and trailers for MSIX app" and the MSI/EXE twin,
  https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images and
  `.../msi/screenshots-and-images` (both updated 2026-08-24).

### Metaphor

- D1: "Your icon should illustrate the concept of your app in a singular element using simple forms."
- D1: "Literal metaphors are best for articulating the purpose and promise in a clear way. A good test for an
  effective icon is when users can tell what it represents without a label."
- D1: "Only use an abstract metaphor in instances where it's impossible to find a literal, self-evident metaphor to
  represent the core functionality of a product."
- D1: "use no more than two metaphors in a single icon. If a single metaphor can be used, that's even better."
- D1: "Icons should not include typography as part of the design. Letters and words on your icon should be avoided
  and only used when it's essential."

For Wakeshot: the sunrise is one literal metaphor (wake, day starts). The sun plus a horizon is still one picture.

### Plate or no plate

- D2, "Transparent backgrounds": "Icons look best with a transparent background. If your app's branding requires your
  icon be plated on a background, that's okay too. However, you'll have to re-implement some theming functionality
  that transparent icons get for free. For example, you might provide a version of your app's icon plated on a two
  different backgrounds, one better suited to a light theme and the other to a dark theme."
- D2 (packaged apps): "If you do not provide these files, your icon will appear on a system icon plate to ensure a
  minimum contrast ratio." That is about missing `altform-unplated` files in an MSIX, not about Win32 ICO files.
- D1 says nothing about plates. Its shape rules are about shapes inside the icon: "When rounded corners are applied
  to an exterior curve, use a 2px radius at 48x48. When rounded corners are applied to an interior curve, use a 1px
  radius instead."

So Microsoft's first choice on Windows 11 is a free shape on a transparent background. A plate is allowed, not
preferred. A coloured rounded-square plate is the Mac (and Android, iOS) habit (not checked here; macOS is
`p6-brand-research-macos`); on Windows it is a brand choice that costs theme work. What Microsoft's own apps do is in section 2.

### Perspective, gradients, shadows

- Perspective, D1: "Icons should be drawn with a straight-on perspective to present the metaphor in a simple easy to
  understand way." Exceptions: a shape that does not read straight on (a cylinder) or an app about 3D. "Layers should
  always be flat and perpendicular to the viewing angle."
- Gradients, D1: "Color gradients, overlays of varying opacity, and tints of color should be kept to a minimum.
  Gradients should be subtle for the most part. Try to limit your gradient ramps to only one or two steps in both the
  horizontal and vertical directions." "The default angle for gradients is 120 degrees." "Avoid very tight
  transitions that would feel like reflections or dimension." Monochrome gradients "give a subtle hint toward an
  ambient light angle coming from the top left".
- Shadows, D1: "Icons are composed of flat objects sitting on top of the layers below it." "Use drop shadows within
  icons to create definition between object layers". "single object metaphors need some depth to feel like part of
  the system". Shadow values "are to be rendered at 48x48 px and scaled up or down from there". The numbers sit in
  pictures, not text (not read).
- Old rule still worth knowing, D3 (Windows 7 era): "Don't use shadows in icons at 24x24 or smaller sizes."

### Colour and contrast

- D1: "Pick colors carefully and avoid relying on color alone to convey meaning. Use shape and metaphor with color to
  communicate."
- D1: "App icons are primarily displayed on either light and dark backgrounds but displayed over desktop background
  images and tints or shades of the system accent color."
- D1: "Use color values in all 3 ranges, dark, medium, light."
- D1: "Make sure at least half of your icon passes a 3.0:1 contrast ratio on light and dark theme."
- D1: "Yellow will never pass an accessible contrast ratio on light theme until it's brown. Reds are more difficult on
  dark theme."
- D1: "Though not required, you have the option to provide separate light and dark theme assets for Taskbar, Start
  and other theme-sensitive areas of Windows." For a Win32 ICO there is no such switch; only MSIX `altform-*` files
  carry theme variants (D2). An unpackaged app can only swap its own window and tray icons at run time.
- High contrast, D1: "Windows 11 no longer requires high contrast assets for app icons." "High contrast icons are
  black and white and should be a direct representation of your app icon."

### ICO frame sizes, and does 256 px matter

- D2 table (verified): context menu, title bar, system tray 16, 20, 24, 32, 40, 48, 64 px from 100% to 400%;
  taskbar, search results, Start all apps list 24, 30, 36, 48, 60, 72, 96; Start pins 32, 40, 48, 64, 80, 96, 256.
- D2: Windows "will look for an exact size match first. If there is no exact match it will look for the next size
  above and scale down."
- D2: "Apps should have, at the bare minimum: 16x16, 24x24, 32x32, 48x48, and 256x256. This covers the most common
  icon sizes, and by providing a 256px icon, ensures Windows should only ever scale your icon down, never up."
- D2 has a heading "Icon sizes (Win32)" with one sentence and no table ("There are standard sizes that are used which
  is a subset of the full set above"), in both the live page and the mirror.
- D3: "Application icons and Control Panel items: The full set includes 16x16, 32x32, 48x48, and 256x256 (code scales
  between 32 and 256)." D3 also: "Only a 32-bit copy of the 256x256 pixel image should be included, and only the
  256x256 pixel image should be compressed" (PNG inside the ICO).

- The shell's image lists (`SHGetImageList`,
  https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shgetimagelist, updated 2025-10-31, read
  2026-11-05, **verified**): `SHIL_JUMBO` "The image is normally 256x256 pixels" and "is fixed at 256 pixels
  regardless of the dpi-aware setting"; `SHIL_EXTRALARGE` "typically 48x48"; small and large scale with DPI.

So yes, a 256 px frame matters: Start pins at 400% ask for 256, and the shell's jumbo list is 256 px whatever the
scale. Without it, Windows scales the 64 px frame up and the icon goes soft in Explorer's big views. Wakeshot's ICO
ships 16, 20, 24, 32, 40, 48, 64 (read from `assets/wakeshot.ico` with a Python ICO header reader on 2026-11-05:
seven 32-bit BMP frames, 43,006 bytes; both exes embed that file through `crates/build-resources.rs`), so it lacks 256
(and 30, 36, 60, 72, 96, which Windows reaches by scaling a larger frame down). `docs/phases/05-tray-ux.md` track
note 3.2 left it out on purpose: "No 256 frame yet ... Change it when the installer or Explorer's large view looks
blurry". Microsoft's own minimum is the stronger trigger. Which Explorer view ("Large icons", "Extra large icons") uses which
list at which scale: **unverified**. Which surface draws Wakeshot's exe icon in Settings > Apps: not checked.

### Store assets (for `p8-store`, EXE path)

- D4, MSI/EXE page: screenshots "Required: 1, Recommended: 4+, Maximum: 10"; "Store logos: Required: 1:1 box art,
  Recommended: 2:3 poster art". It gives no pixel sizes.
- D4, MSIX page (sizes the MSI/EXE page leaves out): "1:1 App tile icon (300 x 300 pixels)": "We strongly recommend
  that you provide this image for your app." 1:1 box art 1080 x 1080 or 2160 x 2160 and 2:3 poster art 720 x 1080 or
  1440 x 2160 are "for games ... This does not apply to apps." Desktop screenshots "1366 x 768 pixels or larger",
  PNG. 16:9 Super hero art 1920 x 1080 or 3840 x 2160, "must not include the product's title or other text".
- Which of these Partner Center asks for an EXE app in 2026 (the MSI/EXE page names "1:1 box art" as required but
  gives no size): **unverified**; read the form when `p8-store` runs.
- D2: "Windows 11 does not use the tile assets, but currently at minimum the Medium tile assets at 100% are required to
  publish to the Microsoft Store." That is for MSIX; Wakeshot's Store path is EXE only (`p8-store.md`).

## 2. Microsoft's own app icons in 2026

### Windows' own apps, as installed on this laptop

Read 2026-11-05 from the installed packages (`appicons.ps1` in the scratch folder: for each package, the largest
`*targetsize-*_altform-unplated.png` named by `Square44x44Logo` in its `AppxManifest.xml`), drawn at 96 px on light
f3f3f3 and dark 202020: `img/brand/win-microsoft-app-icons.png`. **Verified** (files and the picture).

Apps: Windows Terminal 1.25, Dev Home 0.2101 (small tile only), Notepad 11.2607, Paint 11.2605, Snipping Tool 11.2608,
Calculator 11.2607, Microsoft Store 22609, Photos 2026.11090, Microsoft 365 Copilot 19.2610, Outlook 1.2026.1020,
Clock 1.1.158, Camera 2026.2608, To Do, Windows Security, Sound Recorder, Get Help, Media Player, Weather, Phone Link,
Windows Web Experience Pack.

- **No coloured plate behind any of them.** Each is a free shape on a transparent background: a notepad, a palette, a
  bag, a shield, a cloud, a clock. Where the outline is a rectangle (Terminal, Calculator, Photos, Phone Link), the
  rectangle is the object itself (a window, a device, a picture), not a backdrop. Photos comes closest to a plate: a
  rounded square that is itself the picture (a blue landscape).
- **Depth, not flat.** Soft gradients with light from the top left, a few layers with small shadows between them,
  front-facing (Outlook's envelope and Paint's palette tilt slightly). This is D1's style: "flat objects sitting on
  top of the layers below".
- **Theme variants.** Most ship `altform-lightunplated` files as well (14 or more each); Terminal and Camera ship none.
- **Letters only on Microsoft 365 apps:** Outlook's "O" plate and the "M365" tag on Microsoft 365 Copilot.

### The Microsoft 365 refresh (2025)

- "Fluid forms, vibrant colors", Jon Friedman, Microsoft Design, 2025-10-01,
  https://microsoft.design/articles/fluid-forms-vibrant-colors/ (read 2026-11-05, **verified**): "We've moved away
  from bold, static solidity to embrace softer, more fluid forms." "Sharp edges and crisp lines are replaced by smooth
  folds and curves". "Where gradients were once subtle, they're now richer and more vibrant", "featuring exaggerated
  analogous transitions that improve contrast and accessibility." Letter plates kept: "Their brand equity is so
  strong, however, that we decided to keep them". Fewer colours per icon: "the new version uses just three, improving
  legibility at small sizes".
- "The new UI for enterprise AI", Microsoft Design, 2025-11-25,
  https://microsoft.design/articles/the-new-ui-for-enterprise-ai/ (read 2026-11-05, **verified**): refreshed icons
  "across business apps and agents" with "clean shapes, vibrant gradients, depth, and highlights" (Matthieu James,
  lead icon designer).
- Neither article mentions glass, 3D or plates for these icons, nor Windows' own icons.

### Other Microsoft apps

- **PowerToys:** a full-colour device shape (a tan panel with a rainbow screen and three dots), no plate
  (`src/runner/svgs/icon.ico`, rendered here, **verified**).
- **Windows Terminal:** a dark terminal window with a prompt, the object as the shape (installed package,
  **verified**).
- **Dev Home:** retired. "Starting May 2025, Dev Home will no longer be supported as a feature in Windows 11. See
  Advanced Windows settings" (https://learn.microsoft.com/en-us/previous-versions/windows/dev-home/, ms.date
  2025-05-19, read 2026-11-05, **verified**). Its successor is a Settings page, so it has no app icon.

### Any move toward glass or 3D in 2025 to 2026?

- Microsoft 365: yes toward more depth and stronger gradients ("depth, and highlights"), not glass (articles above).
- Windows' inbox apps: their installed icons are the Fluent style of 2021 (front-facing objects, soft gradients).
  A web search for a 2025 or 2026 Windows inbox icon redesign found nothing newer than 2021 (2026-11-05). So no
  Microsoft source shows a Windows move to glass: **no source found**.

### What this means for Wakeshot's app icon

- On Windows, the house style is an object with its own silhouette, not a symbol on a coloured square. Wakeshot's
  violet rounded square is the Mac and phone habit; D2 allows it ("that's okay too") at the cost of theme work.
- Two facts pull the other way: the plate is what keeps gold readable on light backgrounds (section 6, and gold on
  f3f3f3 is 1.35:1), and the notice header PNG has one version for both themes. A free-shape sun needs a darker
  edge or its own colour that passes 3:1 on light and dark (D1: "at least half of your icon").

## 3. Tray icon guidance and practice

### What Microsoft says

There is no current (Windows 11) Microsoft page on notification area icon style. `.../iconography/system-tray-icons`
returned 404 on 2026-11-04 (research.md "Tray UX: icon art"). The rules that exist:

- T1: "Notification Area" (Win32 UX guide, Windows 7 era),
  https://learn.microsoft.com/en-us/windows/win32/uxguide/winenv-notification (updated 2025-03-11, read 2026-11-05,
  **verified**). The page warns it "has not been updated for newer versions of Windows". Key lines:
  - "Use an icon with 16x16, 20x20, and 24x24 pixel versions."
  - Icon type by pattern: "Background task status and access: Program or feature icon".
  - "Choose an easily recognizable icon design. Prefer icons with unique outlines over square or rectangular shaped
    icons. Keep the designs simple prefer symbols over realistic images."
  - "Use icon variations or overlays to indicate status or status changes. Use icon variations to show changes in
    quantities or strengths." Standard overlays (warning, error, disabled, offline): "Use only a single overlay, and
    locate it bottom-right for consistency."
  - "Avoid swaths of pure red, yellow, and green in your base icons. To avoid confusion, reserve these colors to
    communicate status. If your branding uses these colors, consider using muted tones for your base notification
    area icons."
  - "Don't change status too frequently." "Don't use long-running animations to show continuous activities." "An
    icon's presence in the notification area sufficiently indicates continuous activity." "Don't flash the icon."
  - "Don't disable notification area icons. If the icon doesn't currently apply, remove it. However, you can show an
    enabled icon with a disabled status overlay if users can enable from the icon."
  - "never view desktop areas as ways to promote your program or its Brand".
- Fluent 2, "Iconography", https://fluent2.microsoft.design/iconography (read 2026-11-05, **verified**; the page shows
  no date): "Use system icons inside a UI for items like command bars, navigation, or status indicators." "If
  including color on an icon, only use one color and keep contrast in mind." "Only use solid colors on system
  icons". "Filled theme icons are used for highlighting selected states."
- D1 (section 1): "avoid relying on color alone to convey meaning".

Note "Prefer icons with unique outlines over square or rectangular shaped icons": a plated app icon in the tray is
the square the old guide warns against.

### Windows' own tray glyphs (this laptop, build 26300)

- Wifi, volume, battery, the overflow chevron: one-colour glyphs. White ffffff on the dark taskbar; on light, the
  darkest pixels fit black at alpha e4 (research.md "Tray UX: icon art", measured 2026-11-04, **verified**).
- State by shape: wifi bars and battery fill show amount; volume shows mute as a cross. The battery takes a green
  fill while charging (`docs/research/img/tray-taskbar-after-promote.png`, this laptop, **verified** by eye).
- The tray glyphs of Windows 11's own icons are not app icons: they are a separate monoline set (the Segoe Fluent
  Icons look). That they are drawn from the font itself: **unverified**.

### Survey: real tray icons in 2026

Sources marked: **L** this laptop (committed screenshots or the app's installed files, read 2026-11-05), **R** the
app's public source repo (read 2026-11-05), **W** web (vendor page or secondary; see each row). Each icon file was
rendered at 24 px on the measured dark (202021) and light (d8dadc) taskbar colours with
`scratchpad\p6-brand-research-windows\iconsheet.ps1`; picture `img/brand/win-tray-icon-files.png`.

| # | App | Mono or colour | Follows light or dark taskbar | How it shows state | Same as app icon? | Source |
|---|---|---|---|---|---|---|
| 1 | Windows wifi, volume, battery | Mono | Yes | Shape (bars, fill, cross); battery green while charging | No (system glyphs) | L, verified |
| 2 | Windows Security | Colour (blue shield) | No, same shield on both | Badge bottom right: green check, yellow warning, red cross | Yes, the app's shield | L: `SecurityHealthSSO.dll` icon groups 100 to 162, verified |
| 3 | OneDrive | Mono cloud (white set for dark, black outline set for light), plus a blue colour set | Yes | Badges: blue sync arrows, red minus or cross, pause, orange warning, blue info; struck-through cloud | Simplified (cloud only; the app icon, `OneDrive.exe` group 501, is a blue cloud) | L: `FileSync.Resources.dll` 26.195, and its tray on `tray-taskbar-after-promote.png`, verified. Which set is used for a personal or work account: unverified |
| 4 | PowerToys | Colour app icon by default; mono outline by opt-in setting "theme adaptive" (since 2026-01-08) | Only with the opt-in | Orange dot bottom right for "update available" (since 2026-04-22) | Default yes; opt-in mono is a simplified outline | R: `microsoft/PowerToys` `src/runner/tray_icon.cpp`, `general_settings.cpp` (`show_theme_adaptive_tray_icon = false`), `src/runner/svgs/*.ico`, commits #33321 and #47030, verified |
| 5 | Docker Desktop 4.97 | Mono whale | Yes (`-dark` and `-light` file pairs) | Shape: whale with moon (resource saver), with no-entry sign (blocked), with sync arrows, six animation frames (when they play: not checked); blue dot for update; whole whale red for error | Simplified (the whale alone; the app icon `assets/app/icon.ico` is a white whale on a blue disc) | L: `app.asar` `assets/tray/windows/*.ico` and the app icon, rendered here, verified |
| 6 | Telegram Desktop | Mono paper plane by default (`_trayIconMonochrome = true`); colour logo by setting | Yes: reads `SystemUsesLightTheme`; white on dark, black at alpha 228 on light | A dot for unread in mono mode, a counter in colour mode | Simplified (plane only; that the app icon is a white plane on a blue disc: from memory, unverified) | R: `telegramdesktop/tdesktop` `platform/win/tray_win.cpp`, `core/core_settings.h`, `changelog.txt`; L: white plane on `tray-taskbar-after-promote.png`; verified |
| 7 | Ollama | Greyscale: white llama with a thick dark outline, one picture for both taskbars | No (one file, readable on both) | Badge bottom right: dark disc with a down arrow for "update" | Yes, a variant of the app's llama | R: `ollama/ollama` `app/wintray/tray.go`, `app/assets/tray.ico`, `tray_upgrade.ico` (last change 2025-11-04), rendered here, verified |
| 8 | Bitwarden | Colour app icon (`images/icon.ico`); macOS gets a template image | No | None found | Yes | R: `bitwarden/clients` `apps/desktop/src/main/tray.main.ts` lines 33 to 43, verified |
| 9 | Dropbox | Mono glyph: "A solid black Dropbox icon" | Probably (a forum says white on a dark taskbar); no setting | The richest set: syncing arrows, pause, grey offline, grey with X on error, red count for unread, Zs when snoozed | Simplified (the box shape alone) | W: https://help.dropbox.com/sync/sync-icons-windows (updated 2026-09-15), state verified; theme unverified |
| 10 | Discord | Mono white glyph with a thin dark edge | No (one Windows set) | Red dot for unread; mic or headphones with a red slash for muted or deafened; green when in voice | Simplified | W: assets of a 2023 unofficial decompiled mirror (`h4xrOx/discord-decompiled`), unverified for today's client |
| 11 | Steam | Colour Steam logo | No | Weak: small corner overlay for messages and voice | Yes | W: `SteamTracking/SteamTracking` `ClientExtracted/public/steam_tray.ico` (unofficial extraction, file from 2021), unverified for today's client |
| 12 | 1Password 8 | Colour since 8.11.14 (2025-10-14, "visual improvements to the 1Password system tray icon"); black and white before | No setting; users still ask for mono in May 2026 | Changes with locked or unlocked (8.12.26 release note); the look: no source found | No source found | W: https://releases.1password.com/windows/stable/ (lock state verified); colour from a community thread, unverified |
| 13 | Claude desktop | Mono spark: black `Tray-Win32.ico`, white `Tray-Win32-Dark.ico` | Yes, but by the app theme, not the taskbar: an open bug shows a black spark on a dark taskbar when apps are light | None found | Simplified (the spark alone) | W: issue `anthropics/claude-code#97697` (2026-09-27) and a Linux port quoting the shipped code; unverified (second hand) |
| 14 | Slack | No source found | No source found | Badge: blue for unread, red for a mention or DM | No source found | W: https://slack.com/help/articles/360025446073, badges verified |
| 15 | Teams (new) | No source found | No source found | Presence moved into the tray menu (2025); an unread badge per user reports | No source found | W: Microsoft 365 message MC1011152 (verified), badge from a forum (unverified) |
| 16 | NVIDIA App, LM Studio, ChatGPT | No source found | No source found | No source found | No source found | W: only that each has a tray icon; no picture or asset found |

Web rows 9 to 16 were gathered by a helper agent on 2026-11-05 (notes and quotes in
`scratchpad\p6-brand-research-windows\tray-survey-web.md`, not committed). I checked rows 7 and 8 myself at the source.

The survey covers 18 apps plus Windows' own glyphs; 12 of them (rows 2 to 13) have a known look. Count over those 12,
PowerToys counted in both groups: one-colour glyph 8 (OneDrive, PowerToys opt-in, Docker, Telegram, Ollama
greyscale, Dropbox, Discord, Claude); full-colour app icon 5 (Windows Security, PowerToys default, Bitwarden, Steam,
1Password). Of the one-colour group, 5 follow the taskbar theme (OneDrive, PowerToys opt-in, Docker, Telegram,
probably Dropbox), Claude follows the app theme and has a bug for it, Ollama and Discord use one picture with an
outline that reads on both.

Patterns that hold across the survey:

- **Colour is used for state, not for the brand,** in the one-colour group: OneDrive's coloured badges, Docker's red
  whale and blue dot, Dropbox's red count, Discord's red dot and green voice, Windows' green charging battery. This is
  what T1 asks ("reserve these colors to communicate status").
- **State by a bottom-right badge** is the common form (Windows Security, OneDrive, PowerToys, Ollama, Docker,
  Steam), as T1 asks ("locate it bottom-right"). Shape changes are used for the main modes (Docker's moon and
  no-entry, Dropbox's pause, OneDrive's struck-through cloud).
- **Theme following reads the taskbar setting** `SystemUsesLightTheme` (Telegram's code; PowerToys reads the system
  theme). Claude reads the app theme and gets it wrong when the two differ, which Wakeshot already avoids
  (`tray/theme.rs` reads `SystemUsesLightTheme`, "the taskbar's mode").
- **Users ask for mono when an app ships colour:** PowerToys added the theme-adaptive mono icon as an opt-in in
  January 2026 (PR #33321); 1Password users have asked for a mono option since it turned colour in October 2025.
  Two cases; this is a signal, not a measurement.

### High contrast and colour-blind users

- High contrast: Windows Security ships separate black and white shield icons (groups 110 to 162 of
  `SecurityHealthSSO.dll`, picture above). Wakeshot's glyph takes the system text colour in high contrast
  (`brand.md` "Tray icon"). A colour icon must ship its own high contrast versions; D1 says Windows 11 no longer
  requires them for app icons, but the tray is drawn by the app, so nothing does it for you.
- Colour-blind: D1 "avoid relying on color alone". Wakeshot's three states differ in shape (test
  `states_differ_in_shape`, at least 25% of ink in different pixels; research.md). So any colour added later is a
  second cue, never the only one. Gold against white or ink is a lightness difference, not a hue one, so it survives
  red-green colour blindness (reasoning, not tested with a simulator).

### Contrast of the brand colours on the taskbar

WCAG 2 contrast, computed 2026-11-05 with a Python script, against the measured taskbar colours (research.md):

| Colour | Dark taskbar 202021 | Dark hover 2d2d2e | Light taskbar d8dadc | Light hover ecedee |
|---|---|---|---|---|
| Sun gold ffcc4d | 10.85 | 9.17 | **1.07** | **1.28** |
| Night violet 7250f4 | 3.23 | **2.73** | 3.59 | 4.30 |
| Violet light a996ff | 6.58 | 5.56 | **1.76** | 2.11 |
| White ffffff | 16.28 | 13.76 | 1.40 | 1.17 |
| Ink 1a1a1a | 1.07 | 1.27 | 12.42 | 14.85 |

So gold alone disappears on a light taskbar, and the brand violet is just above 3:1 on both bars and below it on the
dark hover plate. A one-colour brand glyph in violet would be weak; a gold mark needs an ink or white edge to read on
light.

### Trade-off for Wakeshot, and my pick

| Option | For | Against |
|---|---|---|
| A. One-colour glyph that matches Windows (today) | Looks native; high contrast is free; most of the survey and all of Windows' own glyphs do this | The brand never shows in the tray; "holding" reads only by shape at 16 to 24 px |
| B. Brand-colour icon (the violet plate) | Recognisable; 5 of the 12 known apps do it, Windows Security among them | A square plate is what T1 warns against; needs separate high contrast art; state then needs badges on a busy 16 px square; gold and violet fail contrast on one of the two bars; users of two apps asked to get mono back |
| C. One-colour glyph, with colour only while holding | Colour carries state, the use T1 and Windows' own battery make of it; the brand gold shows exactly when Wakeshot is doing something; at rest it blends in | Two art paths; gold must sit inside an ink or white outline to read on a light taskbar (1.07:1 alone); a yellow tray mark can read as "warning" |

**Pick: C.** The tray is Wakeshot's only surface that is always on screen, and "holding" is the state a user most needs
to see, because it costs battery and blocks sleep. Colour for state is what Windows itself does (the green charging
battery) and what most one-colour apps in the survey do. The glyph's shape and its ink or white outline stay the
first cue, so contrast, high contrast (no colour at all there) and colour-blind users are covered. Idle and off stay
plain, like Windows' own glyphs. Change it when users read the gold sun as a warning, or when Haik wants the tray to
look exactly like Windows' own glyphs (then A).

## 4. Tray icon vs app icon

From the survey in section 3 (sources there):

| Relation | Apps |
|---|---|
| The same picture as the app icon | Windows Security (the shield), PowerToys by default, Bitwarden, Steam; Ollama nearly (the same llama, greyscale) |
| The brand mark alone, in one colour, without the app icon's background or colours | Telegram (the plane without its blue disc), OneDrive (the cloud in white or black), Docker (the whale without its blue disc), Dropbox (the box), Discord (the Clyde face), Claude (the spark), PowerToys' opt-in mono icon (the outline of its device shape) |
| No link to an app icon | Windows' own system glyphs (wifi, volume, battery) |

- **The common pattern among one-colour apps is "the mark without its plate":** the same silhouette as the brand,
  drawn in the taskbar's ink colour. None of the surveyed apps draws a different picture in the tray than its brand
  mark. **Verified** for OneDrive, Docker and Ollama (tray and app icon files rendered here); Telegram's tray from its
  source, its app icon from memory; the others from the sources in section 3.
- **State variants keep the mark and add to it:** a badge in the bottom-right corner (Windows Security, OneDrive,
  PowerToys, Ollama, Docker's update dot, Steam), or a change to part of the mark (OneDrive's struck-through cloud,
  Docker's whale with a moon or a no-entry sign, Dropbox greyed). The mark stays readable in every state, so the user
  always knows whose icon it is.
- **One picture is reused in three places:** the app icon (mark plus colours, with or without a plate), the tray icon
  (mark in one colour, plus state), and the notice header (the app icon, section 6). Menu glyphs inside the app come
  from the system font, not from the brand (section 5).

For Wakeshot today: the holding tray glyph is the same sunrise as the app icon's mark, without the violet plate and
the gold, which is the "mark without its plate" pattern. Idle (empty sun) and off (struck through) change part of
the mark, like OneDrive's struck-through cloud. So the brand mark and the icons can be one drawing; what differs per
place is colour and plate, not the shape.

## 5. Menu glyphs

### Is mixing Segoe glyphs with one custom glyph normal?

Yes. Three cases, all **verified**:

- Windows' own File Explorer menu on this laptop mixes plain monoline glyphs (Cut, Copy, Properties), monoline
  glyphs with a blue accent part (Open with, Share with, Compress to) and full-colour app icons (Notepad, Microsoft
  365 Copilot, OneDrive) in one menu (`docs/research/img/tray-look-dark-explorer-new.png`, taken on this laptop
  2026-11-03).
- PowerToys Command Palette, Clipboard History (`src/modules/cmdpal/ext/Microsoft.CmdPal.Ext.ClipboardHistory/
  Icons.cs`, `microsoft/PowerToys` main, read 2026-11-05): Segoe glyphs (`\uE77F` paste, `\uE74D` delete), Fluent
  System Icons (`ic_fluent_clipboard_20_regular` and others) and its own SVG (`Assets\ClipboardHistory.svg`) in one
  list of icons.
- Fluent 2 asks one style for system icons: "Only use solid colors on system icons", "only use one color"
  (fluent2.microsoft.design/iconography, read 2026-11-05). A custom glyph fits if it keeps the same stroke, size and
  single colour as its neighbours.

What makes a mix look wrong is a different stroke weight, size or colour, not the mix itself (reasoning from the
cases above, not a Microsoft quote).

### Candidates for Keep awake, instead of the cup

Checked three ways on 2026-11-05, all **verified**:

1. Microsoft's list "Segoe Fluent Icons font": the docs source `hub/apps/design/iconography/segoe-fluent-icons-font.md`
   in the public repo `MicrosoftDocs/windows-dev-docs` (branch `docs`, ms.date 2026-02-19, 1533 rows), read with
   `gh api`. The live page (https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-fluent-icons-font,
   ms.date 2026-10-13, updated 2026-10-14, read 2026-11-05) lists E823 Recent, E916 Stopwatch, E917 Clock, E706
   Brightness, E793 Light, EA80 Lightbulb, E781 LEDLight, E890 View and E7E8 PowerButton with the same names; EC32 sits
   past the part that was read.
2. Microsoft's Segoe MDL2 Assets list, the same repo's `segoe-ui-symbol-font.md` (ms.date 2025-09-02, 1386 rows).
3. The cmap of the fonts on this laptop: `C:\Windows\Fonts\SegoeIcons.ttf` (2033 code points) and `segmdl2.ttf` (1833),
   read with a stdlib Python cmap reader (`scratchpad\p6-brand-research-windows\glyphs.py`). Then each was drawn at 16
   and 32 px in both fonts: `img/brand/win-menu-glyph-candidates.png`.

| Code | Name (Fluent list) | In Fluent list | In MDL2 list | In SegoeIcons.ttf | In segmdl2.ttf | Looks like |
|---|---|---|---|---|---|---|
| EC32 | Cafe | yes | yes | yes | yes | Cup (today's glyph) |
| E917 | Clock | yes | **no** | yes | **no** | Clock; an empty box in MDL2 |
| E823 | Recent | yes | yes | yes | yes | A plain clock in Fluent; a clock in MDL2 |
| E916 | Stopwatch | yes | yes | yes | yes | Stopwatch (the timer picture) |
| E81C | History | yes | yes | yes | yes | Clock with a back arrow |
| E706 | Brightness | yes | yes | yes | yes | Sun with rays |
| E793 | Light | yes | yes | yes | yes | Half-filled sun (reads as a theme or contrast switch) |
| EC8A | LowerBrightness | yes | yes | yes | yes | Small sun |
| EA80 | Lightbulb | yes | yes | yes | yes | Lightbulb |
| E781 | LEDLight | yes | yes | yes | yes | Lit bulb with rays |
| E7E8 | PowerButton | yes | yes | yes | yes | Power sign |
| E708 | QuietHours | yes | yes | yes | yes | Moon |
| E945 | LightningBolt | yes | yes | yes | yes | Bolt |

There is no "Timer" and no "Hourglass" name in either list, and no sunrise.

Notes for the pick (facts, the pick is `p6-brand-directions`'):

- E917 Clock fails the fallback rule: Wakeshot falls back to Segoe MDL2 Assets (`menu_paint.rs` `GLYPH_FACE_OLD`), and
  MDL2 has no E917. E823 Recent draws the same plain clock in Segoe Fluent Icons and is in MDL2 too.
- E706 Brightness is Windows' brightness sign (Quick Settings slider). The brand note already dropped the Sky concept
  because "the Sky's full sun is Windows' brightness sign" (`docs/brand.md`). The same clash applies in a menu.
- E916 Stopwatch says "timed" more than "awake"; the Keep awake item opens a submenu of durations, so a timer picture
  fits that item.
- The menu's other glyphs: E890 View (an eye) for Automatic, E777, E8A5, E711; all are in both fonts and both lists.

### Fluent System Icons as a source for a custom glyph

- Licence: MIT, "Copyright (c) 2020 Microsoft Corporation" (`LICENSE` in `microsoft/fluentui-system-icons`, read
  2026-11-05 with `gh api`, **verified**). Fluent 2: "System icons are open-source under the MIT License."
- The repo is active (last push 2026-11-04) and holds 2999 icon folders, each with SVGs at 16, 20, 24, 28, 32 and 48 px
  in regular and filled (some with light), plus TTF and WOFF fonts in `fonts/` (Regular, Filled, Light, Resizable).
  **Verified** (git tree read 2026-11-05).
- Related names there: Clock, Clock Alarm, Timer, Hourglass, Lightbulb, Weather Sunny, Weather Sunny High, Weather
  Sunny Low, Weather Moon, Drink Coffee, Eye, Power, Sleep. No sunrise and nothing with "wake". **Verified** (same
  tree).
- Use in apps: PowerToys Command Palette loads `ic_fluent_*_20_regular` icons by name (above). Microsoft's Fluent UI
  React icons are built from this repo (its `packages` folder, not read in detail: **unverified**).
- MIT asks that the copyright and permission notice go with copies of the software. For a glyph drawn into
  Wakeshot's code, the safe way is a line in the third-party notices; whether a redrawn shape counts as a copy is a
  legal question, not checked.
- Whether Fluent System Icons and Segoe Fluent Icons share stroke weights at 16 and 20 px: **unverified** (compare
  them side by side before mixing).

## 6. Notices

Wakeshot's notices are tray balloons (`Shell_NotifyIconW` with `NIF_INFO`, `crates/wakeshot-windows/src/notify.rs`),
which Windows 11 shows as a banner-only toast. Two pictures show up in one: a small one in the header, beside the app
name, and a large one in the body.

### Header: the registered app id's picture

- Installed Wakeshot runs under the app id `velopack.HaikAsatryan.Wakeshot`; its key
  `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot` holds `DisplayName` "Wakeshot" and `IconUri`
  naming `current\wakeshot-notice.png` (research.md "Notice header icon checks", **verified** by the e2e and smoke
  runs).
- On screen, 2026-11-05, both from the Start menu and from a direct start: "the header shows the violet plate with the
  gold sun and 'Wakeshot'", "about 22 px" on the 3840 x 2160 screen (`docs/tasks/p6-notice-app-icon.md` Log, 10:24
  and 10:50 entries, **verified** by grabs). That screen runs at 150%: its work area is 2088 of 2160 px, a 72 px
  taskbar (read 2026-11-05 with `System.Windows.Forms.Screen`), and 72 = 48 x 1.5 if the taskbar is 48 px at 100%
  (from memory, **unverified**). So the header picture is about 16 logical px (22 / 1.5 = 14.7 px of
  plate; the step from that to "16 px" is an inference, **unverified**).
- An exe that is not installed (a dev build) gets the shell's own id `NotifyIconGeneratedAumid_<n>`; then the header
  shows the tray icon itself (`docs/research/img/ux/notice-dark-first-run.png`: the white sunrise glyph beside
  `ux_notices.exe`). The shell writes those pictures as PNGs of 28 and 56 px in `%TEMP%` (research.md, **verified**).
- Microsoft's page on these registry values is gone (replaced in the docs repo on 2025-07-28; its URL returned 404 on
  2026-11-05); its last copy (2020) lists `DisplayName`, `IconUri`, `IconBackgroundColor` (`AARRGGBB`) and names no
  picture size (research.md, **verified**). No current Microsoft source gives the header size: **no source found**.
- One PNG serves both themes: the header sits on the notice's light or dark background (which theme setting the
  notice follows, and its light background colour: not checked here, **unverified**). So the header picture must read on both, at about 16 px.
  A free gold sun with no plate would sit on a near-white notice in light theme at about 1.5:1 (gold ffcc4d on white,
  `brand.md`); the violet plate avoids that.

### Body: the balloon icon

- `NOTIFYICONDATAW` (https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw, updated
  2024-11-20, read 2026-11-05, **verified**):
  - `hBalloonIcon`: "The handle of a customized notification icon provided by the application that should be used
    independently of the notification area icon. If this member is non-NULL and the NIIF_USER flag is set in the
    dwInfoFlags member, this icon is used as the notification icon."
  - `NIIF_LARGE_ICON`: "The large version of the icon should be used as the notification icon. This corresponds to the
    icon with dimensions SM_CXICON x SM_CYICON. If this flag is not set, the icon with dimensions XM_CXSMICON x
    SM_CYSMICON is used." "New customized icons (NIIF_USER with hBalloonIcon) must supply an SM_CXICON x SM_CYICON
    version in the supplied icon (hBalloonIcon)."
  - `hIcon` (the tray icon): "It is recommended that you provide both a 16x16 pixel icon and a 32x32 icon".
- Wakeshot sets `NIIF_USER | NIIF_LARGE_ICON` with the app icon at `SM_CXICON` for the taskbar's DPI (`notify.rs`
  lines 293 to 304 and 390 to 394): 32 px at 100%, 48 px at 150%. The banner on screen shows "the large app icon"
  in the body (p6-notice-app-icon Log 10:24).
- Windows stores the balloon as `<toast bannerOnly="true">` with the `ToastImageAndText02` template
  (p6-notice-app-icon Log 10:28, **verified** from Windows' notification database). How large the banner draws the body
  picture at each scale: not measured, **unverified**.
- D3 (Windows 7 era): "Balloon icons: 32x32 and 40x40."

### What this means for the app icon

- The same picture shows at about 16 px (header) and 32 to 48 px (body), on a light or dark notice. So the app icon
  must read at 16 px on both themes, which is the same test as the taskbar and title bar (D2's 16 px row).
- For a dev build the header shows the tray glyph instead; that is not what users see.
