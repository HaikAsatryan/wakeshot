//! Split a `JournalFile` into the entries this version knows and the rest, and merge them back for a save (always
//! `JournalFile::VERSION`; unknown keys kept untouched).

use crate::files::{JournalFile, JournalFileEntry, JournalKey};
use crate::settings::{Journal, JournalEntry};

/// `journal.json` split into what this version knows and the entries a newer version wrote.
///
/// Restore, apply and the ownership check work on `known` and save through [`JournalParts::merge`], so `other` goes
/// back to the file as it was read and a newer version restores those entries at its next start (`JournalKey` doc).
/// The journal counts as empty when `known.entries` is empty, whatever `other` holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalParts {
    pub known: Journal,
    pub other: Vec<JournalFileEntry>,
}

impl JournalParts {
    pub fn split(file: JournalFile) -> Self {
        let mut known = Vec::new();
        let mut other = Vec::new();
        for entry in file.entries {
            match entry.key {
                JournalKey::Known(key) => known.push(JournalEntry {
                    plan: entry.plan,
                    key,
                    original: entry.original,
                    written: entry.written,
                    restored: entry.restored,
                }),
                JournalKey::Other(_) => other.push(entry),
            }
        }
        Self {
            known: Journal {
                owner: file.owner,
                entries: known,
            },
            other,
        }
    }

    /// The file to save: the known entries first, then `other`. Always `JournalFile::VERSION`, never the version that
    /// was read: a newer version reads an older file, never the reverse.
    pub fn merge(&self) -> JournalFile {
        let known = self.known.entries.iter().map(|e| JournalFileEntry {
            plan: e.plan,
            key: JournalKey::Known(e.key),
            original: e.original,
            written: e.written,
            restored: e.restored,
        });
        JournalFile {
            version: JournalFile::VERSION,
            owner: self.known.owner,
            entries: known.chain(self.other.iter().cloned()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::FileTime;
    use crate::settings::{Guid, JournalOwner, SettingKey};

    const BALANCED: Guid = Guid(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const OWNER: JournalOwner = JournalOwner {
        pid: 9120,
        proc_start: FileTime(134_381_211_391_959_336),
        session: 1,
    };

    fn known(key: SettingKey, restored: bool) -> JournalFileEntry {
        JournalFileEntry {
            plan: BALANCED,
            key: JournalKey::Known(key),
            original: 1,
            written: 0,
            restored,
        }
    }

    fn unknown(name: &str) -> JournalFileEntry {
        JournalFileEntry {
            plan: BALANCED,
            key: JournalKey::Other(name.into()),
            original: 10_800,
            written: 0,
            restored: true,
        }
    }

    /// A newer version's file, its unknown entries between and around the known ones.
    fn newer_file() -> JournalFile {
        JournalFile {
            version: 2,
            owner: OWNER,
            entries: vec![
                unknown("hibernateAc"),
                known(SettingKey::LidAc, false),
                unknown("hibernateDc"),
                known(SettingKey::LidDc, true),
            ],
        }
    }

    #[test]
    fn split_keeps_known_and_unknown_entries_apart() {
        let parts = JournalParts::split(newer_file());
        assert_eq!(parts.known.owner, OWNER);
        assert_eq!(
            parts.known.entries,
            [
                JournalEntry {
                    plan: BALANCED,
                    key: SettingKey::LidAc,
                    original: 1,
                    written: 0,
                    restored: false,
                },
                JournalEntry {
                    plan: BALANCED,
                    key: SettingKey::LidDc,
                    original: 1,
                    written: 0,
                    restored: true,
                },
            ]
        );
        assert_eq!(
            parts.other,
            [unknown("hibernateAc"), unknown("hibernateDc")]
        );
    }

    #[test]
    fn merge_keeps_unknown_entries_untouched_and_writes_this_version() {
        let file = JournalParts::split(newer_file()).merge();
        assert_eq!(file.version, JournalFile::VERSION);
        assert_eq!(file.owner, OWNER);
        assert_eq!(
            file.entries,
            [
                known(SettingKey::LidAc, false),
                known(SettingKey::LidDc, true),
                unknown("hibernateAc"),
                unknown("hibernateDc"),
            ]
        );
        assert_eq!(JournalParts::split(file.clone()).merge(), file);
    }

    #[test]
    fn only_unknown_entries_count_as_an_empty_journal() {
        let file = JournalFile {
            version: 2,
            owner: OWNER,
            entries: vec![unknown("hibernateAc")],
        };
        let parts = JournalParts::split(file);
        assert!(parts.known.entries.is_empty());
        assert_eq!(parts.other, [unknown("hibernateAc")]);
    }
}
