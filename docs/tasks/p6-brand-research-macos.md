# Brand research: what a later macOS Wakeshot needs (Liquid Glass icon, menu bar glyph, menu or panel)

Phase: 6 (brand review). Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 120K tokens. Written by
the orchestrator of Haik's brand session, 2026-11-05.
Runs after: none. Parallel with: `p6-brand-research-windows`, `p6-brand-research-identity`, and any building card
(this card builds nothing and edits no code).
Real laptop: nothing on the laptop; web research only.
Scratch folder: `<scratchpad>\p6-brand-research-macos\`.

## Goal

A macOS version is planned for later (`site/index.html`: "macOS is planned for later"). Haik wants the brand built now so
it also works on the Mac: is the Mac icon different, glassy, "Tahoe compliant"; does the Mac menu bar panel differ from
the Windows tray menu. Find the current Apple rules and what good cross-platform apps do, so the brand directions card
can design one mark that works on both.

## Read first

- `docs/brand.md` (the current brand and its reasons).
- `docs/product.md`: search for "mac" to see what is decided about macOS.
- Pictures: `docs/research/img/ux/icon-app.png`, `icon-sheet-dark.png`, `menu-screen-dark-main.png`.

## Deliverables

`docs/research/brand-macos.md`. Write it as you go (a section at a time), so a session limit loses nothing. Mark each
fact **verified** (read at the source, URL and date) or **unverified**. Use Apple's current Human Interface Guidelines
and developer docs, not memory: macOS 26 Tahoe shipped in 2025, and WWDC 2026 (June) may have changed things for the
next macOS. Say which version each rule is from. Answer:

1. **App icon on macOS now.** Liquid Glass icons: Icon Composer, layers (background and foreground groups), the canvas
   and grid, the shape (is the rounded-square mask enforced; what happens to an icon that does not follow it), the
   appearance modes (default, dark, clear, tinted) and what each does to colours, gradients and a gold-on-violet mark.
   What a designer must deliver (`.icon` file, sizes, `.icns` for older macOS). Quote the key lines.
2. **Menu bar extra (the tray icon's twin).** HIG rules: template images, size in points, monochrome or colour, how
   state is shown, how the Tahoe transparent menu bar affects it, SF Symbols use and licence for a custom glyph.
3. **The menu or panel.** Native `NSMenu` (what Liquid Glass does to it automatically) versus a popover or custom
   window (SwiftUI `MenuBarExtra` with the window style, `glassEffect`). What good small menu bar utilities do in 2026
   (for example Amphetamine, Lungo, KeepingYouAwake, Raycast, Bartender or Ice, Stats, iStat Menus, 1Password). Your
   pick for Wakeshot, given its Windows menu is a native-looking menu (`docs/research/tray-menu-look.md`).
4. **One brand across Windows and macOS.** How good cross-platform apps adapt one mark: same picture and colours with a
   per-platform shape and finish, or different icons? Cite at least 8 apps with both versions (for example Slack,
   Spotify, VS Code, Docker, 1Password, Raycast, Figma, Notion, Discord, Arc or its successor, ChatGPT, Claude).
   Should the Windows icon copy the Mac glass look, or should each follow its own platform? Your pick with reasons.
5. **What this asks of the mark now.** Concrete constraints for `p6-brand-directions`: the mark must survive the
   squircle mask, the glass layers, the tinted and clear modes, and a template (one colour) menu bar glyph at 16 to 22
   points. List them as short rules.
6. **Keep-awake apps on the Mac** (Amphetamine, Lungo, KeepingYouAwake, Caffeine, Theine and any newer one, including
   ones for AI agents): their app icons and menu bar glyphs, so the Wakeshot mark does not look like one of them.

Note for Haik's wording: his message said "IOS"; read it as macOS (the Mac menu bar app). If you find a real reason a
keep-awake tool would need an iPhone or iPad app, say so in one line; otherwise skip iOS.

No design work and no mockups: `p6-brand-directions` does that.

## Acceptance checks

- Every numbered question has an answer or says plainly that no source was found.
- Every Apple rule quoted has its URL, the date read, and the macOS version it applies to.
- Question 4 cites at least 8 apps, each with where both icons were seen.

## Out of scope

Code, `docs/brand.md`, `docs/plan.md` and phase files (another orchestrator session owns them). Windows rules
(`p6-brand-research-windows`). Colour and logo theory (`p6-brand-research-identity`). How to build a Mac app in Rust.

## Questions for Haik

None expected. If one comes up, write it here with your default and go on.

1. Not blocking this card. Shut the Lid, a free Mac lid-closed keep-awake app released 2026-11-04, already uses a gold
   half sun over a white line on a blue-indigo rounded square, and a sun-over-horizon menu bar glyph; SF Symbols has
   `sun.horizon.fill` too. Keep the sunrise idea, or start over? Default: keep the idea, but `p6-brand-directions`
   gives it a feature of its own and tests every direction side by side with Shut the Lid's icon and
   `sun.horizon.fill`; drop the sunrise only if no variant reads clearly different.

## Found outside scope

- `docs/brand.md` says "no keep-awake rival uses violet". True for the Windows rivals it checked; on the Mac, Vigil,
  Sleepless, Keep Awake: AI Agents and InsomniaZ use violet plates and Shut the Lid a blue-indigo one
  (`docs/research/brand-macos.md`, section 6). For `p6-brand-research-identity` and `p6-brand-directions`.
- Not checked: whether Microsoft's Fluent icon set or Segoe Fluent Icons has a sun-on-horizon glyph as well (the
  Windows twin of the `sun.horizon` question). For `p6-brand-research-windows`.
- `docs/brand.md`'s wordmark font, Segoe UI Variable, does not exist on the Mac; its noted fallback, Inter, works on
  both. For `p6-brand-research-identity`.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started. Read brand.md, product.md (mac lines), the three pictures. Web research next.
- 2026-11-05: Apple pages read through their JSON copies (exact text). Current macOS is 27 Golden Gate. Section 1
  (app icon) written. Next: menu bar glyph, panel, cross-platform apps, rivals.
- 2026-11-05: sections 2 (menu bar glyph) and 3 (Apple rules for menu or panel) and iOS written. Found: SF Symbols
  has `sun.horizon.fill`, close to our mark; macOS 27 hides menu item images by default. Two helper agents survey
  cross-platform icons (Q4) and Mac keep-awake and menu bar apps (Q3, Q6); results go to the scratch folder.
- 2026-11-05: orchestrator relayed Haik: "IOS" meant macOS; Mac version is a real plan, nothing ships for it yet; no
  budget, free fonts and our own drawing code only.
- 2026-11-05: sections 5 (rules for the mark) and 6 (Mac keep-awake apps, from the helper's survey) written; section 4
  started (platform rules). Waiting for the cross-platform icon survey and the menu bar utility survey.
- 2026-11-05: both helpers done (9 cross-platform apps; about 65 Mac keep-awake apps and 6 menu bar utilities). Found
  Shut the Lid (gold sunrise on blue-indigo, Mac, 2026-11-04); checked its listing, icon and glyph myself, and two of
  the cross-platform icons. Sections 3, 4, 6, short answer and sources finished; research.md entry added.
- 2026-11-05: acceptance checks run (below). Status done.

## Result

Done 2026-11-05.

What was done: desk research on what a later Mac Wakeshot needs from the brand, written to
`docs/research/brand-macos.md` (short answer, sections 1 to 6, iOS, sources). Apple's HIG and developer pages were
read as exact text through their JSON copies; two helper agents surveyed app icons and menus by downloading and
looking at the real images and code.

Key findings: the current Mac is macOS 27 Golden Gate; Mac icons are one layered Icon Composer `.icon` masked to the
rounded square, with six appearances, four of them colourless; a Mac app (Shut the Lid, 2026-11-04) already has a
gold sunrise on a blue-indigo plate, and SF Symbols has `sun.horizon.fill`; violet is common among new Mac
keep-awake apps; 9 of 9 cross-platform apps keep one picture with a per-platform finish and none puts glass on
Windows; macOS 27 hides menu item icons.

Picks: same mark on both platforms, Liquid Glass `.icon` on the Mac and the flat Fluent plate on Windows, from one set
of SVG layers drawn by our art code. Mac menu: native `NSMenu` with the Windows items and words, a status line as a
section header, no item icons, three template glyph states. Change triggers are in sections 3 and 4.

Files changed: `docs/research/brand-macos.md` (new), `docs/research.md` (new section "Brand on macOS" at the end),
this card. No pictures added: none was needed, and the survey images are other people's art.

Acceptance checks:

- Every numbered question answered: yes, sections 1 to 6 each answer theirs; iOS skipped as Haik confirmed. Pass.
- Every Apple rule quoted has URL, date read and macOS version: each quote names its page; the Sources list gives the
  URLs, the date (2026-11-05) and the rule that the current pages apply to macOS 26 and 27; API lines give their
  first macOS version. Pass.
- Question 4 cites at least 8 apps with where both icons were seen: 9 apps, each with its Mac and Windows source in
  the table in section 4. Pass.
- Checked by grep: no em dashes, no lines left "to do".

Not verified (also marked in the report): how our icon renders in Icon Composer's mono, clear and tinted modes (needs
a Mac); Apple's exact icon grid keylines; the SF Symbols licence text itself; whether a status item's menu counts as
a "menu bar" menu for the macOS 27 icon hiding; how colour glyphs look on the clear Tahoe menu bar; the glyph sizes
(Bjango, not Apple); what changed in macOS 27's menu bar icons; the macOS 27 release date (press only).

Open items: Haik's question above (keep the sunrise or not, non-blocking); the three items under "Found outside
scope".