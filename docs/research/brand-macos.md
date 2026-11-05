# Brand on macOS: what a later Mac Wakeshot needs

Desk research for the card `docs/tasks/p6-brand-research-macos.md`, 2026-11-05. Web only: no Mac was used, nothing
was built. Each fact says **verified** (read at the source, with URL and date) or **unverified**. Apple's design
pages render with script; where the normal page would not load, the same text was read from Apple's own JSON copy of
the page (`developer.apple.com/tutorials/data/design/human-interface-guidelines/<page>.json`), which is the data the
page shows.

## Short answer

1. **The current Mac is macOS 27 Golden Gate; "Tahoe compliant" now means Liquid Glass compliant.** The Mac app icon
   is one layered `.icon` file made in Apple's Icon Composer (Mac only, macOS 26.4 or later): square, full-bleed
   layers on a 1024 px canvas, at most four groups, which macOS masks to its rounded square and lights as glass. An
   icon with its own shape gets shrunk onto a grey backing. Users can pick six looks; in four of them (clear and
   tinted) our violet and gold are gone and only the outline reads. Xcode or `actool` makes the fallback for older
   macOS from the same file. (Section 1.)
2. **A Mac app already wears our picture.** Shut the Lid, a free lid-closed keep-awake app, went into the Mac App
   Store on 2026-11-04 with a gold half sun over a white line on a blue-indigo rounded square, and a sun-over-horizon
   menu bar glyph. SF Symbols also has `sun.horizon.fill`, and Apple's terms bar symbols "or images that are
   confusingly similar" in app icons and logos. The sunrise needs a feature of its own, or a new picture, before
   the brand is final. (Sections 2 and 6.)
3. **Violet is not ours alone on the Mac.** New keep-awake apps for AI agents use violet or blue-violet plates
   (Vigil, Sleepless, Keep Awake: AI Agents, Shut the Lid). The cup is still the category sign, as on Windows.
   (Section 6.)
4. **Same mark, each platform's own finish.** All 9 cross-platform apps checked (Slack, Telegram, VS Code, Signal,
   Discord, Figma, Notion, Firefox, 1Password) keep one picture and palette and change only the container and
   finish; 5 already ship a Liquid Glass Mac icon; none copies glass onto Windows, and Microsoft's guide says to avoid
   "reflections or dimension". Pick: a Liquid Glass `.icon` on the Mac, the flat Fluent plate on Windows, both from
   one set of SVG layers drawn by our art code. (Section 4.)
5. **Menu bar: a one-colour template glyph and a native menu.** Apple: "black and clear colors", a 24 pt bar,
   "Display a menu", "not a popover". The glyph is about 16 pt in a 22 pt box with three states by shape, as on
   Windows. The native `NSMenu` gets Liquid Glass for free; macOS 27 hides menu item icons by default, so the Mac
   menu has the Windows items and words without icons. A custom glass panel only if we ever show live data.
   (Sections 2 and 3.)

Rules for the brand directions card: section 5. Not verified: listed in each section and in the card's Result.

## Which macOS is "now"

- The current Mac system is **macOS 27 Golden Gate**, not Tahoe. Apple's macOS page names it and says: "More
  refined design with enhanced Liquid Glass" and "updated window shapes and menu bar icons" (apple.com/macos, read
  2026-11-05; **verified**). It shipped on 2026-09-14 (9to5Mac and Eclectic Light, 2026-09-09 and 2026-09-14;
  **unverified** at Apple). Tahoe is macOS 26 (2025), the release that brought Liquid Glass.
- So "Tahoe compliant" today means "Liquid Glass compliant": the icon format and rules from macOS 26 still hold in 27,
  with a refined rendering. Apple's Icon Composer article: "to compare macOS 26 with macOS 27 rendering, click 26 and
  then 27", and "In iOS, iPadOS, macOS, and watchOS versions earlier than 27, specular highlights appear on when you
  choose Inside or Outside, and Refraction settings have no visible effect" (Creating your app icon using Icon
  Composer, read 2026-11-05; **verified**). One `.icon` file serves both.
- The HIG App icons page was last changed 2026-10-28 ("Added guidance for app icon format options") and 2026-06-08
  ("Refined guidance for Liquid Glass") (its change log, read 2026-11-05; **verified**). The quotes below are from
  that current text, so they apply to macOS 26 and 27.

## 1. App icon on macOS now

### The format: one layered `.icon` file

- "iOS, iPadOS, macOS, and watchOS app icons include a background layer and one or more foreground layers that
  coalesce to create dimensionality. These icons take on Liquid Glass attributes like specular highlights,
  refraction, and translucency." (HIG App icons, Layer design.)
- "In Icon Composer, you define the background layer for your icon, adjust your foreground layer placement, apply
  visual effects like specular highlights and refraction, annotate for default, dark, and mono appearance variants,
  test and preview your icon across system versions, and export your icon for use in Xcode." (Same.)
- Two formats are allowed: "nonlayered bitmap images (such as PNG) or layered Icon Composer files". With a flat PNG,
  "the system attempts to segment your icon, isolating the foreground elements from the background to create an icon
  with two layers", and "It doesn't automatically apply Liquid Glass effects to smaller, more complex elements in an
  icon like text and thin lines." Apple's advice: "Provide your icon in the Icon Composer format for more flexibility
  and control." (HIG App icons, Icon formats, added 2026-10-28.)
- Groups: "organize the layers that appear in the default group into a maximum of four groups to reduce complexity.
  The groups become the layers in the app icon image the platform renders to give the icon its depth." (Creating your
  app icon using Icon Composer.) In the WWDC25 talk: "you can go all the way up to four. We found this number provides
  the right bounds for how much visual complexity an icon should have." (WWDC25 session 361, Create icons with Icon
  Composer, transcript read 2026-11-05.)
- Background: "Icon Composer supports solid colors and gradients for background layers, making it unnecessary to
  import custom background images in most cases. If you do import a background layer, make sure it's full-bleed and
  opaque." (HIG App icons.) So our violet plate becomes an Icon Composer gradient fill, not artwork.
- Edges and opacity: "Prefer clearly defined edges in foreground layers. To ensure system-drawn highlights and
  shadows look best, avoid soft and feathered edges on foreground layer shapes." "Vary opacity in foreground layers to
  increase the sense of depth and liveliness." (HIG App icons.)
- Effects: "there's no need to include specular highlights, drop shadows between layers, beveled edges, blurs, glows,
  and other effects." (HIG App icons, Visual effects.)
- Line weight: "Make sure to avoid extremely thin line weights and sharp corners, because they tend to lose detail and
  crispness in smaller icon sizes at lower resolutions." (HIG App icons, Design.) And in the WWDC25 talk: "Ideally,
  sharp edges and thin lines should be avoided. Instead, using rounder corners makes it easier for the light to
  seamlessly travel on the edges of an element", and "it helps to use bolder line weights as it will preserve details
  at a smaller scale." (WWDC25 session 220, Say hello to the new look of app icons, transcript read 2026-11-05.)
- Consistency: "Provide a visually consistent icon design across all the platforms your app supports. A consistent
  design helps people quickly find your app wherever it appears and prevents people from mistaking your app for
  multiple apps." (HIG App icons, Design.)

All **verified**: HIG page read 2026-11-05 through Apple's JSON copy of
`https://developer.apple.com/design/human-interface-guidelines/app-icons`; applies to macOS 26 and 27.

### The canvas, the grid and the shape

- Canvas: "1024x1024 px", layout shape "Square", "Icon shape after system masking: Rounded rectangle (square)" for
  iOS, iPadOS and macOS (HIG App icons, Specifications). Exported layers should match: "1024 x 1024 pixels for iPhone,
  iPad, and Mac" (Creating your app icon using Icon Composer). **Verified.**
- The mask is enforced. "In iOS, iPadOS, and macOS, icons are square, and the system applies masking to produce
  rounded corners". "For iOS, iPadOS, and macOS icons, provide square layers so the system can apply rounded corners.
  ... Providing layers with pre-defined masking negatively impacts specular highlight effects and makes edges look
  jagged." "Don't export the canvas mask because the system applies that automatically" (HIG App icons; Icon Composer
  article). **Verified.**
- An icon that does not follow it: "Irregularly shaped icons receive a system-provided background." (Adopting Liquid
  Glass, App icons, read 2026-11-05; **verified**.) In the WWDC25 talk: "In order to avoid irregular shapes, the
  canvas shape now acts as a mask to designs", and for an odd shape "the system removes any drop shadows and then
  auto-scales the artwork into our rounded rectangle canvas" (session 220; **verified**). In practice on Tahoe that
  background is a grey squircle with the old icon shrunk inside; developers call it "icon jail" (Rogue Amoeba, "Free
  The Icons", 2026-06-26: "Any icon that failed to do so found itself shrunk down and imprisoned in an ugly gray
  background"; heise and 9to5Mac report the same; **unverified** by us on a Mac). The same post says the first macOS
  27 betas showed only "signs of a turnaround", mainly in Apple's own icons; the current HIG still lists the rounded
  rectangle as the only macOS shape.
- Grid: the grid is in the production templates: "use the grids in the app icon production templates, which you can
  find in Apple Design Resources" (HIG App icons). The WWDC25 talk: "We've updated the design grid to a simpler and
  more evenly spaced structure. And the new grid also features a rounder corner radius" (session 220). Apple Design
  Resources today offers "App Icon Template" for "iOS, iPadOS and watchOS 27" (Figma, Sketch, Photoshop and
  Illustrator) and only a UI Kit for macOS 27 (developer.apple.com/design/resources, read 2026-11-05; **verified**);
  the Mac shares the iOS rounded rectangle and 1024 canvas (HIG Specifications), so that template should fit the Mac
  too (our inference). We did not download it; the exact keylines are **not verified**. "Keep primary content
  centered to avoid truncation when the system adjusts corners or applies masking." (HIG.)

### The appearances and what each does to colour

- macOS 26 and 27 offer six: "Default, dark, clear light, clear dark, tinted light, tinted dark" (HIG App icons,
  Specifications). "people can choose whether their Home Screen app icons are default, dark, clear, or tinted in
  appearance", and "the system automatically generates variants you don't provide." (HIG.) **Verified.**
- In Icon Composer the designer edits three: "default, dark, and mono. For mono, you can preview clear and tinted
  variants as well." (Icon Composer article.) The WWDC25 talk: "we renamed these to default, dark and mono, with the
  artwork producing all the appearances for clear and for tinted." **Verified.**
- Mono needs one white element: "Setting at least one element of your icon to be white, usually the most prominent
  or recognizable part, make sure it shows up strong. and the other colors can be mapped to tones of gray." "Icon
  Composer will do an automatic conversion for this, but it's important to tune it to get the best contrast."
  (session 361; **verified**.)
- Tint: "a dark tint that adds color to the foreground, and a light tint where the color gets directly infused into
  the glass." (session 220; **verified**.)
- Dark: "You can preserve the color palette of your default icon, but be mindful that dark icons are more subdued,
  and clear and tinted icons are even more so." "Color backgrounds generally offer the greatest contrast in dark
  icons." (HIG.) The WWDC25 talk adds: "we actually recommend leaning more into colored backgrounds, so that there is
  a nicer distinction when switching between modes", and offers "a System Light and System Dark gradient that should
  be used instead of pure white or black backgrounds." **Verified.**
- Keep the same picture: "keep your icon's core visual features the same in the default, dark, clear, and tinted
  appearances. Avoid creating custom icon variants that swap elements in and out with each variant" (HIG). **Verified.**

What that does to a gold sun on a violet plate (our reading of the rules above, **not tested** in Icon Composer: it
runs only on a Mac):

| Appearance | Violet plate | Gold sun | White horizon | Risk |
|---|---|---|---|---|
| Default | Kept, gets glass light and depth | Kept | Kept | None; the best case |
| Dark | Kept or darkened by the designer; Apple says keep a coloured background | Kept, maybe a little toned down | Kept | Low |
| Clear light and clear dark | Gone: the plate becomes clear glass over the wallpaper | Becomes frosted white or grey glass | Becomes white or grey | High: violet and gold are lost; only the shapes and their grey levels remain |
| Tinted light and tinted dark | Takes the user's tint colour | A lighter or darker tone of the same tint | Same | High: the same as clear, in one hue |

So in four of the six Mac appearances the brand colours are gone, and the mark must be recognised by its outline
alone. A half sun with rays over a line still reads as a sunrise in one colour (it is drawn that way in our tray
today), so the idea survives; the thin parts are the risk (next point).

### Our current icon against these rules

- The plate with its own rounded corners (radius 3/16) cannot be shipped as is: the Mac wants a square, full-bleed,
  opaque background that the system masks. Drawn corners would get the "pre-defined masking" problem, or a grey
  backing.
- The horizon line and the three rays are thin. The HIG warns that thin lines lose detail and that flat PNGs get no
  glass on "thin lines". In a layered `.icon` they get glass if we put them on their own layer, but they must be
  bolder than on Windows to hold up at 16 and 32 px.
- The sun, the rays and the horizon map well onto Apple's layer model: the background as an Icon Composer gradient
  fill (violet), then one group for the sun disc (the white element in mono), one for the rays, one for the horizon.
  That is three groups, inside the limit of four. (That the background fill is not one of the four groups is our
  reading of the article, **not verified** in the app.)

### What a designer must deliver for the Mac

1. Layer artwork as SVG (PNG only for raster parts), on a 1024 x 1024 canvas, one file per layer, "meaningful names
   that include numbers (increment from back to front)", text outlined, no background colour, no blur, no shadow, no
   mask (Icon Composer article, "Prepare your artwork for export"; **verified**).
2. One Icon Composer file (`AppIcon.icon`) with the background gradient, the groups, the Liquid Glass settings, and
   tuned dark and mono variants, checked at 26 and 27 rendering (same article; **verified**). Icon Composer runs on a
   Mac only: "Requires macOS Tahoe 26.4 or later". It can also "export a flattened version of your icon for marketing
   and communication needs" (developer.apple.com/icon-composer, read 2026-11-05; **verified**). So the final Mac
   icon needs a Mac (or a borrowed one) for one session; the SVG layers can be drawn on Windows.
3. Nothing else for macOS 26 and later. For older macOS, Xcode makes the fallback: "If your app supports previous
   releases ... Xcode automatically generates app icon images at build time for those releases from the Icon
   Composer file." (Same article; **verified**.) A plain asset catalog "For macOS ... you need to supply an asset for
   each size" (Configuring your app icon using an asset catalog; **verified**); the usual set is 16, 32, 128, 256 and
   512 pt at 1x and 2x (**unverified** for app icons; the HIG lists exactly these sizes for macOS document icons).
4. A Mac app built without Xcode (as a Rust app would be) compiles the `.icon` with Apple's `actool` 26 or later into
   `Assets.car`, names it in `CFBundleIconName`, and keeps an `.icns` in `CFBundleIconFile` as the fallback. Tauri's
   bundler does exactly this (commit `5a0ca7e`, "feat(bundler): support Liquid Glass icons", read 2026-11-05); mpv
   reportedly too (search result, not opened). Apple documents no command line for it: **unverified** beyond Tauri.
   `actool` comes with Xcode, so this step also runs on a Mac, for example a macOS build machine. How to build the
   Mac app is out of scope here.

## 2. Menu bar extra (the tray icon's twin)

### Apple's rules

From the HIG "The menu bar" page, section "Menu bar extras" (read 2026-11-05 through its JSON copy; last change
2025-06-09, so it is the macOS 26 and 27 text; **verified**):

- "Consider using a symbol to represent your menu bar extra. You can create an icon or you can choose one of the SF
  Symbols, using it as-is or customizing it to suit your needs. Both interface icons and symbols use black and clear
  colors to define their shapes; the system can apply other colors to the black areas in each image so it looks good
  on both dark and light menu bars, and when your menu bar extra is selected. The menu bar's height is 24 pt."
- "Display a menu", "not a popover", "when people click your menu bar extra. Unless the app functionality you want to
  expose is too complex for a menu, avoid presenting it in a popover." (Apple's dashes between the parts left out.)
- "Let people", "not your app", "decide whether to put your menu bar extra in the menu bar."
- "Avoid relying on the presence of menu bar extras. The system hides and shows menu bar extras regularly".
- "Consider exposing app-specific functionality in other ways, too. For example, you can provide a Dock menu".

The template image, in AppKit (`NSImage.isTemplate`, macOS 10.5 and later, read 2026-11-05; **verified**): "Images
you mark as template images should consist of only black and clear colors. You can use the alpha channel in the image
to adjust the opacity of black content, however."

Size:

- Apple gives the bar height (24 pt, above) and, in AppKit, "The status bar returned by the system has a thickness of
  22 pixels, which corresponds to the thickness of the menu bar" (`NSStatusBar.thickness`, read 2026-11-05;
  **verified**; the two numbers disagree, and the HIG one is newer).
- Apple's current pages give no size for the glyph itself. The best-known designer guide (Bjango, "Designing macOS
  menu bar extras", 2022-04-25) says: "the working area for menu bar extras is fixed at 22pt, and items can not be
  taller"; "A good size for a circular item to feel the same weight as system menu bar items is 16×16pt"; "There
  should typically not be any padding"; "Apple typically uses 35% opacity to indicate disabled elements"; assets can
  be "a single SVG, a single PDF, or a pair of PNGs" (read 2026-11-05; a third party, so **unverified** against
  Apple; it predates Tahoe). On notched MacBooks the bar is taller (37 pt, same source), but the glyph area is not.
- So: draw the glyph in a 22 pt tall box, about 16 pt for its main body, as a vector (PDF or SVG) or as 1x and 2x
  PNGs.

State:

- A template glyph cannot use colour, so state is shown by the picture: a different glyph per state, or opacity
  inside the glyph (alpha is allowed in a template, above). Bjango: "it's possible to use different levels of opacity
  to provide shading. This is often used to indicate state, like volume or Wi-Fi strength" (same article).
- AppKit has a ready "off" look: `NSStatusBarButton.appearsDisabled` "Determines whether the status bar icon has a
  disabled/off appearance while still being functional, such as allowing selection and actions" (macOS 10.10 and
  later, read 2026-11-05; **verified**). That fits our Off state: the struck-through glyph, dimmed by the system, and
  still clickable.
- Our three Windows states (holding: risen sun with rays; idle: empty half sun; off: struck through) carry over as
  three template images with no change in meaning.

The Tahoe menu bar:

- macOS 26 shows the wallpaper through the menu bar (clear by default, as widely reported; the default itself is
  **not verified** at Apple). Settings > Menu Bar has "Show menu bar background: Choose to show your
  wallpaper behind the menu bar, or to show the menu bar background", and an "Allow in the Menu Bar" list where "Some
  applications provide menu bar items for extra functionality" and each app has "Select to show in the menu bar"
  (Apple's Mac User Guide, "Change Menu Bar settings on Mac", macOS Tahoe 26 page, read 2026-11-05; **verified**).
- With a clear bar, the glyph sits straight on the wallpaper. A template glyph is recoloured by the system to light or
  dark to match; a colour glyph keeps its colours on any wallpaper and can vanish on a busy or same-colour one. Users
  report the system picks white or black text and icons by wallpaper brightness (MacRumors forum and others, 2025;
  **unverified**). We found no Apple text on how colour glyphs behave on the clear bar: **not verified**.
- macOS 27 "updated ... menu bar icons" (apple.com/macos, read 2026-11-05; **verified** as a sentence; what changed
  in them is **not verified**).

SF Symbols and their licence:

- The HIG allows an SF Symbol "as-is or customizing it" for the menu bar extra (above). But the SF Symbols page says:
  "Be sure to understand the terms and conditions for using SF Symbols, including the prohibition against using
  symbols", "or images that are confusingly similar", "in app icons, logos, or any other trademarked use." (HIG SF
  Symbols, last change 2025-07-28, read 2026-11-05; **verified**.) Copies of the licence on forums also limit SF
  Symbols to software for Apple platforms (**unverified**; the licence text ships inside the SF Symbols app, which
  needs a Mac).
- **This matters for our mark.** SF Symbols already has a sunrise over a horizon: `sun.horizon` and
  `sun.horizon.fill` (a half sun over a line with short rays; named `sun.and.horizon` in SF Symbols 3, renamed in SF
  Symbols 5), plus `sunrise` and `sunset` (the same with an up or down arrow, since SF Symbols 1). Names and first
  versions from the SFSafeSymbols project's generated list; the pictures seen in that project's 53 px previews
  (`github.com/SFSafeSymbols/SFSafeSymbols`, read 2026-11-05; **verified** there, not in Apple's own app). Our holding
  glyph (half sun, three rays, horizon line) is very close to `sun.horizon.fill`.
- What follows: as a menu bar glyph, a sun-over-horizon is fine (it even looks native). As the app icon and logo, a
  plain sun-over-horizon risks being "confusingly similar" to an SF Symbol, and it reads as Apple's weather sign
  rather than as our brand. We draw our own art, so this is not a copy; but the brand mark should carry something
  of its own that `sun.horizon` does not have. That is a design task for `p6-brand-directions`, listed in section 5.
- SF Symbols do not ship on Windows (their licence is reported to be for Apple platforms, above, **unverified**), and
  Haik's rule is our own drawing code and free fonts only (2026-11-05), so one glyph set drawn by our art code for
  both platforms is the simple path.

## 3. The menu or panel

### What Apple says and what the system does

- The HIG rule, quoted in section 2: display a menu, not a popover, "Unless the app functionality you want to expose
  is too complex for a menu".
- Native menus get Liquid Glass with no work: "Menus have a refreshed look across platforms. They adopt Liquid Glass,
  and menu items for common actions use icons" (Adopting Liquid Glass, Menus and toolbars, read 2026-11-05;
  **verified**; macOS 26).
- macOS 27 took most menu icons away again. "In macOS 27.0, menu bar and context menus present a reduced set of menu
  item images, similar to the behavior prior to macOS 26.0. By default, `NSMenu` hides all menu item symbol images",
  and for apps linked on the 27 SDK "both symbol and non-symbol menu item images are now automatically hidden". The
  system still shows its own images for "Settings, Share, and Print", and an app can force an image with the new
  `preferredImageVisibility` (macOS 27.0 and later). (macOS 27 Golden Gate Release Notes, AppKit, read 2026-11-05;
  **verified**.) We found no Apple text that says whether a status item's menu counts as a "menu bar" menu here:
  **not verified**; the safe reading is yes.
- The HIG Menus page (changed 2026-06-08, "Updated guidance for menu item icons"): "Use menu item icons sparingly and
  with purpose", "Don't display an icon if you can't find one that clearly represents the menu item", and "provide
  icons for all menu items in a group, or none of them." (read 2026-11-05; **verified**.)
- A native menu can carry our status line: `NSMenuItem.sectionHeader(title:)` "Returns a menu item representing a
  section header for a logical grouping of menu commands" (macOS 14.0 and later), and `NSMenuItem.subtitle` "is
  displayed below the standard title" (macOS 14.4 and later) (AppKit docs, read 2026-11-05; **verified**).
- The panel route exists too. SwiftUI `MenuBarExtra` (macOS 13 and later) has a `.menu` style ("renders its contents
  as a menu that pulls down from the icon") and a `.window` style ("renders its contents in a popover-like window";
  "For more complex or data rich menu bar extras") (SwiftUI docs, read 2026-11-05; **verified**). Glass in a custom
  panel comes from `glassEffect(_:in:)` (SwiftUI) or `NSGlassEffectView` (AppKit), both macOS 26.0 and later
  (**verified**). Apple's caution: "Use Liquid Glass effects sparingly", and the regular variant for "components
  [that] have a significant amount of text, such as alerts, sidebars, or popovers" (HIG Materials, last change
  2025-09-09; **verified**).

### What good menu bar apps do

Survey by a helper agent on 2026-11-05 (sources: each app's code for the open-source ones, release notes, manuals and
screenshots; details in section 6 for the keep-awake apps):

| App | A click on the menu bar icon opens | Glass panel? | Source |
|---|---|---|---|
| 1Password 8 | Native drop-down menu; Quick Access is its own floating panel | No evidence | support.1password.com guide (screenshot not seen) |
| Raycast | Small menu: Settings, Check for Updates, Quit | The launcher window is glass in v2, not a menu bar panel | manual.raycast.com |
| Bartender 6 | Hidden items, in the menu bar or its own "Bartender Bar" | "a whole new glassy look, designed to fit right in on Tahoe" | macbartender.com release notes |
| Ice | Left click shows hidden items (or its own Ice Bar panel); right click a native menu | A 2025 test build says "Adopted liquid glass design" | `jordanbaird/Ice` code and notes |
| Stats | Its own popup panel per item, with charts | No, the older blur material | `exelban/stats` code, README image |
| iStat Menus 7 | Its own drop-down panels with graphs | Yes: "iStat Menus 7.5 brings beautiful Liquid Glass menus to macOS 27" | bjango.com |

Keep-awake apps (section 6): **native menu in 12** (Amphetamine, Lungo, KeepingYouAwake, Caffeine, Vigil, Insomnia,
LidAwake, Keep Awake: AI Agents, Electronic Clam, Shut the Lid, Jolt of Caffeine, Staylit), **custom panel in 6**
(Theine, Caffeinated, Owly, StayAwake, Sleepless, Adrafinil; Adrafinil's code uses `glassEffect`). Jolt, Staylit,
Caffeinated and Owly were judged from their first App Store screenshot. Insomnia puts a greyed status line and one
row per Claude Code session in a native menu, and Keep Awake: AI Agents a greyed status line; both are close to our
Windows menu.

The pattern: apps that show data (graphs, lists, live numbers) use a panel; apps that offer a few commands and a
state use the native menu. The best known keep-awake apps (Amphetamine, Lungo, KeepingYouAwake, Caffeine) all use
the native menu.

### Pick for Wakeshot on the Mac

**A native `NSMenu` on the status item, with the same items, order and words as the Windows menu.** Not a custom
glass panel.

- Apple's rule says a menu unless the content is "too complex for a menu"; ours is one status line, a check item, a
  submenu and three commands.
- It is the Mac twin of our Windows choice: Windows' own menu loop, made to look current (`tray-menu-look.md`). On
  the Mac the system does all of it: Liquid Glass on 26 and 27, keyboard, VoiceOver, light and dark, the clear menu
  bar, and whatever macOS 28 changes. That is "glassy and Tahoe compliant" with no glass code of ours.
- The status line ("Holding awake: Claude Code (2 sessions)") fits a section header (`sectionHeader(title:)`, macOS
  14) or a greyed first item, as Insomnia and Keep Awake: AI Agents do; one row per session can go under it.
- No item icons on the Mac (macOS 27 hides them; HIG all or none). The Windows menu keeps its icons.
- A click opens the menu, as the HIG asks. KeepingYouAwake, Caffeine and Electronic Clam toggle on left click and
  hide the menu on right click; that is a hidden gesture, and our menu's first job is to show what holds the PC
  awake.
- Small Mac wording differences belong to the Mac copy work, not the brand: the Mac says "Quit Wakeshot" with
  Command-Q where Windows says "Quit" (HIG app menu table: "Quit YourAppName").

**Change it when** Wakeshot needs to show more than a menu can hold: live timers per session that tick while open,
graphs, sliders or text input. Then use a `MenuBarExtra` `.window` panel with the regular glass (as Adrafinil and
iStat Menus 7.5 do), and keep the brand out of it the same way.

## 4. One brand across Windows and macOS

### What each platform asks

- Apple asks for the same picture everywhere, not the same finish: "Provide a visually consistent icon design across
  all the platforms your app supports" (HIG App icons; the HIG means Apple's platforms, but the reason, "prevents
  people from mistaking your app for multiple apps", holds for Windows too). On the Mac the finish is fixed by the
  system: the rounded rectangle, the glass, the six appearances. Apple warns against faking it: "custom effects are
  static, whereas the system supplies dynamic ones" (HIG App icons, Visual effects; **verified**).
- Microsoft asks for flat layers and soft gradients: "Icons are composed of flat objects sitting on top of the layers
  below it", "Gradients should be subtle for the most part", "The default angle for gradients is 120 degrees", and
  "Avoid very tight transitions that would feel like reflections or dimension" (Design guidelines for Windows app
  icons, learn.microsoft.com, page dated 2021-10-29, updated 2026-07-14, read 2026-11-05; **verified**). Windows icons
  are free-form shapes or plates; Windows applies no mask.
- Microsoft's own 2025 refresh of the Microsoft 365 icons is described as "softer curves, richer gradients" and "more
  dimensional", some say translucent (Neowin, Winaero and others, late 2025 to 2026-01; **unverified**, the Windows
  card covers it). That is still Fluent, not Apple's glass with light on the edges.
- So a glass look on Windows would be a baked, static copy of an effect Windows does not use, and Microsoft's page
  says to avoid "reflections or dimension". The "2026 trend" is different on each platform: glass on the Mac, soft
  Fluent gradients on Windows.

### What cross-platform apps do

Survey by a helper agent on 2026-11-05. Each icon was downloaded and looked at; the Mac icons mostly come from the
vendor's own Mac zip (only the `.icns`, `Assets.car` and `Info.plist` read by HTTP range requests, nothing run),
the rest from the Mac App Store artwork, the Microsoft Store catalog, or the app's repo. I checked two of them myself
(Signal both, Figma Windows): they match the descriptions.

| App | Mac icon | Where the Mac icon was seen | Windows icon | Where the Windows icon was seen |
|---|---|---|---|---|
| Slack | White rounded square, flat four-colour hash, Big Sur style, no Liquid Glass file | Mac App Store id803453959 artwork (4.53.280, 2026-11-04) | Same hash on a white rounded plate | Microsoft Store 9WZDNCRDK3WP images |
| Telegram | Light plate, blue disc with a glowing rim, glass-like (not checked that it is an `.icon`) | Mac App Store id747648890 artwork (13.1.0, 2026-10-21) | Free-form flat blue disc, white plane, no plate | `telegramdesktop/tdesktop` `Resources/art/icon256.png`; Store 9NZTWSQNTD0S |
| VS Code | White rounded square, blue ribbon, Big Sur style, no `.icon` | `Code.icns` in `VSCode-darwin-universal.zip` (stable) | Free-form blue ribbon | `code.ico` in `VSCode-win32-x64-1.145.0.zip` |
| Signal | Liquid Glass `.icon` in `Assets.car`; fallback: blue rounded square, glassy bubble | `signal-desktop-mac-arm64-8.33.0.zip` | Free-form flat blue disc, white bubble | `signalapp/Signal-Desktop` `build/icons/win/icon.ico`; Store XP89119P9F2PCQ |
| Discord | Liquid Glass `.icon`; fallback: blurple rounded square, glass look | `Discord.zip` 0.0.421 for macOS | Free-form flat blurple disc | `app.ico` in `Discord-0.0.311-full.nupkg` (the newest the feed lists); Store XPDC2RH70K22MN |
| Figma | Liquid Glass `.icon`, five coloured layers on a light glass plate | `Figma.zip` (mac-arm) | Same five shapes on a dark flat rounded plate | Icon in `Figma-126.10.13-full.nupkg` |
| Notion | Liquid Glass `.icon` (not rendered); fallback for older macOS: the free-form cube | `Notion-arm64-7.42.1.zip` | Free-form cube | Microsoft Store XPDBVSS44R0L9H |
| Firefox | Liquid Glass `.icon` in the repo; fallback: fox and globe on a dark purple plate | `mozilla-firefox/firefox` `browser/branding/official/firefox.icns` and `Assets.car` | Free-form fox and globe | Same repo, `firefox.ico` |
| 1Password | Glossy grey "safe door" plate with the dial | Mac App Store artwork and `1Password.zip` installer icon | Free-form glossy dial | Microsoft Store 9NZWS5X28P0J tile |

Exact URLs are in the helper's notes; all read 2026-11-05. Not covered, because the icon needs a full installer
download: Spotify, ChatGPT, Claude, Zoom, Teams, Docker Desktop, Raycast, WhatsApp, Obsidian, Arc, Chrome.

The pattern, 9 of 9:

- **Same picture and colours, platform shape and finish.** No app ships one file for both, and none uses different
  pictures. Figma alone changes the plate colour (light on the Mac, dark on Windows).
- **On the Mac, a rounded square every time;** 5 of 9 already ship a Liquid Glass `.icon` (Signal, Discord, Figma,
  Notion, Firefox) and keep a flat `.icns` for older macOS, made from the same flat shapes split into layers. Slack
  and VS Code have not moved to glass yet.
- **On Windows, a free-form mark in 7 of 9** (no plate); Slack and Figma keep a plate. **No Windows icon copies the
  glass look.**

### Pick

**Same mark on both, each in its own platform's finish.** One drawing (today the sunrise; whatever
`p6-brand-directions` settles on after section 6) from one source in our art code; on the Mac a layered Liquid Glass
`.icon` in Apple's rounded square, on Windows the flat Fluent plate with its 120-degree gradient, as now. Do not copy
the glass onto Windows.

Why:

- It is what all nine apps above do, including the ones that already adopted Liquid Glass.
- Apple's consistency rule is about the picture; the finish is the system's job on the Mac, and a baked glass look on
  Windows is a static fake that Microsoft's guide warns against ("reflections or dimension").
- Our Windows icon is mostly seen at 16 to 48 px in the tray, taskbar and Start; glass highlights blur at that size.
- Lean: one set of SVG layers feeds both. The Mac adds only an Icon Composer session.

Keep the plate on Windows. Most apps go free-form there, but ours has a reason: gold on light Start fails contrast
without the violet behind it (`docs/brand.md`, Microsoft: "Yellow will never pass an accessible contrast ratio on
light theme"). Change it when the Windows card finds a free-form sunrise that passes 3:1 on light and dark.

Change the whole pick when Microsoft publishes icon guidance with glass or depth effects, or when Apple stops masking
Mac icons to the rounded rectangle (Rogue Amoeba and others are asking for that; the current HIG still masks).

## 5. What this asks of the mark now

Rules for `p6-brand-directions`. Each says which Apple rule it comes from (sections 1 to 3). Numbers marked "est." are
our estimates, not Apple's.

The app icon:

1. **Square, full bleed, no drawn corners on the Mac.** The background is a square, opaque layer; macOS cuts the
   rounded corners. The Windows plate may keep its own corners. (Mask rule, section 1.)
2. **Centred, with air at the edges.** Nothing that matters near the corners or edges, so the mask never cuts it.
   Our 72% mark is fine; do not grow it past about 80% (est.). (HIG "Keep primary content centered".)
3. **Flat parts in at most four groups.** A background fill (the violet gradient), and at most four foreground
   groups, for example the sun disc, the rays, the horizon. Flat fills, clear hard edges, no own shadows, glows,
   bevels or blurs: the system adds those. (Layer rules, section 1.)
4. **Bold, no hairlines.** The Mac scales one vector file down to 16 px with no per-size hinting (HIG: "The system
   automatically scales your icon to produce smaller variants"), unlike our Windows ICO, which our art code draws per
   size. Our rays and horizon are 1 unit of a 16-unit grid over the mark (our reading of `art.rs`), about 46 px on a
   1024 canvas, so about 0.7 px at 16 px. Make them about twice that, or fewer and thicker (est.), and round their
   ends. (HIG thin lines and sharp corners; WWDC25 "use bolder line weights".)
5. **Reads with no colour.** In clear and tinted modes (four of six Mac looks) the brand colours are gone. The mark
   must be clear from its outline and two or three grey levels. Pick one element to be white in mono (for the
   sunrise: the sun disc). (Mono rule, section 1.)
6. **The same picture in every appearance.** No swapping parts between light, dark and mono. The dark variant keeps a
   coloured background (violet today), a darker shade of it at most; the main colour of the mark stays. (HIG
   appearances; WWDC25 "leaning more into colored backgrounds".)
7. **Not a stock weather symbol, not Shut the Lid.** A bare half sun with short rays over a line is SF Symbols'
   `sun.horizon.fill`, and Apple's terms forbid symbols "or images that are confusingly similar" in app icons and
   logos. And since 2026-11-04 a Mac keep-awake app, Shut the Lid, shows a gold half sun over a white line on a
   blue-indigo plate (section 6). The mark needs a feature of its own that neither has (the shape or number of the
   rays, a cut in the sun, the line, a second element, the plate colour), or a different picture. (Sections 2 and 6.)
8. **Vector source in our own code.** The art code should be able to write each layer as an SVG on a 1024 canvas,
   numbered back to front, so one source feeds both the Windows ICO and the Mac `.icon`. The `.icon` itself is made
   once in Icon Composer on a Mac (macOS 26.4 or later). (Icon Composer article.)

The menu bar glyph:

9. **One colour, alpha allowed.** A template image: black on clear; opacity inside the glyph is fine, colour is not.
   The system tints it for a light or dark, clear or solid menu bar. (HIG menu bar extras; `NSImage.isTemplate`.)
10. **16 pt body in a 22 pt tall box,** no padding, as vector (PDF or SVG) or 1x and 2x PNG. (Bjango, unverified
    against Apple.) The same 16-unit grid the Windows glyph uses at 16 px works at 16 pt, so the shapes can be shared;
    the Mac draws it at 2x on Retina, so 1 unit is 2 px there.
11. **Three states by shape, not colour,** as on Windows: holding (risen sun, rays), idle (empty half sun), off (idle
    struck through, optionally dimmed with `appearsDisabled`). (Section 2.)
12. **Weight like Apple's own extras.** Next to the Wi-Fi, battery and Control Center symbols the strokes should look
    the same weight; a 1-unit stroke on a 16-unit grid (1 pt at 16 pt) is near SF Symbols' regular weight at that
    size (est., not measured on a Mac).
13. **A detail of its own at 16 pt.** SF Symbols' `sun.horizon`, Shut the Lid's glyph and StayAwake's `sun.max` sit
    in the same menu bars. The horizon line and whatever feature rule 7 gives the app icon must survive in the glyph,
    so the glyph reads as ours and not as Apple's weather sign. (Section 6.)

The menu:

14. **No brand colour inside the menu.** The native menu draws itself in Liquid Glass with the system accent; the
    brand lives in the app icon only. That matches the Windows rule "No brand colour in the tray" (`docs/brand.md`).
15. **No item icons on the Mac.** macOS 27 hides them by default, and the HIG asks for all or none per group. The
    Windows menu keeps its icons; the words and the order stay the same on both. (Section 3.)

## 6. Keep-awake apps on the Mac

Survey by a helper agent on 2026-11-05: icons and screenshots downloaded from each app's Mac App Store entry
(`apps.apple.com` and the iTunes lookup API), its website or its GitHub repo, and looked at; glyphs read from repo
assets or code where open source. "Seen" means someone looked at the picture; the rest is marked.

| App | App icon | Menu bar glyph and state | Menu or panel | Last update, price |
|---|---|---|---|---|
| Amphetamine | Pink to orange gradient plate, thick black frame, gold pill (seen) | Round pill, one colour (seen); user can pick other glyphs | Native menu; one popover for an app picker (seen) | 5.3.2, 2023-11-10; free |
| Lungo (Sindre Sorhus) | White plate, latte cup from above that also reads as a speech bubble (seen) | Cup on saucer, template; same when on and off, option "Dim icon while deactivated" (seen) | Native menu (seen, old screenshots) | 2.8.3, 2026-09-15; $4 |
| KeepingYouAwake | Dark cup on a light grey gradient, an Icon Composer `.icon` with glass on (repo file, seen as SVG render) | Cup, template: filled when on, outline when off (repo assets, seen) | Native menu; left click toggles (code) | 1.6.8, 2025-09-12; free, MIT |
| Caffeine (current, IntelliScape) | Bright blue plate, frosted glass cup of coffee, Icon Composer (seen) | Cup, template: full with steam when on, empty when off (repo assets, seen) | Native menu; left click toggles (code) | 1.6.4, 2026-04-20; free, MIT |
| Caffeine (old, Lighthead) | Screen with the Leopard violet aurora light burst and a cup (seen) | Cup (not seen) | Not checked | Gone from the App Store |
| Theine | Light grey plate, wide black cup (seen) | Wide cup; optional accent colour while on, dimmed when off (seen, release notes) | Custom translucent panel with Control Center style round toggles (seen) | 4.3, 2026-10-01; $8.99 |
| Vigil (botus-ai), for AI agents | **Violet to navy** plate, white eye with a blue iris (seen) | SF Symbols: `eye` / `eye.fill`, `bolt.fill`, `moon.zzz` (code) | Native menu (code) | 1.1.3, 2026-06-11; free, MIT |
| StayAwake (TY-teo), for AI agents, lid closed | Navy plate, closed laptop with a gold glowing seam (seen) | SF Symbols: **`sun.max`** / `sun.max.fill` when on, `laptopcomputer`, `moon.zzz` when off (code) | Custom panel, `MenuBarExtra` `.window` style (code, screenshot seen) | 1.0.0, 2026-05-31; free, MIT |
| Sleepless (Aboudjem), lid closed | **Violet to magenta** plate, white cup with steam (seen) | SF Symbols: `cup.and.saucer`, `cup.and.heat.waves.fill` (code) | Custom dark popover, `NSPopover` (code, screenshot seen) | 1.2.7, 2026-06-03; free, MIT |
| Adrafinil (kageroumado), for AI agents, the most starred (525) | Near-black plate, **glowing gold-orange** spiral round a black pupil, an Icon Composer `.icon` (seen) | The spiral eye, template: shut when idle, open when active, small badge for alerts (code) | Custom glass panel: `MenuBarExtra` `.window` with `.glassEffect(.regular)`, session cards, two pill buttons (code, screenshot seen) | 1.14.1, 2026-11-03; free, MIT |
| Insomnia (phin), for AI agents | Not seen (only an `.icns` in the repo) | Text-like "zzZ" (seen) | Native menu with one row per Claude Code session (code, screenshot seen) | 0.3.0, 2026-05-15; free, MIT |
| LidAwake (TurboKach), lid closed | Not seen | SF Symbols: `cup.and.saucer.fill` when awake, `moon.zzz` when not (code) | Native menu (code) | 1.2.0, 2026-09-14; free |
| **Shut the Lid (Yile Ren), lid closed, new** | **Blue to indigo rounded square (glass look), gold half sun over a white laptop-lid line, two pale arcs round the sun** (seen, by the helper and by me) | **Outline half sun with five short rays over a horizon line**, one colour (seen, zoomed from its screenshot; looks like SF Symbols' `sun.horizon`, exact symbol not verified) | Native menu, plus a one-switch settings window (screenshot seen) | 1.0, released 2026-11-04; free (iTunes lookup, read by me 2026-11-05) |
| Keep Awake: AI Agents (Serdar Oztetik) | Navy-violet plate, cream cartoon face with big eyes (seen) | A face: wide eyes and open mouth when awake, closed eyes when sleep is allowed (seen) | Native menu with a greyed status line (seen) | 1.0.1, 2026-09-23; free |
| Electronic Clam (jadhvank), AI agents, lid closed | Dark slate plate, white scallop shell with a blue bolt (seen) | Solid shell with a bolt cut out when on, outline ribbed shell when off (seen); a half fan with ribs, so at 16 px it can look like a half sun with rays | Native menu (code) | 0.7.0, 2026-11-04; free, MIT |
| Stay Awake Pro (Filip Alexandrow) | Blue plate, a screen with an eye whose iris is a small yellow sun with rays (seen) | Not seen | Not seen | 5.0, 2024-04-02; free |

Forty-nine more keep-awake apps in the Mac App Store (one may be an agent monitor instead), icons only, seen on three
contact sheets (iTunes lookup, 2026-11-05), among them Caffeinated, Owly, Jolt of Caffeine, Caffeine Utility,
NeverNap, Mug, Staylit, Insomniacs, Coca, Sleep Control Centre, Caffeinate, CaffeinateBar, Insomnio, InsomniaZ, NoDoze, NoSleep, AwakeBar (three apps
share the name), Awake Bolt, Vigil - Stay Awake (a lighthouse), Vigilante, WakeLease and matcha. Their pictures:
cups by far the most, then eyes, lightning bolts, moons and stars, Z letters, a pill, an owl, a leaf, a lighthouse,
mouse pointers, dots. None of them shows a sun, a sunrise or a horizon line. Violet plates: InsomniaZ (violet square,
pale circle), Insomnio (orange to violet), Stay Awake - No Sleep (violet to red), Auto Mouse Mover (pale lilac).

What this says for our mark:

- **A Mac app already wears our picture.** Shut the Lid went into the Mac App Store on 2026-11-04, the same day Haik
  picked our sunrise: a gold half sun over a white line, on a blue-indigo rounded square, for a lid-closed keep-awake
  tool. Its menu bar glyph is a sun over a horizon with rays, almost our holding glyph. Next to it on a Mac, a
  Wakeshot with today's icon would look like a copy or a sibling. It is Mac-only today and new (version 1.0, one
  day old), so it may vanish or grow. Either way, **the mark cannot stay "gold half sun over a white line on a
  blue-violet plate"** if it must work on the Mac. Together with `sun.horizon.fill` (section 2) this is the strongest
  reason for `p6-brand-directions` to give the sunrise a feature of its own, or to rethink the picture.
- **The cup is the category sign on the Mac too.** Most of the sixty-odd icons are cups. Haik's 2026-11-04 choice of
  the Sunrise over the Cup holds; it just is not unique any more.
- **Suns elsewhere:** StayAwake shows SF Symbols' full sun `sun.max` in the menu bar while it holds; Stay Awake Pro
  has a small sun as an eye's iris; Electronic Clam's ribbed shell glyph looks like a half sun with rays at 16 px.
  So at menu bar size, a half sun with rays is crowded: the glyph needs its own detail too, not just the app icon.
- **Violet is no longer ours alone on the Mac.** Apps aimed at the same users use violet or blue-violet plates:
  Vigil (violet to navy, an eye), Sleepless (violet to magenta, a cup), Keep Awake: AI Agents (navy-violet, a face),
  Shut the Lid (blue to indigo, a sunrise), InsomniaZ and others above. `docs/brand.md` says "no keep-awake rival
  uses violet"; that was checked on Windows rivals only. The colour alone does not set us apart on the Mac. This goes
  to `p6-brand-research-identity` and `p6-brand-directions` (listed under "Found outside scope" in the card).
- **Glyph state is shown by shape everywhere:** filled versus outline (KeepingYouAwake, Caffeine, Vigil), with steam
  versus without (Caffeine, Sleepless), or dimmed (Lungo, Theine). None uses colour by default; Theine offers an
  accent colour as an option. Our three shapes (risen, empty, struck through) fit this well.
- **The eye is taken** (Vigil's icon and glyph, Adrafinil's spiral eye). Our Eye concept was already dropped
  (`docs/brand.md`).
- **Gold glow on dark is used once:** Adrafinil, the most starred agent-aware app we found, has a glowing gold-orange
  spiral on a near-black plate. A gold sun on violet is still far from it, but a dark-mode Wakeshot icon (gold on a
  very dark violet) would move closer. Keep the dark variant clearly violet (rule 6 in section 5).

## iOS

Haik confirmed (2026-11-05, through the orchestrator) that "IOS" meant the Mac menu bar app. We found no reason a
keep-awake tool needs an iPhone or iPad app: an iPhone app cannot hold a Mac or a PC awake. Skipped.

## Sources

All read 2026-11-05. Apple's design and developer pages were read through their JSON copies at
`developer.apple.com/tutorials/data/<path>.json`, which hold the same text. They are Apple's current text, so every
Apple quote in this file applies to macOS 26 Tahoe and macOS 27 Golden Gate unless its line names other versions
(API lines give their first macOS version; the release notes are for 27; the Mac User Guide page is for 26).

Apple, design (HIG):

- App icons: `https://developer.apple.com/design/human-interface-guidelines/app-icons` (change log to 2026-10-28)
- The menu bar: `https://developer.apple.com/design/human-interface-guidelines/the-menu-bar` (2025-06-09)
- Menus: `https://developer.apple.com/design/human-interface-guidelines/menus` (2026-06-08)
- Materials: `https://developer.apple.com/design/human-interface-guidelines/materials` (2025-09-09)
- SF Symbols: `https://developer.apple.com/design/human-interface-guidelines/sf-symbols` (2025-07-28)
- Icons: `https://developer.apple.com/design/human-interface-guidelines/icons` (2025-06-09)

Apple, developer:

- Creating your app icon using Icon Composer: `https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer`
- Configuring your app icon using an asset catalog: `https://developer.apple.com/documentation/xcode/configuring-your-app-icon`
- Adopting Liquid Glass: `https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass`
- macOS 27 Golden Gate Release Notes: `https://developer.apple.com/documentation/macos-release-notes/macos-27-release-notes`
- AppKit: `NSImage.isTemplate`, `NSStatusBar`, `NSStatusBar.thickness`, `NSStatusItem`,
  `NSStatusBarButton.appearsDisabled`, `NSMenuItem.sectionHeader(title:)`, `NSMenuItem.subtitle`, `NSMenuItem.preferredImageVisibility`,
  `NSGlassEffectView`, `NSPopover` (under `https://developer.apple.com/documentation/appkit/`)
- SwiftUI: `MenuBarExtra`, `MenuBarExtraStyle.menu`, `MenuBarExtraStyle.window`, `glassEffect(_:in:)`, Applying
  Liquid Glass to custom views (under `https://developer.apple.com/documentation/swiftui/`)
- Icon Composer: `https://developer.apple.com/icon-composer/`; Apple Design Resources:
  `https://developer.apple.com/design/resources/`; SF Symbols: `https://developer.apple.com/sf-symbols/`
- WWDC25 session 220 "Say hello to the new look of app icons" and 361 "Create icons with Icon Composer"
  (`https://developer.apple.com/videos/play/wwdc2025/220/`, `.../361/`); WWDC26 8012 "Icon Composer for Beginners
  Group Lab" (`https://developer.apple.com/videos/play/wwdc2026/8012/`)
- Developer forums thread 832718, menu item image visibility (Apple engineer answers, June 2026)

Apple, other: `https://www.apple.com/macos/`; Mac User Guide "Change Menu Bar settings on Mac",
`https://support.apple.com/guide/mac-help/mchlad96d366/mac` (macOS Tahoe 26 page).

Microsoft: "Design guidelines for Windows app icons",
`https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-design`.

Third party (not Apple's word):

- Bjango, "Designing macOS menu bar extras", `https://bjango.com/articles/designingmenubarextras/` (2022-04-25)
- Rogue Amoeba, "Free The Icons", `https://weblog.rogueamoeba.com/2026/06/26/free-the-icons/`
- MacStories, "WWDC 2026: macOS 27 Icon Refinements"; 9to5Mac and Eclectic Light on the 27 release date
- Tauri commit `5a0ca7e`, `https://github.com/tauri-apps/tauri/commit/5a0ca7edbbc707199615a91845146e98b6f5e8ca`
- SFSafeSymbols, `https://github.com/SFSafeSymbols/SFSafeSymbols` (symbol names, versions, previews)
- Neowin and Winaero on the 2025 Microsoft 365 icons (unverified context)
- App surveys: each app's Mac App Store entry (iTunes lookup API `https://itunes.apple.com/lookup?id=<id>`), Microsoft
  Store catalog, vendor downloads and GitHub repos, as named in sections 3, 4 and 6. Shut the Lid:
  `https://apps.apple.com/us/app/shut-the-lid/id6821751029`.
