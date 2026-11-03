use std::path::Path;
#[cfg(feature = "screenshots")]
use std::{fs::File, io::BufWriter};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
            CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits,
            ReleaseDC, SRCCOPY, SelectObject,
        },
        System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::{
            EnumThreadWindows, GetClassNameW, GetSystemMetrics, GetWindowRect, IsWindowVisible,
            SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        },
    },
    core::BOOL,
};

/// Visible popup menu windows (class `#32768`) of this thread. TrackPopupMenu creates them on the caller's thread.
pub fn menu_windows() -> Vec<(HWND, RECT)> {
    unsafe extern "system" fn cb(hwnd: HWND, data: LPARAM) -> BOOL {
        let list = unsafe { &mut *(data.0 as *mut Vec<(HWND, RECT)>) };
        let mut class = [0u16; 32];
        let n = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
        if String::from_utf16_lossy(&class[..n]) == "#32768"
            && unsafe { IsWindowVisible(hwnd) }.as_bool()
        {
            let mut r = RECT::default();
            if unsafe { GetWindowRect(hwnd, &mut r) }.is_ok() {
                list.push((hwnd, r));
            }
        }
        BOOL(1)
    }
    let mut list: Vec<(HWND, RECT)> = Vec::new();
    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(cb),
            LPARAM(&mut list as *mut Vec<(HWND, RECT)> as isize),
        );
    }
    list
}

/// Captures the bounding box of the given rects. Pixels outside every rect turn flat grey, so the screenshot
/// shows only our own UI and nothing else that was on Haik's screen.
pub fn capture_union(rects: &[RECT], margin: i32, path: &Path) -> Result<(i32, i32), String> {
    let mut r = *rects.first().ok_or("no rect")?;
    for x in rects {
        r.left = r.left.min(x.left);
        r.top = r.top.min(x.top);
        r.right = r.right.max(x.right);
        r.bottom = r.bottom.max(x.bottom);
    }
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    r.left = (r.left - margin).max(vx);
    r.top = (r.top - margin).max(vy);
    r.right = (r.right + margin).min(vx + vw);
    r.bottom = (r.bottom + margin).min(vy + vh);
    capture_rect(r, rects, path)
}

fn capture_rect(r: RECT, keep: &[RECT], path: &Path) -> Result<(i32, i32), String> {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w <= 0 || h <= 0 {
        return Err(format!("empty rect {r:?}"));
    }
    let mut bgra = vec![0u8; (w * h * 4) as usize];
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        // CAPTUREBLT includes layered windows, which is what menus and their shadows are.
        let blt = BitBlt(
            mem,
            0,
            0,
            w,
            h,
            Some(screen),
            r.left,
            r.top,
            SRCCOPY | CAPTUREBLT,
        );
        SelectObject(mem, old);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(bgra.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        blt.map_err(|e| format!("BitBlt: {e}"))?;
        if lines != h {
            return Err(format!("GetDIBits returned {lines}"));
        }
    }
    let mut rgba = bgra;
    for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let (x, y) = (r.left + i as i32 % w, r.top + i as i32 / w);
        let inside = keep
            .iter()
            .any(|k| x >= k.left && x < k.right && y >= k.top && y < k.bottom);
        if inside {
            px.swap(0, 2);
            px[3] = 255;
        } else {
            *px = [0x80, 0x80, 0x80, 255];
        }
    }
    write_png(path, w as u32, h as u32, &rgba)?;
    Ok((w, h))
}

#[cfg(feature = "screenshots")]
pub fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}

#[cfg(not(feature = "screenshots"))]
pub fn write_png(_: &Path, _: u32, _: u32, _: &[u8]) -> Result<(), String> {
    Err("built without the screenshots feature".into())
}
