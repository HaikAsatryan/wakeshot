# Brand research: Windows app icon, tray icon and menu glyphs in 2026

Phase: 6 (brand review). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens. Written by
the orchestrator of Haik's brand session, 2026-11-05.
Runs after: none. Parallel with: `p6-brand-research-macos`, `p6-brand-research-identity`, and any building card (this
card builds nothing and edits no code).
Real laptop: no power settings, no installs. Screenshots of this laptop's taskbar are allowed (read only).
Scratch folder: `<scratchpad>\p6-brand-research-windows\`.

## Goal

Haik is reviewing the whole brand before launch and everything may change. Find what Microsoft asks for and what good
Windows apps really do in 2026 for the app icon, the tray (notification area) icon and menu item glyphs, so the brand
directions card can decide on facts.

## Read first

- `docs/brand.md` (the current brand and its reasons).
- `docs/research.md`, section "Tray UX: icon art" (colours measured on this laptop, Microsoft quotes already found).
- `crates/wakeshot-windows/src/tray/menu_paint.rs` lines 55 to 70 (the menu glyphs; the Keep awake cup is going away).
- Pictures: `docs/research/img/ux/icon-app.png`, `icon-sheet-dark.png`, `icon-sheet-light.png`,
  `menu-screen-dark-main.png`, `taskbar-dark-holding.png`, `notice-dark-first-run.png`.

## Haik's open questions this card feeds

He is not sure the logo is good, not sure about the colours, not sure about a white or black (monochrome) tray icon,
and asks how the tray icon and the app icon should differ, and whether the brand mark and the icons should be the same.

## Deliverables

`docs/research/brand-windows.md`. Write it as you go (a section at a time), so a session limit loses nothing. Mark each
fact **verified** (read at the source, URL and date) or **unverified** (secondary source, or not read in full). Answer:

1. **App icon guidance, current.** Microsoft's "Design guidelines for Windows app icons" and any newer Fluent 2 icon
   pages: metaphor, plate or no plate (is a coloured rounded-square plate right on Windows 11, or a Mac habit?),
   perspective, gradients, shadows, colour, contrast on light and dark Start, the ICO frame sizes Windows asks for
   (does a 256 px frame matter, for Explorer large icons or Settings?), and the Store asset sizes in case of
   `docs/tasks/p8-store.md`. Quote the key lines.
2. **What Windows' own and Microsoft's newest app icons look like in 2026** (Fluent icons, the Microsoft 365 icon
   refresh if there was one, PowerToys, Windows Terminal, Dev Home or its successor): plated or free shapes, flat or
   with depth, how much gradient. Any 2025 to 2026 change toward glass or 3D.
3. **Tray icon guidance and practice.** Microsoft's notification area guidance (old and current), and a survey of at
   least 15 real tray icons on Windows 11 in 2026: Windows' own glyphs, then apps such as Teams, OneDrive, Dropbox,
   Docker Desktop, Slack, Discord, Steam, 1Password, Bitwarden, NVIDIA, PowerToys, Ollama, LM Studio, the Claude and
   ChatGPT desktop apps, Windows Security. For each: monochrome or colour, does it follow the light or dark taskbar,
   how it shows state (shape, badge, colour). Use this laptop's own taskbar where an app is installed (screenshot
   crops in your scratch folder; commit only tight crops with nothing personal, into `docs/research/img/brand/`),
   otherwise the app's own files or docs. Then the trade-off for Wakeshot, with your pick: monochrome glyph that
   matches Windows, a brand-coloured icon, or a monochrome glyph with a colour only while holding. Include high
   contrast and colour-blind users.
4. **Tray icon vs app icon.** Do good apps use the same picture for both, a simplified mark, or something else? How
   state variants relate to the brand mark. Cite examples.
5. **Menu glyphs.** Is mixing Segoe Fluent Icons with one custom brand glyph in a menu normal? Candidates for the Keep
   awake item instead of the cup, with code points verified in Microsoft's current Segoe Fluent Icons list and present
   in Segoe MDL2 Assets too (our fallback font): at least a clock or timer, a sun, a lightbulb. Fluent System Icons
   (MIT) as a source for custom glyphs: licence and how other apps use it.
6. **Notices.** Which icon Windows shows in a notice header and body for an unpackaged app with a registered app id,
   and at what size, so the app icon reads there.

Keep it to facts and short reasons. No design work and no mockups: `p6-brand-directions` does that.

## Acceptance checks

- Every numbered question above has an answer or says plainly that no source was found.
- Every Microsoft guideline quoted has its URL and the date read.
- The survey has at least 15 apps, each with its source (this laptop, the app's files, or a named page).
- Committed pictures, if any, hold no personal data (no names, no other apps' private text, no notifications).

## Out of scope

Code, `docs/brand.md`, the cards of other sessions, `docs/plan.md` and phase files (another orchestrator session runs
in parallel and owns them). macOS (`p6-brand-research-macos`). Logo and colour theory (`p6-brand-research-identity`).

## Questions for Haik

None expected. If one comes up, write it here with your default and go on.

## Found outside scope

1. `docs/tasks/p6-int-haik-checks.md` line 285 says "The exe's icon resource is right (violet plate, gold sun, 16 to
   256 px)". The exes embed `assets/wakeshot.ico` (`crates/build-resources.rs` line 13), which holds seven frames, 16
   to 64 px, no 256 (read 2026-11-05 with a Python ICO header reader). So "to 256 px" is wrong, or that card read a
   different file. Not fixed (not my card).
2. Microsoft asks for a 256 px frame at the bare minimum (`app-icon-construction`, updated 2026-08-05); Start pins at
   400% and the shell's 256 px jumbo list use it. Track note 3.2 in `docs/phases/05-tray-ux.md` defers it until
   something "looks blurry". The brand directions card may want to add it with the new art. Report section 1.
3. `docs/research.md` gets no section from this card: the orchestrator named my files for this round and
   `research.md` was not among them. The facts and how they were checked are in `docs/research/brand-windows.md`; a
   one-line pointer in `research.md` would help the next reader.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started. Read CLAUDE.md, plan "How we work", brand.md, research.md "Tray UX: icon art",
  menu_paint.rs glyph lines, the six pictures. Report skeleton next, then sources a section at a time.
- 2026-11-05: report skeleton written. Screen is locked (LogonUI runs in the session), so live taskbar captures are
  blank; the survey uses earlier committed captures of this laptop and the apps' own files. One helper agent surveys
  12 closed or remote apps on the web into the scratch folder (`tray-survey-web.md`).
- 2026-11-05: sections 1 (app icon guidance), 3 first half (guidance, Windows glyphs, 6 survey rows from this laptop's
  files and repos) and 5 (menu glyphs, cmap of both fonts) written. Pictures `win-menu-glyph-candidates.png`,
  `win-microsoft-app-icons.png` added. Next: section 2, 6, then the web rows when the helper ends.
- 2026-11-05: section 6 (notices) written. Helper ended; rows 7 and 8 (Ollama, Bitwarden) re-checked at the source by
  me, the rest kept with the helper's marks. Section 3 finished: 16 rows (18 apps plus Windows), brand colour
  contrast on the taskbars, trade-off and pick (C: one-colour glyph, colour only while holding). Picture
  `win-tray-icon-files.png` added. Next: sections 2 and 4, summary.
- 2026-11-05: sections 2 and 4 and the summary written. Checked the shipped ICO (7 frames, 16 to 64, no 256) and the
  live Segoe Fluent Icons page for the candidates. Acceptance checks run (below). Done.

## Acceptance checks run (2026-11-05)

1. Every numbered question answered: pass. Sections 1 to 6 of the report each answer one; where nothing was found
   they say "no source found" (header size, Windows glass redesign, five apps' tray look).
2. Every Microsoft guideline quoted has its URL and date read: pass. 13 Microsoft URLs, each with "read 2026-11-05"
   or under a source list that says so; the two docs-repo files are named by path and branch.
3. Survey of at least 15 apps with a source each: pass. 16 rows, 18 apps plus Windows' own glyphs; every row names
   its source (this laptop's files, the app's repo, or a named page).
4. Committed pictures hold no personal data: pass. `win-menu-glyph-candidates.png` (font glyphs and code points),
   `win-microsoft-app-icons.png` (Microsoft app icons from installed packages), `win-tray-icon-files.png` (tray icon
   files with labels). No screen capture of this laptop is committed: the screen was locked, so none was taken.

## Result

Done 2026-11-05. Report: `docs/research/brand-windows.md` (summary first, then the six questions). Pictures:
`docs/research/img/brand/win-microsoft-app-icons.png`, `win-tray-icon-files.png`, `win-menu-glyph-candidates.png`. No
code, no power settings, no installs touched.

Key findings:

1. Microsoft prefers free shapes on Windows: "Icons look best with a transparent background", a plate is "okay too"
   at the cost of theme work. All 20 Microsoft app icons installed here are free shapes with soft depth, no backdrop
   plate. Microsoft 365's 2025 refresh went to fluid shapes, richer gradients, "depth, and highlights"; no source shows
   a Windows move to glass.
2. The violet plate still does real work: gold on light is 1.35:1 without it, and the notice header uses one PNG at
   about 16 px for light and dark notices.
3. Tray: of 12 apps with a known look, 8 use a one-colour mark, 5 their colour icon (PowerToys both). One-colour apps
   put colour only on state. Gold alone is 1.07:1 on the light taskbar; violet 2.7 to 3.6:1. The tray icon is the
   brand mark without its plate in every one-colour app.
4. Pick for the tray: a one-colour glyph with colour only while holding (gold inside an ink or white outline). Change
   it when users read the gold as a warning, or Haik wants the tray identical to Windows' glyphs.
5. Menu: mixing is normal (Explorer, PowerToys). E917 Clock is missing from Segoe MDL2 Assets, the fallback; E823
   Recent draws the same clock in both fonts. E706 sun is Windows' brightness sign. Fluent System Icons are MIT, no
   sunrise.
6. The ICO has no 256 px frame; Microsoft's minimum includes 256.

Open: the live taskbar was not captured (screen locked); Slack, Teams, NVIDIA, LM Studio and ChatGPT tray looks not
found; Claude desktop's tray pixels second hand; header size in logical px inferred; Partner Center's EXE asset form
not read. Outside scope: three items above (a wrong "16 to 256 px" line in `p6-int-haik-checks.md`, the 256 frame,
no `research.md` pointer).
