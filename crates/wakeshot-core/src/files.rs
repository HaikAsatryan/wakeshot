//! The files in the data folder (architecture section 6), as serde structs. Every file carries `version`.
//!
//! Version rule: a reader takes its own version and older ones. Every file but the journal is read through [`read`],
//! which ignores a file with a newer version (the caller logs it), because a newer file may change a field's shape.
//! The journal is read whole whatever its version: [`JournalFile`] keeps entries with keys it does not know, so a
//! downgrade still restores the keys it does know. Unknown fields are ignored on read, never an error; fields added
//! later are optional.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::process::FileTime;
use crate::settings::{Guid, JournalOwner, OwnedKey, SettingKey};

/// Only the `version` of any data file, read before the rest so a newer file is ignored instead of failing to parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionProbe {
    pub version: u32,
}

/// A data file with a format version, so [`read`] can tell a newer file from a bad one.
pub trait Versioned: DeserializeOwned {
    const VERSION: u32;
}

/// What [`read`] found in a file's bytes.
///
/// Callers may rely on `Newer` meaning the file is well-formed JSON with a `version` above this one's, and `Bad`
/// meaning anything else that is not this version's form (log it, never act on it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read<T> {
    Value(T),
    Newer(u32),
    Bad,
}

/// Reads `config.json`, a hold file, `restore-request.json` or `state.json` by the version rule above, so no reader
/// can forget it. Only the first JSON value counts and bytes after it are ignored, the way Claude's own files are
/// read, so a script that appends a newline or a second object still gives the same hold. A leading UTF-8 byte order
/// mark is skipped: Windows PowerShell 5.1 writes one with `-Encoding UTF8` (Microsoft docs, about_Character_Encoding).
pub fn read<T: Versioned>(bytes: &[u8]) -> Read<T> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut values = serde_json::Deserializer::from_slice(bytes).into_iter::<serde_json::Value>();
    let Some(Ok(value)) = values.next() else {
        return Read::Bad;
    };
    match VersionProbe::deserialize(&value) {
        Ok(probe) if probe.version > T::VERSION => Read::Newer(probe.version),
        Ok(_) => T::deserialize(value).map_or(Read::Bad, Read::Value),
        Err(_) => Read::Bad,
    }
}

/// `config.json`: the settings that survive a restart (decision 31).
///
/// The tray writes it through `Command::WriteConfig`, the CLI writes it directly. A missing file means
/// `Config::default()` (Automatic on).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub automatic: bool,
}

impl Config {
    pub const VERSION: u32 = 1;

    pub fn new(automatic: bool) -> Self {
        Self {
            version: Self::VERSION,
            automatic,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(true)
    }
}

/// `holds\*.json`: one hold from `wakeshot run`, Keep awake, or any script (the user doc is `docs/hold-files.md`).
///
/// `holds::file` parses it into `HoldFacts`. Scripts write it, so it stays loose: only `version` and `pid` are
/// required. `expires` is kept as the text the writer gave (RFC 3339, `Z` or an offset), because turning it into wall
/// ms is parsing, which `holds::file` owns. `screen` counts only in `keep-awake.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldFile {
    pub version: u32,
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proc_start: Option<FileTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
    #[serde(default)]
    pub screen: bool,
}

impl HoldFile {
    pub const VERSION: u32 = 1;

    pub fn new(pid: u32, proc_start: Option<FileTime>) -> Self {
        Self {
            version: Self::VERSION,
            pid,
            proc_start,
            reason: None,
            expires: None,
            screen: false,
        }
    }
}

/// `restore-request.json`: `wakeshot restore` asks a running tray to restore; the file going away is the answer.
///
/// The CLI writes it; the watch thread turns it into `Menu(Restore)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRequest {
    pub version: u32,
}

impl RestoreRequest {
    pub const VERSION: u32 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
        }
    }
}

impl Default for RestoreRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// `state.json`: what the tray is doing, for `wakeshot status` and `wakeshot restore`.
///
/// The CLI may rely on `pid` plus `procStart` naming the tray that wrote it ("not running" when that process is
/// gone), on `holding` being the tooltip labels, and on `overridden` being empty once nothing is ours.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateFile {
    pub version: u32,
    pub pid: u32,
    pub proc_start: FileTime,
    pub automatic: bool,
    pub holding: Vec<String>,
    pub overridden: Vec<OwnedKey>,
    pub stepped_back: bool,
}

impl StateFile {
    pub const VERSION: u32 = 1;
}

/// A journal entry's key as the file has it: one this version knows, or one a newer version wrote.
///
/// Keeping `Other` lets this version read a newer journal without failing on it. An `Other` entry stays in the file
/// untouched: restore, apply and the ownership check skip it, and the journal counts as empty when no known entry is
/// left (the clean-up then clears RunOnce and stops the guard, but saves the file with only those entries), so the
/// newer version restores them at its next start.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JournalKey {
    Known(SettingKey),
    Other(String),
}

/// One entry as stored. The fields are frozen (architecture section 6): a later version only adds fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalFileEntry {
    pub plan: Guid,
    pub key: JournalKey,
    pub original: u32,
    pub written: u32,
    pub restored: bool,
}

/// `journal.json`: the keys Wakeshot owns now, with their originals (decision 24).
///
/// The Windows layer is the only writer, always under `Local\Wakeshot.Journal`. Read whole whatever its
/// `version` (see the module doc); [`crate::settings::Journal`] is the form the core gets. A save always writes
/// [`JournalFile::VERSION`], never the version it read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalFile {
    pub version: u32,
    pub owner: JournalOwner,
    pub entries: Vec<JournalFileEntry>,
}

impl JournalFile {
    pub const VERSION: u32 = 1;
}

impl Versioned for Config {
    const VERSION: u32 = Config::VERSION;
}

impl Versioned for HoldFile {
    const VERSION: u32 = HoldFile::VERSION;
}

impl Versioned for RestoreRequest {
    const VERSION: u32 = RestoreRequest::VERSION;
}

impl Versioned for StateFile {
    const VERSION: u32 = StateFile::VERSION;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);

    fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T) {
        let text = serde_json::to_string(value).unwrap();
        let back: T = serde_json::from_str(&text).unwrap();
        assert_eq!(&back, value, "{text}");
    }

    fn version_of(text: &str) -> u32 {
        serde_json::from_str::<VersionProbe>(text).unwrap().version
    }

    #[test]
    fn config_round_trips_and_reads_the_documented_form() {
        round_trip(&Config::new(false));
        let doc: Config = serde_json::from_str(r#"{ "version": 1, "automatic": true }"#).unwrap();
        assert_eq!(doc, Config::default());
    }

    #[test]
    fn hold_file_round_trips_and_reads_the_documented_form() {
        let full = HoldFile {
            reason: Some("nightly backup".into()),
            expires: Some("2026-11-03T18:00:00Z".into()),
            screen: true,
            ..HoldFile::new(4242, Some(FileTime(134_381_211_391_959_336)))
        };
        round_trip(&full);
        round_trip(&HoldFile::new(7, None));

        let doc: HoldFile = serde_json::from_str(
            r#"{ "version": 1, "pid": 4242, "procStart": "134381211391959336",
                 "reason": "nightly backup", "expires": "2026-11-03T18:00:00Z", "screen": false }"#,
        )
        .unwrap();
        assert_eq!(
            doc,
            HoldFile {
                screen: false,
                ..full
            }
        );
    }

    #[test]
    fn hold_file_needs_only_version_and_pid() {
        let minimal: HoldFile = serde_json::from_str(r#"{ "version": 1, "pid": 4242 }"#).unwrap();
        assert_eq!(minimal, HoldFile::new(4242, None));
        assert_eq!(
            serde_json::to_string(&minimal).unwrap(),
            r#"{"version":1,"pid":4242,"screen":false}"#
        );
        assert!(serde_json::from_str::<HoldFile>(r#"{ "version": 1 }"#).is_err());
    }

    #[test]
    fn proc_start_must_be_a_decimal_string() {
        for bad in [
            r#"{ "version": 1, "pid": 1, "procStart": 134381211391959336 }"#,
            r#"{ "version": 1, "pid": 1, "procStart": "+134381211391959336" }"#,
            r#"{ "version": 1, "pid": 1, "procStart": "" }"#,
        ] {
            assert!(serde_json::from_str::<HoldFile>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn restore_request_round_trips_and_reads_the_documented_form() {
        round_trip(&RestoreRequest::new());
        let doc: RestoreRequest = serde_json::from_str(r#"{ "version": 1 }"#).unwrap();
        assert_eq!(doc, RestoreRequest::new());
    }

    #[test]
    fn state_file_round_trips() {
        round_trip(&StateFile {
            version: StateFile::VERSION,
            pid: 9120,
            proc_start: FileTime(134_381_211_391_959_336),
            automatic: true,
            holding: vec!["Claude Code".into(), "nightly backup".into()],
            overridden: vec![OwnedKey {
                plan: BALANCED,
                key: SettingKey::LidDc,
            }],
            stepped_back: false,
        });
    }

    #[test]
    fn journal_round_trips_and_reads_the_documented_form() {
        let doc = r#"{ "version": 1, "owner": { "pid": 9120, "procStart": "134381211391959336", "session": 1 },
            "entries": [ { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "lidAc", "original": 1,
                           "written": 0, "restored": false } ] }"#;
        let journal: JournalFile = serde_json::from_str(doc).unwrap();
        assert_eq!(
            journal,
            JournalFile {
                version: 1,
                owner: JournalOwner {
                    pid: 9120,
                    proc_start: FileTime(134_381_211_391_959_336),
                    session: 1,
                },
                entries: vec![JournalFileEntry {
                    plan: BALANCED,
                    key: JournalKey::Known(SettingKey::LidAc),
                    original: 1,
                    written: 0,
                    restored: false,
                }],
            }
        );
        round_trip(&journal);
    }

    /// Section 6: a newer file is ignored even when the rest of it would not parse, for every file `read` serves.
    #[test]
    fn read_ignores_newer_files() {
        let config = r#"{ "version": 2, "automatic": "sometimes" }"#;
        assert_eq!(read::<Config>(config.as_bytes()), Read::Newer(2));
        let hold = r#"{ "version": 3, "pid": "not a number", "tags": [] }"#;
        assert_eq!(read::<HoldFile>(hold.as_bytes()), Read::Newer(3));
        let request = r#"{ "version": 2, "plan": "all" }"#;
        assert_eq!(read::<RestoreRequest>(request.as_bytes()), Read::Newer(2));
        let state =
            r#"{ "version": 2, "pid": 1, "procStart": "1", "automatic": true, "holding": {} }"#;
        assert_eq!(read::<StateFile>(state.as_bytes()), Read::Newer(2));
    }

    /// Section 6: fields this version does not know are ignored in its own version and older ones.
    #[test]
    fn read_ignores_added_fields() {
        let config = r#"{ "version": 1, "automatic": false, "theme": "dark" }"#;
        assert_eq!(read(config.as_bytes()), Read::Value(Config::new(false)));
        let hold = r#"{ "version": 1, "pid": 5, "priority": 3 }"#;
        assert_eq!(read(hold.as_bytes()), Read::Value(HoldFile::new(5, None)));
        let request = r#"{ "version": 1, "from": "cli" }"#;
        assert_eq!(read(request.as_bytes()), Read::Value(RestoreRequest::new()));
        let older = r#"{ "version": 0, "automatic": true }"#;
        assert_eq!(
            read::<Config>(older.as_bytes()),
            Read::Value(Config {
                version: 0,
                automatic: true
            })
        );
    }

    /// Scripts write hold files: only the first JSON value counts, and a byte order mark or trailing bytes are no
    /// reason to drop a hold. Anything that is not this version's form is `Bad`.
    #[test]
    fn read_takes_the_first_value_and_rejects_bad_files() {
        let hold = HoldFile::new(42, None);
        for text in [
            "\u{feff}{ \"version\": 1, \"pid\": 42 }",
            "{ \"version\": 1, \"pid\": 42 }\r\n",
            "{ \"version\": 1, \"pid\": 42 } { \"version\": 9 } trailing",
        ] {
            assert_eq!(read(text.as_bytes()), Read::Value(hold.clone()), "{text:?}");
        }
        for bad in [
            "",
            "not json",
            "[1]",
            r#"{ "pid": 42 }"#,
            r#"{ "version": 1 }"#,
            r#"{ "version": -1 }"#,
        ] {
            assert_eq!(read::<HoldFile>(bad.as_bytes()), Read::Bad, "{bad:?}");
        }
        let state = StateFile {
            version: StateFile::VERSION,
            pid: 1,
            proc_start: FileTime(2),
            automatic: true,
            holding: vec![],
            overridden: vec![],
            stepped_back: false,
        };
        let text = serde_json::to_vec(&state).unwrap();
        assert_eq!(read(&text), Read::Value(state));
    }

    #[test]
    fn version_probe_reads_any_file() {
        assert_eq!(version_of(r#"{ "version": 7, "anything": [1, 2] }"#), 7);
    }

    /// Section 6: every version restores the entries whose key it knows, also from a newer journal.
    #[test]
    fn newer_journal_keeps_known_and_unknown_entries() {
        let doc = r#"{ "version": 2, "owner": { "pid": 9120, "procStart": "5", "session": 1, "user": "x" },
            "entries": [
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "sleepDc", "original": 600,
                "written": 0, "restored": false, "since": 12 },
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "hibernateDc", "original": 10800,
                "written": 0, "restored": false } ] }"#;
        let journal: JournalFile = serde_json::from_str(doc).unwrap();
        assert_eq!(journal.version, 2);
        let keys: Vec<&JournalKey> = journal.entries.iter().map(|e| &e.key).collect();
        assert_eq!(
            keys,
            [
                &JournalKey::Known(SettingKey::SleepDc),
                &JournalKey::Other("hibernateDc".into())
            ]
        );
        round_trip(&journal);
    }

    /// The sleep route's keys read as known keys and keep their names on a save; a journal from before them, with the
    /// three hold keys only, still loads as it did.
    #[test]
    fn the_sleep_route_keys_round_trip_and_an_older_journal_still_loads() {
        let doc = r#"{ "version": 1, "owner": { "pid": 9120, "procStart": "5", "session": 1 },
            "entries": [
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "sleepAc", "original": 18000,
                "written": 60, "restored": false },
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "videoIdleAc", "original": 300,
                "written": 60, "restored": false },
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "videoIdleDc", "original": 0,
                "written": 60, "restored": true } ] }"#;
        let journal: JournalFile = serde_json::from_str(doc).unwrap();
        let keys: Vec<&JournalKey> = journal.entries.iter().map(|e| &e.key).collect();
        assert_eq!(
            keys,
            [
                &JournalKey::Known(SettingKey::SleepAc),
                &JournalKey::Known(SettingKey::VideoIdleAc),
                &JournalKey::Known(SettingKey::VideoIdleDc)
            ]
        );
        let text = serde_json::to_string(&journal).unwrap();
        assert!(text.contains(r#""key":"sleepAc""#), "{text}");
        assert!(text.contains(r#""key":"videoIdleDc""#), "{text}");
        round_trip(&journal);

        let older = r#"{ "version": 1, "owner": { "pid": 9120, "procStart": "5", "session": 1 },
            "entries": [
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "lidDc", "original": 1,
                "written": 0, "restored": false },
              { "plan": "381b4222-f694-41f0-9685-ff5bb260df2e", "key": "sleepDc", "original": 600,
                "written": 0, "restored": false } ] }"#;
        let journal: JournalFile = serde_json::from_str(older).unwrap();
        let keys: Vec<&JournalKey> = journal.entries.iter().map(|e| &e.key).collect();
        assert_eq!(
            keys,
            [
                &JournalKey::Known(SettingKey::LidDc),
                &JournalKey::Known(SettingKey::SleepDc)
            ]
        );
        round_trip(&journal);
    }
}
