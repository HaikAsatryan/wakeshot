//! What a bug report needs (architecture section 8, "Log"): the facts of this PC, which the tray logs at each start
//! and the bundle repeats, and the diagnostics bundle `wakeshot diagnostics` writes. The bundle is one text file with
//! Wakeshot's own files only, found by name (an allow list), with user names in paths and anything that looks like a
//! token taken out. It never reads another app's files, so no prompt can reach it.
//!
//! Owner: `p6-logs-diagnostics`.

use std::ffi::{OsString, c_void};
use std::fs;
use std::io::Write;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use wakeshot_core::settings::Guid;
use windows::Win32::Foundation::{ERROR_FILE_EXISTS, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Power::{
    CallNtPowerInformation, SYSTEM_POWER_CAPABILITIES, SystemPowerCapabilities,
};
use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY,
    RegGetValueW,
};
use windows::Win32::System::SystemInformation::{
    IMAGE_FILE_MACHINE, IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_ARM64, IMAGE_FILE_MACHINE_I386,
};
use windows::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};
use windows::Win32::UI::Shell::{FOLDERID_Desktop, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::PCWSTR;

use crate::WinError;
use crate::agent_hooks;
use crate::log::{self, LocalTime};
use crate::power::plan::{self, PlanSetting};
use crate::power::system;
use crate::process;
use crate::store::{DataDir, DataFile};

/// One fact of this PC: a short name and its value in plain words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    pub name: &'static str,
    pub value: String,
}

const WINDOWS_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const CPU_KEY: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
const BIOS_KEY: &str = r"HARDWARE\DESCRIPTION\System\BIOS";

/// Windows 11 keeps major version 10; its first build was 22000 (Microsoft's Windows 11 release information).
const FIRST_WINDOWS_11_BUILD: u32 = 22_000;

/// The facts of this PC now. Reads only, each on its own: one that does not read says so, and the rest still show.
/// No fact names the user: the registry values read here describe the PC and Windows, never its owner.
pub fn machine_facts() -> Vec<Fact> {
    let caps = power_capabilities();
    vec![
        fact(
            "Wakeshot",
            format!(
                "{}, {} build",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::ARCH
            ),
        ),
        fact("Windows", windows_text(&WindowsVersion::read())),
        fact(
            "Processor",
            processor_text(
                reg_text(CPU_KEY, "ProcessorNameString"),
                std::thread::available_parallelism().ok().map(|n| n.get()),
                native_machine(),
            ),
        ),
        fact(
            "PC",
            pc_text(
                reg_text(BIOS_KEY, "SystemManufacturer"),
                reg_text(BIOS_KEY, "SystemFamily"),
                reg_text(BIOS_KEY, "SystemProductName"),
                reg_text(BIOS_KEY, "BIOSVersion"),
            ),
        ),
        fact(
            "Sleep",
            caps.map_or_else(not_read, |caps| sleep_text(&caps)),
        ),
        fact(
            "Power",
            system::power_status().map_or_else(not_read, |(ac, pct)| power_text(ac, pct)),
        ),
        fact(
            "Power plan",
            plan::active_values().map_or_else(not_read, |(plan, values)| plan_text(plan, &values)),
        ),
        fact(
            "Elevated",
            process::current_above_medium().map_or_else(not_read, |above| yes_no(above).to_owned()),
        ),
    ]
}

fn fact(name: &'static str, value: String) -> Fact {
    Fact { name, value }
}

fn not_read(e: WinError) -> String {
    format!("not read ({} failed with {})", e.call, e.code)
}

fn yes_no(yes: bool) -> &'static str {
    if yes { "yes" } else { "no" }
}

/// The version values Windows keeps in `CurrentVersion`, each `None` when it does not read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct WindowsVersion {
    major: Option<u32>,
    minor: Option<u32>,
    build: Option<String>,
    ubr: Option<u32>,
    display: Option<String>,
    edition: Option<String>,
}

impl WindowsVersion {
    fn read() -> Self {
        Self {
            major: reg_dword(WINDOWS_KEY, "CurrentMajorVersionNumber"),
            minor: reg_dword(WINDOWS_KEY, "CurrentMinorVersionNumber"),
            build: reg_text(WINDOWS_KEY, "CurrentBuildNumber"),
            ubr: reg_dword(WINDOWS_KEY, "UBR"),
            display: reg_text(WINDOWS_KEY, "DisplayVersion"),
            edition: reg_text(WINDOWS_KEY, "EditionID"),
        }
    }
}

/// Such as `Windows 11 Professional 25H2, version 10.0.26300.7019`. `ProductName` is not used: it still says
/// Windows 10 on Windows 11.
fn windows_text(v: &WindowsVersion) -> String {
    let build: Option<u32> = v.build.as_deref().and_then(|b| b.parse().ok());
    let mut text = match build {
        Some(b) if b >= FIRST_WINDOWS_11_BUILD => "Windows 11".to_owned(),
        Some(_) => "Windows 10".to_owned(),
        None => "Windows".to_owned(),
    };
    for part in [&v.edition, &v.display].into_iter().flatten() {
        text.push(' ');
        text.push_str(part);
    }
    let num = |n: Option<u32>| n.map_or_else(|| "?".to_owned(), |n| n.to_string());
    text.push_str(&format!(
        ", version {}.{}.{}.{}",
        num(v.major),
        num(v.minor),
        v.build.as_deref().unwrap_or("?"),
        num(v.ubr)
    ));
    text
}

fn processor_text(name: Option<String>, logical: Option<usize>, native: Option<&str>) -> String {
    let mut parts = vec![name.unwrap_or_else(|| "name not read".to_owned())];
    if let Some(n) = logical {
        parts.push(format!("{n} logical processors"));
    }
    if let Some(native) = native {
        parts.push(format!("{native} Windows"));
    }
    parts.join(", ")
}

/// Such as `LENOVO ThinkPad P16s Gen 3 (21FS0000XX), firmware R2CET34W (1.18)`. The family is left out when it is
/// the product name again.
fn pc_text(
    maker: Option<String>,
    family: Option<String>,
    product: Option<String>,
    firmware: Option<String>,
) -> String {
    let family = family.filter(|f| product.as_ref() != Some(f));
    let mut text = [maker, family]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(product) = product {
        if text.is_empty() {
            text = product;
        } else {
            text.push_str(&format!(" ({product})"));
        }
    }
    if text.is_empty() {
        text = "model not read".to_owned();
    }
    if let Some(firmware) = firmware {
        text.push_str(&format!(", firmware {firmware}"));
    }
    text
}

fn sleep_text(caps: &SYSTEM_POWER_CAPABILITIES) -> String {
    format!(
        "Modern Standby {}, S3 {}, hibernate {}, hibernation file {}, lid {}, battery {}",
        yes_no(caps.AoAc),
        yes_no(caps.SystemS3),
        yes_no(caps.SystemS4),
        yes_no(caps.HiberFilePresent),
        yes_no(caps.LidPresent),
        yes_no(caps.SystemBatteriesPresent)
    )
}

fn power_text(on_ac: bool, battery: Option<u8>) -> String {
    let source = if on_ac { "on AC" } else { "on battery" };
    match battery {
        Some(pct) => format!("{source}, battery {pct}%"),
        None => format!("{source}, no battery charge known"),
    }
}

/// The active plan and each value Wakeshot reads from it, with its meaning: lid actions in words, timeouts in
/// seconds (0 is never), the energy saver threshold in percent.
fn plan_text(plan: Guid, values: &[(PlanSetting, Result<u32, WinError>)]) -> String {
    let parts: Vec<String> = values
        .iter()
        .map(|(setting, value)| match value {
            Ok(value) => format!("{} {}", setting.name(), value_words(*setting, *value)),
            Err(e) => format!(
                "{} not read ({} failed with {})",
                setting.name(),
                e.call,
                e.code
            ),
        })
        .collect();
    format!("{plan}: {}", parts.join(", "))
}

fn value_words(setting: PlanSetting, value: u32) -> String {
    match setting {
        PlanSetting::LidAc | PlanSetting::LidDc => {
            let action = match value {
                0 => "Do nothing",
                1 => "Sleep",
                2 => "Hibernate",
                3 => "Shut down",
                _ => "unknown",
            };
            format!("{value} ({action})")
        }
        PlanSetting::SaverThresholdDc => format!("{value}%"),
        _ if value == 0 => "0 (never)".to_owned(),
        _ => format!("{value} s"),
    }
}

fn power_capabilities() -> Result<SYSTEM_POWER_CAPABILITIES, WinError> {
    let mut caps = SYSTEM_POWER_CAPABILITIES::default();
    // SAFETY: the output buffer is a live local of exactly the size passed; no input buffer.
    let status = unsafe {
        CallNtPowerInformation(
            SystemPowerCapabilities,
            None,
            0,
            Some((&raw mut caps).cast::<c_void>()),
            size_of::<SYSTEM_POWER_CAPABILITIES>() as u32,
        )
    };
    match status.0 {
        0 => Ok(caps),
        code => Err(WinError {
            call: "CallNtPowerInformation",
            code: code as u32,
        }),
    }
}

/// The machine Windows runs on, which an x64 build on an ARM64 PC (emulated) does not show in its own arch.
fn native_machine() -> Option<&'static str> {
    let mut process = IMAGE_FILE_MACHINE(0);
    let mut native = IMAGE_FILE_MACHINE(0);
    // SAFETY: the pseudo handle needs no close; both out pointers are live locals.
    unsafe { IsWow64Process2(GetCurrentProcess(), &mut process, Some(&mut native)) }.ok()?;
    Some(match native {
        IMAGE_FILE_MACHINE_ARM64 => "ARM64",
        IMAGE_FILE_MACHINE_AMD64 => "x64",
        IMAGE_FILE_MACHINE_I386 => "x86",
        _ => "unknown",
    })
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// A `REG_SZ` from `HKEY_LOCAL_MACHINE`, in the 64-bit view whatever this build is; `None` when it does not read or
/// is empty.
fn reg_text(key: &str, name: &str) -> Option<String> {
    let (key, name) = (wide(key), wide(name));
    let mut buf = [0u16; 512];
    let mut size = u32::try_from(size_of_val(&buf)).ok()?;
    // SAFETY: the names are NUL-terminated and live for the call; the buffer and its size in bytes are live locals.
    let rc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            REG_ROUTINE_FLAGS(RRF_RT_REG_SZ.0 | RRF_SUBKEY_WOW6464KEY.0),
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if rc != ERROR_SUCCESS {
        return None;
    }
    let len = (size as usize / size_of::<u16>()).min(buf.len());
    let text = String::from_utf16_lossy(&buf[..len]);
    let text = text.trim_end_matches('\0').trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn reg_dword(key: &str, name: &str) -> Option<u32> {
    let (key, name) = (wide(key), wide(name));
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: as in `reg_text`, with a four-byte value.
    let rc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            REG_ROUTINE_FLAGS(RRF_RT_REG_DWORD.0 | RRF_SUBKEY_WOW6464KEY.0),
            None,
            Some((&raw mut value).cast()),
            Some(&mut size),
        )
    };
    (rc == ERROR_SUCCESS).then_some(value)
}

/// The user's Desktop folder from its known folder id, so a Desktop that OneDrive moved is found too.
pub fn desktop() -> Result<PathBuf, WinError> {
    // SAFETY: the folder id is a static; no token means the calling user.
    let path =
        unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None) }.map_err(|e| {
            WinError {
                call: "SHGetKnownFolderPath",
                code: WIN32_ERROR::from_error(&e).map_or(e.code().0 as u32, |w| w.0),
            }
        })?;
    // SAFETY: on success the shell gives a NUL-terminated string it allocated; it is copied once, then freed once.
    let copy = unsafe { OsString::from_wide(path.as_wide()) };
    unsafe { CoTaskMemFree(Some(path.0 as *const _)) };
    Ok(PathBuf::from(copy))
}

/// What the bundle takes out of every text it holds (`apply`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Redact {
    /// The profile folder, such as `C:\Users\haik`; shown as `%USERPROFILE%`.
    profile: Option<String>,
    /// User names, shown as `<user>` where one is a whole path segment.
    users: Vec<String>,
}

/// What a secret's start looks like in the tokens of common services: Anthropic and OpenAI (`sk-`), GitHub, GitLab,
/// Slack, AWS, Google, and a JWT's header. A word that starts so and is at least 16 characters long is taken out.
const TOKEN_STARTS: [&str; 16] = [
    "sk-",
    "sk_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "xoxa-",
    "xoxs-",
    "AKIA",
    "AIza",
    "eyJ",
];

/// A word of letters and digits this long, with both in it and not a GUID, counts as a token too: Wakeshot's own
/// files hold no such word, and a key with a prefix this module does not know usually looks so.
const TOKEN_MIN: usize = 32;

const TOKEN_SHOWN: &str = "[token removed]";

impl Redact {
    /// This user: the profile folder Windows gives (`std::env::home_dir`), its last part, and `USERNAME`.
    pub fn this_user() -> Self {
        let profile = std::env::home_dir().map(|p| p.display().to_string());
        let folder_name = std::env::home_dir()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
        let user = std::env::var("USERNAME").ok();
        Self::new(
            profile.as_deref(),
            [folder_name, user].into_iter().flatten(),
        )
    }

    pub fn new(profile: Option<&str>, users: impl IntoIterator<Item = String>) -> Self {
        let mut users: Vec<String> = users
            .into_iter()
            .filter(|u| u.chars().count() > 1)
            .collect();
        users.sort();
        users.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        Self {
            profile: profile
                .map(|p| p.trim_end_matches(['\\', '/']).to_owned())
                .filter(|p| !p.is_empty()),
            users,
        }
    }

    /// `text` with the profile folder as `%USERPROFILE%`, any `Users\<name>` path piece and any user name that is a
    /// whole path segment as `<user>`, and every token-like word as `[token removed]`. Paths match in any case, with
    /// `\`, `/` or JSON's `\\` between their parts.
    pub fn apply(&self, text: &str) -> String {
        let profiles: Vec<String> = self
            .profile
            .iter()
            .flat_map(|p| [p.clone(), p.replace('\\', "\\\\"), p.replace('\\', "/")])
            .collect();
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        // A user name counts only as a whole segment right after a path separator, so a word in prose stays.
        let mut after_path_sep = false;
        while at < text.len() {
            let rest = &text[at..];
            if let Some(len) = profiles.iter().find_map(|p| whole_match(rest, p)) {
                out.push_str("%USERPROFILE%");
                at += len;
                after_path_sep = false;
                continue;
            }
            if let Some((sep, len)) = users_piece(rest) {
                out.push_str(sep);
                out.push_str("Users");
                out.push_str(sep);
                out.push_str("<user>");
                at += len;
                after_path_sep = false;
                continue;
            }
            if after_path_sep
                && let Some(len) = self.users.iter().find_map(|u| whole_match(rest, u))
            {
                out.push_str("<user>");
                at += len;
                after_path_sep = false;
                continue;
            }
            let c = rest.chars().next().unwrap_or_default();
            if is_word_char(c) && !out.chars().next_back().is_some_and(is_word_char) {
                let len = rest.find(|c: char| !is_word_char(c)).unwrap_or(rest.len());
                let word = &rest[..len];
                out.push_str(if is_token(word) { TOKEN_SHOWN } else { word });
                at += len;
                after_path_sep = false;
                continue;
            }
            out.push(c);
            at += c.len_utf8();
            after_path_sep = c == '\\' || c == '/';
        }
        out
    }
}

/// The starts of a `Users` path piece: JSON's escaped `\\` first, so it is not read as two single ones.
const USERS_HEADS: [(&str, &str); 3] = [
    ("\\\\", "\\\\Users\\\\"),
    ("\\", "\\Users\\"),
    ("/", "/Users/"),
];

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn is_separator(c: char) -> bool {
    matches!(
        c,
        '\\' | '/' | '"' | '\'' | ' ' | '\t' | '\r' | '\n' | '=' | ',' | '(' | '['
    )
}

/// Where a path segment ends: a separator, a dot-free end of the text, or a character no user name has.
fn ends_segment(next: Option<char>) -> bool {
    next.is_none_or(|c| is_separator(c) || matches!(c, ')' | ']' | ';' | ':' | '.'))
}

/// The byte length of `needle` at the start of `hay`, compared without case, when a segment ends right after it.
fn whole_match(hay: &str, needle: &str) -> Option<usize> {
    let len = prefix_len(hay, needle)?;
    ends_segment(hay[len..].chars().next()).then_some(len)
}

/// The byte length in `hay` of `needle` at its start, compared character by character without case.
fn prefix_len(hay: &str, needle: &str) -> Option<usize> {
    let mut hay_chars = hay.char_indices();
    for n in needle.chars() {
        let (_, h) = hay_chars.next()?;
        if !h.to_lowercase().eq(n.to_lowercase()) {
            return None;
        }
    }
    Some(hay_chars.next().map_or(hay.len(), |(i, _)| i))
}

/// `\Users\<name>`, `/Users/<name>` or `\\Users\\<name>` at the start of `hay`: the separator and the whole length.
fn users_piece(hay: &str) -> Option<(&'static str, usize)> {
    // Most positions start with neither separator: skip them before any comparison.
    if !hay.starts_with(['\\', '/']) {
        return None;
    }
    for (sep, head) in USERS_HEADS {
        let Some(len) = prefix_len(hay, head) else {
            continue;
        };
        let name = hay[len..]
            .find(|c: char| is_separator(c) || matches!(c, ')' | ']' | ';'))
            .unwrap_or(hay.len() - len);
        if name > 0 {
            return Some((sep, len + name));
        }
    }
    None
}

fn is_token(word: &str) -> bool {
    if word.len() >= 16 && TOKEN_STARTS.iter().any(|start| word.starts_with(start)) {
        return true;
    }
    word.len() >= TOKEN_MIN
        && word.bytes().any(|b| b.is_ascii_alphabetic())
        && word.bytes().any(|b| b.is_ascii_digit())
        && !is_guid(word)
}

/// `8-4-4-4-12` hex digits: a power plan or a session GUID, which a reader needs.
fn is_guid(word: &str) -> bool {
    let parts: Vec<&str> = word.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(part, len)| part.len() == len && part.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The most of one settings or state file the bundle takes: Wakeshot writes them far smaller.
const SMALL_FILE_MAX: usize = 64 * 1024;

/// One written bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    pub path: PathBuf,
    pub bytes: usize,
    /// How many of Wakeshot's files it holds.
    pub files: usize,
}

/// Writes the bundle into `folder` as `wakeshot-diagnostics-<local time>.txt`, never over a file: `header` (the facts
/// the command gathered), the list of Wakeshot's files in `dir`, then each file whole, all through `redact`.
pub fn write_bundle(
    dir: &DataDir,
    folder: &Path,
    header: &[String],
    redact: &Redact,
    made_ms: i64,
) -> Result<Bundle, WinError> {
    let (text, files) = bundle_text(dir, header, redact, made_ms);
    let stem = format!("wakeshot-diagnostics-{}", LocalTime::of(made_ms).compact());
    let mut last = WinError {
        call: "CreateFileW",
        code: ERROR_FILE_EXISTS.0,
    };
    for n in 1..=9 {
        let name = match n {
            1 => format!("{stem}.txt"),
            n => format!("{stem}-{n}.txt"),
        };
        let path = folder.join(name);
        let created = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path);
        match created {
            Ok(mut file) => {
                file.write_all(text.as_bytes())
                    .and_then(|()| file.sync_all())
                    .map_err(|e| io_error("WriteFile", &e))?;
                return Ok(Bundle {
                    path,
                    bytes: text.len(),
                    files,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                last = io_error("CreateFileW", &e);
            }
            Err(e) => return Err(io_error("CreateFileW", &e)),
        }
    }
    Err(last)
}

fn io_error(call: &'static str, e: &std::io::Error) -> WinError {
    WinError {
        call,
        code: e.raw_os_error().map_or(u32::MAX, |c| c as u32),
    }
}

const INTRO: &str = "\
This file holds Wakeshot's own files only: its logs, settings, state and hold files. It never holds another app's
files, so no prompt and no chat. The profile folder shows as %USERPROFILE%, other user names in paths as <user>, and
anything that looks like a token as [token removed]. Read it before you send it if you like.";

/// The bundle's text and how many files it holds.
fn bundle_text(dir: &DataDir, header: &[String], redact: &Redact, made_ms: i64) -> (String, usize) {
    let files = collected(dir);
    let mut text = format!(
        "Wakeshot diagnostics, made {}\n\n{INTRO}\n\n== This PC ==\n",
        LocalTime::of(made_ms).text()
    );
    for line in header {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("\n== Files ==\n");
    if files.is_empty() {
        text.push_str("No Wakeshot file in the data folder.\n");
    }
    for file in &files {
        text.push_str(&format!(
            "{}: {} bytes, last written {}\n",
            file.name,
            file.size,
            file.written_ms.map_or_else(
                || "at an unknown time".to_owned(),
                |ms| LocalTime::of(ms).text()
            )
        ));
    }
    for file in &files {
        text.push_str(&format!("\n== {} ==\n", file.name));
        let mut bytes = match fs::read(&file.path) {
            Ok(bytes) => bytes,
            Err(e) => {
                text.push_str(&format!("(does not read: {e})\n"));
                continue;
            }
        };
        let cut = !file.log && bytes.len() > SMALL_FILE_MAX;
        if cut {
            bytes.truncate(SMALL_FILE_MAX);
        }
        let body = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
        text.push_str(&body);
        if !body.ends_with('\n') {
            text.push('\n');
        }
        if cut {
            text.push_str(&format!("(cut at {SMALL_FILE_MAX} bytes)\n"));
        }
    }
    text.push_str("\n== End ==\n");
    // One pass over the whole text, so a user name or a token in any file or fact is taken out the same way. CRLF,
    // as in the log, for any editor the user opens it in.
    let text = redact.apply(&text).replace('\n', "\r\n");
    (text, files.len())
}

/// One of Wakeshot's files the bundle takes.
#[derive(Clone, Debug)]
struct Collected {
    /// Its path in the data folder, such as `holds\run-4242.json`: no user name.
    name: String,
    path: PathBuf,
    size: u64,
    written_ms: Option<i64>,
    log: bool,
}

/// Wakeshot's files in `dir`, found by name only: the data folder's own files, `holds\*.json`, `agents\<agent>\*.json`
/// and the log files, in that order. A link, a junction or a folder in their place is left out, so a planted link
/// cannot pull another app's file in.
fn collected(dir: &DataDir) -> Vec<Collected> {
    let mut wanted: Vec<(String, PathBuf, bool)> = [
        DataFile::Config,
        DataFile::State,
        DataFile::Journal,
        DataFile::JournalCorrupt,
        DataFile::JournalCorruptSeen,
        DataFile::RestoreRequest,
    ]
    .into_iter()
    .map(|file| dir.file(file))
    .chain(Some(dir.root().join(agent_hooks::RECORD)))
    .filter_map(|path| {
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some((name, path, false))
    })
    .collect();
    for path in json_files(&dir.holds()) {
        wanted.push((format!("holds\\{}", file_name(&path)), path, false));
    }
    let agents = dir.root().join("agents");
    for folder in plain_entries(&agents).into_iter().filter(|p| p.is_dir()) {
        let agent = file_name(&folder);
        for path in json_files(&folder) {
            wanted.push((
                format!("agents\\{agent}\\{}", file_name(&path)),
                path,
                false,
            ));
        }
    }
    for path in log::log_files(&dir.logs()) {
        wanted.push((format!("logs\\{}", file_name(&path)), path, true));
    }
    wanted
        .into_iter()
        .filter_map(|(name, path, log)| {
            let meta = plain(&path)?;
            meta.is_file().then(|| Collected {
                name,
                size: meta.len(),
                written_ms: meta.modified().ok().and_then(|t| {
                    let since = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                    i64::try_from(since.as_millis()).ok()
                }),
                path,
                log,
            })
        })
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The metadata of `path` itself when it is no reparse point (a link or a junction).
fn plain(path: &Path) -> Option<fs::Metadata> {
    let meta = fs::symlink_metadata(path).ok()?;
    (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0).then_some(meta)
}

/// The entries of `folder` that are no reparse points, sorted by name; none when it is one itself.
fn plain_entries(folder: &Path) -> Vec<PathBuf> {
    if plain(folder).is_none() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| plain(p).is_some())
        .collect();
    paths.sort();
    paths
}

fn json_files(folder: &Path) -> Vec<PathBuf> {
    plain_entries(folder)
        .into_iter()
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::clock;
    use crate::log::{Level, Log, LogWriter, Proc};

    /// A fresh folder under the temp folder, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("wakeshot-diag-{tag}-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn junction(link: &Path, target: &Path) {
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let out = Command::new("cmd")
            .raw_arg(format!(
                r#"/d /c mklink /J "{}" "{}""#,
                link.display(),
                target.display()
            ))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }

    fn redact() -> Redact {
        Redact::new(
            Some(r"C:\Users\Planted.User"),
            ["Planted.User".to_owned(), "planteduser".to_owned()],
        )
    }

    #[test]
    fn the_profile_folder_and_user_names_leave_paths() {
        let r = redact();
        let cases = [
            (
                r#"RunOnce value set to "C:\Users\Planted.User\AppData\Local\x.exe" --restore"#,
                r#"RunOnce value set to "%USERPROFILE%\AppData\Local\x.exe" --restore"#,
            ),
            (
                r#"{"path":"C:\\Users\\planted.user\\.claude"}"#,
                r#"{"path":"%USERPROFILE%\\.claude"}"#,
            ),
            ("c:/users/PLANTED.USER/x", "%USERPROFILE%/x"),
            (r"D:\Users\someone.else\code", r"D:\Users\<user>\code"),
            (r"\\server\Users\bob", r"\\server\Users\<user>"),
            (
                r"D:\planteduser\code and D:\Work\planteduser",
                r"D:\<user>\code and D:\Work\<user>",
            ),
            // Not a whole segment, or not in a path: stays.
            (r"D:\planteduser2\x", r"D:\planteduser2\x"),
            (
                "the planteduser word in prose",
                "the planteduser word in prose",
            ),
            (r"C:\Users\Planted.Userx\y", r"C:\Users\<user>\y"),
        ];
        for (text, want) in cases {
            assert_eq!(r.apply(text), want, "{text}");
        }
    }

    #[test]
    fn tokens_go_and_ids_stay() {
        let r = Redact::default();
        let cases = [
            (
                "key sk-ant-api03-AbCdEf0123456789xyz end",
                "key [token removed] end",
            ),
            (
                "ghp_0123456789abcdefABCDEF0123456789abcd",
                "[token removed]",
            ),
            (
                "github_pat_11ABCDEFG0123456789_abcdefghij",
                "[token removed]",
            ),
            (
                "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.c2ln",
                "Bearer [token removed].[token removed].c2ln",
            ),
            ("token=AKIAIOSFODNN7EXAMPLE1", "token=[token removed]"),
            (
                "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0",
                "[token removed]",
            ),
        ];
        for (text, want) in cases {
            assert_eq!(r.apply(text), want, "{text}");
        }
        let kept = [
            "plan 381b4222-f694-41f0-9685-ff5bb260df2e",
            "procStart 134384809570012460",
            "Codex session 3f9a21bc holds: working",
            "Local\\Wakeshot.GuardStop.41472.1",
            "sk-short",
            "an_ordinary_long_identifier_without_any_digits_at_all",
            "wakeshot.2026-11-07T00-12-33.512Z.log",
        ];
        for text in kept {
            assert_eq!(r.apply(text), text);
        }
    }

    #[test]
    fn texts_of_the_facts() {
        let v = WindowsVersion {
            major: Some(10),
            minor: Some(0),
            build: Some("26300".to_owned()),
            ubr: Some(7019),
            display: Some("25H2".to_owned()),
            edition: Some("Professional".to_owned()),
        };
        assert_eq!(
            windows_text(&v),
            "Windows 11 Professional 25H2, version 10.0.26300.7019"
        );
        let old = WindowsVersion {
            build: Some("19045".to_owned()),
            ..WindowsVersion::default()
        };
        assert_eq!(windows_text(&old), "Windows 10, version ?.?.19045.?");
        assert_eq!(
            windows_text(&WindowsVersion::default()),
            "Windows, version ?.?.?.?"
        );
        assert_eq!(
            pc_text(
                Some("LENOVO".into()),
                Some("ThinkPad P16s Gen 3".into()),
                Some("21FS0000XX".into()),
                Some("R2CET34W (1.18)".into())
            ),
            "LENOVO ThinkPad P16s Gen 3 (21FS0000XX), firmware R2CET34W (1.18)"
        );
        assert_eq!(
            pc_text(None, Some("X1".into()), Some("X1".into()), None),
            "X1"
        );
        assert_eq!(pc_text(None, None, None, None), "model not read");
        assert_eq!(
            processor_text(Some("CPU".into()), Some(24), Some("ARM64")),
            "CPU, 24 logical processors, ARM64 Windows"
        );
        assert_eq!(power_text(true, Some(87)), "on AC, battery 87%");
        assert_eq!(
            power_text(false, None),
            "on battery, no battery charge known"
        );
        let caps = SYSTEM_POWER_CAPABILITIES {
            AoAc: true,
            SystemS4: true,
            LidPresent: true,
            SystemBatteriesPresent: true,
            ..SYSTEM_POWER_CAPABILITIES::default()
        };
        assert_eq!(
            sleep_text(&caps),
            "Modern Standby yes, S3 no, hibernate yes, hibernation file no, lid yes, battery yes"
        );
        let plan = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
        let values = [
            (PlanSetting::LidAc, Ok(1)),
            (PlanSetting::LidDc, Ok(0)),
            (PlanSetting::SleepAc, Ok(18_000)),
            (PlanSetting::SleepDc, Ok(0)),
            (PlanSetting::SaverThresholdDc, Ok(20)),
            (
                PlanSetting::VideoIdleAc,
                Err(WinError {
                    call: "PowerReadACValueIndex",
                    code: 2,
                }),
            ),
        ];
        assert_eq!(
            plan_text(plan, &values),
            "381b4222-f694-41f0-9685-ff5bb260df2e: lidAc 1 (Sleep), lidDc 0 (Do nothing), sleepAc 18000 s, \
             sleepDc 0 (never), saverThresholdDc 20%, videoIdleAc not read (PowerReadACValueIndex failed with 2)"
        );
    }

    /// The facts of this PC, read for real; compared by hand with `winver`, `powercfg /a` and Settings
    /// (`--nocapture`).
    #[test]
    fn real_machine_facts_all_read() {
        let facts = machine_facts();
        let names: Vec<&str> = facts.iter().map(|f| f.name).collect();
        assert_eq!(
            names,
            [
                "Wakeshot",
                "Windows",
                "Processor",
                "PC",
                "Sleep",
                "Power",
                "Power plan",
                "Elevated"
            ]
        );
        for fact in &facts {
            println!("{}: {}", fact.name, fact.value);
            assert!(!fact.value.contains("not read"), "{fact:?}");
        }
        assert!(facts[1].value.starts_with("Windows 1"), "{:?}", facts[1]);
        assert!(desktop().unwrap().is_dir());
    }

    const PROMPT: &str = "PLANTED-PROMPT-7f3a refactor the billing module";
    const TOKEN: &str = "sk-ant-api03-PLANTEDtoken0123456789abcdefXYZ";
    const GH_TOKEN: &str = "ghp_PLANTED0123456789abcdefghijklmnopqrstuv";

    /// The bundle holds Wakeshot's own files with the user name and tokens taken out, and never a prompt or a token
    /// planted in other apps' files, in stray files of the data folder, or behind a junction.
    #[test]
    fn the_bundle_never_holds_a_planted_prompt_or_token() {
        let root = TempDir::new("bundle");
        let home = root.0.join("Users").join("Planted.User");
        let data = DataDir::at(home.join(r"AppData\Local\Wakeshot"));
        let out = root.0.join("out");
        fs::create_dir_all(&out).unwrap();

        // Other apps' files, as an agent writes them: a prompt and a token in each.
        let claude = home.join(".claude");
        write(
            &claude.join("sessions").join("4100.json"),
            &format!(r#"{{"pid":4100,"prompt":"{PROMPT}"}}"#),
        );
        write(&claude.join("sessions").join("4100.key"), TOKEN);
        write(
            &home.join(r".codex\auth.json"),
            &format!(r#"{{"OPENAI_API_KEY":"{TOKEN}"}}"#),
        );
        write(
            &home.join(r".local\share\opencode\service.json"),
            &format!(r#"{{"password":"{GH_TOKEN}"}}"#),
        );

        // Wakeshot's own files, some with the user's paths and a token a script wrote into a hold reason.
        let profile = home.display().to_string();
        write(
            &data.file(DataFile::Config),
            "{\"version\":1,\"automatic\":true}",
        );
        write(
            &data.file(DataFile::State),
            &format!(
                r#"{{"version":1,"holding":["Claude Code (1 session)"],"exe":"{}"}}"#,
                profile.replace('\\', "\\\\")
            ),
        );
        write(
            &data.holds().join("backup.json"),
            &format!(r#"{{"version":1,"pid":4242,"reason":"upload {GH_TOKEN}"}}"#),
        );
        write(
            &data
                .root()
                .join(r"agents\cursor\7ae3e396-ece8-469b-a443-025f1d9ebb91.json"),
            r#"{"version":1,"pid":54692,"label":"Cursor","state":"working"}"#,
        );
        let log = LogWriter::open(&data.logs(), Proc::Tray).unwrap();
        log.line(
            Level::Info,
            "journal",
            &format!(r#"RunOnce value set to "{profile}\AppData\Local\x.exe" --restore"#),
        );
        log.line(Level::Info, "core", &format!("a line with {TOKEN} in it"));

        // Files Wakeshot does not own, inside its own folders, and a junction to another app's folder.
        write(&data.root().join("notes.txt"), PROMPT);
        write(&data.holds().join("readme.txt"), PROMPT);
        write(&data.logs().join("other.log"), PROMPT);
        write(&data.root().join(r"agents\notes.txt"), PROMPT);
        junction(
            &data.root().join(r"agents\claude"),
            &claude.join("sessions"),
        );
        junction(&data.holds().join("linked"), &claude);

        let header = vec![format!(
            "This command: {profile}\\AppData\\Local\\wakeshot.exe"
        )];
        let redact = Redact::new(Some(&profile), ["Planted.User".to_owned()]);
        let bundle = write_bundle(&data, &out, &header, &redact, clock::now().wall_ms).unwrap();
        let text = fs::read_to_string(&bundle.path).unwrap();
        println!("{text}");

        for planted in [
            PROMPT,
            "PLANTED-PROMPT",
            TOKEN,
            GH_TOKEN,
            "PLANTEDtoken",
            "Planted.User",
            "planted.user",
        ] {
            assert!(!text.contains(planted), "the bundle holds {planted:?}");
        }
        assert!(
            text.contains("%USERPROFILE%\\AppData\\Local\\x.exe"),
            "the log's path, redacted"
        );
        assert!(text.contains("upload [token removed]"));
        for name in [
            "== config.json ==",
            "== state.json ==",
            "== holds\\backup.json ==",
            "== agents\\cursor\\7ae3e396-ece8-469b-a443-025f1d9ebb91.json ==",
            "== logs\\wakeshot.log ==",
        ] {
            assert!(text.contains(name), "{name} is missing");
        }
        for stray in [
            "notes.txt",
            "readme.txt",
            "other.log",
            "4100.json",
            "linked",
        ] {
            assert!(!text.contains(stray), "{stray} got in");
        }
        assert_eq!(bundle.files, 5);
        assert!(text.contains("\r\n") && !text.replace("\r\n", "").contains('\n'));
        let name = file_name(&bundle.path);
        assert!(
            name.starts_with("wakeshot-diagnostics-") && name.ends_with(".txt"),
            "{name}"
        );

        // A second bundle in the same second gets its own name and never replaces the first.
        let again = write_bundle(&data, &out, &header, &redact, clock::now().wall_ms).unwrap();
        assert_ne!(again.path, bundle.path);
        assert_eq!(fs::read_to_string(&bundle.path).unwrap(), text);
    }

    #[test]
    fn an_empty_data_folder_still_gives_a_bundle() {
        let root = TempDir::new("empty");
        let data = DataDir::at(root.0.join("missing"));
        let bundle = write_bundle(
            &data,
            &root.0,
            &[],
            &Redact::default(),
            clock::now().wall_ms,
        )
        .unwrap();
        let text = fs::read_to_string(&bundle.path).unwrap();
        assert!(
            text.contains("No Wakeshot file in the data folder."),
            "{text}"
        );
        assert_eq!(bundle.files, 0);
    }
}
