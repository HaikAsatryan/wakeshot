//! The CLI folder in the user Path. The start-at-login Run value is `store::AutoStart`'s, its one writer.
//!
//! Only entries that name our folder change. Every other entry, every separator, the value's type and any bytes after
//! the first NUL stay as they were, so an add and then a remove give back the old bytes.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_FILENAME_EXCED_RANGE, ERROR_GEN_FAILURE, ERROR_INVALID_DATA,
    ERROR_INVALID_PARAMETER, ERROR_MORE_DATA, ERROR_SUCCESS, GetLastError, LPARAM, SetLastError,
    WIN32_ERROR, WPARAM,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_EXPAND_SZ,
    REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE, RegCreateKeyExW, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};
use windows::core::{Owned, PCWSTR};

use crate::WinError;

/// The user's environment, under `HKCU`.
pub const ENVIRONMENT_KEY: &str = "Environment";
/// The user Path value in it.
pub const PATH_VALUE: &str = "Path";

/// Tells running programs that the environment changed. `UserPath::real` uses `broadcast_environment`; tests pass
/// their own, so they never broadcast.
pub type Broadcast = fn() -> Result<(), WinError>;

/// What `add` or `remove` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathChange {
    /// The value was written or deleted, then announced. A failed broadcast leaves the change in place: only
    /// programs started before a sign-out miss it.
    Changed { broadcast: Result<(), WinError> },
    /// Already there (add) or not there (remove): nothing written, nothing announced.
    Unchanged,
}

/// The user Path value: the real one, or a test key.
#[derive(Clone, Debug)]
pub struct UserPath {
    key: String,
    value: String,
    broadcast: Broadcast,
}

impl UserPath {
    /// `HKCU\Environment`, value `Path`, announced with `WM_SETTINGCHANGE`.
    pub fn real() -> Self {
        Self::at(ENVIRONMENT_KEY, PATH_VALUE, broadcast_environment)
    }

    /// `key` is a key under `HKCU`, `value` the value's name in it.
    pub fn at(key: &str, value: &str, broadcast: Broadcast) -> Self {
        Self {
            key: key.to_owned(),
            value: value.to_owned(),
            broadcast,
        }
    }

    /// Adds `folder` at the end unless an entry names it already (case-insensitive, with or without a trailing
    /// backslash). A missing value is created as `REG_EXPAND_SZ`; an existing one keeps its type.
    pub fn add(&self, folder: &Path) -> Result<PathChange, WinError> {
        let folder = folder_units(folder)?;
        let name = wide("RegSetValueExW", &self.value)?;
        let key = create_key(&self.key)?;
        let new = match read(*key, &name)? {
            None => Value {
                kind: REG_EXPAND_SZ,
                bytes: to_bytes(&folder, &[0]),
            },
            Some(old) => {
                let units = old.units()?;
                let (text, rest) = split_text(&units);
                let Some(text) = added(text, &folder) else {
                    return Ok(PathChange::Unchanged);
                };
                if text.len() >= MAX_VALUE_UNITS {
                    return Err(win32("RegSetValueExW", ERROR_FILENAME_EXCED_RANGE));
                }
                Value {
                    kind: old.kind,
                    bytes: to_bytes(&text, rest),
                }
            }
        };
        write(*key, &name, &new)?;
        Ok(self.announce())
    }

    /// Removes every entry that names `folder`, each with one separator. A value left holding only that folder is
    /// deleted, so a user who had no Path value has none again.
    pub fn remove(&self, folder: &Path) -> Result<PathChange, WinError> {
        let folder = folder_units(folder)?;
        let name = wide("RegSetValueExW", &self.value)?;
        let Some(key) = open_key(&self.key)? else {
            return Ok(PathChange::Unchanged);
        };
        let Some(old) = read(*key, &name)? else {
            return Ok(PathChange::Unchanged);
        };
        let units = old.units()?;
        let (text, rest) = split_text(&units);
        match removed(text, &folder) {
            None => return Ok(PathChange::Unchanged),
            Some(Removed::Value) => delete(*key, &name)?,
            Some(Removed::Text(text)) => {
                let new = Value {
                    kind: old.kind,
                    bytes: to_bytes(&text, rest),
                };
                write(*key, &name, &new)?;
            }
        }
        Ok(self.announce())
    }

    fn announce(&self) -> PathChange {
        PathChange::Changed {
            broadcast: (self.broadcast)(),
        }
    }
}

/// Per window, not in total (Microsoft, `SendMessageTimeoutW`). Hung windows are skipped at once; a slow one may take
/// this long, and the hooks run their restore after the Path step within Velopack's 15 s or 30 s, so it stays short.
const BROADCAST_TIMEOUT_MS: u32 = 1000;

/// Windows' limit for one user environment variable, in UTF-16 units (Microsoft, "Environment Variables"). What
/// Windows does with a longer Path is not verified, so the add leaves such a value alone.
const MAX_VALUE_UNITS: usize = 32_767;

const SEMICOLON: u16 = b';' as u16;
const BACKSLASH: u16 = b'\\' as u16;

/// `WM_SETTINGCHANGE` with `"Environment"` to every top-level window, so Explorer, and the terminals it starts from
/// then on, read the new Path.
pub fn broadcast_environment() -> Result<(), WinError> {
    let area = wide("SendMessageTimeoutW", "Environment")?;
    // SAFETY: plain call; the function does not always set the last error, so a stale one must not be read back.
    unsafe { SetLastError(ERROR_SUCCESS) };
    // SAFETY: `area` is NUL-terminated and outlives the call, which returns only after every window had its turn.
    let sent = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(area.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            BROADCAST_TIMEOUT_MS,
            None,
        )
    };
    if sent.0 != 0 {
        return Ok(());
    }
    // SAFETY: plain call.
    let code = unsafe { GetLastError() };
    // Microsoft: a zero result with no last error is a generic failure.
    let code = if code == ERROR_SUCCESS {
        ERROR_GEN_FAILURE
    } else {
        code
    };
    Err(win32("SendMessageTimeoutW", code))
}

/// The Path text with `folder` at the end; `None` when an entry names it already.
fn added(text: &[u16], folder: &[u16]) -> Option<Vec<u16>> {
    let ours = entry_key(folder);
    if text
        .split(|&unit| unit == SEMICOLON)
        .any(|entry| entry_key(entry) == ours)
    {
        return None;
    }
    let mut new = text.to_vec();
    // A text that ends with a separator, or is empty, gets `folder;`: the remove then takes `folder;` out and leaves
    // the old text, where `;folder` would have taken the user's own trailing separator.
    if text.is_empty() || text.last() == Some(&SEMICOLON) {
        new.extend_from_slice(folder);
        new.push(SEMICOLON);
    } else {
        new.push(SEMICOLON);
        new.extend_from_slice(folder);
    }
    Some(new)
}

/// What a remove leaves.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Removed {
    Text(Vec<u16>),
    /// Every entry named our folder.
    Value,
}

/// The Path text without the entries that name `folder`; `None` when none does.
fn removed(text: &[u16], folder: &[u16]) -> Option<Removed> {
    let ours = entry_key(folder);
    let entries: Vec<&[u16]> = text.split(|&unit| unit == SEMICOLON).collect();
    // Joining the kept entries drops exactly one separator per dropped entry, and keeps empty entries as they were.
    let kept: Vec<&[u16]> = entries
        .iter()
        .copied()
        .filter(|entry| entry_key(entry) != ours)
        .collect();
    if kept.len() == entries.len() {
        None
    } else if kept.is_empty() {
        Some(Removed::Value)
    } else {
        Some(Removed::Text(kept.join(&SEMICOLON)))
    }
}

/// How an entry is compared: without trailing backslashes, in small letters. Windows compares paths without case;
/// for a few rare letters Rust's lower case and Windows' own table may differ (not verified).
fn entry_key(entry: &[u16]) -> Vec<char> {
    let mut end = entry.len();
    while end > 0 && entry[end - 1] == BACKSLASH {
        end -= 1;
    }
    char::decode_utf16(entry[..end].iter().copied())
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The folder as one Path entry. Empty, a separator or a NUL inside would add or remove other entries than ours.
fn folder_units(folder: &Path) -> Result<Vec<u16>, WinError> {
    let units: Vec<u16> = folder.as_os_str().encode_wide().collect();
    if entry_key(&units).is_empty() || units.contains(&SEMICOLON) || units.contains(&0) {
        return Err(win32("RegSetValueExW", ERROR_INVALID_PARAMETER));
    }
    Ok(units)
}

/// The text Windows reads, up to the first NUL, and the rest from that NUL on, which the edit keeps as it was.
fn split_text(units: &[u16]) -> (&[u16], &[u16]) {
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    units.split_at(end)
}

fn to_bytes(text: &[u16], rest: &[u16]) -> Vec<u8> {
    text.iter()
        .chain(rest)
        .flat_map(|unit| unit.to_le_bytes())
        .collect()
}

/// A raw registry value: its type and its bytes as stored.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Value {
    kind: REG_VALUE_TYPE,
    bytes: Vec<u8>,
}

impl Value {
    /// A Path of another type, or with an odd byte count, is not ours to fix: it is left alone.
    fn units(&self) -> Result<Vec<u16>, WinError> {
        let (pairs, odd) = self.bytes.as_chunks::<2>();
        if (self.kind != REG_SZ && self.kind != REG_EXPAND_SZ) || !odd.is_empty() {
            return Err(win32("RegQueryValueExW", ERROR_INVALID_DATA));
        }
        Ok(pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect())
    }
}

/// Opens the key under `HKCU` for reading and writing values, and makes it when it is missing.
fn create_key(path: &str) -> Result<Owned<HKEY>, WinError> {
    let name = wide("RegCreateKeyExW", path)?;
    let mut key = HKEY::default();
    // SAFETY: `name` is NUL-terminated and lives for the call; the key is written to a live local.
    let created = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(name.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            None,
            &raw mut key,
            None,
        )
    };
    reg_ok("RegCreateKeyExW", created)?;
    // SAFETY: the key is new and owned only here.
    Ok(unsafe { Owned::new(key) })
}

/// Like `create_key`, but a missing key is `None`: a remove never makes one.
fn open_key(path: &str) -> Result<Option<Owned<HKEY>>, WinError> {
    let name = wide("RegOpenKeyExW", path)?;
    let mut key = HKEY::default();
    // SAFETY: `name` is NUL-terminated and lives for the call; the key is written to a live local.
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(name.as_ptr()),
            None,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            &raw mut key,
        )
    };
    if opened == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    reg_ok("RegOpenKeyExW", opened)?;
    // SAFETY: the key is new and owned only here.
    Ok(Some(unsafe { Owned::new(key) }))
}

/// The value's type and bytes exactly as stored: `RegGetValueW` would add a missing terminator or expand
/// `REG_EXPAND_SZ`, and either would break the byte-for-byte remove.
fn read(key: HKEY, name: &[u16]) -> Result<Option<Value>, WinError> {
    // The value can grow between the size query and the read; a few tries cover that.
    for _ in 0..4 {
        let mut kind = REG_VALUE_TYPE::default();
        let mut size = 0u32;
        // SAFETY: an open key; `name` is NUL-terminated and lives for the call; a size query writes only the locals.
        let query = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                Some(&raw mut kind),
                None,
                Some(&raw mut size),
            )
        };
        if query == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        reg_ok("RegQueryValueExW", query)?;
        let mut bytes = vec![0u8; size as usize];
        let mut got = size;
        // SAFETY: as above; `bytes` holds `got` bytes and lives for the call.
        let read = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                Some(&raw mut kind),
                Some(bytes.as_mut_ptr()),
                Some(&raw mut got),
            )
        };
        if read == ERROR_MORE_DATA {
            continue;
        }
        if read == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        reg_ok("RegQueryValueExW", read)?;
        bytes.truncate(got as usize);
        return Ok(Some(Value { kind, bytes }));
    }
    Err(win32("RegQueryValueExW", ERROR_MORE_DATA))
}

fn write(key: HKEY, name: &[u16], value: &Value) -> Result<(), WinError> {
    // For string types, Windows reads the unit just past the data, and when the data does not end in a NUL but that
    // unit is one, it stores the NUL too (seen in this file's tests, research "Registry values and the user Path"). A
    // non-zero unit there keeps a value without terminator byte-exact, and keeps that read inside our own buffer.
    let mut buf = Vec::with_capacity(value.bytes.len() + 2);
    buf.extend_from_slice(&value.bytes);
    buf.extend_from_slice(&[0xFF, 0xFF]);
    let data = &buf[..value.bytes.len()];
    // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated; both live for the call.
    let set = unsafe { RegSetValueExW(key, PCWSTR(name.as_ptr()), None, value.kind, Some(data)) };
    reg_ok("RegSetValueExW", set)
}

fn delete(key: HKEY, name: &[u16]) -> Result<(), WinError> {
    // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated and lives for the call.
    let deleted = unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) };
    reg_ok("RegDeleteValueW", deleted)
}

/// NUL-terminated UTF-16. An inner NUL would name another key or value, so it is refused.
fn wide(call: &'static str, text: &str) -> Result<Vec<u16>, WinError> {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    if units.contains(&0) {
        return Err(win32(call, ERROR_INVALID_PARAMETER));
    }
    units.push(0);
    Ok(units)
}

fn reg_ok(call: &'static str, result: WIN32_ERROR) -> Result<(), WinError> {
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(win32(call, result))
    }
}

fn win32(call: &'static str, code: WIN32_ERROR) -> WinError {
    WinError { call, code: code.0 }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use windows::Win32::Foundation::ERROR_TIMEOUT;
    use windows::Win32::System::Registry::{REG_DWORD, REG_MULTI_SZ, RegEnumValueW};
    use windows::core::PWSTR;

    use super::*;
    use crate::store::ScratchKey;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn text(units: &[u16]) -> String {
        String::from_utf16(units).unwrap()
    }

    fn add_text(old: &str, folder: &str) -> Option<String> {
        added(&units(old), &units(folder)).map(|new| text(&new))
    }

    fn remove_text(old: &str, folder: &str) -> Option<Option<String>> {
        removed(&units(old), &units(folder)).map(|left| match left {
            Removed::Text(new) => Some(text(&new)),
            Removed::Value => None,
        })
    }

    const F: &str = r"C:\Users\Haik\AppData\Local\HaikAsatryan.Wakeshot\current";

    #[test]
    fn add_text_table() {
        let f = F;
        let cases: &[(&str, &str, Option<String>)] = &[
            // Plain: one separator before our folder.
            (r"C:\A;C:\B", f, Some(format!(r"C:\A;C:\B;{f}"))),
            // A trailing `;` stays the style: our entry ends with one, so a remove leaves the old trailing `;`.
            (r"C:\A;C:\B;", f, Some(format!(r"C:\A;C:\B;{f};"))),
            (r"C:\A;;", f, Some(format!(r"C:\A;;{f};"))),
            (";", f, Some(format!(";{f};"))),
            // An empty value: `F;`, so a remove leaves an empty value, not a deleted one.
            ("", f, Some(format!("{f};"))),
            // Already there: any case, with or without trailing backslashes, on either side.
            (&format!(r"C:\A;{f}"), f, None),
            (&format!(r"C:\A;{f}\;C:\B"), f, None),
            (&format!(r"{f}\\"), f, None),
            (&format!(r"C:\A;{}", f.to_uppercase()), f, None),
            (
                &format!(r"C:\A;{f}"),
                &format!(r"{}\", f.to_lowercase()),
                None,
            ),
            // A longer folder that starts with ours is another folder.
            (&format!(r"C:\A;{f}2"), f, Some(format!(r"C:\A;{f}2;{f}"))),
            (
                &format!(r"C:\A;{f}\bin"),
                f,
                Some(format!(r"C:\A;{f}\bin;{f}")),
            ),
        ];
        for (old, folder, want) in cases {
            assert_eq!(&add_text(old, folder), want, "add {folder:?} to {old:?}");
        }
    }

    #[test]
    fn add_text_matches_non_ascii_letters_in_any_case() {
        // Armenian capitals and small letters, as in a user folder named in Armenian.
        let upper = "C:\\Users\\\u{540}\u{531}\u{545}\u{53F}\\bin";
        let lower = "c:\\users\\\u{570}\u{561}\u{575}\u{56F}\\BIN";
        assert_eq!(add_text(&format!(r"C:\A;{upper}"), lower), None);
    }

    #[test]
    fn remove_text_table() {
        let f = F;
        let cases: &[(&str, &str, Option<Option<String>>)] = &[
            (
                &format!(r"C:\A;C:\B;{f}"),
                f,
                Some(Some(r"C:\A;C:\B".into())),
            ),
            (
                &format!(r"C:\A;C:\B;{f};"),
                f,
                Some(Some(r"C:\A;C:\B;".into())),
            ),
            (&format!(r"C:\A;;{f};"), f, Some(Some(r"C:\A;;".into()))),
            (&format!(";{f};"), f, Some(Some(";".into()))),
            (&format!("{f};"), f, Some(Some(String::new()))),
            (&format!(r"{f};C:\A"), f, Some(Some(r"C:\A".into()))),
            // Every entry that names it goes, also ones added by hand, in any case or with a trailing backslash.
            (
                &format!(r"C:\A;{f};C:\B;{}\", f.to_uppercase()),
                f,
                Some(Some(r"C:\A;C:\B".into())),
            ),
            // Only our folder: the value goes.
            (f, f, Some(None)),
            (&format!(r"{f}\"), f, Some(None)),
            (&format!("{f};{f}"), f, Some(None)),
            // Not there: nothing to do.
            (r"C:\A;C:\B", f, None),
            ("", f, None),
            (&format!(r"C:\A;{f}2"), f, None),
        ];
        for (old, folder, want) in cases {
            assert_eq!(
                &remove_text(old, folder),
                want,
                "remove {folder:?} from {old:?}"
            );
        }
    }

    #[test]
    fn add_then_remove_gives_back_the_old_text() {
        for old in [
            "",
            ";",
            ";;",
            r"C:\A",
            r"C:\A;",
            r"C:\A;;",
            r"C:\A;C:\B",
            r"C:\A;C:\B;",
            r";C:\A",
        ] {
            let new = added(&units(old), &units(F)).unwrap();
            assert_eq!(
                removed(&new, &units(F)),
                Some(Removed::Text(units(old))),
                "{old:?}"
            );
        }
    }

    #[test]
    fn remove_after_a_create_deletes_the_value() {
        // A missing value is created holding only our folder (`UserPath::add`), and that is what a remove deletes.
        assert_eq!(removed(&units(F), &units(F)), Some(Removed::Value));
    }

    // --- The registry, on the scratch key. ---

    thread_local! {
        static SENT: Cell<u32> = const { Cell::new(0) };
    }

    /// Counts instead of broadcasting. Each test runs on its own thread, so the count is the test's own.
    fn counting() -> Result<(), WinError> {
        SENT.with(|sent| sent.set(sent.get() + 1));
        Ok(())
    }

    fn failing() -> Result<(), WinError> {
        SENT.with(|sent| sent.set(sent.get() + 1));
        Err(win32("SendMessageTimeoutW", ERROR_TIMEOUT))
    }

    fn sent() -> u32 {
        SENT.with(Cell::get)
    }

    fn changed() -> PathChange {
        PathChange::Changed { broadcast: Ok(()) }
    }

    fn bytes(units: &[u16]) -> Vec<u8> {
        units.iter().flat_map(|unit| unit.to_le_bytes()).collect()
    }

    fn sz(text: &str) -> Vec<u8> {
        let mut all = units(text);
        all.push(0);
        bytes(&all)
    }

    /// The scratch key and the value `Path` in it.
    struct Fixture {
        scratch: ScratchKey,
        path: UserPath,
    }

    impl Fixture {
        fn new(test_name: &str) -> Self {
            Self::with(test_name, counting)
        }

        fn with(test_name: &str, broadcast: Broadcast) -> Self {
            let scratch = ScratchKey::new(test_name);
            let path = UserPath::at(scratch.path(), PATH_VALUE, broadcast);
            Self { scratch, path }
        }

        fn set(&self, kind: REG_VALUE_TYPE, data: &[u8]) {
            let key = create_key(self.scratch.path()).unwrap();
            let value = Value {
                kind,
                bytes: data.to_vec(),
            };
            write(*key, &wide("test", PATH_VALUE).unwrap(), &value).unwrap();
        }

        fn get(&self) -> Option<Value> {
            let key = open_key(self.scratch.path()).unwrap().unwrap();
            read(*key, &wide("test", PATH_VALUE).unwrap()).unwrap()
        }
    }

    fn folder() -> &'static Path {
        Path::new(F)
    }

    #[test]
    fn add_creates_a_missing_value_as_expand_sz_and_remove_deletes_it() {
        let fx = Fixture::new("install::add_creates_a_missing_value");
        let before = sent();

        assert_eq!(fx.path.add(folder()), Ok(changed()));
        assert_eq!(
            fx.get(),
            Some(Value {
                kind: REG_EXPAND_SZ,
                bytes: sz(F)
            })
        );
        assert_eq!(fx.path.add(folder()), Ok(PathChange::Unchanged));
        assert_eq!(sent() - before, 1, "only a real change broadcasts");

        assert_eq!(fx.path.remove(folder()), Ok(changed()));
        assert_eq!(fx.get(), None);
        assert_eq!(fx.path.remove(folder()), Ok(PathChange::Unchanged));
        assert_eq!(sent() - before, 2);
    }

    /// Sets `old`, adds, checks the added value, removes, and checks the old value is back byte for byte.
    fn round_trip(test_name: &str, kind: REG_VALUE_TYPE, old: &[u8], added_bytes: &[u8]) {
        let fx = Fixture::new(test_name);
        fx.set(kind, old);
        let before = sent();

        assert_eq!(fx.path.add(folder()), Ok(changed()), "{test_name}: add");
        assert_eq!(
            fx.get(),
            Some(Value {
                kind,
                bytes: added_bytes.to_vec()
            }),
            "{test_name}: after add"
        );
        assert_eq!(fx.path.add(folder()), Ok(PathChange::Unchanged));
        assert_eq!(
            fx.path.remove(folder()),
            Ok(changed()),
            "{test_name}: remove"
        );
        assert_eq!(
            fx.get(),
            Some(Value {
                kind,
                bytes: old.to_vec()
            }),
            "{test_name}: after remove"
        );
        assert_eq!(sent() - before, 2, "{test_name}: broadcasts");
    }

    #[test]
    fn round_trip_keeps_reg_sz() {
        round_trip(
            "install::round_trip_keeps_reg_sz",
            REG_SZ,
            &sz(r"C:\A;C:\B"),
            &sz(&format!(r"C:\A;C:\B;{F}")),
        );
    }

    #[test]
    fn round_trip_keeps_reg_expand_sz_unexpanded() {
        round_trip(
            "install::round_trip_keeps_reg_expand_sz",
            REG_EXPAND_SZ,
            &sz(r"%USERPROFILE%\.cargo\bin;C:\B"),
            &sz(&format!(r"%USERPROFILE%\.cargo\bin;C:\B;{F}")),
        );
    }

    #[test]
    fn round_trip_keeps_a_trailing_semicolon() {
        round_trip(
            "install::round_trip_keeps_a_trailing_semicolon",
            REG_EXPAND_SZ,
            &sz(r"C:\A;C:\B;"),
            &sz(&format!(r"C:\A;C:\B;{F};")),
        );
    }

    #[test]
    fn round_trip_keeps_an_empty_value() {
        // Zero bytes, and a lone terminator: both are an empty Path to Windows, and both come back as they were.
        round_trip(
            "install::round_trip_keeps_an_empty_value_0",
            REG_EXPAND_SZ,
            &[],
            &bytes(&units(&format!("{F};"))),
        );
        round_trip(
            "install::round_trip_keeps_an_empty_value_2",
            REG_SZ,
            &sz(""),
            &sz(&format!("{F};")),
        );
    }

    #[test]
    fn round_trip_keeps_a_value_without_terminator() {
        round_trip(
            "install::round_trip_keeps_a_value_without_terminator",
            REG_EXPAND_SZ,
            &bytes(&units(r"C:\A")),
            &bytes(&units(&format!(r"C:\A;{F}"))),
        );
    }

    #[test]
    fn round_trip_keeps_bytes_after_the_first_nul() {
        let mut old = sz(r"C:\A");
        old.extend(sz("left over"));
        let mut new = sz(&format!(r"C:\A;{F}"));
        new.extend(sz("left over"));
        round_trip(
            "install::round_trip_keeps_bytes_after_the_first_nul",
            REG_EXPAND_SZ,
            &old,
            &new,
        );
    }

    #[test]
    fn windows_stores_a_nul_just_past_string_data() {
        // Why `write` puts a guard unit after the data: without it, a NUL there is stored too.
        let fx = Fixture::new("install::windows_stores_a_nul_just_past_string_data");
        let key = create_key(fx.scratch.path()).unwrap();
        let name = wide("test", PATH_VALUE).unwrap();
        let data = sz(r"C:\A");
        let without_nul = &data[..data.len() - 2];
        // SAFETY: an open key with `KEY_SET_VALUE`; `name` is NUL-terminated; both live for the call.
        let set = unsafe {
            RegSetValueExW(
                *key,
                PCWSTR(name.as_ptr()),
                None,
                REG_EXPAND_SZ,
                Some(without_nul),
            )
        };
        assert_eq!(set, ERROR_SUCCESS);
        assert_eq!(
            fx.get().unwrap().bytes,
            data,
            "the NUL past the data was stored"
        );

        write(
            *key,
            &name,
            &Value {
                kind: REG_EXPAND_SZ,
                bytes: without_nul.to_vec(),
            },
        )
        .unwrap();
        assert_eq!(
            fx.get().unwrap().bytes,
            without_nul,
            "the guard keeps it out"
        );
    }

    #[test]
    fn our_folder_listed_twice_by_hand() {
        let fx = Fixture::new("install::our_folder_listed_twice_by_hand");
        let old = sz(&format!(r"C:\A;{F};{}\", F.to_lowercase()));
        fx.set(REG_EXPAND_SZ, &old);
        let before = sent();

        assert_eq!(fx.path.add(folder()), Ok(PathChange::Unchanged));
        assert_eq!(
            fx.get().unwrap().bytes,
            old,
            "already there: nothing written"
        );
        assert_eq!(fx.path.remove(folder()), Ok(changed()));
        assert_eq!(
            fx.get(),
            Some(Value {
                kind: REG_EXPAND_SZ,
                bytes: sz(r"C:\A")
            })
        );
        assert_eq!(sent() - before, 1);
    }

    #[test]
    fn other_types_and_odd_lengths_are_left_alone() {
        for (n, (kind, data)) in [
            (REG_DWORD, 7u32.to_le_bytes().to_vec()),
            (REG_MULTI_SZ, {
                let mut data = sz(r"C:\A");
                data.extend([0, 0]);
                data
            }),
            (REG_SZ, vec![b'C', 0, b':']),
        ]
        .into_iter()
        .enumerate()
        {
            let fx = Fixture::new(&format!("install::other_types_and_odd_lengths_{n}"));
            fx.set(kind, &data);
            let before = sent();
            assert_eq!(
                fx.path.add(folder()),
                Err(win32("RegQueryValueExW", ERROR_INVALID_DATA))
            );
            assert_eq!(
                fx.path.remove(folder()),
                Err(win32("RegQueryValueExW", ERROR_INVALID_DATA))
            );
            assert_eq!(fx.get(), Some(Value { kind, bytes: data }));
            assert_eq!(sent(), before);
        }
    }

    #[test]
    fn a_folder_that_is_no_path_entry_is_refused() {
        let fx = Fixture::new("install::a_folder_that_is_no_path_entry");
        fx.set(REG_EXPAND_SZ, &sz(r"C:\A;;C:\B"));
        for bad in ["", r"\", r"C:\A;C:\B", "C:\\A\0B"] {
            let refused = Err(win32("RegSetValueExW", ERROR_INVALID_PARAMETER));
            assert_eq!(fx.path.add(Path::new(bad)), refused, "add {bad:?}");
            assert_eq!(fx.path.remove(Path::new(bad)), refused, "remove {bad:?}");
        }
        assert_eq!(fx.get().unwrap().bytes, sz(r"C:\A;;C:\B"));
    }

    #[test]
    fn a_path_too_long_for_windows_is_left_alone() {
        let fx = Fixture::new("install::a_path_too_long_for_windows");
        // Under the limit alone, over it with our folder added.
        let long = format!(r"C:\{}", "a".repeat(32_740));
        fx.set(REG_EXPAND_SZ, &sz(&long));
        assert_eq!(
            fx.path.add(folder()),
            Err(win32("RegSetValueExW", ERROR_FILENAME_EXCED_RANGE))
        );
        assert_eq!(fx.get().unwrap().bytes, sz(&long));
    }

    #[test]
    fn remove_with_no_key_or_no_value_changes_nothing() {
        let fx = Fixture::new("install::remove_with_no_key_or_no_value");
        assert_eq!(fx.path.remove(folder()), Ok(PathChange::Unchanged));
        let no_key = UserPath::at(
            &format!(r"{}\missing", fx.scratch.path()),
            PATH_VALUE,
            counting,
        );
        assert_eq!(no_key.remove(folder()), Ok(PathChange::Unchanged));
        assert!(
            open_key(&format!(r"{}\missing", fx.scratch.path()))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_failed_broadcast_keeps_the_change() {
        let fx = Fixture::with("install::a_failed_broadcast_keeps_the_change", failing);
        let failed = PathChange::Changed {
            broadcast: Err(win32("SendMessageTimeoutW", ERROR_TIMEOUT)),
        };
        assert_eq!(fx.path.add(folder()), Ok(failed));
        assert_eq!(fx.get().unwrap().bytes, sz(F));
        assert_eq!(fx.path.remove(folder()), Ok(failed));
        assert_eq!(fx.get(), None);
    }

    #[test]
    fn the_value_keeps_its_name_case() {
        // Windows names it `Path`, but a user or a tool may have written `PATH`; the edit must not rename it.
        let fx = Fixture::new("install::the_value_keeps_its_name_case");
        let key = create_key(fx.scratch.path()).unwrap();
        let upper = Value {
            kind: REG_EXPAND_SZ,
            bytes: sz(r"C:\A"),
        };
        write(*key, &wide("test", "PATH").unwrap(), &upper).unwrap();

        assert_eq!(fx.path.add(folder()), Ok(changed()));
        assert_eq!(value_names(fx.scratch.path()), ["PATH"]);
        assert_eq!(fx.path.remove(folder()), Ok(changed()));
        assert_eq!(value_names(fx.scratch.path()), ["PATH"]);
        assert_eq!(fx.get(), Some(upper));
    }

    fn value_names(path: &str) -> Vec<String> {
        let name = wide("test", path).unwrap();
        let mut key = HKEY::default();
        // SAFETY: `name` is NUL-terminated; the key is written to a live local and owned below.
        let opened = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(name.as_ptr()),
                None,
                KEY_QUERY_VALUE,
                &raw mut key,
            )
        };
        assert_eq!(opened, ERROR_SUCCESS);
        // SAFETY: the key is new and owned only here.
        let key = unsafe { Owned::new(key) };
        let mut names = Vec::new();
        for index in 0.. {
            // Value names are at most 16,383 characters (Microsoft, "Registry element size limits").
            let mut buf = vec![0u16; 16_384];
            let mut len = buf.len() as u32;
            // SAFETY: an open key with `KEY_QUERY_VALUE`; `buf` holds `len` characters.
            let result = unsafe {
                RegEnumValueW(
                    *key,
                    index,
                    Some(PWSTR(buf.as_mut_ptr())),
                    &raw mut len,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if result != ERROR_SUCCESS {
                break;
            }
            names.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        names
    }

    #[test]
    fn real_names_the_user_environment() {
        let real = UserPath::real();
        assert_eq!(
            (real.key.as_str(), real.value.as_str()),
            ("Environment", "Path")
        );
    }
}
