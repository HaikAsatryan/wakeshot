use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::Instant,
};

use windows::Win32::System::SystemInformation::GetLocalTime;

static START: OnceLock<Instant> = OnceLock::new();
// Toast events arrive on WinRT threads, so the file sits behind a mutex.
static FILE: Mutex<Option<File>> = Mutex::new(None);

/// The spike writes next to its sources so the next worker finds the results in the repo.
pub fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("out")
}

pub fn img_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("research")
        .join("img")
}

pub fn path(name: &str) -> PathBuf {
    out_dir().join(name)
}

pub fn init(name: &str) {
    START.get_or_init(Instant::now);
    let _ = std::fs::create_dir_all(out_dir());
    let _ = std::fs::create_dir_all(img_dir());
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path(name))
        .ok();
    *FILE.lock().unwrap() = file;
    let t = unsafe { GetLocalTime() };
    line(&format!(
        "==== start {:04}-{:02}-{:02} {:02}:{:02}:{:02} args {:?}",
        t.wYear,
        t.wMonth,
        t.wDay,
        t.wHour,
        t.wMinute,
        t.wSecond,
        std::env::args().skip(1).collect::<Vec<_>>()
    ));
}

pub fn line(msg: &str) {
    let ms = START.get().map(|s| s.elapsed().as_millis()).unwrap_or(0);
    if let Some(f) = FILE.lock().unwrap().as_mut() {
        let _ = writeln!(f, "[{ms:>7} ms] {msg}");
        let _ = f.flush();
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::log::line(&format!($($arg)*)) };
}
