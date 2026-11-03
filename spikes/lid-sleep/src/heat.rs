use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW,
};
use windows::core::{HSTRING, PCWSTR};

/// The ACPI thermal zone counters, read through PDH (the same data as the WMI class
/// `Win32_PerfFormattedData_Counters_ThermalZoneInformation`, without COM).
pub struct Thermal {
    query: PDH_HQUERY,
    counters: Vec<(&'static str, PDH_HCOUNTER)>,
}

pub const COUNTERS: [(&str, &str); 4] = [
    ("temp_k", r"\Thermal Zone Information(*)\Temperature"),
    (
        "hp_temp_dk",
        r"\Thermal Zone Information(*)\High Precision Temperature",
    ),
    (
        "passive_limit_pct",
        r"\Thermal Zone Information(*)\% Passive Limit",
    ),
    (
        "throttle_reasons",
        r"\Thermal Zone Information(*)\Throttle Reasons",
    ),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub counter: &'static str,
    pub zone: String,
    pub value: f64,
    pub status: u32,
}

pub fn kelvin_to_c(k: f64) -> f64 {
    k - 273.15
}

impl Thermal {
    pub fn open() -> Result<Thermal, String> {
        let mut q = PDH_HQUERY::default();
        let rc = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut q) };
        if rc != 0 {
            return Err(format!("PdhOpenQueryW {rc:#x}"));
        }
        let mut counters = Vec::new();
        for (name, path) in COUNTERS {
            let mut c = PDH_HCOUNTER::default();
            let rc = unsafe { PdhAddEnglishCounterW(q, &HSTRING::from(path), 0, &mut c) };
            if rc == 0 {
                counters.push((name, c));
            } else {
                crate::log!("heat", "PdhAddEnglishCounterW({path}) {rc:#x}");
            }
        }
        Ok(Thermal { query: q, counters })
    }

    pub fn sample(&self) -> Result<Vec<Reading>, String> {
        let rc = unsafe { PdhCollectQueryData(self.query) };
        if rc != 0 {
            return Err(format!("PdhCollectQueryData {rc:#x}"));
        }
        let mut out = Vec::new();
        for (name, c) in &self.counters {
            let mut size = 0u32;
            let mut count = 0u32;
            let rc = unsafe {
                PdhGetFormattedCounterArrayW(*c, PDH_FMT_DOUBLE, &mut size, &mut count, None)
            };
            if rc != PDH_MORE_DATA {
                out.push(Reading {
                    counter: name,
                    zone: "<none>".into(),
                    value: f64::NAN,
                    status: rc,
                });
                continue;
            }
            // u64 keeps the buffer aligned for the item structs Windows writes into it.
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            let rc = unsafe {
                PdhGetFormattedCounterArrayW(*c, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items))
            };
            if rc != 0 {
                out.push(Reading {
                    counter: name,
                    zone: "<none>".into(),
                    value: f64::NAN,
                    status: rc,
                });
                continue;
            }
            for i in 0..count as usize {
                let it = unsafe { &*items.add(i) };
                let zone = unsafe { it.szName.to_string() }.unwrap_or_default();
                out.push(Reading {
                    counter: name,
                    zone,
                    value: unsafe { it.FmtValue.Anonymous.doubleValue },
                    status: it.FmtValue.CStatus,
                });
            }
        }
        Ok(out)
    }
}

/// Default for `--heat-limit`: well above what this laptop reached under full load with the lid
/// open (67 C), well below its critical trip point (113 C).
pub const DEFAULT_LIMIT_C: f64 = 85.0;

/// The hottest zone in C, so a closed-lid run can stop before the laptop gets hot.
pub fn max_c(r: &[Reading]) -> Option<f64> {
    let hp = r
        .iter()
        .filter(|x| x.counter == "hp_temp_dk" && x.value.is_finite())
        .map(|x| kelvin_to_c(x.value / 10.0));
    let plain = r
        .iter()
        .filter(|x| x.counter == "temp_k" && x.value.is_finite())
        .map(|x| kelvin_to_c(x.value));
    let best = hp.fold(None, |m: Option<f64>, c| Some(m.map_or(c, |m| m.max(c))));
    best.or_else(|| plain.fold(None, |m: Option<f64>, c| Some(m.map_or(c, |m| m.max(c)))))
}

pub fn summary(r: &[Reading]) -> String {
    let mut parts = Vec::new();
    for x in r.iter().filter(|x| x.counter == "hp_temp_dk") {
        let c = kelvin_to_c(x.value / 10.0);
        let limit = r
            .iter()
            .find(|y| y.counter == "passive_limit_pct" && y.zone == x.zone)
            .map(|y| format!("{:.0}%", y.value))
            .unwrap_or_else(|| "?".into());
        parts.push(format!("{} {c:.1} C limit {limit}", x.zone));
    }
    if parts.is_empty() {
        for x in r.iter().filter(|x| x.counter == "temp_k") {
            parts.push(format!("{} {:.1} C", x.zone, kelvin_to_c(x.value)));
        }
    }
    if parts.is_empty() {
        "<no thermal zone>".into()
    } else {
        parts.join(", ")
    }
}

impl Drop for Thermal {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(counter: &'static str, zone: &str, value: f64) -> Reading {
        Reading {
            counter,
            zone: zone.into(),
            value,
            status: 0,
        }
    }

    #[test]
    fn summary_prefers_high_precision() {
        let s = summary(&[
            r("temp_k", r"\_TZ.THM0", 323.0),
            r("hp_temp_dk", r"\_TZ.THM0", 3231.0),
            r("passive_limit_pct", r"\_TZ.THM0", 100.0),
        ]);
        assert_eq!(s, r"\_TZ.THM0 50.0 C limit 100%");
    }

    #[test]
    fn summary_falls_back_to_kelvin() {
        assert_eq!(summary(&[r("temp_k", "z", 300.15)]), "z 27.0 C");
        assert_eq!(summary(&[]), "<no thermal zone>");
    }

    #[test]
    fn max_takes_the_hottest_zone() {
        let m = max_c(&[
            r("hp_temp_dk", "a", 3231.0),
            r("hp_temp_dk", "b", 3631.5),
            r("temp_k", "a", 400.0),
        ]);
        assert!((m.unwrap() - 90.0).abs() < 0.01, "{m:?}");
    }

    #[test]
    fn max_falls_back_to_kelvin_and_skips_bad_reads() {
        let m = max_c(&[
            r("hp_temp_dk", "<none>", f64::NAN),
            r("temp_k", "a", 323.15),
        ]);
        assert!((m.unwrap() - 50.0).abs() < 0.01, "{m:?}");
        assert_eq!(max_c(&[]), None);
    }
}
