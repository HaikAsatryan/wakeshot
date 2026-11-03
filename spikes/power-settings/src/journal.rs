use std::path::Path;

use windows::core::GUID;

use crate::power::{self, Setting, Source, guid_str, parse_guid};

/// What the spike changed, so `restore` can undo it after a crash or kill. A value is saved before
/// it is written; a copy right after it is made (its GUID does not exist before).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Journal {
    pub active_before: Option<GUID>,
    pub values: Vec<Value>,
    pub copies: Vec<GUID>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    pub scheme: GUID,
    pub setting: Setting,
    pub source: Source,
    pub original: u32,
    pub written: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    WriteBack {
        scheme: GUID,
        setting: Setting,
        source: Source,
        value: u32,
    },
    KeepUsers {
        scheme: GUID,
        setting: Setting,
        source: Source,
        current: u32,
    },
    SetActive(GUID),
    DeleteCopy(GUID),
}

impl Journal {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        if let Some(g) = &self.active_before {
            out.push_str(&format!("active-before {}\n", guid_str(g)));
        }
        for v in &self.values {
            out.push_str(&format!(
                "value {} {} {} {} {}\n",
                guid_str(&v.scheme),
                v.setting.name,
                v.source.name(),
                v.original,
                v.written
            ));
        }
        for c in &self.copies {
            out.push_str(&format!("copy {}\n", guid_str(c)));
        }
        out
    }

    pub fn parse(text: &str) -> Result<Journal, String> {
        let mut j = Journal::default();
        for (n, line) in text.lines().enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let bad = || format!("journal line {}: {line:?}", n + 1);
            match parts.as_slice() {
                [] => {}
                ["active-before", g] => j.active_before = Some(parse_guid(g).ok_or_else(bad)?),
                ["value", scheme, name, src, original, written] => j.values.push(Value {
                    scheme: parse_guid(scheme).ok_or_else(bad)?,
                    setting: power::setting_by_name(name).ok_or_else(bad)?,
                    source: Source::parse(src).ok_or_else(bad)?,
                    original: original.parse().map_err(|_| bad())?,
                    written: written.parse().map_err(|_| bad())?,
                }),
                ["copy", g] => j.copies.push(parse_guid(g).ok_or_else(bad)?),
                _ => return Err(bad()),
            }
        }
        Ok(j)
    }

    /// Record a value before it is written. Keeps the first original, so a second write of the same
    /// field never saves our own value as the user's.
    pub fn record(
        &mut self,
        scheme: GUID,
        setting: Setting,
        source: Source,
        original: u32,
        written: u32,
    ) {
        if let Some(v) = self
            .values
            .iter_mut()
            .find(|v| v.scheme == scheme && v.setting == setting && v.source == source)
        {
            v.written = written;
        } else {
            self.values.push(Value {
                scheme,
                setting,
                source,
                original,
                written,
            });
        }
    }

    /// Restore only what still holds our value; anything else was changed by the user or another
    /// tool and stays as it is. `usable` lists the plans that exist now and are not spike copies.
    pub fn plan(
        &self,
        active_now: GUID,
        usable: &[GUID],
        current: impl Fn(&Value) -> Option<u32>,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        for v in &self.values {
            match current(v) {
                Some(c) if c == v.written => actions.push(Action::WriteBack {
                    scheme: v.scheme,
                    setting: v.setting,
                    source: v.source,
                    value: v.original,
                }),
                Some(c) => actions.push(Action::KeepUsers {
                    scheme: v.scheme,
                    setting: v.setting,
                    source: v.source,
                    current: c,
                }),
                // The scheme is gone; nothing to restore there.
                None => {}
            }
        }
        if self.copies.contains(&active_now) {
            if let Some(target) = self.switch_target(usable) {
                actions.push(Action::SetActive(target));
            }
        } else if self.values.iter().any(|v| v.scheme == active_now) {
            // Values on the active scheme apply only after it is set active again.
            actions.push(Action::SetActive(active_now));
        }
        for c in &self.copies {
            actions.push(Action::DeleteCopy(*c));
        }
        actions
    }

    /// The plan that was active before can be gone, or be another spike's copy: switching to it
    /// would fail or leave a test plan active, and the active copy could then not be deleted.
    fn switch_target(&self, usable: &[GUID]) -> Option<GUID> {
        let ok = |g: &GUID| usable.contains(g) && !self.copies.contains(g);
        self.active_before
            .filter(ok)
            .or(Some(power::SCHEME_BALANCED).filter(ok))
            .or_else(|| usable.iter().copied().find(|g| !self.copies.contains(g)))
    }
}

pub fn load(path: &Path) -> Result<Option<Journal>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Journal::parse(&text).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn save(path: &Path, j: &Journal) {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, j.to_text()).expect("write journal");
    std::fs::rename(&tmp, path).expect("rename journal");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::{LID, SCHEME_BALANCED, SLEEP};

    const COPY: GUID = GUID::from_u128(0x11111111_2222_3333_4444_555555555555);

    fn sample() -> Journal {
        let mut j = Journal {
            active_before: Some(SCHEME_BALANCED),
            ..Default::default()
        };
        j.record(SCHEME_BALANCED, LID, Source::Ac, 1, 0);
        j.record(SCHEME_BALANCED, SLEEP, Source::Dc, 600, 607);
        j.copies.push(COPY);
        j
    }

    #[test]
    fn text_round_trip() {
        let j = sample();
        assert_eq!(Journal::parse(&j.to_text()).unwrap(), j);
    }

    #[test]
    fn bad_line_is_an_error() {
        assert!(Journal::parse("value x lid ac 1 0").is_err());
        assert!(Journal::parse("what").is_err());
    }

    #[test]
    fn second_write_keeps_first_original() {
        let mut j = sample();
        j.record(SCHEME_BALANCED, LID, Source::Ac, 0, 2);
        let v = &j.values[0];
        assert_eq!((v.original, v.written), (1, 2));
        assert_eq!(j.values.len(), 2);
    }

    #[test]
    fn restore_keeps_user_changes() {
        let j = Journal {
            copies: vec![],
            ..sample()
        };
        // Lid still holds our 0; the user moved the battery timer to 900.
        let plan = j.plan(SCHEME_BALANCED, &[SCHEME_BALANCED], |v| {
            Some(if v.setting == LID { 0 } else { 900 })
        });
        assert_eq!(
            plan,
            vec![
                Action::WriteBack {
                    scheme: SCHEME_BALANCED,
                    setting: LID,
                    source: Source::Ac,
                    value: 1
                },
                Action::KeepUsers {
                    scheme: SCHEME_BALANCED,
                    setting: SLEEP,
                    source: Source::Dc,
                    current: 900
                },
                Action::SetActive(SCHEME_BALANCED),
            ]
        );
    }

    #[test]
    fn restore_leaves_copy_before_deleting_it() {
        let j = Journal {
            values: vec![],
            ..sample()
        };
        let plan = j.plan(COPY, &[SCHEME_BALANCED], |_| None);
        assert_eq!(
            plan,
            vec![Action::SetActive(SCHEME_BALANCED), Action::DeleteCopy(COPY)]
        );
    }

    #[test]
    fn restore_does_not_switch_plan_the_user_picked() {
        let other = GUID::from_u128(0x99);
        let j = Journal {
            values: vec![],
            ..sample()
        };
        assert_eq!(
            j.plan(other, &[SCHEME_BALANCED, other], |_| None),
            vec![Action::DeleteCopy(COPY)]
        );
    }

    #[test]
    fn restore_falls_back_when_the_plan_before_is_gone_or_a_spike_copy() {
        let gone = GUID::from_u128(0x77);
        let j = Journal {
            active_before: Some(gone),
            copies: vec![COPY],
            values: vec![],
        };
        assert_eq!(
            j.plan(COPY, &[SCHEME_BALANCED], |_| None),
            vec![Action::SetActive(SCHEME_BALANCED), Action::DeleteCopy(COPY)]
        );
        let other = GUID::from_u128(0x99);
        assert_eq!(
            j.plan(COPY, &[other], |_| None),
            vec![Action::SetActive(other), Action::DeleteCopy(COPY)]
        );
    }
}
