# Brand identity research

Card `p6-brand-research-identity`, 2026-11-05. Web research only, no drawings (`p6-brand-directions` draws). The
Windows and macOS platform rules are in the two other brand research cards; this report links to them where needed.

How to read it: **verified** means read at the source on the date given. **Unverified** means not checked at a source
(a press report, a copy, or memory). **Judgement** marks my opinion as the designer; it is not a fact.

The current brand (`docs/brand.md`): the Sunrise mark (a half sun with three rays over a horizon line), gold `#FFCC4D`
on a night violet `#7250F4` rounded square, a monochrome tray glyph in three states, the name in Segoe UI Variable
Semibold, no separate logo.

## Verdict in short

All judgement, built on the facts in the sections below.

- **The mark is a good tray glyph and a weak brand mark.** It is simple, works in one colour and reads "wake" at once,
  but it is the stock "sunrise" glyph: Google's Material Symbols `wb_twilight` is nearly the same drawing, free under
  Apache 2.0, and icon sets file it under Weather or camera white balance. It cannot be owned (sections 2 and 3).
- **The violet is the 2026 "AI purple" and is not ours.** `#7250F4` is dE00 1.5 from Qwen and 1.7 from Proton, close to
  Google's coding agent Jules, and 9 of 20 sampled coloured AI and developer brands sit in the same hue band (17 of 29
  with the violet neighbours I added on purpose). The gold sun on it also fails
  3:1 under protan colour-blind simulation (sections 4 and 8).
- **Keep:** the three tray states, the "awake while work runs, then lets go" story, monochrome in the tray, the rule
  against Claude's orange.
- **Change:** give the mark one feature only Wakeshot has (top ideas: the sun rising from a terminal prompt, a lit
  window at night, a pilot light; section 6), and move the plate to a mid-tone in an empty hue (olive first,
  magenta-rose second) with a white mark (section 4). A black plate fails Microsoft's 3:1 rule on dark Start.
- **Follow from 2026:** flat and bold, one accent, motion tied to the states, a clean two-layer split for Apple's Liquid
  Glass. Skip grain, fake glass and loud gradients (section 5).
- **Type:** Segoe UI and Segoe UI Variable are allowed in pictures and video rendered as bitmaps on Windows, but not on
  a Mac or a CI runner. Use Instrument Sans SemiBold (OFL) for the wordmark everywhere (section 7).

## 1. The parts of a brand for an app like this

| Part | What it is | Launch or later | Same picture as the mark? |
|---|---|---|---|
| Mark | The one shape that means Wakeshot | Launch | It is the mark |
| App icon, Windows | The mark on a plate, in the `.ico` (16 to 256 px) for Start, taskbar, Settings, the installer | Launch | Same mark; detail drops at 16 to 24 px |
| App icon, macOS | Apple's rounded square grid and layered (Liquid Glass) icon | With the Mac version (a real plan, Haik via the orchestrator, 2026-11-05); the mark must be chosen so it works there now | Same mark, Apple's plate shape |
| Tray glyph | Monochrome, 16 to 32 px, one per state (holding, idle, off) | Launch (built) | Same silhouette, no colour, no plate |
| Mac menu bar glyph | Monochrome template image, one per state | With the Mac version; the mark must work as one now | Same silhouette as the tray glyph |
| Wordmark | The name set in one font and weight | Launch | Not a picture; one font everywhere |
| Lockup | Mark plus wordmark in a fixed layout (horizontal) | Launch, for the README header and the social card | Fixed spacing, one or two versions only |
| Favicon | Browser tab icon for the site | Launch | Same mark; may drop the plate on a transparent tab |
| Social preview | The card GitHub, X, Slack and others show for a link | Launch | Lockup plus one line of what it does |
| Colours | Brand colour, accent, text and page colours | Launch | One set across all parts |
| Type | Wordmark font; page font | Launch | One wordmark font |
| Motion | The mark's state change as an animation (sun rises, sets) | Can wait until the video (`p6-media`) | Same mark, animated |
| Voice | How the app and docs speak | Launch (already set by the notices) | Not a picture |

Facts behind the sizes:

- **GitHub social preview**: "at least 640 by 320 pixels (1280 by 640 pixels for best display)", "PNG, JPG, or GIF
  file under 1 MB", PNG with transparency is supported, but "If you aren't sure, we recommend using an image with a
  solid background". Verified:
  https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview
  (read 2026-11-05).
- **Favicon set**: Evil Martians' guide (dated 2026-01-21 in its page data) asks for five icons and a manifest:
  `favicon.ico` 32x32, `icon.svg`, `apple-touch-icon.png` 180x180, and for a web app `icon-192.png`, `icon-512.png`
  and a maskable 512 with "bigger paddings". It also names an SVG dark version for modern browsers. Verified:
  https://evilmartians.com/chronicles/how-to-favicon-in-2021-six-files-that-fit-most-needs (read 2026-11-05). The
  site needs no manifest; three files are enough (judgement).
- Windows icon sizes and plate rules: see `p6-brand-research-windows`. macOS: see `p6-brand-research-macos`.

Judgement:

- **Needed at launch:** mark, Windows app icon, tray glyphs, wordmark, one horizontal lockup, favicon (three files),
  the GitHub social card at 1280x640 on a solid background, colours, wordmark font, and five lines of voice rules.
  Without the social card, a shared link shows GitHub's generated default card (not verified what it shows in
  2026), which wastes the launch posts.
- **Can wait:** the macOS icon and menu bar glyph as files (but not as a design test: every mark idea in question 6
  is judged on whether it works on Apple's plate and as a menu bar template image), Store art, motion (until the
  video), a stacked lockup, stickers.
- **Must be the same picture:** the mark in the app icon, favicon, lockup and social card. The tray glyph keeps the
  same silhouette but drops colour and the plate. A tray glyph that is a different picture from the app icon breaks
  the link the user makes between "the thing I installed" and "the thing in my tray".
- **May differ:** the plate shape per platform, the level of detail per size (fewer or thicker rays at 16 px), and
  the tray's lack of colour.
- **Voice rules** (judgement, taken from how the notices already speak): say what happened and why, in one short
  sentence; name the cause ("Claude Code is working"); never cute, never alarm words; no exclamation marks; the same
  words in tooltip, notice and docs.

## 2. What makes a mark good

### The checklist (facts: sources verified 2026-11-05)

| # | Test | What the sources say |
|---|---|---|
| 1 | Simple: few shapes, few corners | Paul Rand: "The role of the logo is to point, to designate, in as simple a manner as possible." Sagi Haviv: "For a logo to work, and work for a long time, it must be simple." Apple: "Embrace simplicity in your icon design." Microsoft: "Use as few shapes with as few corners as possible" |
| 2 | Distinctive: not a picture others already use | Haviv: "a tremendous challenge to walk the line between simplicity and distinctiveness." Jenni Romaniuk (Ehrenberg-Bass): an asset's strength is "both uniqueness and fame" |
| 3 | Appropriate: fits what the product does or feels like | Haviv: "appropriate for the client." Microsoft: "An icon should be a metaphor for its app", and "A good test for an effective icon is when users can tell what it represents without a label." Rand's limit: "A logo is rarely a description of a business" |
| 4 | Legible at 16 px | Microsoft: "a silhouette that's distinctive, yet legible at small sizes." Apple: fine details "may be hard to discern at smaller sizes" |
| 5 | Works in one colour | Microsoft: high contrast icons "are black and white and should be a direct representation of your app icon"; "avoid relying on color alone to convey meaning" |
| 6 | One focus, no letters | Microsoft: "use no more than two metaphors in a single icon", "Icons should not include typography". Apple: "Find a concept or element that captures the essence of your app" and make it the core idea |
| 7 | Ownable: nobody else can claim it | Romaniuk's uniqueness, above. A glyph from a free icon set is free for anyone to use (judgement from the licences in question 3) |
| 8 | Lasting: not tied to a style | Rand: "A design that is complex, like a fussy illustration or an arcane abstraction, harbors a self-destruct mechanism" |

Sources:

- Paul Rand, "Logos, Flags, and Escutcheons" (AIGA, 1991):
  https://www.paulrand.design/writing/articles/1991-logos-flags-and-escutcheons.html (read 2026-11-05).
- Sagi Haviv interview, Logo Design Love, October 2010: https://www.logodesignlove.com/sagi-haviv-interview (read
  2026-11-05). The often quoted "appropriate, distinctive, simple" as one phrase was not found at a primary source.
- Jenni Romaniuk, "Brands need distinctive assets" (Admap, via Ehrenberg-Bass; the page gives no exact date):
  http://marketingscience.info/brands-need-distinctive-assets (read 2026-11-05).
- Apple, Human Interface Guidelines, App icons (read from the page's JSON data 2026-11-05):
  https://developer.apple.com/design/human-interface-guidelines/app-icons
- Microsoft, "Design guidelines for Windows app icons" (page updated 2026-07-14):
  https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-design (read 2026-11-05).

### The Sunrise against the checklist (judgement)

| # | Test | Score | Why |
|---|---|---|---|
| 1 | Simple | Pass | Five parts (half disc, three rays, a line), all plain geometry |
| 2 | Distinctive | **Fail** | It is the stock "sunrise" glyph of almost every icon set (question 3) |
| 3 | Appropriate | Partial | It says "wake" and "morning". But Wakeshot's moment is the opposite: the PC stays up through the night or while you are away. A sunrise says "time to get up", which is an alarm clock's job. The rise and set story works only in motion, and the app icon never moves |
| 4 | Legible at 16 px | Partial | At 16 and 20 px the three rays break into loose pixels (`icon-sheet-*.png`; `docs/brand.md` says the same) |
| 5 | One colour | Pass | The tray glyph is the mark in one colour and reads well |
| 6 | One focus | Pass | One metaphor, no letters |
| 7 | Ownable | **Fail** | Google's Material Symbols `wb_twilight` is nearly the same drawing, free under Apache 2.0. Anyone may ship it |
| 8 | Lasting | Pass | Flat, no trend effects |

Plain verdict: **a good tray glyph, a weak brand mark.** It passes the craft tests and fails the two that make a mark
a brand: it is not distinctive and cannot be owned. In the tray, a stock glyph is a strength: people read it at once,
like Windows' own glyphs. As the app icon and the face of the project, it is a free icon-set glyph on a plate in the
most crowded colour of 2026 (question 4). The fix is not to lose the sunrise idea but to give the mark one feature
that only Wakeshot has (question 6).

## 3. Is the Sunrise distinctive?

### What exists (facts)

The sunrise glyphs below were downloaded from each set's GitHub repository on 2026-11-05 and rendered side by side
with `site/wakeshot.svg` in headless Edge (script and picture kept in the card's scratch folder, not in the repo).

| Where | Name | Looks like ours? | Category or tags | Licence |
|---|---|---|---|---|
| Google Material Symbols | `wb_twilight` | **Nearly identical**: half sun, three rays, one horizon line | Category "Images" (camera white balance presets); tags include "dawn", "dusk", "sunset", "twilight" | Apache 2.0 |
| Phosphor Icons | `SunHorizon` | Same idea: half disc with rays over a line | | MIT |
| Microsoft Fluent UI System Icons | `Weather Sunny Low` | Same idea plus two short lines under the horizon | Weather (by its name) | MIT |
| Bootstrap Icons | `sunrise`, `sunrise-fill` | Same idea plus an up arrow | | Not re-read |
| Lucide | `sunrise` | Same idea plus an up arrow | | ISC |
| Tabler Icons | `sunrise` | Same idea plus an up arrow | Category "Weather"; tags "sun, morning, dawn, day, horizon, weather, sunny, daylight, sunrise" | Not re-read |
| Unicode emoji | U+1F305 "sunrise", U+1F304 "sunrise over mountains" | The same subject as a picture | | |
| The Noun Project | search "sunrise" | "Icons (4,515)" | | Not checked |
| Segoe Fluent Icons (the Windows system icon font) | none | The documented glyph list has no sunrise; its only sun-like glyphs are Brightness ones | | |
| Apple SF Symbols | `sunrise`, `sunrise.fill` | **Unverified**: needs the SF Symbols app on a Mac (left to `p6-brand-research-macos`) | | |

Sources, all read 2026-11-05: https://github.com/google/material-design-icons (codepoints and `LICENSE`) and
https://fonts.google.com/metadata/icons (categories and tags); https://github.com/phosphor-icons/core;
https://github.com/microsoft/fluentui-system-icons; https://github.com/twbs/icons;
https://github.com/lucide-icons/lucide; https://github.com/tabler/tabler-icons;
https://unicode.org/Public/emoji/latest/emoji-test.txt; https://thenounproject.com/browse/icons/term/sunrise/;
https://github.com/MicrosoftDocs/windows-dev-docs (`hub/apps/design/iconography/segoe-fluent-icons-font.md`).

Brands named after the same idea draw it differently (facts, from the Simple Icons data and logo files, read
2026-11-05):

- Sunrise, the Swiss telecom (`sunrise.ch`): a ring with its lower part filled; no rays.
- Laravel Horizon (a Laravel queue dashboard): a circle cut by a wave.
- Sunrise Calendar (bought by Microsoft in 2015, closed in 2016): its logo was not found in a short search.
- No well-known software product with the literal half sun and rays turned up in a short search. Stock logo sites sell
  it as a generic template (for example Logowik's "Half Circle Sun Logo Template", seen in search results only, not
  opened). That no product uses it is unverified.

### Judgement

- **Very generic.** Our mark is the default drawing of the word "sunrise". Material's `wb_twilight` is close enough
  that a reviewer could believe we took it.
- **Reads as weather first.** The sets that file the glyph put it under Weather (Tabler, Fluent) or camera white
  balance (Material). The second reading is "morning" or "sunrise alarm". "Keeps my PC awake" comes only with the
  name next to it.
- **Different from the rivals is not the same as distinctive.** The Sunrise was picked because PowerToys Awake is a
  cup (`docs/brand.md`). That still holds: no keep-awake rival uses a sunrise. But a mark also has to stand apart from
  every icon a user sees in a day, and weather widgets, alarm apps and icon fonts all use this picture.
- Brands that took "sunrise" or "horizon" as a name abstracted it into one closed shape (a ring with a filled lower
  part, a circle with a wave). An ownable version of our idea would go the same way.

## 4. Is the colour distinctive?

### The field (facts)

Hex values are from the Simple Icons data (https://github.com/simple-icons/simple-icons, `data/simple-icons.json`,
latest commit 2026-11-04, downloaded 2026-11-05). Simple Icons keeps one colour per brand and names a source for each,
given in the last column. For a multi-colour logo it is the colour Simple Icons picked.

| Product | Colour | Hex | Source named by Simple Icons |
|---|---|---|---|
| Claude | Terracotta orange | `#D97757` | https://claude.ai |
| Anthropic | Near black | `#191919` | https://www.anthropic.com |
| Google Gemini | Soft violet | `#8E75B2` | https://gemini.google.com |
| Google Jules (Google's coding agent) | Violet | `#715CD7` | https://jules.google |
| GitHub Copilot | Black | `#000000` | https://primer.style/foundations/icons/copilot-24 |
| Cursor | Black | `#000000` | https://cursor.com/brand |
| Windsurf | Near black | `#0B100F` | https://windsurf.com/brand |
| Warp | Sky blue | `#01A4FF` | https://warp.dev |
| Raycast | Coral red | `#FF6363` | https://www.raycast.com/press |
| Linear | Indigo | `#5E6AD2` | https://linear.app |
| Zed | Blue | `#084CCF` | Zed's `logo_96.svg` on GitHub |
| Ghostty | Blue | `#3551F3` | Ghostty's `ghostty-logo.svg` on GitHub |
| Vercel | Black | `#000000` | https://vercel.com/geist/brands |
| Ollama | Black | `#000000` | Ollama issue 2152 on GitHub |
| LM Studio | Black | `#000000` | https://files.lmstudio.ai/lmstudio_icon2.svg |
| OpenCode | Black | `#000000` | OpenCode's `mark.svg` on GitHub |
| Cline | Near black | `#18181B` | https://cline.bot |
| Mistral AI | Orange | `#FA520F` | https://chat.mistral.ai |
| Perplexity | Teal | `#1FB8CD` | https://www.perplexity.ai |
| DeepSeek | Blue | `#5786FE` | https://www.deepseek.com |
| Qwen | Violet | `#6950EF` | https://qwen.ai |
| Meta AI | Violet | `#9844FF` | Meta AI's logo SVG |
| Poe | Indigo | `#5D5CDE` | https://poe.com |
| Hugging Face | Yellow | `#FFD21E` | https://huggingface.co/brand |
| TRAE | Green | `#32F08C` | https://www.trae.ai |

Not in Simple Icons (its data has no Microsoft or OpenAI brands):

- **Microsoft Copilot**: a rainbow gradient icon from September 2023; in 2026 a "flatter and simpler version" (Office
  Watch, 2026-08-18, https://office-watch.com/2026/new-copilot-logo-2026/), built from two squares that move in three
  states, "thinking", "planning", "gathering information" (Design Compass, 2026-10-06,
  https://designcompass.org/en/2026/10/06/microsoft-copilot-brand-identity-collins/), and an app that is "a largely
  black and white, text-forward affair" (Engadget, 2026-05-28,
  https://engadget.com/2183246/microsoft-debuts-a-more-buttoned-up-look-for-copilot). All read 2026-11-05. The new
  icon's exact colours: unverified.
- **ChatGPT and OpenAI**: the Blossom mark in black or white; one design blog describes the 2025 palette as "greys and
  blues" (https://octet.design/journal/openai-new-logo-gets-bold-rebrand, search result only). Unverified:
  `openai.com/brand` refused the fetch (HTTP 403) on 2026-11-05.
- **Codex** (OpenAI's coding agent and app): no source for its icon colour found. Unverified.
- **Keep-awake rivals** (from their icon files, read 2026-11-04, `docs/brand.md`): PowerToys Awake is Windows blue,
  LidGuard navy with green and teal, Agents Don't Sleep navy with amber.

Violet outside AI, for the distance check (same Simple Icons data): Proton `#6D4AFF`, Stripe `#635BFF`, .NET
`#512BD4`, Obsidian `#7C3AED`, Twitch `#9146FF`, Vite `#9135FF`, Discord `#5865F2`, WezTerm `#4E49EE`, Astro
`#BC52EE`. Other developer brands in the hue map below (same data): Homebrew `#FBB040`, Supabase `#3FCF8E`, Netlify
`#00C7B7`, Tailwind CSS `#06B6D4`.

### How close our colours are (facts, computed)

CIEDE2000 colour difference (dE00), computed on 2026-11-05 with a script of mine (`colour.py` in the card's scratch
folder; the standard formula). A common rule of thumb, not checked at a standards source: below about 2, most people
must look closely to tell two colours apart.

| Ours | Nearest brand colour | dE00 |
|---|---|---|
| Night violet `#7250F4` | Qwen `#6950EF` | 1.5 |
| | Proton `#6D4AFF` | 1.7 |
| | Stripe `#635BFF` | 3.9 |
| | Google Jules `#715CD7` | 4.6 |
| | Obsidian `#7C3AED` | 4.9 |
| | Twitch `#9146FF` | 5.0 |
| Sun gold `#FFCC4D` | Hugging Face `#FFD21E` | 4.8 |
| | Homebrew `#FBB040` | 9.3 |

Hue map of the 29 coloured brands above (OKLCH hue, computed): 3 at red and orange (hue 20 to 60: Raycast, Mistral,
Claude), 2 at amber and yellow (60 to 100), **none at yellow-green (100 to 150)**, 3 at green and teal (150 to 200),
3 at cyan and blue (200 to 260), **17 at blue-violet and violet (260 to 310)**, 1 at magenta (310 to 360: Astro, at
313). Nine more are black or near black. The sample leans violet because I added the violet neighbours on purpose;
without them, 9 of the 20 coloured AI and developer brands still sit at 260 to 310.

### Is violet the "AI purple" cliché? (facts, then judgement)

- Cursor's head of design Ryo Lu, in a Y Combinator talk with partner Aaron Epstein, named "ubiquitous purple
  gradients" as part of the look AI tools produce, and called it "AI slop" (StartupHub summary, 2025-11-20,
  https://www.startuphub.ai/ai-news/ai-video/2025/design-in-the-ai-era-beyond-the-purple-gradient, read 2026-11-05;
  the talk itself not watched).
- "The AI Purple Problem" (Jainil Prajapati, DEV Community, 2025-10-08,
  https://dev.to/jaainil/ai-purple-problem-make-your-ui-unmistakable-3ono, read 2026-11-05) ties it to Tailwind's
  indigo defaults and to AI tools repeating the median look. It says the hue is fine; the sameness is the problem.
- These are opinion pieces; no study that counts how many AI brands use violet was found.
- Judgement: **yes, violet and violet gradients read as "AI product" in 2026**, and our exact violet is a near twin of
  Proton's and Qwen's and close to Google's own coding agent, Jules. Our plate is also a violet gradient, the exact
  thing the critics name. It does not look like Claude (good), but it looks like everyone else.

### What colour space is open (judgement)

The rule from `docs/brand.md` stays: nothing near Claude's orange (OKLCH hue about 20 to 60), nothing that could pass
for an Anthropic tool.

| Space | Who is there | Fit for "calm, trusted, native" | Risk |
|---|---|---|---|
| **Olive to green-gold (OKLCH hue about 105 to 130)** | Nobody in the sample | Reads as a status light that is on, which is the product: lit while work runs; calm when deep | Bright yellow-green fails contrast on light backgrounds and reads loud, so it must be a deep olive mid-tone with a white mark |
| **Magenta to rose (about 320 to 360)** | Only Astro in the sample, but GraphQL's magenta in the wider data (below) | Warm and new among AI tools | Reads playful; sits on GraphQL's colour; deeper raspberry fails on dark Start |
| **A dark night plate with one light accent** | Black is the default of Cursor, Vercel, Ollama, GitHub Copilot, and now the Copilot app | The calmest look; it follows where the big brands moved in 2026 | Fails Microsoft's 3:1 rule on dark Start (below); black alone is not distinctive |
| Violet (260 to 310) | 17 brands, AI and developer tools | | The cliché; our violet is a near twin of Proton and Qwen |
| Blue (200 to 260) | Windows itself, PowerToys Awake, Warp, Zed | Native | Disappears among Windows' own blue; a keep-awake rival owns it |
| Navy, teal, green | LidGuard, Agents Don't Sleep, Supabase, Netlify, Perplexity | | Two keep-awake rivals sit there |
| Orange and amber | Claude, Mistral, Agents Don't Sleep | | Blocked by our own rule |

A dark plate fails on Windows (fact, computed): Microsoft asks that at least half the icon pass 3:1 on light and on
dark Start (question 2 source). The plate is most of the icon, so its relative luminance must sit between 0.143 and
0.265 to pass 3:1 on both `#F3F3F3` and `#202020` (the Start colours in `docs/brand.md`). A night plate such as
`#14121F` scores 1.1 on dark Start. So the night-and-light idea has to live in the mark, not in a black plate, or
needs separate light and dark icons (whether an unpackaged app can ship those is for `p6-brand-research-windows`).

A check against all 3,469 brands in the Simple Icons data (computed 2026-11-05): every saturated colour has a near
twin somewhere, so a colour alone is never unique; what counts is the field a user sees beside us. Nearest brands:

| Colour | Nearest in the whole data (dE00) |
|---|---|
| Our violet `#7250F4` | Moonrepo (a developer build tool) 0.8, CodeBuddy 1.5 (an AI coding assistant, unverified), Qwen 1.5, Proton 1.7 |
| Our gold `#FFCC4D` | Radarr 1.4, Fillout 1.9, daisyUI 2.0 |
| Olive `#5F7E00` | Ethiopian Airlines 4.9, Sway 5.0, pino (a Node logger) 7.1, Gitea 9.9 |
| Olive `#767700` | Sway 5.1, pino 8.2, Ethiopian Airlines 10.8 |
| Green `#278800` | Dlib 3.6, Ecosia 3.8, htop 4.0 |
| Magenta `#DE00A2` | Trino 0.2, FutureLearn 0.5, GraphQL 2.2 |
| Rose `#E40087` | Roll20 0.7, Veepee 1.8, Dgraph 2.7 |

My pick for `p6-brand-directions` to try first: **a mid-tone olive plate (OKLCH hue about 110 to 125), a white mark,
and the warm gold kept as one small light.** Olive is the most open colour among developer and AI brands (nearest
developer brand dE00 7.1), far from Claude's orange (dE00 37 to 45), and a mid-tone passes on both Start themes.
Second, if Haik wants warmth: magenta-rose, knowing it sits near GraphQL. Starting points from the script, at full
chroma, to be softened when drawn: olive `#5F7E00` or `#767700`; magenta `#DE00A2`. On these, white scores 4.5 to 4.8
and gold only about 3.0 to 3.2, which is why the mark should be white. Change it when the drawn olive reads as "army",
"warning" or "money", or when Haik prefers a darker, quieter look and accepts separate light and dark icons.

## 5. 2026 visual trends

### Facts

- **Liquid Glass (Apple).** Apple's app icons are now layered: "one or more foreground layers that coalesce to create
  dimensionality. These icons take on Liquid Glass attributes like specular highlights, refraction, and translucency."
  (Apple HIG, App icons, read 2026-11-05.) macOS 26 Tahoe also put every app icon in the same rounded square (Rogue
  Amoeba, 2026-06-26, https://weblog.rogueamoeba.com/2026/06/26/free-the-icons/, search result only). After a
  backlash ("Tahoe's Terrible Icons", Daring Fireball, 2025-11-07, search result only), macOS 27 (its June 2026
  betas) shows "a flattening
  of the Liquid Glass effect", "colours are much bolder", icons "sharper and more defined", and Apple kept the uniform
  shape (TidBITS, Adam Engst, 2026-06-22,
  https://tidbits.com/2026/06/22/macos-27-golden-gate-sharpens-tahoes-blurry-icons/, read 2026-11-05).
- **Microsoft.** Office icons, October 2025: "cleaner shapes", "vibrant colors", "a connected design system nodding to
  Copilot's influence" (Microsoft 365 Insider blog, 2025-10-16,
  https://techcommunity.microsoft.com/blog/microsoft365insiderblog/new-microsoft-365-icons-for-the-ai-era/4458674,
  read 2026-11-05). Copilot in 2026: a flatter, simpler mark of two squares that moves in three states, and an app
  that is "largely black and white" (question 4 sources). Microsoft's icon guidance still says gradients "should be
  subtle", at 120 degrees, light from the top left, with flat layers and soft shadows (question 2 source).
- **Logo trends.** LogoLounge's 2026 report (15 trends from more than 30,000 logos) says "Motion is a big overarching
  theme of this year's report" (The Dieline, 2026-06-11,
  https://thedieline.com/logoloung-releases-its-2026-trend-report-and-everything-is-on-the-move/, read 2026-11-05;
  the rest of that article is behind a paywall, so the 15 trends were not read).
- **2.5D icons.** Cursor ships its app icon "in 2.5D (default), 2D, and 3D, light and dark" (https://cursor.com/brand,
  read 2026-11-05).
- **Pantone Color of the Year 2026**: "Cloud Dancer", PANTONE 11-4201, an off-white (https://www.pantone.com/color-of-the-year/2026,
  read 2026-11-05; announced 2025-12-04 per press reports, not checked at Pantone).
- **AI look.** The purple gradient as the AI cliché: question 4.
- No data source was found for grain, soft 3D in general, or variable type as 2026 trends; the rows below on those are
  judgement only.

### Lasting or fad, and fit for Wakeshot (judgement)

Wakeshot should feel calm, trusted and native: a utility that sits in the tray for years and never draws attention.

| Trend | Lasting or fad | Fit | Why |
|---|---|---|---|
| Liquid Glass and depth | Lasting on Apple, as a platform rule; the heavy version is already being pulled back | Yes on Mac, by following Apple's layers; no on Windows | Apple applies the glass itself; we only need a mark that splits into a clean back layer and one foreground shape. Never fake glass on Windows |
| Soft 3D, 2.5D | Middle: comes and goes every few years | Small dose only | Microsoft's own icons use flat layers with soft shadows; one subtle shadow under the foreground shape is native, a rendered 3D object is not |
| Grain and noise | Fad | No | Turns to mud at 16 to 48 px; fine on a web hero, never in the icon |
| Bold flat marks | Lasting | **Yes** | Copilot and macOS 27 both moved back toward flatter and bolder; a flat mark survives every platform effect |
| Gradients | Lasting when subtle, fad when loud | Subtle only | Microsoft asks one or two steps at 120 degrees; the loud blue-to-purple gradient is the AI cliché |
| Monochrome with one accent | Lasting | **Yes** | It is how Windows draws its own glyphs, and where Copilot and Cursor went (OpenAI too, unverified); the accent carries the brand |
| Variable type | Lasting as technology, not a look | Neutral | Useful for the site; a wordmark needs one fixed weight |
| Motion marks | Lasting and growing (LogoLounge 2026, Copilot's state motion) | **Strong yes** | Wakeshot has real states (holding, idle, off). A mark whose motion is the product (light comes on while work runs, goes out when done) is the best use of this trend in the field |
| Pantone Cloud Dancer | Fad by design (yearly) | Barely | It only suggests a warm off-white page instead of pure white; it does not touch the icon |

Plain summary: build a **flat, bold mark with one accent colour, a state-driven motion, and a clean two-layer split for
Apple's glass**. Skip grain, fake glass and loud gradients.

## 6. Name and meaning

### What the words mean (facts, Wiktionary, read 2026-11-05)

- **wake** (verb): "To be or remain awake; not to sleep." An obsolete sense: "To be alert; to keep watch." (noun)
  "The state of forbearing sleep, especially for solemn or festive purposes; a vigil." (noun, nautical) "The path left
  behind a ship on the surface of the water." https://en.wiktionary.org/wiki/wake
- **shot** (noun): "An opportunity or attempt"; "A single serving of espresso"; "Any injection"; "A single snapshot or
  an unbroken sequence of photographic film exposures"; "The act of launching a ball or similar object toward a goal."
  https://en.wiktionary.org/wiki/shot

### What "Wakeshot" suggests (judgement)

The name stays (Haik, relayed by the orchestrator, 2026-11-05), so every idea below starts from it. Read together: "a shot of wakefulness", one dose of awake, like an espresso shot. That fits the product well: a hold
is one dose, taken while work runs and then gone. The second reading, a camera shot, is the risk: with a shutter or a
frame in the mark, "Wakeshot" reads as a screenshot tool. The "keep watch" and "vigil" sense of wake is the quiet
truth of the product: it keeps watch while an agent works and you are away.

The product's own story gives the other half: awake only while work runs, then lets go; three states (holding, idle,
off); used mostly when the user is not looking (night, lunch, a closed lid).

### Mark ideas (judgement)

Every idea must also work as a Mac app icon (Apple's rounded square, one clean foreground layer for Liquid Glass) and
as a Mac menu bar template glyph (one colour, three states), since a Mac version is planned.

| # | Idea | Strongest reason for | Strongest reason against | Mac |
|---|---|---|---|---|
| 1 | **Sunrise (current)**: half sun, three rays, horizon | Reads "wake" at once; the rise and set motion is the hold and release; the tray glyphs are built and Haik liked them | A stock glyph (Material `wb_twilight` is nearly the same) that reads as weather; morning is the wrong time for a tool used at night | Works |
| 2 | **Sunrise from a prompt**: the half sun rising from a terminal underscore `_` instead of a plain horizon | Keeps the chosen story and the tray states, and adds the one feature nobody else has: the work in the terminal holds the sun up. In motion the underscore blinks, then the sun rises | The underscore is just a line at 16 px, so the ownable part shows only from about 24 px up; the rays still break at 16 px | Works; the sun is the foreground layer |
| 3 | **Lit window at night**: a dark field with one warm lit window | Says "someone is still working while you sleep", which is exactly when Wakeshot matters; squares are crisp at 16 px | A lone square in the tray reads as "stop" or as a generic window; needs a twist (a moon notch in one corner) to be ownable | Works; good glass split (lit pane over the night) |
| 4 | **Pilot light**: one round warm light in a ring, lit while work runs | The simplest possible mark: perfect at 16 px, the state is the mark (lit, empty, struck), and its motion is a soft glow | A dot in a ring reads as "record", a radio button or a notification badge; hard to own without a distinct container | Works; very clean as a menu bar glyph |
| 5 | **Espresso shot**: a small shot glass or cup | The name's pun lands at once | The cup is the category sign (PowerToys Awake; Zhorn Caffeine's coffee pot, WakeKeeper and the GNOME Caffeine extension, per `docs/research/rivals-others.md`), so we look like a copy | Works, but sits among the copies |
| 6 | **Open eye** | Says "awake" literally | Already tested and dropped: blurs at 16 px, idle and off look alike (`docs/brand.md`); eyes mean "visibility" or "privacy" in every icon set | Weak at menu bar size |
| 7 | **Camera shutter or aperture** (shot as photo) | Uses the second half of the name | Reads as a screenshot app, the wrong product | Works technically; wrong meaning |
| 8 | **Ship's wake**: a point leaving a V-shaped trail | Abstract, calm and ownable; "work in motion leaves a trail"; the V can echo a check mark | Few users know this sense of "wake"; reads as speed or a plain chevron; says nothing about being awake | Works |
| 9 | **Night watch**: a crescent moon holding one lit star | Night plus a light that stays on: "awake while it is dark" | The moon is the system sign for sleep and Do Not Disturb on Windows and Mac, the opposite meaning, right next to our glyph | Clashes with the Mac Focus moon |
| 10 | **Sun held up**: the half sun lifted above the horizon by a short bar, set back down when done | Shows the act itself, the PC held up while work runs; still a sunrise family, so the tray story stays | A held-up sun is an unusual picture that needs the motion to explain it; risks looking like a lamp or a trophy | Works |

The three strongest (judgement):

1. **Sunrise from a prompt (2)**: the smallest step from today's decision with the biggest gain in ownability. It keeps
   the tray states Haik approved and adds a developer cue only Wakeshot has.
2. **Lit window at night (3)**: the truest picture of when the product matters, and the one place the small gold
   light earns its keep; needs the moon notch or another twist so the tray glyph is not a "stop" square.
3. **Pilot light (4)**: the cleanest motion mark and the best 16 px glyph; its weakness (generic dot) is a drawing
   problem that a distinct container can solve.

Change this ranking when `p6-brand-directions` draws them at 16, 24 and 48 px: legibility at 16 px decides ties.

## 7. Type

### May Segoe UI be used in pictures and video? (facts)

Microsoft's "Font redistribution FAQ" (page updated 2024-11-19,
https://learn.microsoft.com/en-us/typography/fonts/font-faq, read 2026-11-05) covers every font installed with
Windows, Segoe UI and Segoe UI Variable included:

- Pictures: "Can I make graphic files using the fonts, say a screengrab, advertisement, meme or poster and share, sell
  or redistribute those graphic files?" Answer: "We view creating graphic files as being essentially the same as
  printing from an output device", and print output has no restrictions "Unless you are using an application that is
  specifically licensed for home, student, or non-commercial use".
- Video: text in videos is "the same as graphic files, providing the captions or text is rendered as a bitmap image,
  and the font files themselves are not embedded within the video file."
- Logos: "we do not restrict you from making logos using the Windows-supplied fonts" (same home and student caveat).
- In an app: graphic files "for a logo or banner" are allowed; "The graphic file must be an image of a word, phrase or
  passage of text."
- Web: naming the font in a CSS font stack is fine; copying it to a server or converting it to WOFF is not.
- Not allowed: copying the fonts "to other computers or servers", converting them "to other formats", or modifying
  them.
- "Segoe UI Variable is not available for licensing or use outside of Microsoft products or on non-Windows
  platforms." (Segoe UI can be licensed from Monotype.)

Answer to the open item in `docs/brand.md`: **yes**, Segoe UI and Segoe UI Variable may be used in rendered pictures
and video, under three conditions: rendered on a licensed Windows PC from the fonts Windows installed; the output is a
bitmap (PNG, video frames) with no font file inside; the tool is not a home, student or non-commercial edition.
Unverified: whether an SVG with the text turned into outlines counts as a "graphic file" or as a format conversion; to
be safe, ship bitmaps only.

What it rules out (judgement from the terms): rendering the video on a Linux or macOS CI runner (the font would have to
be copied there), the wordmark on the Mac site or app as live text, and a self-hosted web font. With a Mac version
planned, a wordmark that can go everywhere needs a free font. The Windows app keeps the system font, and the page keeps
the system font stack.

### Three free wordmark fonts (licences are facts; fit is judgement)

All three are SIL Open Font License 1.1, per their `METADATA.pb` and `OFL.txt` in https://github.com/google/fonts
(read 2026-11-05). I rendered "Wakeshot" at weight 600 in nine free fonts and Segoe UI Variable to compare (picture in
the card's scratch folder).

| Font | Designer, added to Google Fonts | Look | For | Against |
|---|---|---|---|---|
| **Instrument Sans** (pick) | Rodrigo Fuenzalida, Jordan Egstad; 2023-04-13; weight and width axes | Compact, calm grotesque | Distinct but quiet; not yet the default of the field; the width axis helps fit the lockup and the social card | Narrower than Segoe, so a lockup beside Windows text looks a little tighter |
| **Figtree** | Erik Kennedy; 2022-07-21; weight axis | Friendly geometric | Warm and simple; round letters suit a light or sun mark | Geometric shapes feel less like Windows' humanist Segoe |
| **Inter** | Rasmus Andersson; 2020-01-24; weight and optical size axes | Neutral grotesque | The most legible and safest; wide language support | The default font of tech in 2026, so a wordmark in it looks like every other app (the type version of "AI purple") |

Considered and dropped: **Geist** (Vercel) and **Mona Sans** (GitHub) are free but are other companies' brand fonts,
so a wordmark in them reads as theirs. **Selawik** (Microsoft, OFL 1.1, "an open source replacement for Segoe UI",
last change 2023-06-05, its README says it "is missing kerning to match Segoe UI";
https://github.com/microsoft/Selawik, read 2026-11-05) is a fallback for Segoe-like text, not a wordmark.
**Atkinson Hyperlegible Next** (Braille Institute, 2025) reads very well but its letter shapes are quirky in a
wordmark. **Bricolage Grotesque**, **Manrope**, **Onest**: fine fonts, but louder (Bricolage) or close to Inter.

Pick: **Instrument Sans SemiBold** for the wordmark and lockup in every picture, the video and the Mac material. The
Windows app UI stays Segoe UI Variable. Change it when the drawn lockup looks cramped next to the mark at social-card
size; then use Inter SemiBold at its display optical size.

## 8. Colour rules

### The rules behind the checks (facts)

- WCAG 2.2 (W3C Recommendation, 2024-12-12, https://www.w3.org/TR/WCAG22/, read 2026-11-05): 1.4.1 "Color is not
  used as the only visual means of conveying information"; 1.4.3 text "at least 4.5:1" (large text 3:1; large is "at
  least 18 point or 14 point bold", about 24 px or 18.7 px bold); 1.4.6 enhanced "at least 7:1"; 1.4.11 non-text
  (icons, controls, focus rings) "at least 3:1 against adjacent color(s)". Contrast ratio is "(L1 + 0.05) / (L2 +
  0.05)" of relative luminance.
- Microsoft (question 2 source): "Make sure at least half of your icon passes a 3.0:1 contrast ratio on light and dark
  theme", and "Yellow will never pass an accessible contrast ratio on light theme until it's brown."
- Colour-blind simulation: Machado, Oliveira and Fernandes (2009) matrices at full severity, applied in linear sRGB.
  This is my choice of method; other simulators (Brettel, Vienot) can differ by a few tenths.

### The check list (script-ready)

Inputs: every colour of the palette by role: `plate` (each gradient end), `mark` (each colour on the plate), `glyph`
(tray and menu bar), `text`, `link`, `bg` (each page background), `panel`, `control`. Start colours from
`docs/brand.md`: light `#F3F3F3`, dark `#202020`. Mac Dock and menu bar colours: unverified, to come from
`p6-brand-research-macos`.

```text
id   check                                                     pass if   source
C1   each plate end vs light Start #F3F3F3                     >= 3.0    Microsoft icon guide
C2   each plate end vs dark Start #202020                      >= 3.0    Microsoft icon guide
     (C1 and C2 together: plate relative luminance 0.143 to 0.265)
C3   each mark colour vs each plate end                        >= 3.0    WCAG 1.4.11
C4   C3 after protan, deutan and tritan simulation             >= 3.0    WCAG 1.4.11 plus our rule
C5   tray glyph vs light and dark taskbar, each state          >= 3.0    WCAG 1.4.11
C6   the three states differ by shape, checked in one colour   yes       WCAG 1.4.1
C7   body text vs each bg and panel                            >= 4.5    WCAG 1.4.3 (aim 7.0, 1.4.6)
C8   large text (24 px, or 18.7 px bold) vs bg                 >= 3.0    WCAG 1.4.3
C9   link vs bg; links underlined, or link vs body text        >= 4.5    WCAG 1.4.3; 1.4.1
C10  buttons, toggles, focus ring vs adjacent colour           >= 3.0    WCAG 1.4.11
C11  yellow or gold never as text or a thin line on light bg    never     Microsoft icon guide
C12  every brand colour, dE00 from Claude orange #D97757        >= 20     our rule (threshold is judgement)
C13  brand main colour, dE00 from rivals and nearest AI tools   >= 10     judgement
C14  gradients: run C1 to C4 at both ends and the midpoint      all pass  judgement
```

### The current palette through the list (facts, computed 2026-11-05)

| Check | Result |
|---|---|
| C1, C2 plate ends `#7957F4` and `#6B49F3` | Pass: 4.2 and 4.9 on light, 3.5 and **3.0** on dark (the dark end passes by nothing) |
| C3 gold on the plate ends | Pass: 3.1 and 3.6 |
| C4 gold on the plate, colour-blind | **Fail**: protan 2.6 and 2.8; tritan 2.7 on the lighter end; deutan passes (3.2, 3.6) |
| C12 Claude distance | Pass: violet 44 and 46, gold 32 |
| C13 nearest brands | **Fail**: violet is dE00 1.5 from Qwen, 1.7 from Proton |
| A dark night plate `#14121F` (tested as an option) | **Fails C2**: 1.1 on dark Start |

So the gold sun on the violet plate, the main picture of today's brand, loses its 3:1 edge for people with protan
colour blindness (about 1 in 100 men; prevalence not checked at a source). A white mark on a mid-tone plate passes
every check above (question 4's starting points `#5F7E00`, `#767700`, `#278800`, `#DE00A2`, `#E40087` with a white mark
pass C1 to C4 and C12, computed 2026-11-05).
