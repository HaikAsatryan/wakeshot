//! The device rule (architecture section 2, `SoundPlayer`): the default playback device, unless it is a screen's
//! audio (HDMI or DisplayPort, which often has no speakers); then the speakers. Headphones and headsets stay as they
//! are, so someone wearing them gets no sound out loud.
//!
//! Owner: `p5-sounds`. Ported from `spikes/lid-sleep/src/sound.rs` (`pick`, the form factor reads).

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{ERROR_NOT_FOUND, PROPERTYKEY, WIN32_ERROR};
use windows::Win32::Media::Audio::{
    DEVICE_STATE_ACTIVE, DigitalAudioDisplayDevice, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, PKEY_AudioEndpoint_FormFactor, Speakers, eConsole, eRender,
};
use windows::Win32::System::Com::StructuredStorage::{
    PROPVARIANT, PropVariantClear, PropVariantToStringAlloc, PropVariantToUInt32,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::PWSTR;

use crate::WinError;

/// A playback endpoint, as the device rule sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    /// `IMMDevice::GetId`.
    pub id: String,
    /// For the harness only. The log leaves it out: a Bluetooth headset is often named after its owner.
    pub name: String,
    /// `PKEY_AudioEndpoint_FormFactor` (`EndpointFormFactor`); `None` when it cannot be read.
    pub form_factor: Option<i32>,
}

/// Why the rule picked the endpoint it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PickWhy {
    Default,
    SpeakersInsteadOfDisplay,
    SpeakersNoDefault,
    FirstNoDefault,
}

impl PickWhy {
    /// For the log.
    pub fn text(self) -> &'static str {
        match self {
            PickWhy::Default => "the default device",
            PickWhy::SpeakersInsteadOfDisplay => "speakers instead of the default screen audio",
            PickWhy::SpeakersNoDefault => "speakers, no default device",
            PickWhy::FirstNoDefault => "first playback device, no default device",
        }
    }
}

/// The device rule over the active playback endpoints: the index of the one to play on, and why. `None` only when
/// there is no active endpoint.
pub fn pick(default_id: Option<&str>, active: &[Endpoint]) -> Option<(usize, PickWhy)> {
    let speakers = active
        .iter()
        .position(|e| e.form_factor == Some(Speakers.0));
    match default_id.and_then(|id| active.iter().position(|e| e.id == id)) {
        Some(d) if active[d].form_factor == Some(DigitalAudioDisplayDevice.0) => match speakers {
            Some(s) => Some((s, PickWhy::SpeakersInsteadOfDisplay)),
            None => Some((d, PickWhy::Default)),
        },
        Some(d) => Some((d, PickWhy::Default)),
        None => speakers
            .map(|s| (s, PickWhy::SpeakersNoDefault))
            .or_else(|| (!active.is_empty()).then_some((0, PickWhy::FirstNoDefault))),
    }
}

/// The form factor in words, for the log and the harness.
pub fn form_factor_name(form_factor: Option<i32>) -> &'static str {
    match form_factor {
        Some(0) => "remote network device",
        Some(1) => "speakers",
        Some(2) => "line level",
        Some(3) => "headphones",
        Some(4) => "microphone",
        Some(5) => "headset",
        Some(6) => "handset",
        Some(7) => "digital passthrough",
        Some(8) => "S/PDIF",
        Some(9) => "screen audio (HDMI or DisplayPort)",
        Some(_) => "unknown form factor",
        None => "form factor not readable",
    }
}

/// The endpoint the rule picks right now, and why: what the next chime would play on. Runs on a thread of its own,
/// so the caller's COM apartment does not matter.
pub fn picked_endpoint() -> Result<(Endpoint, PickWhy), WinError> {
    std::thread::Builder::new()
        .name("wakeshot-sound-pick".into())
        .spawn(|| {
            let _com = super::player::Com::init()?;
            pick_device().map(|p| (p.endpoint, p.why))
        })
        .map_err(|e| super::player::spawn_error(&e))?
        .join()
        .unwrap_or(Err(super::player::thread_lost()))
}

pub(super) struct Picked {
    pub device: IMMDevice,
    pub endpoint: Endpoint,
    pub why: PickWhy,
}

/// The device the rule picks. COM must be initialized on this thread.
pub(super) fn pick_device() -> Result<Picked, WinError> {
    // SAFETY: COM is initialized on this thread (caller's duty); no outer object.
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|e| win_error("CoCreateInstance(MMDeviceEnumerator)", &e))?;
    let devices = active(&enumerator)?;
    let endpoints: Vec<Endpoint> = devices.iter().map(|(_, e)| e.clone()).collect();
    let no_device = WinError {
        call: "EnumAudioEndpoints",
        code: ERROR_NOT_FOUND.0,
    };
    let (index, why) = pick(default_id(&enumerator).as_deref(), &endpoints).ok_or(no_device)?;
    let (device, endpoint) = devices.into_iter().nth(index).ok_or(no_device)?;
    Ok(Picked {
        device,
        endpoint,
        why,
    })
}

/// Every active playback endpoint. One that cannot be described is left out, so a broken driver does not silence the
/// others.
fn active(enumerator: &IMMDeviceEnumerator) -> Result<Vec<(IMMDevice, Endpoint)>, WinError> {
    // SAFETY: a live enumerator; plain flags.
    let list = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }
        .map_err(|e| win_error("EnumAudioEndpoints", &e))?;
    // SAFETY: a live collection.
    let count = unsafe { list.GetCount() }.map_err(|e| win_error("GetCount", &e))?;
    let mut out = Vec::new();
    for i in 0..count {
        // SAFETY: a live collection; `i` is below its count.
        let Ok(device) = (unsafe { list.Item(i) }) else {
            continue;
        };
        if let Some(endpoint) = describe(&device) {
            out.push((device, endpoint));
        }
    }
    Ok(out)
}

/// The console role is the one Windows plays system notification sounds on.
fn default_id(enumerator: &IMMDeviceEnumerator) -> Option<String> {
    // SAFETY: a live enumerator; fails with E_NOTFOUND when there is no default.
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }.ok()?;
    // SAFETY: a live device; the string is ours to free.
    unsafe { device.GetId() }.ok().map(take_string)
}

fn describe(device: &IMMDevice) -> Option<Endpoint> {
    // SAFETY: a live device; the string is ours to free.
    let id = take_string(unsafe { device.GetId() }.ok()?);
    Some(Endpoint {
        id,
        name: property(device, &PKEY_Device_FriendlyName)
            .and_then(|p| {
                // SAFETY: a valid PROPVARIANT; the string is ours to free.
                unsafe { PropVariantToStringAlloc(&p.0) }.ok()
            })
            .map(take_string)
            .unwrap_or_default(),
        form_factor: property(device, &PKEY_AudioEndpoint_FormFactor)
            .and_then(|p| {
                // SAFETY: a valid PROPVARIANT.
                unsafe { PropVariantToUInt32(&p.0) }.ok()
            })
            .and_then(|v| i32::try_from(v).ok()),
    })
}

/// A property value that frees what it holds.
struct Property(PROPVARIANT);

impl Drop for Property {
    fn drop(&mut self) {
        // SAFETY: a PROPVARIANT from `GetValue`, cleared once.
        let _ = unsafe { PropVariantClear(&raw mut self.0) };
    }
}

fn property(device: &IMMDevice, key: &PROPERTYKEY) -> Option<Property> {
    // SAFETY: a live device.
    let store = unsafe { device.OpenPropertyStore(STGM_READ) }.ok()?;
    // SAFETY: a live store; `key` lives for the call.
    unsafe { store.GetValue(key) }.ok().map(Property)
}

/// Copies a COM-allocated string and frees it.
fn take_string(p: PWSTR) -> String {
    // SAFETY: a NUL-terminated string from COM, or null.
    let s = if p.is_null() {
        String::new()
    } else {
        unsafe { p.to_string() }.unwrap_or_default()
    };
    // SAFETY: allocated by COM for us, freed once; null is allowed.
    unsafe { CoTaskMemFree(Some(p.0.cast_const().cast())) };
    s
}

pub(super) fn win_error(call: &'static str, error: &windows::core::Error) -> WinError {
    WinError {
        call,
        code: WIN32_ERROR::from_error(error).map_or(error.code().0 as u32, |e| e.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(id: &str, form_factor: i32) -> Endpoint {
        Endpoint {
            id: id.into(),
            name: format!("{id} device"),
            form_factor: Some(form_factor),
        }
    }

    #[test]
    fn device_rule_picks_speakers_for_display_audio() {
        const SPEAKERS: i32 = 1;
        const HEADPHONES: i32 = 3;
        const HEADSET: i32 = 5;
        const DISPLAY: i32 = 9;
        let display = endpoint("display", DISPLAY);
        let speakers = endpoint("speakers", SPEAKERS);
        let headphones = endpoint("headphones", HEADPHONES);
        let headset = endpoint("headset", HEADSET);
        // (case, default, active, picked id, why)
        let table = [
            (
                "display default with speakers",
                Some("display"),
                vec![display.clone(), speakers.clone(), headphones.clone()],
                "speakers",
                PickWhy::SpeakersInsteadOfDisplay,
            ),
            (
                "display default with no speakers",
                Some("display"),
                vec![display.clone(), headphones.clone()],
                "display",
                PickWhy::Default,
            ),
            (
                "headphones default",
                Some("headphones"),
                vec![display.clone(), speakers.clone(), headphones.clone()],
                "headphones",
                PickWhy::Default,
            ),
            (
                "headset default",
                Some("headset"),
                vec![speakers.clone(), headset.clone()],
                "headset",
                PickWhy::Default,
            ),
            (
                "speakers default",
                Some("speakers"),
                vec![display.clone(), speakers.clone()],
                "speakers",
                PickWhy::Default,
            ),
            (
                "no default, speakers present",
                None,
                vec![display.clone(), speakers.clone()],
                "speakers",
                PickWhy::SpeakersNoDefault,
            ),
            (
                "default not active, no speakers",
                Some("gone"),
                vec![headphones.clone(), display.clone()],
                "headphones",
                PickWhy::FirstNoDefault,
            ),
        ];
        for (case, default, active, id, why) in table {
            let (index, got) = pick(default, &active).expect(case);
            assert_eq!((active[index].id.as_str(), got), (id, why), "{case}");
        }
        assert_eq!(pick(Some("speakers"), &[]), None);
    }

    #[test]
    fn an_unreadable_form_factor_counts_as_neither() {
        let unknown = Endpoint {
            id: "unknown".into(),
            name: String::new(),
            form_factor: None,
        };
        let display = endpoint("display", 9);
        let active = [display, unknown];
        assert_eq!(pick(Some("display"), &active), Some((0, PickWhy::Default)));
        assert_eq!(form_factor_name(None), "form factor not readable");
        assert_eq!(
            form_factor_name(Some(9)),
            "screen audio (HDMI or DisplayPort)"
        );
    }
}
