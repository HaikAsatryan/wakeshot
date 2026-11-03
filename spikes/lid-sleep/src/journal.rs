use std::path::Path;

use windows::core::GUID;

use crate::power::{guid_str, parse_guid};

/// Windows' built-in Balanced plan: the fallback when the plan that was active before is gone.
pub const SCHEME_BALANCED: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);

/// The spike never writes the user's own plan, only temporary copies. This file lists each copy
/// right after it is made (its GUID does not exist before), so `restore` can undo a run that was
/// killed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Journal {
    pub active_before: Option<GUID>,
    pub copies: Vec<GUID>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    SetActive(GUID),
    DeleteCopy(GUID),
}

impl Journal {
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        if let Some(g) = &self.active_before {
            out.push_str(&format!("active-before {}\n", guid_str(g)));
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
                ["copy", g] => j.copies.push(parse_guid(g).ok_or_else(bad)?),
                _ => return Err(bad()),
            }
        }
        Ok(j)
    }

    /// Leave a copy before deleting it (Windows refuses to delete the active plan), and do not
    /// switch away from a plan the user picked in the meantime. `usable` lists the plans that
    /// exist now and are not spike copies.
    pub fn plan(&self, active_now: GUID, usable: &[GUID]) -> Vec<Action> {
        let mut actions = Vec::new();
        if self.copies.contains(&active_now)
            && let Some(target) = self.switch_target(usable)
        {
            actions.push(Action::SetActive(target));
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
            .or(Some(SCHEME_BALANCED).filter(ok))
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
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, j.to_text()).expect("write journal");
    std::fs::rename(&tmp, path).expect("rename journal");
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);
    const COPY: GUID = GUID::from_u128(0x11111111_2222_3333_4444_555555555555);

    fn sample() -> Journal {
        Journal {
            active_before: Some(USER),
            copies: vec![COPY],
        }
    }

    #[test]
    fn text_round_trip() {
        let j = sample();
        assert_eq!(Journal::parse(&j.to_text()).unwrap(), j);
        assert_eq!(Journal::parse("").unwrap(), Journal::default());
    }

    #[test]
    fn bad_line_is_an_error() {
        assert!(Journal::parse("copy nope").is_err());
        assert!(Journal::parse("value x lid ac 1 0").is_err());
    }

    #[test]
    fn leaves_copy_before_deleting_it() {
        assert_eq!(
            sample().plan(COPY, &[USER]),
            vec![Action::SetActive(USER), Action::DeleteCopy(COPY)]
        );
    }

    #[test]
    fn keeps_a_plan_the_user_picked() {
        let other = GUID::from_u128(0x99);
        let usable = [USER, other];
        assert_eq!(
            sample().plan(other, &usable),
            vec![Action::DeleteCopy(COPY)]
        );
        assert_eq!(sample().plan(USER, &usable), vec![Action::DeleteCopy(COPY)]);
    }

    // The review's case: the lid copy was made from the power spike's copy, which `spike restore`
    // then deleted.
    #[test]
    fn falls_back_to_balanced_when_the_plan_before_is_gone() {
        let gone = GUID::from_u128(0x77);
        let j = Journal {
            active_before: Some(gone),
            copies: vec![COPY],
        };
        let other = GUID::from_u128(0x99);
        assert_eq!(
            j.plan(COPY, &[other, SCHEME_BALANCED]),
            vec![Action::SetActive(SCHEME_BALANCED), Action::DeleteCopy(COPY)]
        );
        assert_eq!(
            j.plan(COPY, &[other]),
            vec![Action::SetActive(other), Action::DeleteCopy(COPY)]
        );
        assert_eq!(j.plan(COPY, &[]), vec![Action::DeleteCopy(COPY)]);
    }

    // The plan before still exists but is the power spike's copy, so it is not in `usable`.
    #[test]
    fn never_switches_back_to_another_spike_copy() {
        let power_copy = GUID::from_u128(0x55);
        let j = Journal {
            active_before: Some(power_copy),
            copies: vec![COPY],
        };
        assert_eq!(
            j.plan(COPY, &[SCHEME_BALANCED]),
            vec![Action::SetActive(SCHEME_BALANCED), Action::DeleteCopy(COPY)]
        );
    }
}
