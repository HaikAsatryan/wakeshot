use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_ELEVATION,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenElevation, TokenIntegrityLevel,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Elevation and integrity level of this process. An elevated tray app does not get Explorer's
/// `TaskbarCreated` broadcast (UIPI), so the Explorer restart test is only valid at Medium.
pub struct TokenInfo {
    pub elevated: bool,
    pub integrity_rid: u32,
}

impl TokenInfo {
    pub fn integrity_name(&self) -> &'static str {
        match self.integrity_rid {
            0x1000 => "Low",
            0x2000 => "Medium",
            0x2100 => "Medium Plus",
            0x3000 => "High",
            0x4000 => "System",
            _ => "other",
        }
    }
}

pub fn info() -> TokenInfo {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return TokenInfo {
                elevated: false,
                integrity_rid: 0,
            };
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let _ = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        );
        let mut buf = vec![0u8; 256];
        let mut rid = 0;
        if GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buf.as_mut_ptr() as *mut _),
            buf.len() as u32,
            &mut len,
        )
        .is_ok()
        {
            let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
            let sid = label.Label.Sid;
            let count = *GetSidSubAuthorityCount(sid);
            rid = *GetSidSubAuthority(sid, (count - 1) as u32);
        }
        let _ = CloseHandle(token);
        TokenInfo {
            elevated: elevation.TokenIsElevated != 0,
            integrity_rid: rid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_own_token() {
        let t = info();
        assert_ne!(t.integrity_name(), "other", "rid 0x{:x}", t.integrity_rid);
    }
}
