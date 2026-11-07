use super::*;
use crate::tray::theme::Contrast;
use wakeshot_core::holds::HoldKind;
use wakeshot_core::text::Status;
use wakeshot_core::view::{IconState, KeepAwake};

const DARK: Theme = Theme {
    light: false,
    contrast: None,
};
const LIGHT: Theme = Theme {
    light: true,
    contrast: None,
};
const BOTH: Columns = Columns {
    marks: true,
    icons: true,
};

fn action(command: MenuCommand, checked: bool, enabled: bool) -> MenuItem {
    MenuItem::Action {
        text: text::action(command),
        command,
        checked,
        enabled,
    }
}

fn label(text: &str, icon: Option<IconState>) -> MenuItem {
    MenuItem::Label {
        text: text.to_owned(),
        icon,
    }
}

const STATES: [IconState; 3] = [IconState::Holding, IconState::Idle, IconState::Off];

fn keep_awake() -> Vec<MenuItem> {
    let mut items: Vec<MenuItem> = [
        KeepAwake::Off,
        KeepAwake::Minutes(30),
        KeepAwake::Minutes(60),
        KeepAwake::Minutes(120),
        KeepAwake::Minutes(240),
        KeepAwake::UntilOff,
    ]
    .into_iter()
    .map(|k| action(MenuCommand::KeepAwake(k), k == KeepAwake::Minutes(60), true))
    .collect();
    items.push(MenuItem::Separator);
    items.push(action(MenuCommand::KeepScreenOn(true), false, true));
    items
}

/// The product menu as `present` builds it: holding for two Claude Code sessions and Keep awake 1 hour.
fn product_menu() -> Vec<MenuItem> {
    product_menu_holding(&[
        text::hold_group(HoldKind::Agent, "Claude Code", 2),
        text::keep_awake_group(KeepAwake::Minutes(25)),
    ])
}

fn product_menu_holding(groups: &[String]) -> Vec<MenuItem> {
    let status = Status {
        icon: IconState::Holding,
        stepped_back: false,
        requests_only: false,
        screen_on: false,
    };
    vec![
        MenuItem::Label {
            text: text::status_line(&status, groups),
            icon: Some(status.icon),
        },
        MenuItem::Separator,
        action(MenuCommand::Automatic(false), true, true),
        MenuItem::Submenu {
            text: text::keep_awake_menu(),
            items: keep_awake(),
        },
        MenuItem::Separator,
        action(MenuCommand::Restore, false, true),
        action(MenuCommand::OpenLog, false, true),
        action(MenuCommand::Quit, false, true),
    ]
}

/// Each colour encodes its own index, so a test sees which system colour filled which field.
fn sample_contrast() -> Contrast {
    let c = |i: u8| Rgb::new(i, i, 0x80);
    Contrast {
        window: c(5),
        window_text: c(8),
        highlight: c(13),
        highlight_text: c(14),
        gray_text: c(17),
        button_face: c(15),
        button_text: c(18),
        hot_light: c(26),
    }
}

/// A contrast theme from its `.theme` file's `[Control Panel\Colors]`, in this order: `Window`, `WindowText`,
/// `Hilight`, `HilightText`, `GrayText`, `ButtonFace`, `ButtonText`, `HotTrackingColor`.
const fn theme_file(c: [[u8; 3]; 8]) -> Theme {
    const fn rgb(c: [u8; 3]) -> Rgb {
        Rgb::new(c[0], c[1], c[2])
    }
    Theme {
        light: false,
        contrast: Some(Contrast {
            window: rgb(c[0]),
            window_text: rgb(c[1]),
            highlight: rgb(c[2]),
            highlight_text: rgb(c[3]),
            gray_text: rgb(c[4]),
            button_face: rgb(c[5]),
            button_text: rgb(c[6]),
            hot_light: rgb(c[7]),
        }),
    }
}

/// Windows 11's four contrast themes as they ship, from `C:\Windows\Resources\Ease of Access Themes` on the
/// test laptop. Which file is which theme in Settings: `docs/research.md`, "Tray UX: menu painting".
const CONTRAST_THEMES: [(&str, Theme); 4] = [
    (
        "Aquatic, hcblack.theme",
        theme_file([
            [32, 32, 32],
            [255, 255, 255],
            [142, 227, 240],
            [38, 59, 80],
            [166, 166, 166],
            [32, 32, 32],
            [255, 255, 255],
            [117, 233, 252],
        ]),
    ),
    (
        "Desert, hcwhite.theme",
        theme_file([
            [255, 250, 239],
            [61, 61, 61],
            [144, 57, 9],
            [255, 245, 227],
            [103, 103, 103],
            [255, 250, 239],
            [32, 32, 32],
            [28, 94, 117],
        ]),
    ),
    (
        "Dusk, hc1.theme",
        theme_file([
            [45, 50, 54],
            [255, 255, 255],
            [161, 191, 222],
            [33, 45, 59],
            [166, 166, 166],
            [45, 50, 54],
            [182, 246, 240],
            [112, 235, 222],
        ]),
    ),
    (
        "Night sky, hc2.theme",
        theme_file([
            [0, 0, 0],
            [255, 255, 255],
            [214, 180, 253],
            [43, 43, 43],
            [166, 166, 166],
            [0, 0, 0],
            [255, 238, 50],
            [128, 128, 255],
        ]),
    ),
];

/// The contrast theme in which a screen check saw the status line in the commands' yellow.
const NIGHT_SKY: Theme = CONTRAST_THEMES[3].1;

/// One item painted at the origin of a canvas of its own size.
struct Painted {
    px: Vec<u32>,
    w: i32,
    layout: Layout,
    palette: Palette,
    clip: Option<RECT>,
}

impl Painted {
    fn new(item: &MenuItem, state: ItemState, dpi: u32, theme: &Theme) -> Self {
        Self::sized(item, state, dpi, theme, scale(320, dpi))
    }

    fn sized(item: &MenuItem, state: ItemState, dpi: u32, theme: &Theme, w: i32) -> Self {
        let fonts = Fonts::new(dpi).expect("fonts");
        let layout = Layout::new(dpi, BOTH);
        let palette = Palette::for_theme(theme);
        let h = if matches!(item, MenuItem::Separator) {
            layout.separator
        } else {
            layout.row
        };
        let mut canvas = Canvas::new(w, h).expect("canvas");
        let rect = RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };
        let clip =
            paint_item(canvas.dc, rect, item, state, &palette, &fonts, &layout).expect("paint");
        Self {
            px: canvas.pixels().to_vec(),
            w,
            layout,
            palette,
            clip,
        }
    }

    fn at(&self, x: i32, y: i32) -> Rgb {
        let p = self.px[(y * self.w + x) as usize];
        Rgb::new((p >> 16) as u8, (p >> 8) as u8, p as u8)
    }

    /// Pixels in columns `x0..x1` of the text line that are not `fill`: ink of a text or a glyph.
    fn ink(&self, x0: i32, x1: i32, fill: Rgb) -> usize {
        let top = self.layout.line_top;
        (top..top + self.layout.line)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| self.at(x, y) != fill)
            .count()
    }
}

const MOUSE: ItemState = ItemState {
    selected: false,
    keyboard_cues: false,
};
const HOT: ItemState = ItemState {
    selected: true,
    keyboard_cues: false,
};

#[test]
fn layout_matches_winui_at_96_dpi() {
    let l = Layout::new(96, Columns::default());
    assert_eq!(l.row, 33, "2 + 4 + 20 + 5 + 2");
    assert_eq!(l.separator, 3);
    assert_eq!((l.separator_top, l.separator_line), (1, 1));
    assert_eq!((l.inset_x, l.inset_y, l.radius), (4, 2, 4));
    assert_eq!((l.line_top, l.line), (6, 20));
    assert_eq!((l.mark, l.icon, l.chevron, l.font), (12, 16, 12, 14));
    assert_eq!(l.mark_left, 15, "item margin 4 + padding 11");
    assert_eq!(l.text_left, 15, "no columns");
    assert_eq!(l.right, 15);
    assert_eq!(
        l.chevron_right, 51,
        "gap 24 + chevron 12 + padding 11 + margin 4"
    );

    let marks = Layout::new(
        96,
        Columns {
            marks: true,
            icons: false,
        },
    );
    assert_eq!(marks.text_left, 43);
    let icons = Layout::new(
        96,
        Columns {
            marks: false,
            icons: true,
        },
    );
    assert_eq!((icons.icon_left, icons.text_left), (15, 43));
    let both = Layout::new(96, BOTH);
    assert_eq!(
        (both.mark_left, both.icon_left, both.text_left),
        (15, 43, 71)
    );
    assert_eq!(both.width_for(100, false), 71 + 100 + 15);
    assert_eq!(both.width_for(100, true), 71 + 100 + 51);
}

#[test]
fn layout_scales_at_120_144_and_192_dpi() {
    // (dpi, row, separator, font, mark, icon, text_left with both columns, radius)
    for (dpi, row, separator, font, mark, icon, text_left, radius) in [
        (120, 41, 4, 18, 15, 20, 89, 5),
        (144, 50, 5, 21, 18, 24, 107, 6),
        (192, 66, 6, 28, 24, 32, 142, 8),
    ] {
        let l = Layout::new(dpi, BOTH);
        assert_eq!(l.row, row, "row at {dpi}");
        assert_eq!(l.separator, separator, "separator at {dpi}");
        assert_eq!(l.font, font, "font at {dpi}");
        assert_eq!(l.mark, mark, "mark at {dpi}");
        assert_eq!(l.icon, icon, "icon at {dpi}");
        assert_eq!(l.text_left, text_left, "text at {dpi}");
        assert_eq!(l.radius, radius, "radius at {dpi}");
        assert!(l.separator_line >= 1);
        assert!(
            l.line_top + l.line <= l.row - l.inset_y,
            "the line fits the row at {dpi}"
        );
        assert!(l.mark_left < l.icon_left && l.icon_left < l.text_left);
    }
}

#[test]
fn palette_light_and_dark_match_the_tokens() {
    let light = Palette::for_theme(&LIGHT);
    assert_eq!(light.background, Rgb::new(0xf9, 0xf9, 0xf9));
    assert_eq!(light.text, Rgb::new(0x1a, 0x1a, 0x1a), "#E4000000");
    assert_eq!(light.secondary, Rgb::new(0x5f, 0x5f, 0x5f), "#9E000000");
    assert_eq!(light.disabled, Rgb::new(0x9f, 0x9f, 0x9f), "#5C000000");
    assert_eq!(light.hover, Rgb::new(0xf0, 0xf0, 0xf0), "#09000000");
    assert_eq!(light.separator, Rgb::new(0xea, 0xea, 0xea), "#0F000000");
    assert_eq!(light.border, Rgb::new(0xea, 0xea, 0xea), "#0F000000");

    let dark = Palette::for_theme(&DARK);
    assert_eq!(dark.background, Rgb::new(0x2c, 0x2c, 0x2c));
    assert_eq!(dark.text, Rgb::new(0xff, 0xff, 0xff));
    assert_eq!(dark.secondary, Rgb::new(0xcf, 0xcf, 0xcf), "#C5FFFFFF");
    assert_eq!(dark.disabled, Rgb::new(0x79, 0x79, 0x79), "#5DFFFFFF");
    assert_eq!(dark.hover, Rgb::new(0x38, 0x38, 0x38), "#0FFFFFFF");
    assert_eq!(dark.hover_text, Rgb::new(0xff, 0xff, 0xff));
    assert_eq!(dark.separator, Rgb::new(0x3d, 0x3d, 0x3d), "#15FFFFFF");
    assert_eq!(
        dark.border,
        Rgb::new(0x23, 0x23, 0x23),
        "#33000000, the Phase 1 border"
    );
    // The status line is WinUI's secondary text, as the marks and the chevron; its glyph takes the tray icon's
    // colours for the same mode.
    assert_eq!(light.label, light.secondary);
    assert_eq!(dark.label, dark.secondary);
    assert_eq!(light.status, art::ON_LIGHT);
    assert_eq!(dark.status, art::ON_DARK);
}

#[test]
fn palette_contrast_uses_system_colours() {
    let c = sample_contrast();
    for light in [false, true] {
        let p = Palette::for_theme(&Theme {
            light,
            contrast: Some(c),
        });
        assert_eq!(p.background, c.window);
        assert_eq!(p.text, c.button_text);
        assert_eq!(p.secondary, c.button_text);
        assert_eq!(p.label, c.window_text);
        assert_eq!(p.disabled, c.gray_text);
        assert_eq!(p.hover, c.highlight);
        assert_eq!(p.hover_text, c.highlight_text);
        assert_eq!(p.hover_secondary, c.highlight_text);
        assert_eq!(p.separator, c.window_text);
        assert_eq!(p.border, c.window_text);
        // One colour, the status line's own text colour.
        assert_eq!(p.status, GlyphColours::one(p.label.into()));
    }
}

#[test]
fn contrast_status_line_is_window_text_in_every_theme() {
    let text = "Holding awake: Claude Code (2 sessions)";
    for (name, theme) in CONTRAST_THEMES {
        let c = theme.contrast.expect("a contrast theme");
        let p = Palette::for_theme(&theme);
        // WinUI's high contrast secondary text, set apart from the commands wherever the theme's colours differ.
        assert_eq!(p.label, c.window_text, "{name}");
        assert_eq!(p.status, GlyphColours::one(c.window_text.into()), "{name}");
        assert_eq!(p.text, c.button_text, "{name}");
        assert_eq!(p.label != p.text, c.window_text != c.button_text, "{name}");
        // Marks at rest and the chevron stay with the commands, as WinUI's `MenuFlyoutSubItemChevron`.
        assert_eq!(p.secondary, c.button_text, "{name}");
        let (plain, _) = contrast(p.label, p.background);
        assert!(plain >= 7.0, "{name}: {plain:.2}");
        println!(
            "{name}: status line {:02x?}, commands {:02x?}, on {:02x?}: {plain:.2}",
            <[u8; 3]>::from(p.label),
            <[u8; 3]>::from(p.text),
            <[u8; 3]>::from(p.background)
        );

        // As drawn: the line's fully covered text and its glyph are the status line's colour, never the commands'.
        for dpi in [96, 144] {
            let row = Painted::new(&label(text, Some(IconState::Holding)), MOUSE, dpi, &theme);
            let drawn = |colour: Rgb| {
                (0..row.layout.row)
                    .flat_map(|y| (0..row.w).map(move |x| (x, y)))
                    .filter(|&(x, y)| row.at(x, y) == colour)
                    .count()
            };
            let l = row.layout;
            let in_glyph = (0..l.row)
                .flat_map(|y| (l.icon_left..l.text_left).map(move |x| (x, y)))
                .filter(|&(x, y)| row.at(x, y) == c.window_text)
                .count();
            assert!(drawn(c.window_text) > in_glyph, "text at {dpi}: {name}");
            assert!(in_glyph > 0, "glyph at {dpi}: {name}");
            if c.window_text != c.button_text {
                assert_eq!(drawn(c.button_text), 0, "at {dpi}: {name}");
            }
        }
    }
}

#[test]
fn marks_follow_the_command() {
    assert_eq!(mark_for(MenuCommand::Automatic(true)), Some(Mark::Check));
    assert_eq!(mark_for(MenuCommand::Automatic(false)), Some(Mark::Check));
    assert_eq!(mark_for(MenuCommand::KeepScreenOn(true)), Some(Mark::Check));
    for k in [KeepAwake::Off, KeepAwake::Minutes(30), KeepAwake::UntilOff] {
        assert_eq!(mark_for(MenuCommand::KeepAwake(k)), Some(Mark::Radio));
    }
    for c in [
        MenuCommand::Restore,
        MenuCommand::OpenLog,
        MenuCommand::Quit,
    ] {
        assert_eq!(mark_for(c), None);
    }
    assert_eq!(Mark::Check.glyph(), 0xE73E);
    assert_eq!(Mark::Radio.glyph(), 0xE915);
}

#[test]
fn icons_only_for_commands_with_a_known_picture() {
    let menu = product_menu();
    let icons: Vec<Option<Icon>> = menu.iter().map(icon_for).collect();
    assert_eq!(
        icons,
        [
            Some(Icon::Tray(IconState::Holding)),
            None,
            Some(Icon::Font(ICON_AUTOMATIC)),
            Some(Icon::Font(ICON_KEEP_AWAKE)),
            None,
            Some(Icon::Font(ICON_RESTORE)),
            Some(Icon::Font(ICON_LOG)),
            Some(Icon::Font(ICON_QUIT)),
        ]
    );
    for state in STATES {
        assert_eq!(icon_for(&label("x", Some(state))), Some(Icon::Tray(state)));
    }
    assert_eq!(icon_for(&label("x", None)), None, "a line with no state");
    assert!(keep_awake().iter().all(|i| icon_for(i).is_none()));
    let other = MenuItem::Submenu {
        text: "&Other".into(),
        items: Vec::new(),
    };
    assert_eq!(icon_for(&other), None, "a new submenu needs its own pick");
    // Keep awake's icon is the clock "Recent".
    assert_eq!(ICON_KEEP_AWAKE, 0xE823);
}

#[test]
fn item_glyphs_are_in_both_icon_fonts() {
    // Where Segoe Fluent Icons is missing the menu draws with Segoe MDL2 Assets, and a glyph a font lacks draws as
    // an empty box, so every glyph the menu uses must be in both.
    const ALL: [u16; 8] = [
        GLYPH_CHECK,
        GLYPH_RADIO,
        GLYPH_CHEVRON,
        ICON_AUTOMATIC,
        ICON_KEEP_AWAKE,
        ICON_RESTORE,
        ICON_LOG,
        ICON_QUIT,
    ];
    let fonts = Fonts::new(96).expect("fonts");
    for face in [GLYPH_FACE, GLYPH_FACE_OLD] {
        let font = create_font(face, scale(ICON, 96)).expect("font");
        let installed = fonts.is_face(font, face);
        let missing: Vec<String> = ALL
            .iter()
            .filter(|&&g| fonts.missing(font, &[g]) > 0)
            .map(|g| format!("{g:04X}"))
            .collect();
        // "Clock" E917 is not in MDL2's cmap (`docs/brand.md`, "Tray icon"); seeing that shows the check can
        // find a missing glyph at all.
        let control = fonts.missing(font, &[0xE917]);
        delete(font.into());
        if !installed {
            println!("{face} is not installed here: not checked");
            continue;
        }
        assert!(missing.is_empty(), "{face} lacks {missing:?}");
        if face == GLYPH_FACE_OLD {
            assert_eq!(control, 1, "{face} has E917 now, or the check sees no gap");
        }
    }
}

#[test]
fn columns_follow_marks_and_icons() {
    assert_eq!(Columns::of(&product_menu()), BOTH);
    assert_eq!(
        Columns::of(&keep_awake()),
        Columns {
            marks: true,
            icons: false
        }
    );
    let plain = [action(MenuCommand::Quit, false, true)];
    assert_eq!(
        Columns::of(&plain),
        Columns {
            marks: false,
            icons: true
        }
    );
    assert_eq!(Columns::of(&[]), Columns::default());
    // The status glyph takes the icon column like any icon; a line with no state takes none.
    assert_eq!(
        Columns::of(&[label("Idle", Some(IconState::Idle))]),
        Columns {
            marks: false,
            icons: true
        }
    );
    assert_eq!(Columns::of(&[label("x", None)]), Columns::default());
}

#[test]
fn access_key_underline_only_with_keyboard_cues() {
    assert_eq!(
        ItemState::from_ods(0),
        ItemState {
            selected: false,
            keyboard_cues: true
        }
    );
    assert_eq!(
        ItemState::from_ods(ODS_NOACCEL.0 | ODS_SELECTED.0),
        ItemState {
            selected: true,
            keyboard_cues: false
        }
    );
    let quit = action(MenuCommand::Quit, false, true);
    let cues = ItemState {
        selected: false,
        keyboard_cues: true,
    };
    assert_eq!(
        prefix_format(&quit, cues),
        DRAW_TEXT_FORMAT(0),
        "underlined"
    );
    assert_eq!(prefix_format(&quit, MOUSE), DT_HIDEPREFIX);
    let label = label("Holding awake", Some(IconState::Holding));
    assert_eq!(
        prefix_format(&label, cues),
        DT_HIDEPREFIX,
        "a status line has no key"
    );

    // Painted: with cues the underline adds ink below the text; without, it does not.
    let with = Painted::new(&quit, cues, 96, &DARK);
    let without = Painted::new(&quit, MOUSE, 96, &DARK);
    let l = with.layout;
    let ink = |p: &Painted| p.ink(l.text_left, l.text_left + 60, p.palette.background);
    assert!(
        ink(&with) > ink(&without),
        "{} > {}",
        ink(&with),
        ink(&without)
    );
}

#[test]
fn visible_text_drops_access_key_marks() {
    let v = |s: &str| String::from_utf16(&visible(s)).expect("utf-16");
    assert_eq!(v("&Keep awake"), "Keep awake");
    assert_eq!(v("Keep &screen on"), "Keep screen on");
    assert_eq!(v("Save && &Quit"), "Save & Quit");
    assert_eq!(v("trailing &"), "trailing ");
}

#[test]
fn label_never_highlights() {
    let label = label(
        "Holding awake: Claude Code (2 sessions)",
        Some(IconState::Holding),
    );
    let p = Painted::new(&label, HOT, 96, &DARK);
    let (x, y) = (p.layout.inset_x + 2, p.layout.row / 2);
    assert_eq!(p.at(x, y), p.palette.background, "no hover fill on a label");
    assert!(
        p.ink(
            p.layout.text_left,
            p.layout.text_left + 40,
            p.palette.background
        ) > 0
    );

    let disabled = action(MenuCommand::KeepScreenOn(true), false, false);
    let p = Painted::new(&disabled, HOT, 96, &DARK);
    assert_eq!(
        p.at(x, y),
        p.palette.background,
        "no hover fill on a disabled item"
    );
}

/// The status line painted with its state, and the same line with none, as it was drawn before the glyph.
fn status_pair(state: IconState, dpi: u32, theme: &Theme) -> (Painted, Painted) {
    let text = "Holding awake: Claude Code (2 sessions)";
    (
        Painted::new(&label(text, Some(state)), MOUSE, dpi, theme),
        Painted::new(&label(text, None), MOUSE, dpi, theme),
    )
}

/// Pixels of the whole row in columns `x0..x1` that are not `fill`.
fn row_ink(p: &Painted, x0: i32, x1: i32, fill: Rgb) -> usize {
    (0..p.layout.row)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| p.at(x, y) != fill)
        .count()
}

/// The left, right, top and bottom of the ink in columns `x0..x1` of the row, edges exclusive at the far end.
fn ink_box(p: &Painted, x0: i32, x1: i32, fill: Rgb) -> (i32, i32, i32, i32) {
    let mut b = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for y in 0..p.layout.row {
        for x in x0..x1 {
            if p.at(x, y) != fill {
                b = (b.0.min(x), b.1.max(x + 1), b.2.min(y), b.3.max(y + 1));
            }
        }
    }
    b
}

#[test]
fn status_row_draws_the_tray_glyph() {
    let contrast = Theme {
        light: false,
        contrast: Some(sample_contrast()),
    };
    // The icon size follows the dpi as every size does: 16 epx is 16, 20, 24, 28 and 32 px.
    for (dpi, icon) in [(96, 16), (120, 20), (144, 24), (168, 28), (192, 32)] {
        assert_eq!(Layout::new(dpi, BOTH).icon, icon, "icon at {dpi}");
        for theme in [LIGHT, DARK, contrast] {
            for state in STATES {
                let at = format!("{state:?} at {dpi} dpi, light {}", theme.light);
                let (with, without) = status_pair(state, dpi, &theme);
                let (l, bg, colours) = (with.layout, with.palette.background, with.palette.status);
                // The tray's own frame at the icon size, laid over the row in the glyph's box; every other
                // pixel, the text and its ellipsis included, as the line drew without a glyph.
                let glyph = art::colorize(&art::tray_mask(state, l.icon.unsigned_abs()), colours);
                let (left, top) = glyph_origin(&l);
                for y in 0..l.row {
                    for x in 0..with.w {
                        let under = without.at(x, y);
                        let want = if in_glyph_box(&l, x, y) {
                            let i = (((y - top) * l.icon + (x - left)) * 4) as usize;
                            let g = &glyph.pixels[i..i + 4];
                            lerp(under, Rgb::new(g[0], g[1], g[2]), f32::from(g[3]) / 255.0)
                        } else {
                            under
                        };
                        assert_eq!(with.at(x, y), want, "{x},{y}: {at}");
                    }
                }
                // Ink in the icon column, none in the check column, and the text starts where it did.
                assert!(row_ink(&with, l.icon_left, l.text_left, bg) > 0, "{at}");
                assert_eq!(row_ink(&with, 0, l.icon_left, bg), 0, "{at}");
                let text = |p: &Painted| ink_box(p, l.icon_left + l.icon, p.w, bg).0;
                assert_eq!(text(&with), text(&without), "{at}");
                assert!(text(&with) >= l.text_left, "{at}");
                // Centred in the icon box: the glyph's widest part, its line, sits in the middle of its frame.
                let (x0, x1, y0, y1) = ink_box(&with, 0, l.text_left, bg);
                assert!((x0 + x1 - (2 * l.icon_left + l.icon)).abs() <= 1, "{at}");
                assert!(y0 >= top && y1 <= top + l.icon, "{at}");
                // The light is Moss while holding and nowhere else.
                let light = Rgb::new(colours.light[0], colours.light[1], colours.light[2]);
                let lit = (0..l.row)
                    .flat_map(|y| (l.icon_left..l.text_left).map(move |x| (x, y)))
                    .filter(|&(x, y)| with.at(x, y) == light)
                    .count();
                if theme.contrast.is_none() {
                    assert_eq!(lit > 0, state == IconState::Holding, "{at}");
                }
            }
        }
    }
}

#[test]
fn status_glyph_sits_where_the_row_icons_sit() {
    // The Automatic row's eye and the status glyph, each across its own ink: centred on the same column within a
    // pixel, at every scale.
    for dpi in [96, 120, 144, 168, 192] {
        let (status, _) = status_pair(IconState::Holding, dpi, &DARK);
        let auto = Painted::new(
            &action(MenuCommand::Automatic(false), false, true),
            MOUSE,
            dpi,
            &DARK,
        );
        let l = status.layout;
        let bg = status.palette.background;
        let (a0, a1, _, _) = ink_box(&status, l.icon_left, l.text_left, bg);
        let (b0, b1, _, _) = ink_box(&auto, l.icon_left, l.text_left, bg);
        println!("{dpi} dpi: glyph {a0}..{a1}, eye {b0}..{b1}");
        assert!(((a0 + a1) - (b0 + b1)).abs() <= 2, "{dpi} dpi");
    }
}

fn linear(c: u8) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(l: [f64; 3]) -> f64 {
    0.2126 * l[0] + 0.7152 * l[1] + 0.0722 * l[2]
}

fn ratio(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Protan, deutan and tritan at full strength in linear sRGB (Machado, Oliveira and Fernandes 2009), as `art.rs`
/// checks the tray glyph.
const MACHADO: [[[f64; 3]; 3]; 3] = [
    [
        [0.152286, 1.052583, -0.204868],
        [0.114503, 0.786281, 0.099216],
        [-0.003882, -0.048116, 1.051998],
    ],
    [
        [0.367322, 0.860646, -0.227968],
        [0.280085, 0.672501, 0.047413],
        [-0.011820, 0.042940, 0.968881],
    ],
    [
        [1.255528, -0.076749, -0.178779],
        [-0.078411, 0.930809, 0.147602],
        [0.004733, 0.691367, 0.303900],
    ],
];

/// WCAG 2 contrast, plain and under the worst of the three simulations.
fn contrast(a: Rgb, b: Rgb) -> (f64, f64) {
    let lin = |c: Rgb| [linear(c.r), linear(c.g), linear(c.b)];
    let plain = ratio(luminance(lin(a)), luminance(lin(b)));
    let blind = MACHADO
        .iter()
        .map(|m| {
            let seen = |c: Rgb| {
                let l = lin(c);
                luminance(
                    m.map(|row| (row[0] * l[0] + row[1] * l[1] + row[2] * l[2]).clamp(0.0, 1.0)),
                )
            };
            ratio(seen(a), seen(b))
        })
        .fold(f64::MAX, f64::min);
    (plain, blind)
}

#[test]
fn status_glyph_keeps_3_to_1() {
    // WCAG 2 non-text contrast (1.4.11) on the drawn pixels: every fully covered pixel of each part of the glyph
    // against the menu fill, plain and colour-blind.
    for theme in [LIGHT, DARK] {
        for dpi in [96, 120, 144] {
            for state in STATES {
                let (p, _) = status_pair(state, dpi, &theme);
                let (l, bg, colours) = (p.layout, p.palette.background, p.palette.status);
                let mut parts = vec![("ink", colours.ink)];
                if state == IconState::Holding {
                    parts.push(("light", colours.light));
                }
                for (name, rgb) in parts {
                    let part = Rgb::new(rgb[0], rgb[1], rgb[2]);
                    let drawn: Vec<Rgb> = (0..l.row)
                        .flat_map(|y| (l.icon_left..l.text_left).map(move |x| (x, y)))
                        .map(|(x, y)| p.at(x, y))
                        .filter(|&c| c == part)
                        .collect();
                    assert!(!drawn.is_empty(), "{name} drawn: {state:?} at {dpi}");
                    for c in drawn {
                        let (plain, blind) = contrast(c, bg);
                        assert!(
                            plain >= 3.0 && blind >= 3.0,
                            "{name} {plain:.2} ({blind:.2}): {state:?} at {dpi}, light {}",
                            theme.light
                        );
                    }
                    if dpi == 96 && state == IconState::Holding {
                        let (plain, blind) = contrast(part, bg);
                        println!(
                            "light {}: {name} {:02x?} on {:02x?}: {plain:.2} (colour-blind {blind:.2})",
                            theme.light,
                            rgb,
                            <[u8; 3]>::from(bg)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn paints_offscreen() {
    for dpi in [96, 144] {
        for theme in [DARK, LIGHT] {
            let palette = Palette::for_theme(&theme);
            let l = Layout::new(dpi, BOTH);
            let hover_spot = (l.inset_x + 2, l.row / 2);

            // A checked toggle, hot: hover fill, the mark, the icon and the text.
            let auto = action(MenuCommand::Automatic(false), true, true);
            let p = Painted::new(&auto, HOT, dpi, &theme);
            assert_eq!(
                p.at(hover_spot.0, hover_spot.1),
                palette.hover,
                "hover at {dpi}"
            );
            assert_eq!(
                p.at(0, hover_spot.1),
                palette.background,
                "the inset keeps the fill"
            );
            assert_eq!(
                p.at(l.inset_x, l.inset_y),
                palette.background,
                "a round corner"
            );
            assert!(
                p.ink(l.mark_left, l.mark_left + l.mark, palette.hover) > 0,
                "check at {dpi}"
            );
            assert!(
                p.ink(l.icon_left, l.icon_left + l.icon, palette.hover) > 0,
                "icon at {dpi}"
            );
            assert!(
                p.ink(l.text_left, l.text_left + l.font * 3, palette.hover) > 0,
                "text at {dpi}"
            );
            assert_eq!(p.clip, None);

            // Unchecked and at rest: no fill and no mark, but the icon stays.
            let off = action(MenuCommand::Automatic(true), false, true);
            let p = Painted::new(&off, MOUSE, dpi, &theme);
            assert_eq!(p.at(hover_spot.0, hover_spot.1), palette.background);
            assert_eq!(
                p.ink(l.mark_left, l.mark_left + l.mark, palette.background),
                0,
                "no mark"
            );
            assert!(p.ink(l.icon_left, l.icon_left + l.icon, palette.background) > 0);

            // A chosen length: the bullet.
            let hour = action(MenuCommand::KeepAwake(KeepAwake::Minutes(60)), true, true);
            let p = Painted::new(&hour, MOUSE, dpi, &theme);
            assert!(
                p.ink(l.mark_left, l.mark_left + l.mark, palette.background) > 0,
                "bullet"
            );

            // The submenu: chevron at the right, and the whole item to clip.
            let sub = MenuItem::Submenu {
                text: text::keep_awake_menu(),
                items: keep_awake(),
            };
            let p = Painted::new(&sub, MOUSE, dpi, &theme);
            let right = p.w - l.right;
            assert!(
                p.ink(right - l.chevron, right, palette.background) > 0,
                "chevron at {dpi}"
            );
            assert_eq!(
                p.clip,
                Some(RECT {
                    left: 0,
                    top: 0,
                    right: p.w,
                    bottom: l.row
                })
            );

            // The separator: one line in the separator colour, the fill around it.
            let p = Painted::new(&MenuItem::Separator, HOT, dpi, &theme);
            for x in [0, p.w / 2, p.w - 1] {
                assert_eq!(p.at(x, l.separator_top), palette.separator, "line at {dpi}");
                assert_eq!(p.at(x, 0), palette.background, "above the line at {dpi}");
                assert_eq!(
                    p.at(x, l.separator - 1),
                    palette.background,
                    "below at {dpi}"
                );
            }
        }
    }
}

#[test]
fn paints_at_the_rect_given() {
    let fonts = Fonts::new(96).expect("fonts");
    let layout = Layout::new(96, BOTH);
    let palette = Palette::for_theme(&DARK);
    let (w, h) = (300, 60);
    let mut canvas = Canvas::new(w, h).expect("canvas");
    canvas.pixels().fill(0x00ff_00ff);
    let rect = RECT {
        left: 3,
        top: 5,
        right: 203,
        bottom: 5 + layout.row,
    };
    let quit = action(MenuCommand::Quit, false, true);
    paint_item(canvas.dc, rect, &quit, MOUSE, &palette, &fonts, &layout).expect("paint");
    let px = canvas.pixels();
    let at = |x: i32, y: i32| px[(y * w + x) as usize];
    assert_eq!(at(2, 10), 0x00ff_00ff, "left of the rect is untouched");
    assert_eq!(at(203, 10), 0x00ff_00ff, "right of the rect is untouched");
    assert_eq!(at(10, 4), 0x00ff_00ff, "above the rect is untouched");
    assert_eq!(at(3, 5), pixel(palette.background));
    assert_eq!(at(202, 5 + layout.row - 1), pixel(palette.background));
}

/// Every kind of item the menus have, so the text, both marks, every icon and the chevron are drawn, plus a
/// disabled item with its mark.
fn every_kind() -> Vec<MenuItem> {
    let mut items = product_menu();
    items.extend(keep_awake());
    items.push(action(MenuCommand::KeepScreenOn(true), true, false));
    items
}

fn spread(c: Rgb) -> u8 {
    c.r.max(c.g).max(c.b) - c.r.min(c.g).min(c.b)
}

#[test]
fn fold_takes_the_mean_coverage() {
    let (light, text) = (Rgb::new(249, 249, 249), Rgb::new(26, 26, 26));
    // A fringe pixel, its subpixels covered 149, 129 and 109 of 223: the mean is 129 of 223.
    assert_eq!(
        fold(Rgb::new(100, 120, 140), light, text),
        Rgb::new(120, 120, 120)
    );
    // Untouched and fully covered stay as they are.
    assert_eq!(fold(light, light, text), light);
    assert_eq!(fold(text, light, text), text);
    // Light on dark the same way round.
    let (dark, white) = (Rgb::new(44, 44, 44), Rgb::new(255, 255, 255));
    assert_eq!(
        fold(Rgb::new(44, 150, 255), dark, white),
        Rgb::new(150, 150, 150)
    );
    // Yellow on black: blue never moves, so only red and green count.
    let (black, yellow) = (Rgb::new(0, 0, 0), Rgb::new(255, 255, 0));
    assert_eq!(
        fold(Rgb::new(200, 100, 0), black, yellow),
        Rgb::new(150, 150, 0)
    );
    // Ink the colour of the fill draws nothing to fold.
    assert_eq!(fold(Rgb::new(1, 2, 3), black, black), Rgb::new(1, 2, 3));
    // Past the ink is the ink.
    assert_eq!(fold(Rgb::new(0, 0, 0), light, text), text);
}

/// Whether `(x, y)` is in the status glyph's box.
fn in_glyph_box(l: &Layout, x: i32, y: i32) -> bool {
    let (left, top) = glyph_origin(l);
    (left..left + l.icon).contains(&x) && (top..top + l.icon).contains(&y)
}

#[test]
fn text_and_glyphs_draw_grayscale() {
    // Windows' own XAML menus draw grayscale (chroma 0 in Explorer's menu text); ClearType's fringes turned the
    // grey check red-brown. Light and dark are greys only, so any spread between R, G and B is a fringe. The status
    // glyph's Moss light is the one colour, by design; its box is checked on its own
    // (`status_row_draws_the_tray_glyph`).
    for dpi in [96, 120, 144] {
        for theme in [LIGHT, DARK] {
            for item in every_kind() {
                let tray = matches!(icon_for(&item), Some(Icon::Tray(_)));
                for state in [MOUSE, HOT] {
                    let p = Painted::new(&item, state, dpi, &theme);
                    let worst = (0..p.px.len() as i32)
                        .map(|i| (i % p.w, i / p.w))
                        .filter(|&(x, y)| !(tray && in_glyph_box(&p.layout, x, y)))
                        .map(|(x, y)| spread(p.at(x, y)))
                        .max()
                        .unwrap_or(0);
                    assert!(
                        worst <= 2,
                        "chroma {worst} at {dpi} dpi, light {}: {item:?}",
                        theme.light
                    );
                }
            }
            // Smoothed, not aliased: an aliased text would also have no chroma, but only two colours.
            let quit = action(MenuCommand::Quit, false, true);
            let p = Painted::new(&quit, MOUSE, dpi, &theme);
            let l = p.layout;
            let mut greys: Vec<u8> = (l.line_top..l.line_top + l.line)
                .flat_map(|y| (l.text_left..l.text_left + l.font * 2).map(move |x| (x, y)))
                .map(|(x, y)| p.at(x, y).g)
                .collect();
            greys.sort_unstable();
            greys.dedup();
            assert!(greys.len() > 4, "{} greys at {dpi} dpi", greys.len());
        }
    }
}

/// How far `c` is from the nearest blend of `from` and `to`, in its worst channel.
fn off_blend(c: Rgb, from: Rgb, to: Rgb) -> u8 {
    let d = |a: u8, b: u8| f32::from(b) - f32::from(a);
    let (dr, dg, db) = (d(from.r, to.r), d(from.g, to.g), d(from.b, to.b));
    let along = d(from.r, c.r) * dr + d(from.g, c.g) * dg + d(from.b, c.b) * db;
    let t = (along / (dr * dr + dg * dg + db * db)).clamp(0.0, 1.0);
    let q = lerp(from, to, t);
    [(c.r, q.r), (c.g, q.g), (c.b, q.b)]
        .into_iter()
        .map(|(a, b)| a.abs_diff(b))
        .max()
        .unwrap_or(0)
}

#[test]
fn high_contrast_text_has_no_fringes() {
    // Coloured system colours, so a fringe shows as R apart from G; each one blends with black, so a gamma-correct
    // blend stays on the line between the two as well.
    let c = Contrast {
        window: Rgb::new(0, 0, 0),
        window_text: Rgb::new(0xff, 0xff, 0xff),
        highlight: Rgb::new(0x1a, 0xeb, 0xff),
        highlight_text: Rgb::new(0, 0, 0),
        gray_text: Rgb::new(0x3f, 0xf2, 0x3f),
        button_face: Rgb::new(0, 0, 0),
        button_text: Rgb::new(0xff, 0xff, 0),
        hot_light: Rgb::new(0xff, 0xff, 0),
    };
    let theme = Theme {
        light: false,
        contrast: Some(c),
    };
    for dpi in [96, 144] {
        for item in every_kind() {
            if matches!(item, MenuItem::Separator) {
                continue;
            }
            let enabled = !matches!(item, MenuItem::Action { enabled: false, .. });
            for state in [MOUSE, HOT] {
                let hot = state.selected && enabled && !matches!(item, MenuItem::Label { .. });
                // In high contrast every ink of an item is one colour.
                let (fill, ink) = if hot {
                    (c.highlight, c.highlight_text)
                } else if matches!(item, MenuItem::Label { .. }) {
                    (c.window, c.window_text)
                } else if enabled {
                    (c.window, c.button_text)
                } else {
                    (c.window, c.gray_text)
                };
                let p = Painted::new(&item, state, dpi, &theme);
                let l = p.layout;
                // Inside the hover shape, clear of its round corners.
                let (x0, x1) = (l.inset_x + l.radius, p.w - l.inset_x - l.radius);
                let worst = (l.line_top..l.line_top + l.line)
                    .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                    .map(|(x, y)| off_blend(p.at(x, y), fill, ink))
                    .max()
                    .unwrap_or(0);
                assert!(worst <= 2, "{worst} off the blend at {dpi} dpi: {item:?}");
                assert!(p.ink(x0, x1, fill) > 0, "drawn at {dpi} dpi: {item:?}");
            }
        }
    }
}

/// Three hold groups and Keep awake: wider than the text cap.
fn long_menu() -> Vec<MenuItem> {
    product_menu_holding(&[
        text::hold_group(HoldKind::Agent, "Claude Code", 2),
        text::hold_group(HoldKind::Agent, "Codex", 1),
        text::hold_group(HoldKind::Run, "cargo build --release", 1),
        text::keep_awake_group(KeepAwake::Minutes(85)),
    ])
}

#[test]
fn a_long_status_line_ends_in_its_column() {
    // "Holding awake" in Armenian, which draws with a fallback face, made long.
    let armenian = "\u{54a}\u{561}\u{570}\u{578}\u{582}\u{574} \u{567} \u{561}\u{580}\u{569}\u{578}\u{582}\u{576}, "
        .repeat(8);
    let english = long_menu().swap_remove(0);
    for label in [english, label(&armenian, Some(IconState::Holding))] {
        for dpi in [96, 120, 144] {
            let fonts = Fonts::new(dpi).expect("fonts");
            let layout = Layout::new(dpi, BOTH);
            // What Windows draws: the width asked plus what it adds.
            let w = measure(&label, &fonts, &layout).0 as i32 + fonts.windows_adds();
            assert_eq!(
                w,
                layout.width_for(scale(TEXT_MAX, dpi), false),
                "capped at {dpi} dpi"
            );
            let p = Painted::sized(&label, MOUSE, dpi, &DARK, w);
            let end = w - layout.right;
            let bg = p.palette.background;
            assert_eq!(p.ink(end, w, bg), 0, "nothing past the text at {dpi} dpi");
            assert!(
                p.ink(end - layout.font, end, bg) > 0,
                "the ellipsis reaches the end at {dpi} dpi"
            );
        }
    }
}

#[test]
fn non_latin_labels_get_a_font_that_has_them() {
    let fonts = Fonts::new(96).expect("fonts");
    // "Holding awake" in Armenian and in Russian, as escapes so the source stays ASCII.
    let armenian = "\u{54a}\u{561}\u{570}\u{578}\u{582}\u{574} \u{567} \u{561}\u{580}\u{569}\u{578}\u{582}\u{576}";
    let russian =
        "\u{41d}\u{435} \u{434}\u{430}\u{451}\u{442} \u{443}\u{441}\u{43d}\u{443}\u{442}\u{44c}";
    for label in [armenian, russian] {
        let shown = visible(label);
        let font = fonts.font_for(&shown);
        println!(
            "{label:?}: text font {:?} misses {}; drawn with {:?}",
            fonts.text_face(),
            fonts.missing(fonts.text, &shown),
            fonts.face_of(font)
        );
        assert_eq!(
            fonts.missing(font, &shown),
            0,
            "{label:?} drawn with {:?}",
            fonts.face_of(font)
        );
        if fonts.missing(fonts.text, &shown) > 0 {
            assert_ne!(font, fonts.text, "a font that has them, not the text font");
        }
    }
    // Latin stays on the text font.
    assert_eq!(fonts.font_for(&visible("&Quit")), fonts.text);
    // Measure and paint take the same font, so an Armenian label is as wide as its glyphs.
    let layout = Layout::new(96, BOTH);
    let label = label(armenian, Some(IconState::Holding));
    let (w, _) = measure(&label, &fonts, &layout);
    assert!(w > 60, "measured {w}");
}

#[test]
fn measure_asks_less_what_windows_adds() {
    for dpi in [96, 120, 144] {
        let fonts = Fonts::new(dpi).expect("fonts");
        let layout = Layout::new(dpi, BOTH);
        let quit = action(MenuCommand::Quit, false, true);
        let text_px = fonts.text_width(fonts.text, "&Quit", true);
        let (w, h) = measure(&quit, &fonts, &layout);
        println!(
            "dpi {dpi}: Windows adds {} px, faces {:?} and {:?}",
            fonts.windows_adds(),
            fonts.text_face(),
            fonts.glyph_face()
        );
        assert_eq!(
            i64::from(w),
            i64::from(layout.width_for(text_px, false) - fonts.windows_adds())
        );
        assert_eq!(i64::from(h), i64::from(layout.row));
        assert_eq!(
            measure(&MenuItem::Separator, &fonts, &layout),
            (0, layout.separator.unsigned_abs())
        );
    }
}

#[test]
fn a_long_label_is_capped() {
    let fonts = Fonts::new(96).expect("fonts");
    let layout = Layout::new(96, BOTH);
    let long = label(&"Holding awake: ".repeat(20), Some(IconState::Holding));
    let (w, _) = measure(&long, &fonts, &layout);
    let cap = layout.width_for(scale(TEXT_MAX, 96), false) - fonts.windows_adds();
    assert_eq!(i64::from(w), i64::from(cap));
}

/// The main menu as the core builds it in `icon`: holding for two Claude Code sessions and Keep awake with 25
/// minutes left, idle with Automatic on, off with Automatic off.
fn built_menu(icon: IconState) -> Vec<MenuItem> {
    use wakeshot_core::present::{Holder, Snapshot, build};
    use wakeshot_core::process::{FileTime, ProcessRef};
    use wakeshot_core::time::Now;
    let now = Now {
        wall_ms: 1_767_225_600_000,
        mono_ms: 0,
    };
    let holding = icon == IconState::Holding;
    let agent = Holder {
        kind: HoldKind::Agent,
        label: Some("Claude Code".into()),
        expires_wall_ms: None,
    };
    let keep_awake = Holder {
        kind: HoldKind::KeepAwake,
        label: None,
        expires_wall_ms: Some(now.wall_ms + 25 * 60_000),
    };
    let snapshot = Snapshot {
        automatic: icon != IconState::Off,
        holds: if holding {
            vec![agent.clone(), agent, keep_awake]
        } else {
            vec![]
        },
        keep_awake: if holding {
            KeepAwake::Minutes(30)
        } else {
            KeepAwake::Off
        },
        screen_on: false,
        requests_on: holding,
        requests_only: false,
        stepped_back: false,
        overridden: vec![],
        tray: ProcessRef {
            pid: 1,
            start: FileTime(0),
        },
    };
    let (view, _) = build(&snapshot, now);
    assert_eq!(view.icon, icon);
    view.menu.items
}

/// Writes the review renders as PNG into `WAKESHOT_RENDER_DIR`: the product menu and its Keep awake submenu side
/// by side, at 96, 120 and 144 dpi, light, dark and high contrast (Night sky), the four contrast themes at 120 dpi
/// (Aquatic, Desert, Dusk, Night sky, top to bottom), the menu with a long status line at 144 dpi, light, and the
/// status glyph sheet: the main menu holding, idle and off side by side, a row each for dark and light at 100% and
/// at 150%. Run: `cargo test -p wakeshot-windows renders_for_review -- --ignored`.
#[test]
#[ignore = "writes pictures for review"]
fn renders_for_review() {
    let Some(dir) = std::env::var_os("WAKESHOT_RENDER_DIR") else {
        println!("set WAKESHOT_RENDER_DIR");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    let main = product_menu();
    let sub = keep_awake();
    // A real contrast theme rather than this PC's classic colours, which draw the status line and the commands in
    // one black and so cannot show the difference.
    for dpi in [96u32, 120, 144] {
        for (name, theme) in [("light", LIGHT), ("dark", DARK), ("contrast", NIGHT_SKY)] {
            // The main menu with Keep awake open, and in the submenu "2 hours" under the mouse.
            let file = dir.join(format!("menu-render-{dpi}-{name}.png"));
            render(
                &[(&main, Some(3)), (&sub, Some(3))],
                dpi,
                &theme,
                MOUSE.keyboard_cues,
                &file,
            );
        }
        // Scratch check: an Armenian status line and keyboard cues.
        let armenian = vec![
            label(
                "\u{54a}\u{561}\u{570}\u{578}\u{582}\u{574} \u{567} \u{561}\u{580}\u{569}\u{578}\u{582}\u{576}: Claude Code (2 sessions)",
                Some(IconState::Holding),
            ),
            MenuItem::Separator,
            action(MenuCommand::Automatic(false), true, true),
            action(MenuCommand::Quit, false, true),
        ];
        let file = dir.join(format!("menu-check-armenian-{dpi}.png"));
        render(&[(&armenian, Some(2))], dpi, &DARK, true, &file);
    }
    // 144 dpi, as the review's screen pictures (150%), where a long line made the menu 853 px wide.
    let file = dir.join("menu-render-long-144-light.png");
    render(&[(&long_menu(), None)], 144, &LIGHT, false, &file);

    let themes: Vec<_> = CONTRAST_THEMES
        .iter()
        .map(|(_, theme)| paint_menus(&[(&main, Some(3)), (&sub, Some(3))], 120, theme, false))
        .collect();
    let file = dir.join("menu-render-themes-contrast.png");
    let sheet = stack(&themes, scale(16, 120));
    std::fs::write(&file, crate::tray::art::png(&sheet)).expect("write");
    println!("{}: {} x {}", file.display(), sheet.width, sheet.height);

    let menus = STATES.map(built_menu);
    let row: Vec<(&Vec<MenuItem>, Option<usize>)> = menus.iter().map(|m| (m, None)).collect();
    let mut rows = Vec::new();
    for dpi in [96, 144] {
        for theme in [DARK, LIGHT] {
            rows.push(paint_menus(&row, dpi, &theme, false));
        }
    }
    let file = dir.join("menu-render-status-glyph.png");
    let sheet = stack(&rows, scale(16, 144));
    std::fs::write(&file, crate::tray::art::png(&sheet)).expect("write");
    println!("{}: {} x {}", file.display(), sheet.width, sheet.height);
}

fn render(
    menus: &[(&Vec<MenuItem>, Option<usize>)],
    dpi: u32,
    theme: &Theme,
    cues: bool,
    file: &std::path::Path,
) {
    let image = paint_menus(menus, dpi, theme, cues);
    std::fs::write(file, crate::tray::art::png(&image)).expect("write");
    println!("{}", file.display());
}

/// Rows one under another on the renders' grey, left aligned, `gap` apart.
fn stack(rows: &[crate::tray::art::Rgba], gap: i32) -> crate::tray::art::Rgba {
    let gap = gap.unsigned_abs();
    let width = rows.iter().map(|r| r.width).max().unwrap_or(0);
    let height =
        rows.iter().map(|r| r.height).sum::<u32>() + gap * (rows.len() as u32).saturating_sub(1);
    let mut pixels = [0x80, 0x80, 0x80, 0xff].repeat((width * height) as usize);
    let mut top = 0;
    for r in rows {
        for y in 0..r.height {
            let from = (y * r.width * 4) as usize;
            let to = (((top + y) * width) * 4) as usize;
            pixels[to..to + (r.width * 4) as usize]
                .copy_from_slice(&r.pixels[from..from + (r.width * 4) as usize]);
        }
        top += r.height + gap;
    }
    crate::tray::art::Rgba {
        width,
        height,
        pixels,
    }
}

/// Menus side by side on the renders' grey, each item as Windows sizes it, `hot` the item under the mouse.
fn paint_menus(
    menus: &[(&Vec<MenuItem>, Option<usize>)],
    dpi: u32,
    theme: &Theme,
    cues: bool,
) -> crate::tray::art::Rgba {
    let fonts = Fonts::new(dpi).expect("fonts");
    let palette = Palette::for_theme(theme);
    let gap = scale(8, dpi);
    let sizes: Vec<(Layout, i32, i32)> = menus
        .iter()
        .map(|(items, _)| {
            let layout = Layout::new(dpi, Columns::of(items));
            // What Windows draws: the width asked plus what it adds.
            let width = items
                .iter()
                .map(|i| measure(i, &fonts, &layout).0 as i32 + fonts.windows_adds())
                .max()
                .unwrap_or(0);
            let height = items
                .iter()
                .map(|i| measure(i, &fonts, &layout).1 as i32)
                .sum();
            (layout, width, height)
        })
        .collect();
    let w: i32 = sizes.iter().map(|s| s.1).sum::<i32>() + gap * (menus.len() as i32 - 1);
    let h = sizes.iter().map(|s| s.2).max().unwrap_or(0);
    let mut canvas = Canvas::new(w, h).expect("canvas");
    canvas.pixels().fill(0x0080_8080);
    let mut x = 0;
    for ((items, hot), (layout, width, _)) in menus.iter().zip(&sizes) {
        let mut y = 0;
        for (i, item) in items.iter().enumerate() {
            let height = measure(item, &fonts, layout).1 as i32;
            let rect = RECT {
                left: x,
                top: y,
                right: x + width,
                bottom: y + height,
            };
            let state = ItemState {
                selected: *hot == Some(i),
                keyboard_cues: cues,
            };
            paint_item(canvas.dc, rect, item, state, &palette, &fonts, layout).expect("paint");
            y += height;
        }
        x += width + gap;
    }
    println!(
        "{dpi} dpi: {w} x {h}, row {} px, font {} px, marks {} px, icons {} px, faces {:?} {:?}",
        sizes[0].0.row,
        sizes[0].0.font,
        sizes[0].0.mark,
        sizes[0].0.icon,
        fonts.text_face(),
        fonts.glyph_face()
    );
    crate::tray::art::Rgba {
        width: w.unsigned_abs(),
        height: h.unsigned_abs(),
        pixels: canvas
            .pixels()
            .iter()
            .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8, 0xff])
            .collect(),
    }
}
