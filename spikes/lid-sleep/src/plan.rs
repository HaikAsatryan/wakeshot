use std::path::PathBuf;

use windows::core::GUID;

use crate::journal::{self, Action, Journal};
use crate::power::{self, BOTH, Setting, Source, guid_str};
use crate::sys;

pub fn journal_path() -> PathBuf {
    crate::out_dir().join("journal.txt")
}

/// Every spike copy, from either spike, is named so. A copy is named right after it is made, so
/// for a moment it still carries the user's plan name.
pub const SPIKE_PREFIX: &str = "Wakeshot spike";

/// The power spike's journal. While it exists, the active plan may be that spike's copy.
pub fn power_journal_path() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("power-settings")
        .join("out")
        .join("journal.txt")
}

pub fn is_spike_plan(g: &GUID) -> bool {
    power::friendly_name(g).starts_with(SPIKE_PREFIX)
}

/// Plans that exist now and are not spike copies: the only ones restore may switch to.
fn usable_plans() -> Vec<GUID> {
    power::schemes()
        .into_iter()
        .filter(|g| !is_spike_plan(g))
        .collect()
}

/// Logs the active plan, every plan, and the watched values, through the API and `powercfg`.
pub fn log_values(tag: &str) {
    let active = power::active_scheme();
    match &active {
        Ok(g) => crate::log!(
            tag,
            "active plan {} ({})",
            guid_str(g),
            power::friendly_name(g)
        ),
        Err(rc) => crate::log!(tag, "active plan: rc {rc}"),
    }
    for g in power::schemes() {
        crate::log!(tag, "plan {} ({})", guid_str(&g), power::friendly_name(&g));
    }
    if let Ok(g) = active {
        for s in power::WATCHED {
            let v: Vec<String> = BOTH
                .iter()
                .map(|src| match power::read(&g, s, *src) {
                    Ok(v) => format!("{} {v}", src.name()),
                    Err(rc) => format!("{} <rc {rc}>", src.name()),
                })
                .collect();
            let note = if s == power::LID {
                let names: Vec<&str> = BOTH
                    .iter()
                    .filter_map(|src| power::read(&g, s, *src).ok())
                    .map(power::lid_action_name)
                    .collect();
                format!(" ({})", names.join(", "))
            } else {
                String::new()
            };
            crate::log!(tag, "{}: {}{note}", s.name, v.join(", "));
        }
    }
    for (q, sub, setting) in [
        ("/qh", "SUB_BUTTONS", "LIDACTION"),
        ("/q", "SUB_SLEEP", "STANDBYIDLE"),
        ("/q", "SUB_VIDEO", "VIDEOIDLE"),
        ("/qh", "SUB_SLEEP", "RTCWAKE"),
    ] {
        let (rc, out) = sys::run("powercfg", &[q, "SCHEME_CURRENT", sub, setting]);
        let current: Vec<&str> = out
            .lines()
            .filter(|l| l.contains("Current"))
            .map(str::trim)
            .collect();
        crate::log!(
            tag,
            "powercfg {q} SCHEME_CURRENT {sub} {setting} rc {rc}: {}",
            current.join(" | ")
        );
    }
}

/// A temporary copy of the user's active plan. The user's own plan is never written; restore is
/// one switch back plus one delete.
pub struct PlanCopy {
    pub original: GUID,
    pub copy: GUID,
    done: bool,
}

impl PlanCopy {
    pub fn make(name: &str, writes: &[(Setting, Source, u32)]) -> Result<PlanCopy, String> {
        if let Ok(Some(_)) = journal::load(&journal_path()) {
            return Err("out/journal.txt exists. Run `restore` first.".into());
        }
        // A copy of the power spike's copy would carry its test values, and restore would put
        // that copy back instead of the user's plan.
        if power_journal_path().exists() {
            return Err(format!(
                "the power spike's journal {} exists. Run its `restore` first.",
                power_journal_path().display()
            ));
        }
        let original = power::active_scheme().map_err(|rc| format!("active scheme rc {rc}"))?;
        if is_spike_plan(&original) {
            return Err(format!(
                "the active plan {} ({}) is a spike copy. Run both restores first.",
                guid_str(&original),
                power::friendly_name(&original)
            ));
        }
        for g in power::schemes().iter().filter(|g| is_spike_plan(g)) {
            crate::log!(
                "plan",
                "warning: a spike copy is left over: {} ({})",
                guid_str(g),
                power::friendly_name(g)
            );
        }
        let copy =
            power::duplicate(&original).map_err(|rc| format!("PowerDuplicateScheme rc {rc}"))?;
        journal::save(
            &journal_path(),
            &Journal {
                active_before: Some(original),
                copies: vec![copy],
            },
        );
        let mut pc = PlanCopy {
            original,
            copy,
            done: false,
        };
        let rc = power::write_friendly_name(&copy, name);
        crate::log!(
            "plan",
            "copy {} of {} made, named (rc {rc})",
            guid_str(&copy),
            guid_str(&original)
        );
        for (s, src, v) in writes {
            let rc = power::write(&copy, *s, *src, *v);
            crate::log!("plan", "copy {}.{} = {v}: rc {rc}", s.name, src.name());
            if rc != 0 {
                pc.restore();
                return Err(format!("write {}.{} rc {rc}", s.name, src.name()));
            }
        }
        let rc = power::set_active(&copy);
        crate::log!("plan", "PowerSetActiveScheme(copy) rc {rc}");
        if rc != 0 {
            pc.restore();
            return Err(format!("PowerSetActiveScheme rc {rc}"));
        }
        Ok(pc)
    }

    /// Safe to call more than once.
    pub fn restore(&mut self) -> bool {
        if self.done {
            return true;
        }
        let j = Journal {
            active_before: Some(self.original),
            copies: vec![self.copy],
        };
        let ok = restore_journal(&j);
        if ok {
            let _ = std::fs::remove_file(journal_path());
            crate::log!(
                "plan",
                "restored: user's plan active, copy deleted, journal removed"
            );
            self.done = true;
        } else {
            crate::log!("plan", "RESTORE FAILED: journal kept, run `restore`");
        }
        ok
    }
}

impl Drop for PlanCopy {
    fn drop(&mut self) {
        self.restore();
    }
}

pub fn restore_journal(j: &Journal) -> bool {
    let active = power::active_scheme().unwrap_or(GUID::zeroed());
    let mut ok = true;
    for a in j.plan(active, &usable_plans()) {
        match a {
            Action::SetActive(g) => {
                let rc = power::set_active(&g);
                crate::log!("restore", "PowerSetActiveScheme({}) rc {rc}", guid_str(&g));
                ok &= rc == 0;
            }
            Action::DeleteCopy(g) => {
                let rc = power::delete(&g);
                crate::log!("restore", "PowerDeleteScheme({}) rc {rc}", guid_str(&g));
                // 2: already gone.
                ok &= rc == 0 || rc == 2;
            }
        }
    }
    let left = power::schemes().into_iter().any(|s| j.copies.contains(&s));
    let active_now = power::active_scheme().ok();
    let on_copy = active_now.is_some_and(|a| j.copies.contains(&a));
    crate::log!(
        "restore",
        "check: active {}, copy left {left}",
        active_now.map(|g| guid_str(&g)).unwrap_or_default()
    );
    ok && !left && !on_copy
}

pub fn restore_mode() -> i32 {
    match journal::load(&journal_path()) {
        Ok(None) => {
            crate::log!("restore", "no journal: nothing to restore");
            0
        }
        Ok(Some(j)) => {
            if restore_journal(&j) {
                let _ = std::fs::remove_file(journal_path());
                crate::log!("restore", "done, journal removed");
                0
            } else {
                crate::log!("restore", "FAILED, journal kept");
                1
            }
        }
        Err(e) => {
            crate::log!("restore", "bad journal: {e}");
            1
        }
    }
}
