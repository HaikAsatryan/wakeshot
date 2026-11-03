//! Lid-close sound (card `p1-lid-sound`): which endpoint to play on, our own chime through WASAPI on that
//! endpoint, a loopback listener that shows whether audio really reached an endpoint, and a read of the audio state.

use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{PROPERTYKEY, S_OK};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, DEVICE_STATE_ACTIVE,
    ERole, IAudioCaptureClient, IAudioClient, IAudioRenderClient, IAudioSessionControl2,
    IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
    PKEY_AudioEndpoint_FormFactor, PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT, SND_SYSTEM,
    WAVE_FORMAT_PCM, WAVEFORMATEX, WAVEFORMATEXTENSIBLE, eCommunications, eConsole, eMultimedia,
    eRender,
};
use windows::Win32::Media::KernelStreaming::WAVE_FORMAT_EXTENSIBLE;
use windows::Win32::Media::Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT};
use windows::Win32::System::Com::StructuredStorage::{
    PropVariantToStringAlloc, PropVariantToUInt32,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows::core::{HSTRING, Interface, PCWSTR, PWSTR, w};

pub const FF_SPEAKERS: i32 = 1;
pub const FF_DISPLAY: i32 = 9;

#[derive(Clone, Debug, PartialEq)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub form_factor: i32,
}

pub fn form_factor_name(ff: i32) -> &'static str {
    match ff {
        0 => "remote network",
        1 => "speakers",
        2 => "line level",
        3 => "headphones",
        4 => "microphone",
        5 => "headset",
        6 => "handset",
        7 => "digital passthrough",
        8 => "SPDIF",
        9 => "display audio (HDMI or DisplayPort)",
        _ => "unknown",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickWhy {
    Default,
    SpeakersInsteadOfDisplay,
    SpeakersNoDefault,
    FirstActiveNoDefault,
}

/// The device rule: the default device, unless it is a screen's audio (HDMI or DisplayPort), which often has no
/// speakers (the DELL U2723QE has none, and Windows made it the default when it was plugged in). Then the speakers.
/// Headphones and headsets stay as they are: someone wearing them should not get a sound out loud.
pub fn pick<'a>(
    default_id: Option<&str>,
    active: &'a [Endpoint],
) -> Option<(&'a Endpoint, PickWhy)> {
    let speakers = active.iter().find(|e| e.form_factor == FF_SPEAKERS);
    match default_id.and_then(|id| active.iter().find(|e| e.id == id)) {
        Some(d) if d.form_factor == FF_DISPLAY => match speakers {
            Some(s) => Some((s, PickWhy::SpeakersInsteadOfDisplay)),
            None => Some((d, PickWhy::Default)),
        },
        Some(d) => Some((d, PickWhy::Default)),
        None => speakers
            .map(|s| (s, PickWhy::SpeakersNoDefault))
            .or_else(|| active.first().map(|e| (e, PickWhy::FirstActiveNoDefault))),
    }
}

pub const CHIME_RATE: u32 = 48_000;
/// A codec that powers up on the first sample may swallow the start of a short sound; this silence takes that hit
/// instead of the tone (a guess, not verified on this laptop).
pub const LEAD_MS: u32 = 150;
const TAIL_MS: u32 = 100;

/// Two rising tones, 16-bit mono PCM. 1 to 1.6 kHz, where small laptop speakers are loud; peak about -6 dBFS,
/// a little above the Windows sounds (about -9 dBFS), so it is heard at the same volume setting.
pub fn chime() -> Vec<i16> {
    let mut out = vec![0i16; (CHIME_RATE * LEAD_MS / 1000) as usize];
    for (freq, secs, decay) in [(1046.5f32, 0.16f32, 8.0f32), (1568.0, 0.34, 5.0)] {
        let n = (CHIME_RATE as f32 * secs) as usize;
        for i in 0..n {
            let t = i as f32 / CHIME_RATE as f32;
            let attack = (t / 0.005).min(1.0);
            // A short ramp at the end of each tone avoids a click.
            let release = ((secs - t) / 0.01).clamp(0.0, 1.0);
            let env = attack * release * (-t * decay).exp();
            let v = (2.0 * PI * freq * t).sin() * env * 0.5;
            out.push((v * i16::MAX as f32) as i16);
        }
    }
    out.extend(std::iter::repeat_n(
        0i16,
        (CHIME_RATE * TAIL_MS / 1000) as usize,
    ));
    out
}

pub fn chime_ms() -> u64 {
    chime().len() as u64 * 1000 / CHIME_RATE as u64
}

pub fn db(peak: f32) -> f64 {
    if peak <= 0.0 {
        -200.0
    } else {
        20.0 * (peak as f64).log10()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleKind {
    F32,
    I16,
    I32,
    Other,
}

pub fn sample_kind(tag: u16, bits: u16, ext_float: Option<bool>) -> SampleKind {
    let float = match ext_float {
        Some(f) => f,
        None => tag as u32 == WAVE_FORMAT_IEEE_FLOAT,
    };
    match (float, bits) {
        (true, 32) => SampleKind::F32,
        (false, 16) => SampleKind::I16,
        (false, 32) => SampleKind::I32,
        _ => SampleKind::Other,
    }
}

/// Largest absolute sample, full scale 1.0.
pub fn peak(bytes: &[u8], kind: SampleKind) -> f32 {
    match kind {
        SampleKind::F32 => bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c).abs())
            .fold(0.0, f32::max),
        SampleKind::I16 => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (i16::from_le_bytes(*c) as f32 / 32768.0).abs())
            .fold(0.0, f32::max),
        SampleKind::I32 => bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| (i32::from_le_bytes(*c) as f64 / 2147483648.0).abs() as f32)
            .fold(0.0, f32::max),
        SampleKind::Other => 0.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub start_ms: u64,
    pub last_ms: u64,
    pub peak_db: f64,
}

/// Turns loopback packets into sounds: a sound starts at the first packet above the threshold and ends once
/// `gap_ms` pass with nothing above it. Loopback delivers no packets while nothing plays, so the end is found by time.
pub struct Segments {
    threshold_db: f64,
    gap_ms: u64,
    cur: Option<Seg>,
}

impl Segments {
    pub fn new(threshold_db: f64, gap_ms: u64) -> Self {
        Self {
            threshold_db,
            gap_ms,
            cur: None,
        }
    }

    /// True when this packet starts a new sound.
    pub fn packet(&mut self, now_ms: u64, peak_db: f64) -> bool {
        if peak_db < self.threshold_db {
            return false;
        }
        match self.cur.as_mut() {
            Some(s) => {
                s.last_ms = now_ms;
                s.peak_db = s.peak_db.max(peak_db);
                false
            }
            None => {
                self.cur = Some(Seg {
                    start_ms: now_ms,
                    last_ms: now_ms,
                    peak_db,
                });
                true
            }
        }
    }

    /// The finished sound, once it has been quiet for the gap (or at once when `force`).
    pub fn poll(&mut self, now_ms: u64, force: bool) -> Option<Seg> {
        match self.cur {
            Some(s) if force || now_ms >= s.last_ms + self.gap_ms => self.cur.take(),
            _ => None,
        }
    }
}

fn com() {
    // Each thread joins the multithreaded apartment; a thread that already has an apartment keeps it.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
}

fn enumerator() -> windows::core::Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

fn take_pwstr(p: PWSTR) -> String {
    let s = unsafe { p.to_string() }.unwrap_or_default();
    unsafe { CoTaskMemFree(Some(p.0 as _)) };
    s
}

fn prop_string(dev: &IMMDevice, key: &PROPERTYKEY) -> String {
    let Ok(store) = (unsafe { dev.OpenPropertyStore(STGM_READ) }) else {
        return String::new();
    };
    let Ok(v) = (unsafe { store.GetValue(key) }) else {
        return String::new();
    };
    match unsafe { PropVariantToStringAlloc(&v) } {
        Ok(p) => take_pwstr(p),
        Err(_) => String::new(),
    }
}

fn prop_u32(dev: &IMMDevice, key: &PROPERTYKEY) -> Option<u32> {
    let store = unsafe { dev.OpenPropertyStore(STGM_READ) }.ok()?;
    let v = unsafe { store.GetValue(key) }.ok()?;
    unsafe { PropVariantToUInt32(&v) }.ok()
}

fn describe(dev: &IMMDevice) -> windows::core::Result<Endpoint> {
    let id = take_pwstr(unsafe { dev.GetId()? });
    Ok(Endpoint {
        id,
        name: prop_string(dev, &PKEY_Device_FriendlyName),
        form_factor: prop_u32(dev, &PKEY_AudioEndpoint_FormFactor).map_or(-1, |v| v as i32),
    })
}

fn active(en: &IMMDeviceEnumerator) -> windows::core::Result<Vec<(IMMDevice, Endpoint)>> {
    let col = unsafe { en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };
    let mut out = Vec::new();
    for i in 0..unsafe { col.GetCount()? } {
        let dev = unsafe { col.Item(i)? };
        let ep = describe(&dev)?;
        out.push((dev, ep));
    }
    Ok(out)
}

fn default_id(en: &IMMDeviceEnumerator, role: ERole) -> Option<String> {
    let dev = unsafe { en.GetDefaultAudioEndpoint(eRender, role) }.ok()?;
    unsafe { dev.GetId() }.ok().map(take_pwstr)
}

fn volume(dev: &IMMDevice) -> String {
    let r = (|| -> windows::core::Result<(f32, bool)> {
        let v: IAudioEndpointVolume = unsafe { dev.Activate(CLSCTX_ALL, None)? };
        Ok(unsafe { (v.GetMasterVolumeLevelScalar()?, v.GetMute()?.as_bool()) })
    })();
    match r {
        Ok((level, mute)) => format!("volume {:.1}%, muted {mute}", level * 100.0),
        Err(e) => format!("volume unreadable: {e}"),
    }
}

fn sessions(dev: &IMMDevice) -> windows::core::Result<Vec<String>> {
    let mgr: IAudioSessionManager2 = unsafe { dev.Activate(CLSCTX_ALL, None)? };
    let list = unsafe { mgr.GetSessionEnumerator()? };
    let mut out = Vec::new();
    for i in 0..unsafe { list.GetCount()? } {
        let ctl = unsafe { list.GetSession(i)? };
        let ctl2: IAudioSessionControl2 = ctl.cast()?;
        let system = unsafe { ctl2.IsSystemSoundsSession() } == S_OK;
        let pid = unsafe { ctl2.GetProcessId() }.unwrap_or(0);
        let state = unsafe { ctl.GetState() }.map_or(-1, |s| s.0);
        let name = unsafe { ctl.GetDisplayName() }
            .map(take_pwstr)
            .unwrap_or_default();
        let vol = ctl
            .cast::<ISimpleAudioVolume>()
            .and_then(|v| unsafe { Ok((v.GetMasterVolume()?, v.GetMute()?.as_bool())) });
        let vol = match vol {
            Ok((l, m)) => format!("session volume {:.1}%, muted {m}", l * 100.0),
            Err(e) => format!("session volume unreadable: {e}"),
        };
        out.push(format!(
            "system sounds {system}, pid {pid}, state {state} (0 inactive, 1 active, 2 expired), {vol}, name '{name}'"
        ));
    }
    Ok(out)
}

/// Logs the default device per role, every active playback endpoint with its volume, and the audio sessions on
/// each (the "System sounds" slider of the volume mixer is the session with `system sounds true`). Reads only.
pub fn read_state(tag: &str) {
    let tag = tag.to_string();
    let _ = std::thread::spawn(move || {
        com();
        let en = match enumerator() {
            Ok(e) => e,
            Err(e) => {
                crate::log!(&tag, "no device enumerator: {e}");
                return;
            }
        };
        for (role, name) in [
            (eConsole, "console"),
            (eMultimedia, "multimedia"),
            (eCommunications, "communications"),
        ] {
            crate::log!(&tag, "default {name}: {:?}", default_id(&en, role));
        }
        match active(&en) {
            Ok(list) => {
                for (dev, ep) in &list {
                    crate::log!(
                        &tag,
                        "endpoint {} '{}' form factor {} ({}), {}",
                        ep.id,
                        ep.name,
                        ep.form_factor,
                        form_factor_name(ep.form_factor),
                        volume(dev)
                    );
                    match sessions(dev) {
                        Ok(s) => {
                            for line in s {
                                crate::log!(&tag, "  session: {line}");
                            }
                        }
                        Err(e) => crate::log!(&tag, "  sessions unreadable: {e}"),
                    }
                }
            }
            Err(e) => crate::log!(&tag, "endpoints unreadable: {e}"),
        }
    })
    .join();
}

/// The Phase 0 call, kept as the control: the user's SystemNotification sound, on the default device, in the
/// System Sounds session.
pub fn play_alias(tag: &str) {
    let t = Instant::now();
    let ok = unsafe {
        PlaySoundW(
            w!("SystemNotification"),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT | SND_SYSTEM,
        )
    };
    crate::log!(
        tag,
        "PlaySound(SystemNotification, SND_SYSTEM) {} in {} us",
        ok.as_bool(),
        t.elapsed().as_micros()
    );
}

/// Loads the audio stack once at start, as a long-running app would have it loaded, so the first chime does not
/// pay for it at the lid close. Plays nothing.
pub fn warm_up(tag: &str) {
    let tag = tag.to_string();
    let _ = std::thread::spawn(move || {
        com();
        let t = Instant::now();
        let r = (|| -> windows::core::Result<()> {
            let en = enumerator()?;
            let _ = active(&en)?;
            let dev = unsafe { en.GetDefaultAudioEndpoint(eRender, eConsole)? };
            let client: IAudioClient = unsafe { dev.Activate(CLSCTX_ALL, None)? };
            let mix = unsafe { client.GetMixFormat()? };
            unsafe { CoTaskMemFree(Some(mix as _)) };
            Ok(())
        })();
        crate::log!(
            &tag,
            "audio warm-up {:?} in {} ms",
            r,
            t.elapsed().as_millis()
        );
    })
    .join();
}

/// Picks the endpoint, starts our chime on it, reports on `started` once the stream runs, then waits until the
/// audio engine has taken the whole buffer. A buffer that is not taken means the engine stopped playing.
pub fn play_chime(tag: &str, started: Option<Sender<()>>) -> JoinHandle<()> {
    let tag = tag.to_string();
    std::thread::spawn(move || {
        com();
        let t = Instant::now();
        if let Err(e) = play_chime_inner(&tag, t, started) {
            crate::log!(
                &tag,
                "chime failed after {} ms: {e}",
                t.elapsed().as_millis()
            );
        }
    })
}

fn play_chime_inner(
    tag: &str,
    t: Instant,
    started: Option<Sender<()>>,
) -> windows::core::Result<()> {
    let en = enumerator()?;
    let list = active(&en)?;
    let eps: Vec<Endpoint> = list.iter().map(|(_, e)| e.clone()).collect();
    let def = default_id(&en, eConsole);
    let Some((ep, why)) = pick(def.as_deref(), &eps) else {
        crate::log!(tag, "chime: no active playback endpoint");
        return Ok(());
    };
    let dev = &list.iter().find(|(_, e)| e.id == ep.id).unwrap().0;
    let pick_ms = t.elapsed().as_millis();
    let pcm = chime();
    let fmt = WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: 1,
        nSamplesPerSec: CHIME_RATE,
        nAvgBytesPerSec: CHIME_RATE * 2,
        nBlockAlign: 2,
        wBitsPerSample: 16,
        cbSize: 0,
    };
    let client: IAudioClient = unsafe { dev.Activate(CLSCTX_ALL, None)? };
    // The buffer holds the whole sound, so one write is enough and nothing has to feed it on time.
    let hns = pcm.len() as i64 * 10_000_000 / CHIME_RATE as i64 + 2_000_000;
    unsafe {
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            hns,
            0,
            &fmt,
            None,
        )?
    };
    let init_ms = t.elapsed().as_millis();
    let size = unsafe { client.GetBufferSize()? };
    let n = (pcm.len() as u32).min(size);
    let render: IAudioRenderClient = unsafe { client.GetService()? };
    let buf = unsafe { render.GetBuffer(n)? };
    unsafe {
        std::ptr::copy_nonoverlapping(pcm.as_ptr() as *const u8, buf, n as usize * 2);
        render.ReleaseBuffer(n, 0)?;
        client.Start()?;
    }
    let start_ms = t.elapsed().as_millis();
    if let Some(tx) = started {
        let _ = tx.send(());
    }
    crate::log!(
        tag,
        "chime started in {start_ms} ms (device picked at {pick_ms}, stream ready at {init_ms}) on '{}' ({}), pick {why:?}, default {:?}, {}, {} of {} frames, sound {} ms",
        ep.name,
        form_factor_name(ep.form_factor),
        def,
        volume(dev),
        n,
        pcm.len(),
        chime_ms()
    );
    let limit = Duration::from_millis(chime_ms() + 3000);
    let t_play = Instant::now();
    let mut left = n;
    while t_play.elapsed() < limit {
        left = unsafe { client.GetCurrentPadding()? };
        if left == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let played_ms = t_play.elapsed().as_millis();
    // Let the last period leave the engine before the stream stops.
    std::thread::sleep(Duration::from_millis(100));
    unsafe { client.Stop()? };
    if left == 0 {
        crate::log!(
            tag,
            "chime: the engine took the whole sound in {played_ms} ms"
        );
    } else {
        crate::log!(
            tag,
            "chime: NOT played out: {left} frames still queued after {played_ms} ms"
        );
    }
    Ok(())
}

/// Loopback listeners on every active playback endpoint. They log each sound that reaches an endpoint, with its
/// length and peak, whatever process played it.
pub struct Listener {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Listener {
    pub fn start() -> Listener {
        let stop = Arc::new(AtomicBool::new(false));
        let eps = std::thread::spawn(|| {
            com();
            enumerator()
                .and_then(|en| active(&en))
                .map(|l| l.into_iter().map(|(_, e)| e).collect::<Vec<_>>())
        })
        .join()
        .unwrap_or_else(|_| Ok(Vec::new()));
        let eps = match eps {
            Ok(e) => e,
            Err(e) => {
                crate::log!("listen", "endpoints unreadable: {e}");
                Vec::new()
            }
        };
        let threads = eps
            .into_iter()
            .map(|ep| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    com();
                    if let Err(e) = listen(&ep, &stop) {
                        crate::log!("listen", "'{}': stopped on error: {e}", ep.name);
                    }
                })
            })
            .collect();
        Listener { stop, threads }
    }

    pub fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads {
            let _ = t.join();
        }
    }
}

fn listen(ep: &Endpoint, stop: &AtomicBool) -> windows::core::Result<()> {
    let en = enumerator()?;
    let id = HSTRING::from(ep.id.as_str());
    let dev = unsafe { en.GetDevice(PCWSTR(id.as_ptr()))? };
    let client: IAudioClient = unsafe { dev.Activate(CLSCTX_ALL, None)? };
    let mix = unsafe { client.GetMixFormat()? };
    let f = unsafe { std::ptr::read_unaligned(mix) };
    let (tag, ch, rate, bits, align) = (
        f.wFormatTag,
        f.nChannels,
        f.nSamplesPerSec,
        f.wBitsPerSample,
        f.nBlockAlign,
    );
    let ext_float = (tag as u32 == WAVE_FORMAT_EXTENSIBLE).then(|| {
        let x = unsafe { std::ptr::read_unaligned(mix as *const WAVEFORMATEXTENSIBLE) };
        let sub = x.SubFormat;
        sub == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
    });
    let kind = sample_kind(tag, bits, ext_float);
    let init = unsafe {
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            2_000_000,
            0,
            mix,
            None,
        )
    };
    unsafe { CoTaskMemFree(Some(mix as _)) };
    init?;
    let cap: IAudioCaptureClient = unsafe { client.GetService()? };
    unsafe { client.Start()? };
    crate::log!(
        "listen",
        "'{}' ({}): loopback on, mix {kind:?}, {ch} ch, {rate} Hz",
        ep.name,
        form_factor_name(ep.form_factor)
    );
    let t0 = Instant::now();
    let mut segs = Segments::new(-50.0, 300);
    let (mut packets, mut frames_total, mut silent) = (0u64, 0u64, 0u64);
    let mut heard = 0u32;
    let mut next_beat = 15_000u64;
    let report = |s: Seg| {
        crate::log!(
            "listen",
            "'{}': sound ended: {} ms long, peak {:.1} dBFS",
            ep.name,
            s.last_ms - s.start_ms,
            s.peak_db
        );
    };
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
        loop {
            if unsafe { cap.GetNextPacketSize()? } == 0 {
                break;
            }
            let mut data = std::ptr::null_mut();
            let mut frames = 0u32;
            let mut flags = 0u32;
            unsafe { cap.GetBuffer(&mut data, &mut frames, &mut flags, None, None)? };
            packets += 1;
            frames_total += frames as u64;
            let p = if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                silent += 1;
                0.0
            } else {
                let bytes =
                    unsafe { std::slice::from_raw_parts(data, frames as usize * align as usize) };
                peak(bytes, kind)
            };
            unsafe { cap.ReleaseBuffer(frames)? };
            let now = t0.elapsed().as_millis() as u64;
            if segs.packet(now, db(p)) {
                heard += 1;
                crate::log!(
                    "listen",
                    "'{}': SOUND starts, first packet peak {:.1} dBFS",
                    ep.name,
                    db(p)
                );
            }
        }
        let now = t0.elapsed().as_millis() as u64;
        if let Some(s) = segs.poll(now, false) {
            report(s);
        }
        if now >= next_beat {
            crate::log!(
                "listen",
                "'{}': alive, {packets} packets, {:.1} s of audio, {silent} silent packets",
                ep.name,
                frames_total as f64 / rate as f64
            );
            next_beat += 15_000;
        }
    }
    if let Some(s) = segs.poll(u64::MAX, true) {
        report(s);
    }
    unsafe { client.Stop()? };
    crate::log!(
        "listen",
        "'{}': off. {heard} sound(s) heard, {packets} packets, {:.1} s of audio",
        ep.name,
        frames_total as f64 / rate as f64
    );
    Ok(())
}

/// Mode `sound-test`, lid open: the Phase 0 alias sound, then our chime on the picked endpoint, with loopback
/// listeners on every active playback endpoint.
pub fn sound_test(args: &crate::Args) -> i32 {
    read_state("before");
    let listener = Listener::start();
    std::thread::sleep(Duration::from_millis(1000));
    if !args.flag("--chime-only") {
        play_alias("test");
        std::thread::sleep(Duration::from_millis(3000));
    }
    let _ = play_chime("test", None).join();
    std::thread::sleep(Duration::from_millis(1500));
    listener.stop();
    read_state("after");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(id: &str, ff: i32) -> Endpoint {
        Endpoint {
            id: id.into(),
            name: id.into(),
            form_factor: ff,
        }
    }

    #[test]
    fn pick_rule() {
        let spk = ep("spk", FF_SPEAKERS);
        let dell = ep("dell", FF_DISPLAY);
        let hp = ep("hp", 3);
        let both = [dell.clone(), spk.clone()];
        assert_eq!(pick(Some("spk"), &both), Some((&both[1], PickWhy::Default)));
        assert_eq!(
            pick(Some("dell"), &both),
            Some((&both[1], PickWhy::SpeakersInsteadOfDisplay))
        );
        let only_dell = [dell.clone()];
        assert_eq!(
            pick(Some("dell"), &only_dell),
            Some((&only_dell[0], PickWhy::Default))
        );
        let with_hp = [hp, spk];
        assert_eq!(
            pick(Some("hp"), &with_hp),
            Some((&with_hp[0], PickWhy::Default))
        );
        assert_eq!(
            pick(None, &with_hp),
            Some((&with_hp[1], PickWhy::SpeakersNoDefault))
        );
        assert_eq!(
            pick(Some("gone"), &only_dell),
            Some((&only_dell[0], PickWhy::FirstActiveNoDefault))
        );
        assert_eq!(pick(None, &[]), None);
    }

    #[test]
    fn chime_shape() {
        let c = chime();
        let lead = (CHIME_RATE * LEAD_MS / 1000) as usize;
        assert!(c[..lead].iter().all(|&s| s == 0));
        assert!(c[lead..lead + 2400].iter().any(|&s| s != 0));
        let peak = c.iter().map(|s| s.unsigned_abs()).max().unwrap() as f32 / 32768.0;
        assert!(peak > 0.4 && peak <= 0.5, "peak {peak}");
        assert!(c[c.len() - 4800..].iter().all(|&s| s == 0));
        assert!((700..=800).contains(&chime_ms()), "{} ms", chime_ms());
    }

    #[test]
    fn peaks_and_db() {
        let f: Vec<u8> = [0.25f32, -0.5, 0.1]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        assert_eq!(peak(&f, SampleKind::F32), 0.5);
        let i: Vec<u8> = [100i16, -16384, 5]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        assert_eq!(peak(&i, SampleKind::I16), 0.5);
        assert!((db(0.5) + 6.02).abs() < 0.01);
        assert_eq!(db(0.0), -200.0);
        assert_eq!(sample_kind(3, 32, None), SampleKind::F32);
        assert_eq!(sample_kind(0xFFFE, 32, Some(true)), SampleKind::F32);
        assert_eq!(sample_kind(0xFFFE, 32, Some(false)), SampleKind::I32);
        assert_eq!(sample_kind(1, 16, None), SampleKind::I16);
        assert_eq!(sample_kind(1, 24, None), SampleKind::Other);
    }

    #[test]
    fn segments() {
        let mut s = Segments::new(-50.0, 300);
        assert!(!s.packet(0, -80.0));
        assert!(s.packet(10, -20.0));
        assert!(!s.packet(500, -9.0));
        assert!(!s.packet(510, -70.0));
        assert_eq!(s.poll(700, false), None);
        assert_eq!(
            s.poll(800, false),
            Some(Seg {
                start_ms: 10,
                last_ms: 500,
                peak_db: -9.0
            })
        );
        assert_eq!(s.poll(900, false), None);
        assert!(s.packet(1000, -30.0));
        assert!(s.poll(1001, true).is_some());
    }
}
