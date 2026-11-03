use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Internal,
    External,
    /// Miracast and indirect displays (remote, virtual). Not a docked screen for our purpose.
    Other,
}

/// Output technology values from `DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY` (wingdi.h).
pub fn kind(tech: i32) -> Kind {
    match tech {
        // INTERNAL (0x80000000), LVDS, DISPLAYPORT_EMBEDDED, UDI_EMBEDDED.
        i32::MIN | 6 | 11 | 13 => Kind::Internal,
        // MIRACAST, INDIRECT_WIRED, INDIRECT_VIRTUAL, DISPLAYPORT_USB_TUNNEL is 18 and external.
        15..=17 => Kind::Other,
        -1 => Kind::Other,
        _ => Kind::External,
    }
}

pub fn tech_name(tech: i32) -> &'static str {
    match tech {
        -1 => "OTHER",
        0 => "HD15",
        1 => "SVIDEO",
        2 => "COMPOSITE",
        3 => "COMPONENT",
        4 => "DVI",
        5 => "HDMI",
        6 => "LVDS",
        8 => "D_JPN",
        9 => "SDI",
        10 => "DISPLAYPORT_EXTERNAL",
        11 => "DISPLAYPORT_EMBEDDED",
        12 => "UDI_EXTERNAL",
        13 => "UDI_EMBEDDED",
        14 => "SDTVDONGLE",
        15 => "MIRACAST",
        16 => "INDIRECT_WIRED",
        17 => "INDIRECT_VIRTUAL",
        18 => "DISPLAYPORT_USB_TUNNEL",
        i32::MIN => "INTERNAL",
        _ => "unknown",
    }
}

#[derive(Clone, Debug)]
pub struct Target {
    pub tech: i32,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Count {
    pub internal: u32,
    pub external: u32,
    pub other: u32,
    pub sm_cmonitors: i32,
}

impl Count {
    pub fn short(&self) -> String {
        format!(
            "int {} ext {} other {} (SM_CMONITORS {})",
            self.internal, self.external, self.other, self.sm_cmonitors
        )
    }
}

pub fn count(targets: &[Target]) -> Count {
    let mut c = Count::default();
    for t in targets {
        match kind(t.tech) {
            Kind::Internal => c.internal += 1,
            Kind::External => c.external += 1,
            Kind::Other => c.other += 1,
        }
    }
    c.sm_cmonitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
    c
}

/// Active display paths from the display config API. No admin, no WMI.
pub fn active_targets() -> Result<Vec<Target>, u32> {
    for _ in 0..3 {
        let mut np = 0u32;
        let mut nm = 0u32;
        let rc = unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) };
        if rc.0 != 0 {
            return Err(rc.0);
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
        let rc = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut np,
                paths.as_mut_ptr(),
                &mut nm,
                modes.as_mut_ptr(),
                None,
            )
        };
        // The topology can change between the two calls.
        if rc == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if rc.0 != 0 {
            return Err(rc.0);
        }
        paths.truncate(np as usize);
        return Ok(paths
            .iter()
            .map(|p| Target {
                tech: p.targetInfo.outputTechnology.0,
                name: target_name(p),
            })
            .collect());
    }
    Err(ERROR_INSUFFICIENT_BUFFER.0)
}

fn target_name(p: &DISPLAYCONFIG_PATH_INFO) -> String {
    let mut n = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
    n.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
    n.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
    n.header.adapterId = p.targetInfo.adapterId;
    n.header.id = p.targetInfo.id;
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut n.header) };
    if rc != 0 {
        return format!("<name rc {rc}>");
    }
    let len = n
        .monitorFriendlyDeviceName
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(64);
    let name = String::from_utf16_lossy(&n.monitorFriendlyDeviceName[..len]);
    if name.is_empty() {
        "<no friendly name>".into()
    } else {
        name
    }
}

pub fn now() -> String {
    match active_targets() {
        Ok(t) => count(&t).short(),
        Err(rc) => format!("<QueryDisplayConfig rc {rc}>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(tech: i32) -> Target {
        Target {
            tech,
            name: String::new(),
        }
    }

    #[test]
    fn internal_panels() {
        for tech in [i32::MIN, 6, 11, 13] {
            assert_eq!(kind(tech), Kind::Internal, "{tech}");
        }
    }

    #[test]
    fn external_screens() {
        for tech in [0, 4, 5, 10, 12, 18] {
            assert_eq!(kind(tech), Kind::External, "{tech}");
        }
    }

    #[test]
    fn virtual_screens_are_not_docked() {
        for tech in [15, 16, 17, -1] {
            assert_eq!(kind(tech), Kind::Other, "{tech}");
        }
    }

    #[test]
    fn counts_by_kind() {
        let c = count(&[t(i32::MIN), t(10), t(5), t(17)]);
        assert_eq!((c.internal, c.external, c.other), (1, 2, 1));
    }
}
