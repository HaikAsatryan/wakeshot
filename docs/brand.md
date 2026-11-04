# Brand

What Wakeshot looks like, and why. Decided with Haik on 2026-11-04 (his words: "Yes default to all your work done. I
like it."). The options he saw, with real taskbar mockups and rival icons, were rendered from a prototype of the art
code; the art itself is `crates/wakeshot-windows/src/tray/art.rs`.

## The idea

Wakeshot is awake only while work runs, then lets go. The picture for that is a sunrise: the sun is up and shining
while something holds the PC awake, and only an outline when nothing does. In motion (the README loop, the video's
first shot), the sun rises when an agent starts and sets when it is done.

## Colours

| Name | Hex | Where |
|---|---|---|
| Night violet | `#7250F4` | The brand colour. The app icon plate runs `#7957F4` (top left) to `#6B49F3` (bottom right) at 120 degrees |
| Sun gold | `#FFCC4D` | The sun in the app icon; the "holding" highlight on the page and in the video. Never as text on a light page |
| Violet text | `#5B3FD6` | Links and buttons on a light page |
| Violet light | `#A996FF` | Links on a dark page |
| Ink | `#1A1A1A` | Text; the tray glyph on a light taskbar |
| Night | `#14121F` | Dark page background |
| Paper | `#FFFFFF` | Light page background |
| Mist | `#F3F3F3` | Light panels |

Why: no keep-awake rival uses violet. PowerToys Awake is Windows blue, LidGuard navy with green and teal, Agents
Don't Sleep navy with amber (their icon files, read 2026-11-04). Windows' default accent is blue, so a violet icon
stands out in Start. Gold is the warm light against the night, and violet and gold are opposite colours, so the small
icon still reads. The old orange was dropped: it sits near Agents Don't Sleep's amber and in the same family as
Claude's own orange (`#D97757`, reported by copies of Anthropic's brand guide, not read at the source), so it could
look like an official Claude tool.

Contrast (WCAG), checked with a script on 2026-11-04:

- Plate ends on light Start `#F3F3F3`: 4.2 and 4.9. On dark Start `#202020`: 3.5 and 3.0. Microsoft asks 3:1 on light
  and dark for at least half of the icon ("Design guidelines for Windows app icons"); the whole plate passes.
- Gold on the plate ends: 3.1 and 3.6. White on them: 4.7 and 5.4. Gold on white: 1.5, so never gold text on light.
- Page: ink on paper 17.4, violet text on paper 6.7, violet light on night 7.5.

## Tray icon

- **No brand colour in the tray.** The glyph is white on a dark taskbar, `#1A1A1A` on a light one, and the system text
  colour in high contrast, like Windows' own tray glyphs. Why: a coloured tray glyph looks foreign next to Windows'
  own and breaks high contrast.
- **Three states, not two** (Haik's yes, 2026-11-04). Holding: a risen half sun with three rays over the horizon.
  Idle: an empty half sun on the horizon. Off: the empty sun struck through. Why three: with two, idle and off look
  the same, so a user who turned Automatic off and forgot sees nothing wrong, starts a long agent run, and the PC
  sleeps mid-run. The off glyph is idle struck through, the Windows sign for off, so there is nothing new to learn.
  Change it when users say the off icon confuses them.
- **The Sunrise, not the Cup** (Haik's yes, 2026-11-04). Why: PowerToys Awake's tray icon is a cup in every state
  (`microsoft/PowerToys`, `src/modules/awake/Awake/Assets/Awake/`, read 2026-11-04); next to it, our cup reads as a
  second copy, and a cup is the sign for the whole category. The sunrise says "wake" without a word. Eye and Sky were
  dropped: the Eye blurs at 16 px and its idle and off look alike; the Sky's full sun is Windows' brightness sign.
  Cost: the sun's curves leave more soft pixels at 16 and 20 px than the Cup; from 24 px it is as sharp.

## App icon

A violet rounded square with the holding sunrise on it: the sun and rays in gold, the horizon in white (Haik's yes,
2026-11-04). The mark fills 72% of the icon. The plate keeps the gold readable on light backgrounds: Microsoft says
"Yellow will never pass an accessible contrast ratio on light theme until it's brown". A rounded square, not a disc,
so the horizon keeps its full width. Its corners (radius 3/16 of the icon) are rounder than Microsoft's "2px radius at
48x48" for shapes inside an icon; change them when the icon looks out of place in Start.

## Logo and wordmark

No separate logo: the app icon plus the name "Wakeshot" in Segoe UI Variable Semibold. Microsoft: "Icons should not
include typography", and Windows shows the name next to the icon anyway. The web page uses the system font stack (no
fonts from other hosts, `p6-pages`). Change it when a banner or Store art needs a fixed lockup.

Not checked: whether Segoe UI may be used in rendered pictures and the video. `p6-media` checks it; the fallback is
Inter (SIL Open Font License).
