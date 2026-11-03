//! Power plan reads and writes: `Win32Power` and `DryRunPower`, the Windows forms of the core's
//! `ownership::io::PlanIo` (architecture section 5, "Power settings").
//!
//! Owner: power track (`p3-power-plan`).

use wakeshot_core::settings::SettingKey;

/// A plan value the IO thread reads. Wider than `SettingKey` because `PlanValues` also needs AC sleep and the energy
/// saver threshold, which Wakeshot reads but never writes.
///
/// Owner: power track (Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanSetting {
    LidAc,
    LidDc,
    SleepAc,
    SleepDc,
    SaverThresholdDc,
}

impl From<SettingKey> for PlanSetting {
    fn from(key: SettingKey) -> Self {
        match key {
            SettingKey::LidAc => Self::LidAc,
            SettingKey::LidDc => Self::LidDc,
            SettingKey::SleepDc => Self::SleepDc,
        }
    }
}
