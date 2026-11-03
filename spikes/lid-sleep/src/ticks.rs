use std::time::SystemTime;

use crate::{display, heat, sys};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// Unbiased time skipped too: Windows counted it as sleep (S3, S4 or a Modern Standby phase
    /// that stops it).
    Slept,
    /// Unbiased time ran on but our process did not: paused, for example by the Desktop Activity
    /// Moderator in Modern Standby.
    Paused,
}

/// A step is a gap when the wall clock moved at least 2 s more than the tick asked for.
pub fn classify(interval_ms: i64, wall_ms: i64, unbiased_ms: i64) -> Option<Gap> {
    let late = wall_ms - interval_ms;
    if late < 2000 {
        return None;
    }
    if wall_ms - unbiased_ms > late / 2 {
        Some(Gap::Slept)
    } else {
        Some(Gap::Paused)
    }
}

#[derive(Clone, Debug)]
pub struct GapInfo {
    pub kind: Gap,
    /// Wall clock ms since the Unix epoch when the gap began (the tick before it).
    pub start_ms: u64,
    pub len_ms: i64,
    pub text: String,
}

pub struct Ticker {
    interval_ms: u64,
    start_wall: SystemTime,
    last_wall: SystemTime,
    last_unbiased: u64,
    last_interrupt: u64,
    thermal: Option<heat::Thermal>,
    n: u64,
    pub gaps: Vec<GapInfo>,
    /// Hottest zone at the last thermal read (every fifth tick).
    pub heat_c: Option<f64>,
}

impl Ticker {
    pub fn new(interval_ms: u64) -> Ticker {
        let thermal = match heat::Thermal::open() {
            Ok(t) => Some(t),
            Err(e) => {
                crate::log!("tick", "no thermal reads: {e}");
                None
            }
        };
        let now = SystemTime::now();
        Ticker {
            interval_ms,
            start_wall: now,
            last_wall: now,
            last_unbiased: sys::unbiased_ms(),
            last_interrupt: sys::interrupt_ms(),
            thermal,
            n: 0,
            gaps: Vec::new(),
            heat_c: None,
        }
    }

    pub fn elapsed_s(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.start_wall)
            .unwrap_or_default()
            .as_secs()
    }

    /// Logs one line of state and returns a gap if the PC slept or our process was paused.
    pub fn tick(&mut self, phase: &str) -> Option<Gap> {
        self.n += 1;
        let wall = SystemTime::now();
        let unb = sys::unbiased_ms();
        let intr = sys::interrupt_ms();
        let wall_ms = wall
            .duration_since(self.last_wall)
            .unwrap_or_default()
            .as_millis() as i64;
        let unb_ms = (unb - self.last_unbiased) as i64;
        let intr_ms = (intr - self.last_interrupt) as i64;
        self.last_wall = wall;
        self.last_unbiased = unb;
        self.last_interrupt = intr;
        let ps = sys::power_status();
        let lock = match sys::session_locked() {
            Some(true) => "locked",
            Some(false) => "unlocked",
            None => "?",
        };
        let spi = match sys::system_power_information() {
            Ok(i) => format!("{} s", i.TimeRemaining),
            Err(st) => format!("<{st:#x}>"),
        };
        let es = match sys::system_execution_state() {
            Ok(v) => format!("{v:#x}"),
            Err(st) => format!("<{st:#x}>"),
        };
        let temp = match (&self.thermal, self.n % 5 == 1) {
            (Some(t), true) => match t.sample() {
                Ok(r) => {
                    self.heat_c = heat::max_c(&r);
                    format!(", heat {}", heat::summary(&r))
                }
                Err(e) => format!(", heat {e}"),
            },
            _ => String::new(),
        };
        crate::log!(
            "tick",
            "[{phase}] t {} s, step wall {wall_ms} unbiased {unb_ms} interrupt {intr_ms} ms, idle {} s, AC {}, bat {}%, {lock}, displays {}, idle-timer left {spi}, exec-state {es}{temp}",
            self.elapsed_s(),
            sys::idle_ms() / 1000,
            ps.on_ac,
            ps.battery_percent,
            display::now(),
        );
        let gap = classify(self.interval_ms as i64, wall_ms, unb_ms);
        if let Some(g) = gap {
            let text = format!(
                "GAP {g:?} [{phase}] ending {}: wall {wall_ms} ms, unbiased {unb_ms} ms, interrupt {intr_ms} ms",
                crate::log::wall_clock()
            );
            crate::log!("tick", "{text}");
            let end_ms = wall
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.gaps.push(GapInfo {
                kind: g,
                start_ms: end_ms.saturating_sub(wall_ms as u64),
                len_ms: wall_ms,
                text,
            });
        }
        gap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_step_is_no_gap() {
        assert_eq!(classify(1000, 1000, 1000), None);
        assert_eq!(classify(1000, 2900, 2900), None);
    }

    #[test]
    fn sleep_stops_unbiased_time() {
        assert_eq!(classify(1000, 61_000, 1_100), Some(Gap::Slept));
    }

    #[test]
    fn pause_keeps_unbiased_time() {
        assert_eq!(classify(1000, 61_000, 60_900), Some(Gap::Paused));
    }
}
