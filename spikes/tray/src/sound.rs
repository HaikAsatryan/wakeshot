use windows::{
    Win32::{
        Foundation::HMODULE,
        Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT, SND_RESOURCE, SND_SYSTEM},
    },
    core::{HSTRING, PCWSTR, w},
};

use crate::theme::reg_string;

/// Plays a sound from the user's sound scheme. SND_SYSTEM puts it under the "System Sounds" volume slider;
/// SND_NODEFAULT stops Windows from playing the default beep when the event has no sound.
pub fn play_alias(alias: &str) -> bool {
    let a = HSTRING::from(alias);
    unsafe {
        PlaySoundW(
            PCWSTR(a.as_ptr()),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT | SND_SYSTEM,
        )
    }
    .as_bool()
}

/// Plays the WAVE resource "CHIME" embedded in the exe.
pub fn play_bundled(module: HMODULE) -> bool {
    unsafe {
        PlaySoundW(
            w!("CHIME"),
            Some(module),
            SND_RESOURCE | SND_ASYNC | SND_NODEFAULT | SND_SYSTEM,
        )
    }
    .as_bool()
}

/// The file the user's current scheme maps to an event; empty means the user set it to "(None)".
pub fn scheme_file(event: &str) -> Option<String> {
    let key = HSTRING::from(format!(
        "AppEvents\\Schemes\\Apps\\.Default\\{event}\\.Current"
    ));
    reg_string(false, PCWSTR(key.as_ptr()), PCWSTR::null())
}
