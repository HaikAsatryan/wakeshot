//! The power settings Wakeshot may own, and the journal that says which ones it owns now.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

use crate::process::FileTime;

/// A power plan id. The core cannot use the `windows` crate's GUID, so it keeps the 128 bits itself.
///
/// Owner: core track (Phase 2). The value is the GUID read as one hex number, the same as `windows::core::GUID`'s
/// `from_u128` and `to_u128`, so the Windows layer converts without reordering bytes. Text form: lowercase
/// `8-4-4-4-12` without braces, as in the journal; parsing also takes uppercase.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guid(pub u128);

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.0;
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (v >> 96) as u32,
            (v >> 80) as u16,
            (v >> 64) as u16,
            (v >> 48) as u16,
            v & 0xffff_ffff_ffff
        )
    }
}

// The derived form would print a 39-digit number nobody can match against `powercfg` output.
impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// A text that is not a GUID in `8-4-4-4-12` form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseGuidError;

impl fmt::Display for ParseGuidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a GUID in 8-4-4-4-12 form")
    }
}

impl std::error::Error for ParseGuidError {}

impl FromStr for Guid {
    type Err = ParseGuidError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() != 36 {
            return Err(ParseGuidError);
        }
        let mut value = 0u128;
        for (i, &b) in bytes.iter().enumerate() {
            if matches!(i, 8 | 13 | 18 | 23) {
                if b != b'-' {
                    return Err(ParseGuidError);
                }
                continue;
            }
            let digit = char::from(b).to_digit(16).ok_or(ParseGuidError)?;
            value = (value << 4) | u128::from(digit);
        }
        Ok(Guid(value))
    }
}

impl Serialize for Guid {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(de::Error::custom)
    }
}

/// The only settings Wakeshot ever writes (decision 17). A write takes this type, so no other setting can be written
/// by mistake.
///
/// Owner: core track (Phase 2). Frozen in the journal as `"lidAc"`, `"lidDc"`, `"sleepDc"`: a later version may add
/// keys, never rename these, or an older version could not restore them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingKey {
    LidAc,
    LidDc,
    SleepDc,
}

/// One key on one plan, because a plan switch keeps the old plan's entries apart from the new one's.
///
/// Owner: power track (Phase 3) names it in `UserChanged`; the core track (Phase 2) marks it released. Also in
/// `state.json`, so the CLI can say what is overridden.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OwnedKey {
    pub plan: Guid,
    pub key: SettingKey,
}

/// What the IO thread read from the active plan, so the core can tell the user's own values from targets.
///
/// Owner: power track (Phase 3) reads it. The core may rely on every field coming from one read of the plan named
/// in `active`; a failed read never arrives as values (`Event::Plan(Err)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanValues {
    pub active: Guid,
    pub lid_ac: u32,
    pub lid_dc: u32,
    pub sleep_ac: u32,
    pub sleep_dc: u32,
    pub saver_threshold_dc: u32,
}

/// The process that owns the journal, so a restore runs only when that exact process is gone.
///
/// Owner: power track (Phase 3). Frozen in `journal.json` as `{ "pid", "procStart", "session" }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalOwner {
    pub pid: u32,
    pub proc_start: FileTime,
    pub session: u32,
}

/// One owned key: what it was before we wrote it, and what we wrote, so ownership is "live equals `written`".
///
/// Owner: power track (Phase 3) writes it; the core track (Phase 2) keeps a mirror from `Done(Applied)` and
/// `Done(Restored)`. `restored` means the original is written back and only the `PowerSetActiveScheme` is left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    pub plan: Guid,
    pub key: SettingKey,
    pub original: u32,
    pub written: u32,
    pub restored: bool,
}

/// The journal as this version understands it: entries whose key it knows. The file form, which also keeps keys
/// from a newer version, is [`crate::files::JournalFile`].
///
/// Owner: power track (Phase 3) loads it; the core gets it once in `Event::Started`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Journal {
    pub owner: JournalOwner,
    pub entries: Vec<JournalEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";

    #[test]
    fn guid_text_round_trips_and_matches_windows_order() {
        let g: Guid = BALANCED.parse().unwrap();
        assert_eq!(g, Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e));
        assert_eq!(g.to_string(), BALANCED);
        assert_eq!(format!("{g:?}"), BALANCED);
        assert_eq!(BALANCED.to_uppercase().parse::<Guid>(), Ok(g));
    }

    #[test]
    fn guid_rejects_other_forms() {
        for bad in [
            "",
            "{381b4222-f694-41f0-9685-ff5bb260df2e}",
            "381b4222f69441f09685ff5bb260df2e",
            "381b4222-f694-41f0-9685-ff5bb260df2g",
            "381b4222-f694-41f0+9685-ff5bb260df2e",
        ] {
            assert_eq!(bad.parse::<Guid>(), Err(ParseGuidError), "{bad}");
        }
    }

    #[test]
    fn setting_keys_use_the_journal_names() {
        let names: Vec<String> = [SettingKey::LidAc, SettingKey::LidDc, SettingKey::SleepDc]
            .iter()
            .map(|k| serde_json::to_string(k).unwrap())
            .collect();
        assert_eq!(names, [r#""lidAc""#, r#""lidDc""#, r#""sleepDc""#]);
    }
}
