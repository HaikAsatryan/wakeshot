use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use windows::Win32::System::SystemInformation::GetLocalTime;

struct Logger {
    file: Option<File>,
    start: Instant,
}

static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

pub fn init(path: Option<&Path>) {
    let file = path.map(|p| {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .expect("open log file")
    });
    let _ = LOGGER.set(Mutex::new(Logger {
        file,
        start: Instant::now(),
    }));
}

pub fn wall_clock() -> String {
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

pub fn line(tag: &str, msg: &str) {
    let Some(lock) = LOGGER.get() else {
        println!("[{tag}] {msg}");
        return;
    };
    let mut l = lock.lock().unwrap();
    let ms = l.start.elapsed().as_millis();
    let text = format!("{} +{:>7}ms [{tag}] {msg}", wall_clock(), ms);
    println!("{text}");
    if let Some(f) = l.file.as_mut() {
        // Flush every line: a test can end in sleep or a kill, and the log is the only record.
        let _ = writeln!(f, "{text}");
        let _ = f.flush();
    }
}

#[macro_export]
macro_rules! log {
    ($tag:expr, $($arg:tt)*) => {
        $crate::log::line($tag, &format!($($arg)*))
    };
}
