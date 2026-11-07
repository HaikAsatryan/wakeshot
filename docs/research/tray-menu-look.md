# Tray menu look

Desk research, 2026-11-03, then an on-screen check the same day ("Side by side on screen"). The desk part used no
screen: no menus opened, no apps run, no screenshots.

Short answer: our menu is the classic Win32 menu. It does have Windows 11 rounded corners, but the small 4 px kind
that Windows gives menus; current XAML menus use 8 px, a 14 px font instead of 12 px, rows about 33 px apart instead
of 21, and an acrylic background. Pick: keep the classic menu loop, round it with one DWM call, and owner-draw the
items (details in "Pick"). Values marked "not verified" or "est." were not checked on screen.

## Our menu today

Measured 2026-11-03 from `docs/research/img/tray-menu-switch3-light-flush.png` (light) and
`docs/research/img/tray-menu-switch4-dark-flush.png` (dark), with a stdlib-only Python PNG reader (throwaway
scripts, not kept). Values are physical pixels in the picture.

Scale of the pictures: 125% (120 dpi). Reason: the pictures were taken at 13:01 to 13:02 while only the laptop screen
(1920x1200 at 125%) was on (`docs/research.md`, "Live checks"), and the capital "A" of "Automatic" is 11 px tall. The
menu font is Segoe UI 9 pt (registry read below), which is 15 px at 120 dpi; Segoe UI caps are about 0.7 of that,
so 10.5 px. At 150% they would be about 12.6 px. So "at 100%" values below are the picture value divided by 1.25.

How the capture works (a throwaway tray test program of that day, not kept): `BitBlt` of the screen with
`CAPTUREBLT`, then every pixel
outside the `#32768` window rect is painted flat grey `808080`. So **the drop shadow is cut away in every picture**:
whatever Windows draws outside the window rect is not in our pictures. Pixels inside the rect but outside the rounded
corner show what was behind the menu.

| Property | Light | Dark | At 100% |
| --- | --- | --- | --- |
| Window size | 377 x 325 | 377 x 325 | about 302 x 260 |
| Corners | Rounded, anti-aliased arc from (5,0) to (0,5) in every corner | Same | radius about 4 px |
| Border | 1 px `e5e5e5` | 1 px `3e3e3e` | 1 px |
| Shadow | Not in the pictures (masked, see above) | Same | unknown from pictures |
| Background | `f9f9f9` | frame `2b2b2b`, item rows `2c2c2c` | |
| Inner padding | 2 px above the first item and below the last | Same (`2b2b2b` rows 1 to 2) | about 2 px |
| Item pitch | 26 px (label tops 48, 74; 111, 137, 163, 189, 215; 252, 278, 304) | Same | about 21 px |
| Separator slot | 11 px (26 + 11 = 37 from label to label across a separator) | Same | about 9 px |
| Separator line | 1 px `d7d7d7`, x 11 to 365 (inset 11 px each side) | 1 px `3e3e3e`, x 0 to 376 (edge to edge, same colour as the border) | 1 px |
| Font | Segoe UI 9 pt: cap height 11 px | Same | 12 px font |
| Text colour | `000000` (pure black) | `ffffff` (pure white) | |
| Text smoothing | ClearType: colour fringes `b16300` and `8bd5f9` at glyph edges | ClearType: `9d552c`, `7abfff` | |
| Disabled text | `9f9f9f` on `f9f9f9` | `6d6d6d` on `2c2c2c` | |
| Label left edge | x 45 | x 45 | 36 px |
| Check mark | box x 15 to 24, y 50 to 56 (10 x 7) | Same | about 8 x 6 |
| Submenu arrow | chevron x 352 to 356, y 74 to 83 (5 x 10), right edge 20 px from the window edge | x 351 to 356 | about 4 x 8 |
| Space right of the longest label | 69 px (label ends at x 308) | Same | about 55 px |
| Hover colour | Not in the pictures (no item was hot) | Same | |

Menu font check: `reg query "HKCU\Control Panel\Desktop\WindowMetrics"` (read only), `MenuFont` is a `LOGFONTW` with
`lfHeight` `0xFFFFFFEE` (-18), weight 400, face "Segoe UI", saved at `AppliedDPI` `0x90` (144). -18 px at 144 dpi is
9 pt. This is what `SPI_GETNONCLIENTMETRICS` `lfMenuFont` returns, which classic menus use (not verified by a call:
read from the registry values that back it).

The corner radius matches DWM's small rounded corner (4 px at 100%, so 5 px at 125%). So our menu does have Windows 11
rounded corners; they are the small kind. See "Differences" for why it may still read as square.

## What Windows 11 says a modern menu looks like

Sources read 2026-11-03.

- **Corner radius.** "Geometry in Windows 11" (learn.microsoft.com/windows/apps/design/signature-experiences/geometry,
  updated 2026-02-19): "Transient UI elements ... use an 8px corner radius. This includes elements like ContentDialog,
  Flyout, MenuFlyout, and TeachingTip. ToolTip is an exception, using 4px". In-page controls use 4 px.
- **Classic menus get the small radius.** "Apply rounded corners in desktop apps"
  (learn.microsoft.com/windows/apps/desktop/modernize/ui/apply-rounded-corners, updated 2026-07-09), Example 4: for a
  custom menu window, call `DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL)` "using the
  small radius because menus are auxiliary UI", "to follow the rounding policy of other standard menus". The page gives
  no pixel values for `DWMWCP_ROUND` and `DWMWCP_ROUNDSMALL`. Our measured 4 px (at 100%) matches the small kind; that
  `DWMWCP_ROUND` is 8 px is inferred from the Geometry page, not stated (not verified).
- **DWM attributes** (learn.microsoft.com/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute, updated 2026-03-09):
  `DWMWA_WINDOW_CORNER_PREFERENCE` (33) and `DWMWA_BORDER_COLOR` from build 22000; `DWMWA_SYSTEMBACKDROP_TYPE` from
  build 22621. `DWMSBT_TRANSIENTWINDOW` is "Desktop Acrylic ... in its brightest variant"
  (learn.microsoft.com/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type). The apply-rounded-corners page
  also says per-pixel alpha layering and window regions make a window impossible to round.
- **Font.** "Typography in Windows" (learn.microsoft.com/windows/apps/design/signature-experiences/typography, updated
  2026-07-14): Windows 11 UI font is Segoe UI Variable; Body is 14/20 epx (size/line height), Caption 12/16. The same
  page lists Segoe UI (not Segoe UI Variable) as the UI font for Armenian, and Segoe UI Variable covers "Latin, Greek,
  and Cyrillic" only. Wakeshot is English only today (`product.md`); if Armenian is ever added, a self-drawn menu
  must keep font fallback working.
- **WinUI 3 `MenuFlyout` template** (github.com/microsoft/microsoft-ui-xaml,
  `controls/dev/CommonStyles/MenuFlyout_themeresources.xaml`, last commit 2026-05-30, read with `gh api`):
  - Presenter: `CornerRadius` = `OverlayCornerRadius` (8), border 1 px `SurfaceStrokeColorFlyout`, padding `0,2,0,2`,
    `SystemBackdrop` = `AcrylicBackgroundFillColorDefaultBackdrop`, background `DesktopAcrylicTransparentBrush`.
  - Item: margin `4,2,4,2`, corner radius `ControlCornerRadius` (4), font size `ControlContentThemeFontSize` (14 in
    `DensityStyles/Compact.xaml`; the default value is in a file the code search did not index, not verified there,
    but it matches the 14 px Body size above). Padding `11,8,11,9` for touch and `11,4,11,5` "when flyout was invoked
    with pen, mouse or keyboard".
  - Item hover `SubtleFillColorSecondary`, pressed `SubtleFillColorTertiary`, text `TextFillColorPrimary`, disabled
    `TextFillColorDisabled`, separator `DividerStrokeColorDefault`, 1 px high, padding `-4,1,-4,1`.
  - Check mark glyph `E73E`, submenu chevron `E974`, both 12 px in `SymbolThemeFontFamily` (Segoe Fluent Icons).
  - So a mouse-opened item is about 4 + 20 + 5 = 29 epx tall plus 4 epx of margins: about 33 epx from item to item
    (computed from the values, not measured). Ours is about 21.
- **Colours** (`controls/dev/CommonStyles/Common_themeresources_any.xaml`, same repo):

  | Key | Light | Dark |
  | --- | --- | --- |
  | `TextFillColorPrimary` | `#E4000000` | `#FFFFFF` |
  | `TextFillColorDisabled` | `#5C000000` | `#5DFFFFFF` |
  | `SubtleFillColorSecondary` (hover) | `#09000000` | `#0FFFFFFF` |
  | `SubtleFillColorTertiary` (pressed) | `#06000000` | `#0AFFFFFF` |
  | `SurfaceStrokeColorFlyout` (border) | `#0F000000` | `#33000000` |
  | `DividerStrokeColorDefault` (separator) | `#0F000000` | `#15FFFFFF` |

## Why it looks like that

Read 2026-11-03 in the code of that day's throwaway tray test program (not kept): its menu building and showing, its
theme code, its manifest and its build script.

- **Classic menu, drawn by Windows.** `CreatePopupMenu` plus `AppendMenuW` with `MF_STRING`, `MF_GRAYED` (status line),
  `MF_CHECKED` (Automatic), `MF_POPUP` (Keep awake), `MF_SEPARATOR`, and `CheckMenuRadioItem` for the radio bullet in
  the submenu. No `MF_OWNERDRAW`, no `SetMenuInfo`, no item bitmaps. So user32 draws every pixel inside the `#32768`
  window: font, item height, check mark, arrow, separator and colours are Windows' classic menu defaults.
- **Shown with** `SetForegroundWindow`, then `TrackPopupMenuEx` with `TPM_RIGHTALIGN` or `TPM_LEFTALIGN` (from
  `SM_MENUDROPALIGNMENT`), `TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY`, then `PostMessage
  WM_NULL`. None of these flags change the look. `TPM_NOANIMATION` is not set, so the user's menu animation applies.
- **Font.** Classic menus use the system menu font (`lfMenuFont`): Segoe UI 9 pt here (registry read above). The test
  program sets no font. Not verified that user32 reads exactly `lfMenuFont` on Windows 11 (the measured cap height
  matches it).
- **Dark mode.** `theme.rs` loads undocumented uxtheme ordinals: 135 `SetPreferredAppMode(AllowDark)` once at start,
  before the first menu, then 104 and 136 (`RefreshImmersiveColorPolicyState`, `FlushMenuThemes`). That is what makes
  the menu dark; there is no documented API for dark classic menus (not verified that none exists: no Microsoft page
  found that documents one).
- **Corners.** The test program makes no DWM call on the menu window. The 4 px rounding comes from Windows' own policy
  for standard menus (the apply-rounded-corners page above, and our measurement).
- **Manifest** (embedded as `RT_MANIFEST` 1 by `build.rs`): Common Controls 6 dependency, `supportedOS` Windows 10/11
  GUID, per-monitor v2 DPI (`true/pm` plus `PerMonitorV2`), `asInvoker`. Common Controls 6 matters for themed common
  controls, not for menus (menus are user32). PMv2 is why the menu is drawn at the monitor's 120 dpi.
- **Shadow.** The test program asks for nothing. What Windows draws is outside our pictures (see "How the capture
  works").

## What the named apps use

Checked 2026-11-03, read only: installed files (dumpbin `/imports`, a byte-string search script,
`AppxManifest.xml`), `Get-AppxPackage`, the `NotifyIconSettings` registry list, and the apps' public
source on GitHub where it exists. No app was run.

| App | Version here | What draws its menu | Evidence |
| --- | --- | --- | --- |
| Telegram | 7.3.3 (`%APPDATA%\Telegram Desktop\Telegram.exe`) | Its own Qt widget `Ui::PopupMenu` (lib_ui): a frameless translucent window that paints its own rounded rect and shadow | Exe (250 MB) has about 20,800 `QWidget` strings and no Qt DLL sits next to it (only `modules\x64\d3d`), so Qt is linked in. `tdesktop` `Telegram/SourceFiles/platform/win/tray_win.cpp` (last commit 2026-09-24): on `contextMenuRequested` it calls `_menu->popup(position)` on a `Ui::PopupMenu`. `lib_ui` `ui/widgets/popup_menu.cpp`: `Qt::NoDropShadowWindowHint`, `WA_TranslucentBackground`, `_roundRect(_st.radius, ...)`, `_boxShadow(_st.shadow)`. Its `DWMWA_WINDOW_CORNER_PREFERENCE` call is in `ui_window_win.cpp` for main windows, not menus. |
| Wi-Fi quick settings (left-click on the network icon) | `ShellExperienceHost` 10.0.26100.8115 | System XAML (`Windows.UI.Xaml`) with WinUI 2 controls | `Windows.UI.QuickActions.dll` in `C:\Windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy` has 112 `Windows.UI.Xaml` and 34 `Microsoft.UI.Xaml.Controls` type names and `MenuFlyout`, `MenuFlyoutItem`, `ContextMenuItem_ManageQuickActions` strings. The package manifest depends on `Microsoft.UI.Xaml.CBS` (WinUI 2 for inbox apps). It is a flyout panel, not a menu; its own right-click menu is a `MenuFlyout`. |
| Network icon right-click menu (taskbar) | `MicrosoftWindows.Client.Core` 1000.26100.134 | System XAML `MenuFlyout`, drawn by the taskbar (hosted in `explorer.exe`: not verified) | `SystemTray.dll` and `Taskbar.View.dll` in `MicrosoftWindows.Client.Core_cw5n1h2txyewy` have `Windows.UI.Xaml` types, `MenuFlyout`, `MenuFlyoutItem`, `MenuFlyoutSeparator`, and `ContextMenu*` strings. Which DLL builds the network icon's menu: not verified. |
| Windows Security (tray icon) | `SecurityHealthSystray.exe` 10.0.26100.8115 | Classic Win32 menu from a menu resource, same kind as ours | `SecurityHealthSystray.exe` imports no tray or menu function (only window basics and `CoCreateInstance`). `SecurityHealthSSO.dll` imports `Shell_NotifyIconW`, `LoadMenuW`, `GetSubMenu`, `TrackPopupMenuEx`, `DeleteMenu`, `DestroyMenu`, and has no `uxtheme`, `dwmapi`, dark-mode or `MenuFlyout` strings. That the exe loads this DLL through COM is inferred, not verified. |
| Chrome | 156.0.8078.80 | Chrome's own Views menu (Skia-drawn), rounded bubble border | Chromium `chrome/browser/ui/views/status_icons/status_icon_win.cc` (last commit 2026-10-16): `views::MenuRunner(...)->RunMenuAt(...)`. `ui/views/controls/menu/menu_config_win.cc`: font = system menu font (`SystemFont::kMenu`, the same 9 pt font as ours), `use_bubble_border = corner_radius > 0`. `menu_config.cc`: `item_vertical_margin = 6`. Radius token `kMenuRadius` maps to `kMediumSmall` = 12 px (`chrome_layout_provider.cc`, `layout_provider.cc`). The tray icon shows only while Chrome runs in the background (not verified here; `chrome.exe` does have a `NotifyIconSettings` entry on the test laptop). |
| Copilot | No `Microsoft.Copilot` package here. The Copilot tray entry is `M365Copilot.exe` in `Microsoft.MicrosoftOfficeHub` 19.2610.35011 | WinUI 3 `MenuFlyout` through a XAML island, most likely | The package ships `Microsoft.UI.Xaml.dll` (14.4 MB), `Microsoft.UI.Xaml.Controls.dll` (6.3 MB), `Microsoft.WinUI.dll`, the Windows App SDK runtime, .NET (`coreclr.dll`) and WebView2. `M365Copilot.exe` (23 MB) has `Shell_NotifyIcon NIM_ADD`, `TaskbarCreated`, `SystemTrayContextMenuEnabled`, `DesktopWindowXamlSource`, and `winrt::Microsoft::UI::Xaml::Controls::MenuFlyoutItem` / `MenuFlyoutSubItem` / `MenuFlyoutSeparator` strings. It also imports `TrackPopupMenu`. Which one the tray right-click uses: not verified. The consumer Copilot app: not installed here, not checked. |
| Explorer right-click (Windows reference) | `MicrosoftWindows.Client.FileExp` 1000.26100.10 | XAML ("the XAML context menu"); "Show more options" opens the old Win32 menu | Windows Insider blog, build 23403 (2023-03-08): "We are adding access key shortcuts into the XAML context menu in File Explorer." Windows Developer blog (2021-07-19): "'Show more options' loads the Windows 10 context menu as-is". `FileExplorerExtensions.dll` is WinUI 3 (189 `Microsoft.UI.Xaml` names, no `Windows.UI.Xaml`) and has `ContextMenuFlyout`, `MenuFlyoutControl.cpp`, `MenuFlyoutItem`; that it draws the file right-click menu is not verified. |

So of the six apps named for comparison, five draw their own menu or a XAML one (Telegram, Wi-Fi quick settings, Chrome,
Copilot, Explorer) and one uses the same classic Win32 menu as ours (Windows Security). The old Explorer menu behind
"Show more options" is also the classic one: a good side-by-side reference for "same as ours".

Also found: `windows-reactor` 0.100.0 (Microsoft, in the `windows-rs` repo, `crates/libs/reactor`, last commit
2026-10-30), the "WinUI library for Rust" that `product.md` names, already has a tray icon (`native/notifyicon`,
handles the shell restart) and a tray menu (`native/transient_menu.rs`): a WinUI 3 `MenuFlyout` shown from a 2 x 2 px
`DesktopWindowXamlSource` host window, themed from `SystemUsesLightTheme`. Its menu model has plain items,
separators and submenus only: no check or radio items yet. `native/bootstrap.rs` needs the
`Microsoft.WindowsAppRuntime.2` framework package, minimum 2.6.4, and shows an install dialog if it is missing.

## Differences and what each costs

"Modern" below means the WinUI 3 `MenuFlyout` values above, because Explorer, the taskbar, quick settings and Copilot
all use a XAML `MenuFlyout` (system XAML and WinUI 3 share the Windows 11 look; that the system XAML copy uses the
same numbers is not verified). Ours is measured; "epx" is the value at 100%. Windows 10 risk: none, the product is
Windows 11 only (`product.md`, "Windows 11 only"). Every new API below exists on all Windows 11 builds unless a build
is named.

| # | Difference | Ours | Modern | How the others get it | How we could get it, and the cost |
| --- | --- | --- | --- | --- | --- |
| 1 | Corner radius | 4 epx (5 px at 125%) | 8 epx (Geometry page); Chrome 12; Telegram its own | XAML `OverlayCornerRadius`; Chrome and Telegram paint it | `DwmSetWindowAttribute(menu window, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND)`. The menu window handle comes in `WM_ENTERIDLE` `lParam` when `wParam` is `MSGF_MENU` (learn.microsoft.com/windows/win32/dlgbox/wm-enteridle), or from `EnumThreadWindows`. About 30 lines, feature `Win32_Graphics_Dwm`, no crate, no runtime cost. **Not verified** that DWM honours it on a `#32768` window, nor that it applies before the first frame (`WM_ENTERIDLE` comes once the menu is idle, so small corners may flash). |
| 2 | Font | Segoe UI 9 pt = 12 epx (system menu font) | Segoe UI Variable 14 epx (Body 14/20) | XAML `ControlContentThemeFontSize`. Chrome keeps the system menu font, same as ours | Classic items cannot change font: it is the user's system-wide menu font. Only owner-drawn items pick their own font ("Using Menus", owner-drawn section: the item data "might contain ... a handle to the logical font"). Segoe UI Variable is installed (`C:\Windows\Fonts\SegUIVar.ttf`, registry font list). |
| 3 | Item height | 26 px pitch at 125% = about 21 epx | about 33 epx with mouse (padding 4/5 + 20 line + margins 2/2), about 41 with touch | XAML item padding and margin | Owner-drawn: we set `itemHeight` in `WM_MEASUREITEM`. Classic: no setting (not verified that none exists; item bitmaps can make rows taller, a hack). |
| 4 | Hover and pressed | Not in our pictures | Rounded 4 epx rect, inset 4 epx, `#09000000` / `#0FFFFFFF` | XAML visual states | Owner-drawn: draw it on `ODS_SELECTED`. Classic: whatever the theme draws (to check on screen). |
| 5 | Material | Opaque `f9f9f9` / `2b2b2b` | Acrylic (`AcrylicBackgroundFillColorDefaultBackdrop`) | XAML `SystemBackdrop` | `DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_TRANSIENTWINDOW` (build 22621+) shows only where the window paints nothing opaque. A classic or owner-drawn menu paints its own background, so: own popup window or XAML only (inferred from the DWM docs, not verified). |
| 6 | Border | 1 px `e5e5e5` / `3e3e3e` (lighter than the dark fill) | 1 px `#0F000000` / `#33000000` (darker than the dark fill) | XAML presenter border | `DWMWA_BORDER_COLOR` on the menu window may recolour it; **not verified** that DWM draws the border of a menu window. Small. |
| 7 | Shadow | Unknown: cut away by the capture | Flyout shadow (size not looked up) | XAML; Chrome and Telegram paint their own | First see it on screen (below). |
| 8 | Text colour | `000000` / `ffffff` | `1a1a1a` effective (`#E4000000` on `f9f9f9`) / `ffffff` | XAML tokens | Owner-drawn: any colour. Tiny. |
| 9 | Text smoothing | ClearType colour fringes | Not checked (XAML popups often grey-scale; not verified) | Renderer | Owner-drawn GDI: `lfQuality` picks it. Tiny. |
| 10 | Check mark | System glyph, about 8 x 6 epx, label at 36 epx | `E73E`, 12 epx, Segoe Fluent Icons | XAML `FontIcon` | Owner-drawn: draw `E73E` with Segoe Fluent Icons (`SegoeIcons.ttf` is installed). |
| 11 | Submenu arrow | System chevron about 4 x 8 epx, 16 epx from the edge | `E974`, 12 epx | XAML `FontIcon` | Owner-drawn: draw `E974`. Windows may still draw its own arrow on an owner-drawn popup item (widely reported, not verified on this build); to check. |
| 12 | Separator | Light: 1 px `d7d7d7`, inset 11 px. Dark: 1 px `3e3e3e` edge to edge, touching the border | 1 px `#0F000000` / `#15FFFFFF` | XAML | Owner-drawn separator item (`MFT_OWNERDRAW` with `MFT_SEPARATOR`, not verified that it is drawn by us). |
| 13 | Disabled line | `9f9f9f` on `f9f9f9` (2.5:1), `6d6d6d` on `2c2c2c` (2.7:1), WCAG formula | Light gives the same `9f9f9f`; dark `797979` | XAML `TextFillColorDisabled` | Not a look gap: Windows itself greys it this much. Whether the status line should be a disabled item is wording. |
| 14 | Icons in items | None | Optional `Icon` per item | XAML | Owner-drawn can draw them. Which items get icons is a product call; out of scope. |
| 15 | High contrast | System colours, automatic | XAML high-contrast dictionary (`SystemColor*` brushes in `MenuFlyout_themeresources.xaml`) | Built in | Owner-drawn must use `GetSysColor` (`COLOR_MENU`, `COLOR_MENUTEXT`, `COLOR_HIGHLIGHT`, `COLOR_GRAYTEXT`) when high contrast is on. About 20 lines. |
| 16 | Keyboard and screen readers | Built in | Built in | | Owner-drawn: mnemonics need `WM_MENUCHAR` ("Using Menus": "your application must process the WM_MENUCHAR message"); screen readers need `MSAAMENUINFO` as the first field of the item data (learn.microsoft.com/windows/win32/api/oleacc/ns-oleacc-msaamenuinfo). Own popup: we would build all of it, including a UI Automation provider. |

What most likely reads as "old": items 2 and 3 (small font, cramped rows) and item 1 (4 versus 8 px), with 5
(flat versus acrylic) as a smaller part. This is a judgment from the numbers; the side-by-side look below settles it.

## Options

Sizes and line counts marked "est." are estimates, not measured. "Tray checks" means the tray's three checks:
`TaskbarCreated` re-add, theme switch without restart, Medium token.

| Option | Fixes (numbers from the table above) | Code | Exe size and dependencies | Startup and memory | Main risk | Tray checks |
| --- | --- | --- | --- | --- | --- | --- |
| A. Classic menu plus `DWMWCP_ROUND` (and maybe `DWMWA_BORDER_COLOR`) on the menu window | 1, maybe 6 | About 30 lines | About +1 KB est.; `dwmapi` is a system DLL; feature `Win32_Graphics_Dwm`; no crate | None | DWM may ignore it on a menu window; small corners may flash first | All kept: tray window and theme path unchanged, no elevation |
| B. Owner-drawn items in the classic menu, plus A | 1 to 4, 6, 8 to 12; keeps 13 to 16 with extra work | 300 to 500 lines est.: measure, draw, `WM_MENUCHAR`, `MSAAMENUINFO`, high-contrast colours | +10 to 30 KB est.; GDI and `dwmapi` only; no crate, no runtime | Fonts made per menu open; no measurable cost expected (not measured) | Windows drawing its own arrow over ours; high contrast cannot be tested on the test laptop | All kept: the menu is built on each open, so colours follow the theme at the next open; `TaskbarCreated` handling does not change; no elevation |
| C. Our own popup window (Direct2D and DirectWrite, DWM corners, acrylic backdrop) | All, including 5 | 1,500 to 3,000 lines est.: focus, light dismiss, keyboard, submenus, placement at any taskbar edge, per-monitor DPI, UI Automation provider, animation | +50 to 150 KB est.; system DLLs only | A Direct2D device per open, a few MB (not measured) | Accessibility and focus bugs that Windows' menu loop already solves | Kept if built right; all is ours to get right |
| D1. WinUI 3 `MenuFlyout` in a XAML island (`windows-reactor`, or our own `DesktopWindowXamlSource`) | All: it is the same control the shell and, most likely, Copilot use | Small for the menu itself, but `windows-reactor`'s menu has no check or radio items yet, and its tray menu code is internal (`pub(super)`), so we would adopt its app model or write our own island | Needs `Microsoft.WindowsAppRuntime.2` 2.6.4+ installed (here: 119.5 MB, 301 files on disk) or self-contained (in the runtime here, `Microsoft.ui.xaml.dll` alone is 14.5 MB and `Microsoft.UI.Xaml.Controls.dll` 7.1 MB; MB = 1024 x 1024 bytes) | Microsoft, self-contained unpackaged: "Slower to load, and uses more memory since code pages aren't shared with other apps" (learn.microsoft.com/windows/apps/package-and-deploy/deploy-overview, updated 2026-10-02). Not measured here | A runtime the installer must add, kept serviced, for x64 and ARM64; `windows-reactor` is 0.100.0 | `TaskbarCreated`: `windows-reactor` re-adds on shell restart (its `notifyicon` comment). Theme: it reads `SystemUsesLightTheme` and refreshes. Medium: fine. Not tested by us |
| D2. System XAML island (`Windows.UI.Xaml`, in Windows) | Unclear: shell parts get the Windows 11 look from WinUI 2 (`Microsoft.UI.Xaml.CBS`); without it the in-box styles may be the older look (not verified) | Like D1 | No runtime to install | XAML in process (not measured) | WinUI 2 for unpackaged apps needs package identity (not verified) | Not checked |
| E. WebView2 page as a menu | All, by hand in HTML | Like C plus a web page | WebView2 runtime plus browser helper processes | Highest of all (not measured) | Same as C, plus a browser for a menu | Not checked |

## Pick

**B, in two steps: A first, then owner-drawn items.**

Reason: B fixes what most likely reads as old (font size, row height, corner radius, glyphs, colours) and keeps
Windows' own menu loop, so keyboard use, mnemonics (with `WM_MENUCHAR`), light dismiss, placement, submenus and
screen readers (with `MSAAMENUINFO`) stay Windows' work. Cost: about 300 to 500 lines in `wakeshot-windows`, about 10
to 30 KB of exe (est.), no new crate, no runtime, no extra process. It keeps all three tray checks: the tray window
and `TaskbarCreated` path do not change, colours are read at each menu open so a theme switch shows at the next open
without a restart, and nothing needs elevation. What B gives up: the acrylic backdrop (item 5). A alone is a 30-line
try that tells us whether DWM will round a menu window at 8 px; B then builds on it.

One more design point that the pictures should settle: Windows' own tray menus are drawn by the taskbar, and
`windows-reactor` themes its tray menu from `SystemUsesLightTheme` (the taskbar mode). Our test program followed
`AppsUseLightTheme` (app mode, through `AllowDark`). With "Windows mode dark, app mode light" (a common mix), our menu
would be light next to dark taskbar menus. With owner-drawn items we choose the colours, so either rule is one line.

**Change it when:** move to D1 (WinUI 3 `MenuFlyout`) when one of these is true:

- Wakeshot needs a real window (for example a settings page), so the WinUI runtime cost is paid anyway.
- `windows-reactor` reaches a stable release, its tray menu supports check and radio items, and our installer can add
  the Windows App Runtime without a prompt.
- A side-by-side look shows that the flat background (no acrylic) is the gap that still matters after B.

## Side by side on screen

2026-11-03 15:47 to 16:12, on the test laptop: only the laptop screen (1920x1200
at 125%), on battery. Every app's menu was opened with a synthetic right-click on its tray icon (left-click for quick
settings) and closed with Esc; no item of another app was clicked. The Explorer menus come from a scratch folder with
one dummy file; the classic one from Shift+right-click. Our menu and the other tray menus were shot on a plain
backdrop window of ours (dark `202020`, light `f3f3f3`), so the shadow shows and nothing else of the screen does.
Private bits are greyed: the Wi-Fi name, a Chrome extension name, and quick settings' media panel is cut away.

Pictures in `docs/research/img/`, each in `dark`, `light` and (where listed) `custom` (Windows mode dark, app mode
light):

- Ours: `tray-look-<theme>-ours-classic.png` (the first menu), `tray-look-<theme>-ours-owner.png` (the pick, with
  the submenu open by keyboard), `...-ours-owner-without-dwm.png`, `...-ours-owner-forced-hc.png`.
- Reference: `tray-look-<theme>-explorer-new.png` and `...-explorer-classic.png` (the best single pair: Windows' own
  new menu and Windows' own classic menu). These two keep only the pixels that changed when the menu opened, so in
  `tray-look-light-explorer-new.png` the menu pixels that matched the white window behind are grey: some icons and the
  edge are missing there. The dark one is complete.
- Tray menus: `tray-look-<theme>-network-menu.png`, `...-quick-settings.png`, `...-security.png`, `...-chrome.png`,
  `...-copilot.png`, `...-telegram.png`.
- 100% and 150% without a screen: `tray-look-render-96dpi-<light|dark|hc>.png` and `...-144dpi-...` (items only, drawn
  by the test program's render mode with the same code; no frame).

### Measured (dark, 125%; epx = px / 1.25)

Measured with a throwaway stdlib script on the raw pictures: corner
radius from how far the edge is indented on the top rows, border and fill from pixels, item pitch from the tops of
the text lines, shadow from the difference to the known backdrop. "About" values were read from the picture by eye.

| Menu | What draws it | Corner | Border (dark) | Item pitch | Text | Shadow |
| --- | --- | --- | --- | --- | --- | --- |
| Ours, classic (the first menu) | Win32 menu | 4 epx | `3e3e3e`, lighter than the fill | 26 px = 21 epx | Segoe UI 9 pt (12 epx) | faint (3 levels below) |
| Ours, classic + `DWMWCP_ROUND` (option A) | Win32 menu | 8 epx | `3e3e3e` | 21 epx | 12 epx | DWM shadow (7 levels below, 12 px to the left) |
| **Ours, owner-drawn + `DWMWCP_ROUND` (pick B)** | Win32 menu loop, our drawing | 8 epx | `414141` (DWM's) | 41 px = 33 epx | Segoe UI Variable Text 14 epx | DWM shadow (18 levels below, 12 px left) |
| Ours, owner-drawn, no DWM call | same | square | `a0a0a0`, light grey | 33 epx | 14 epx | small, below only |
| Explorer, new menu | XAML | about 8 epx | dark, thin | 40 px = 32 epx | about 14 epx, icons, shortcut text | yes |
| Network icon, right-click | system XAML | about 8 epx | `1a1a1a`, darker than the fill `2d2d2d` | 44 px = 35 epx | about 14 epx, icons | yes |
| Explorer, classic (Shift+right-click) | Win32 menu | small | light | 30 px = 24 epx | 12 epx, access keys underlined | yes |
| Windows Security | Win32 menu | small | none seen | 40 px = 32 epx (taller than ours; why not checked) | 12 epx | strong below (10 levels) |
| Copilot (`M365Copilot.exe`) | own (icons, XAML look) | small | `3e3e3e` | 32 px = 26 epx | about 13 epx, icons | light |
| Chrome | Chrome Views | about 12 epx | none | about 35 px = 28 epx | 12 epx | yes |
| Telegram | Qt, own widget | about 8 to 10 epx | none | 48 px = 38 epx | own, larger | own, faint |

Our owner-drawn menu next to Explorer's new menu: same corner, same font and size, item pitch 33 versus 32 epx, the
same check, radio and chevron glyphs (Segoe Fluent Icons), rounded 4 epx hover inset 4 epx. What still differs: no
acrylic (flat fill), no icons in items (a product call), no pressed colour (owner-drawn items get no pressed state),
DWM's border `414141` instead of the darker XAML border (`DWMWA_BORDER_COLOR` set to the WinUI token `232323` works on
the menu window: tested in dark, raw picture only), and GDI ClearType text instead of XAML's text rendering (not
compared closely).

### Which mode the tray menus follow

In Custom mode (Windows mode dark, app mode light; set and put back through Settings), 16:08 to 16:10: the network
icon's menu and Windows Security's menu were dark, Copilot's and Telegram's were light. Ours: the owner-drawn menu
was dark (it reads `SystemUsesLightTheme`), the classic `AllowDark` menu was light. So Windows' own tray menus follow
the Windows mode, which is product decision 3; and `AllowDark` follows the app mode, now tested apart (pictures
`tray-look-custom-*`).

### How the pick behaves (on screen)

- `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND)` works on the `#32768` menu window, for the
  classic and the owner-drawn menu alike.
- Owner-drawn items without that call lose Windows' rounding: square corners and a light `a0a0a0` border. So B needs
  the DWM call.
- When to call it: at `HCBT_CREATEWND` DWM answers "handle is invalid"; a `WH_CALLWNDPROCRET` hook never sees the menu
  window's `WM_CREATE` (most likely its window procedure runs in the kernel: not verified). It works at the first
  `WM_DRAWITEM` (window from `WindowFromDC`) and at `WM_ENTERIDLE` (about 20 ms later). Each menu open creates a new
  menu window. Whether one frame shows square corners before the call: not measured; no picture showed it.
- Owner-drawn separators (`MFT_OWNERDRAW | MFT_SEPARATOR`) get `WM_MEASUREITEM` and `WM_DRAWITEM`.
- Windows adds 16 px (at 120 dpi) to the width asked in `WM_MEASUREITEM` (391 asked, 407 drawn).
- Windows draws no arrow of its own over ours once the item is clipped out with `ExcludeClipRect`: one chevron only.
- Keyboard on a real right-click: Down moves through the items (it also stops on the disabled status line, which is
  drawn hot), Right opens the submenu, Left closes it, Enter returns the command (`1001`), Esc closes. The mnemonic
  `k` arrives as `WM_MENUCHAR`; returning `MNC_EXECUTE` for the popup item opens the submenu. With `TPM_RETURNCMD`
  and without `TPM_NONOTIFY`, no `WM_COMMAND` came.
- Screen readers: `AccessibleObjectFromWindow` on the menu window gives every item's name from `MSAAMENUINFO` ("Keep
  awake", the `&` removed) and the states (checked, unavailable, has popup); separators have the separator role.
- Theme switch without restart: after Settings switched to Light, the next menu was light; the tray icon changed too.
- `TaskbarCreated`: after the icon was removed by a test message, posting the registered `TaskbarCreated` message
  re-added it (`NIM_ADD` TRUE). Simulated: no real Explorer restart in this test. Token: Medium in every run.
- DPI: on screen only 125% was possible (the 150% DELL was not connected, and the scale is the user's own setting). At
  125%: items 41 px, font 18 px, glyphs 15 px. The render at 96 dpi gives 33, 14 and 12 px, the WinUI values; at 144 dpi
  50, 21 and 18 px. The frame corners at 100% were not seen on screen.
- High contrast: the forced branch draws with `GetSysColor` (`COLOR_WINDOW`, `COLOR_BTNTEXT`, `COLOR_HIGHLIGHT`,
  `COLOR_HIGHLIGHTTEXT`, `COLOR_GRAYTEXT`, `COLOR_WINDOWTEXT`, the MenuFlyout high contrast mapping); pictures
  `...-ours-owner-forced-hc.png` and the `hc` renders. A real high contrast switch was not tested (the user's own theme
  stays).
- Cost: after about 40 menu opens the test program had 17.2 MB working set and 2.7 MB private (the first classic menu:
  16 to 19 MB and 2.0 to 3.3 MB). Its exe grew from 519,168 to 676,864 bytes (x64; ARM64 648,192), but that includes
  test-only code (render, backdrop, capture messages); the menu code alone was not measured. The first build at 15:45
  was 580,608 bytes; why later builds are bigger was not checked.

Pick stays **B** (owner-drawn items plus `DWMWCP_ROUND` at the first paint), with the same "change it when" triggers.

## Not checked on screen

Checked on screen on 2026-11-03 ("Side by side on screen" above), except: 100% scale on screen, a real high contrast
switch, a real Explorer restart. Chrome's menu appears only while Chrome runs in the background with a tray icon (it
did on the test laptop).
