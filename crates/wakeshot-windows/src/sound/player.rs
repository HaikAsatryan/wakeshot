//! The WASAPI player (architecture section 2, `SoundPlayer`; section 4, "Tray threads", Sound row): one short thread
//! per sound, a shared-mode stream on the device the rule picks, the "No sounds" scheme honored.

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use wakeshot_core::command::{Failure, Outcome, SoundEnd};
use wakeshot_core::event::Event;
use wakeshot_core::view::Cue;
use windows::Win32::Foundation::{E_UNEXPECTED, ERROR_SUCCESS, ERROR_TIMEOUT};
use windows::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioClient, IAudioRenderClient, IMMDevice,
    WAVE_FORMAT_PCM, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::Media::KernelStreaming::{KSDATAFORMAT_SUBTYPE_PCM, WAVE_FORMAT_EXTENSIBLE};
use windows::Win32::Media::Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
use windows::core::{GUID, PCWSTR, w};

use super::SoundPlayer;
use super::chime;
use super::device::{self, PickWhy, form_factor_name, win_error};
use crate::WinError;
use crate::log::{Level, Log};
use crate::sink::EventSink;

/// How long `play_and_wait` waits: the slowest stream start seen on the test laptop (0.8 s) plus the longest chime
/// (under 1.2 s), with room to spare (architecture section 5, "Lock and hibernate").
pub(super) const WAIT_LIMIT: Duration = Duration::from_secs(3);

/// `play_and_wait`'s error when the limit passed: the chime may still be starting or playing.
pub(super) const WAIT_CALL: &str = "sound wait";

/// A stream the engine has not drained this long after the chime should have ended has stalled (an endpoint that
/// went away mid-play, say); the thread gives up instead of polling forever.
const STALL_MS: u32 = 2_000;

/// How often the feed loop looks at the buffer; small next to the 100 ms tail, so the stream stops soon after the
/// chime ends.
const POLL: Duration = Duration::from_millis(10);

/// The engine may give a smaller buffer than asked; the feed loop tops it up. The whole chime plus this margin is
/// asked, so on the test laptop one write is enough.
const BUFFER_MARGIN_MS: u32 = 200;

const AREA: &str = "sound";

/// Plays Wakeshot's chimes through WASAPI shared mode. `play` posts its result through the sink and writes one log
/// line per sound (the core leaves a sound's log to its runner); `play_and_wait` returns its result and logs the same
/// line.
pub struct WasapiPlayer {
    shared: Arc<Shared>,
}

impl WasapiPlayer {
    pub fn new(sink: Arc<dyn EventSink>, log: Arc<dyn Log>) -> Self {
        Self::with_audio(sink, log, Arc::new(Wasapi), WAIT_LIMIT)
    }

    fn with_audio(
        sink: Arc<dyn EventSink>,
        log: Arc<dyn Log>,
        audio: Arc<dyn Audio>,
        wait_limit: Duration,
    ) -> Self {
        Self {
            shared: Arc::new(Shared {
                sink,
                log,
                audio,
                wait_limit,
            }),
        }
    }
}

impl SoundPlayer for WasapiPlayer {
    fn play(&self, cue: Cue) {
        let called = Instant::now();
        let shared = Arc::clone(&self.shared);
        let spawned = spawn(move || {
            let result = shared.run(cue, called);
            shared.post(cue, result);
        });
        if let Err(error) = spawned {
            self.shared.failed(cue, error);
            self.shared.post(cue, Err(error));
        }
    }

    fn play_and_wait(&self, cue: Cue) -> Result<SoundEnd, WinError> {
        let called = Instant::now();
        let (tx, rx) = mpsc::channel();
        let shared = Arc::clone(&self.shared);
        let spawned = spawn(move || {
            // Nobody is listening once the wait gave up; the run has logged its result already.
            let _ = tx.send(shared.run(cue, called));
        });
        if let Err(error) = spawned {
            self.shared.failed(cue, error);
            return Err(error);
        }
        let left = self.shared.wait_limit.saturating_sub(called.elapsed());
        match rx.recv_timeout(left) {
            Ok(result) => result,
            // The thread plays on; if the PC hibernates now, the chime is cut, which is all a hint can lose.
            Err(RecvTimeoutError::Timeout) => Err(WinError {
                call: WAIT_CALL,
                code: ERROR_TIMEOUT.0,
            }),
            Err(RecvTimeoutError::Disconnected) => Err(thread_lost()),
        }
    }
}

/// What the player needs from Windows, so the tests drive `play` and `play_and_wait` without audio.
trait Audio: Send + Sync {
    /// The current sound scheme's key name, `None` when it cannot be read.
    fn scheme(&self) -> Option<String>;

    /// Plays `cue` to its end on the device the rule picks; blocks. `called` is when the player was asked.
    fn play(&self, cue: Cue, called: Instant) -> Result<Played, WinError>;
}

/// How a chime went, for the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Played {
    form_factor: Option<i32>,
    why: PickWhy,
    /// From the call to a running stream.
    running_ms: u128,
    /// From the call to the engine having taken the whole chime.
    done_ms: u128,
    /// The mix format was one this player does not write, so Windows converted the tested 16-bit mono.
    converted: bool,
}

struct Shared {
    sink: Arc<dyn EventSink>,
    log: Arc<dyn Log>,
    audio: Arc<dyn Audio>,
    wait_limit: Duration,
}

impl Shared {
    fn run(&self, cue: Cue, called: Instant) -> Result<SoundEnd, WinError> {
        if is_no_sounds(self.audio.scheme().as_deref()) {
            self.log.line(
                Level::Info,
                AREA,
                &format!(
                    "{} chime skipped: the sound scheme is No sounds",
                    cue_name(cue)
                ),
            );
            return Ok(SoundEnd::Skipped);
        }
        match self.audio.play(cue, called) {
            Ok(played) => {
                self.log.line(Level::Info, AREA, &played_text(cue, played));
                Ok(SoundEnd::Played)
            }
            Err(error) => {
                self.failed(cue, error);
                Err(error)
            }
        }
    }

    fn post(&self, cue: Cue, result: Result<SoundEnd, WinError>) {
        let event = Event::Done(Outcome::Sound(result.map_err(Failure::from)));
        if let Err(error) = self.sink.post(event) {
            self.log.line(
                Level::Warn,
                AREA,
                &format!(
                    "{} chime result not posted: {}",
                    cue_name(cue),
                    failed_text(error)
                ),
            );
        }
    }

    fn failed(&self, cue: Cue, error: WinError) {
        self.log.line(
            Level::Warn,
            AREA,
            &format!("{} chime: {}", cue_name(cue), failed_text(error)),
        );
    }
}

fn cue_name(cue: Cue) -> &'static str {
    match cue {
        Cue::LidClosedHolding => "lid closed",
        Cue::GoingToSleep => "going to sleep",
        Cue::BatteryFloor => "battery floor",
    }
}

fn played_text(cue: Cue, played: Played) -> String {
    format!(
        "{} chime played on {} ({}), stream running {} ms after the call, done after {} ms{}",
        cue_name(cue),
        form_factor_name(played.form_factor),
        played.why.text(),
        played.running_ms,
        played.done_ms,
        if played.converted {
            ", converted by Windows"
        } else {
            ""
        }
    )
}

/// The house wording ("X failed with N"); an HRESULT reads better in hex.
fn failed_text(error: WinError) -> String {
    if error.code >= 0x8000_0000 {
        format!("{} failed with 0x{:08X}", error.call, error.code)
    } else {
        format!("{} failed with {}", error.call, error.code)
    }
}

/// The key name of the "No Sounds" scheme. On the test laptop `HKCU\AppEvents\Schemes\Names\.None` is
/// `@mmres.dll,-801`, which loads as "No Sounds" (research, "Tray UX: chimes"). Case is ignored, as registry names are.
const NO_SOUNDS: &str = ".None";

/// True when the scheme is "No sounds". A scheme that cannot be read plays: only a clear No sounds keeps quiet.
fn is_no_sounds(scheme: Option<&str>) -> bool {
    scheme.is_some_and(|s| s.eq_ignore_ascii_case(NO_SOUNDS))
}

/// The current sound scheme's key name (`HKCU\AppEvents\Schemes`, default value): `.Default`, `.None` for "No
/// sounds", `.Current` once the user changed a sound, or a saved scheme's name. `None` when it cannot be read.
pub fn current_scheme() -> Option<String> {
    // Longer than any built-in name; a longer one fails with ERROR_MORE_DATA and is not No sounds either.
    let mut buf = [0u16; 128];
    let mut bytes = u32::try_from(size_of_val(&buf)).unwrap_or(0);
    // SAFETY: `buf` and `bytes` live for the call and `bytes` is the buffer's size in bytes; a null value name reads
    // the key's default value.
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("AppEvents\\Schemes"),
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&raw mut bytes),
        )
    };
    if read != ERROR_SUCCESS {
        return None;
    }
    let units = buf.get(..usize::try_from(bytes / 2).unwrap_or(0))?;
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    Some(String::from_utf16_lossy(units.get(..end)?))
}

fn spawn<F: FnOnce() + Send + 'static>(run: F) -> Result<JoinHandle<()>, WinError> {
    thread::Builder::new()
        .name("wakeshot-sound".into())
        .spawn(run)
        .map_err(|e| spawn_error(&e))
}

pub(super) fn spawn_error(e: &std::io::Error) -> WinError {
    WinError {
        call: "CreateThread",
        code: e.raw_os_error().map_or(E_UNEXPECTED.0 as u32, |c| c as u32),
    }
}

/// A sound thread that ended without an answer. Release builds abort on a panic, so only a test sees this.
pub(super) fn thread_lost() -> WinError {
    WinError {
        call: "sound thread",
        code: E_UNEXPECTED.0 as u32,
    }
}

/// COM on one of this module's own threads, undone when dropped. Every COM object of the thread must be dropped
/// first, so it is the first local of the function that makes them.
pub(super) struct Com(PhantomData<*const ()>);

impl Com {
    pub(super) fn init() -> Result<Self, WinError> {
        // SAFETY: no reserved pointer; a new thread of this module with no apartment yet.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_err() {
            return Err(WinError {
                call: "CoInitializeEx",
                code: hr.0 as u32,
            });
        }
        Ok(Self(PhantomData))
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: pairs the successful `CoInitializeEx` in `init`, on the same thread (the guard is not `Send`).
        unsafe { CoUninitialize() };
    }
}

struct Wasapi;

impl Audio for Wasapi {
    fn scheme(&self) -> Option<String> {
        current_scheme()
    }

    fn play(&self, cue: Cue, called: Instant) -> Result<Played, WinError> {
        let _com = Com::init()?;
        let picked = device::pick_device()?;
        let timing = stream(&picked.device, cue, called)?;
        Ok(Played {
            form_factor: picked.endpoint.form_factor,
            why: picked.why,
            running_ms: timing.running_ms,
            done_ms: timing.done_ms,
            converted: timing.converted,
        })
    }
}

struct Timing {
    running_ms: u128,
    done_ms: u128,
    converted: bool,
}

/// A format pointer from `GetMixFormat`, freed once.
struct MixFormat(*mut WAVEFORMATEX);

impl Drop for MixFormat {
    fn drop(&mut self) {
        // SAFETY: allocated by COM for us; freed once.
        unsafe { CoTaskMemFree(Some(self.0.cast_const().cast())) };
    }
}

/// The format played at both tested lid closes: 16-bit mono, 48 kHz, converted by the engine.
const FALLBACK: Layout = Layout {
    rate: 48_000,
    channels: 1,
    sample: Sample::I16,
};

/// Plays the chime on `device` in its mix format and returns once the engine has taken all of it.
fn stream(device: &IMMDevice, cue: Cue, called: Instant) -> Result<Timing, WinError> {
    // SAFETY: a live device; COM is up on this thread.
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .map_err(|e| win_error("IMMDevice::Activate", &e))?;
    // SAFETY: a live client; the format is ours to free.
    let mix = MixFormat(
        unsafe { client.GetMixFormat() }
            .map_err(|e| win_error("IAudioClient::GetMixFormat", &e))?,
    );
    let fallback = WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: FALLBACK.channels,
        nSamplesPerSec: FALLBACK.rate,
        nAvgBytesPerSec: FALLBACK.rate * FALLBACK.frame_bytes() as u32,
        nBlockAlign: FALLBACK.frame_bytes() as u16,
        wBitsPerSample: 16,
        cbSize: 0,
    };
    let (format, layout, flags) = match mix_layout(mix.0) {
        Some(layout) => (mix.0.cast_const(), layout, 0),
        None => (
            &raw const fallback,
            FALLBACK,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
        ),
    };
    let pcm = chime::render(cue, layout.rate);
    let length_ms = chime::length_ms(cue);
    // 100 ns units.
    let buffer = i64::from(length_ms + BUFFER_MARGIN_MS) * 10_000;
    // SAFETY: `format` points to a valid format that lives for the call (`mix` or `fallback`).
    unsafe { client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, buffer, 0, format, None) }
        .map_err(|e| win_error("IAudioClient::Initialize", &e))?;
    // SAFETY: an initialized client.
    let size = unsafe { client.GetBufferSize() }
        .map_err(|e| win_error("IAudioClient::GetBufferSize", &e))? as usize;
    // SAFETY: an initialized client.
    let render: IAudioRenderClient =
        unsafe { client.GetService() }.map_err(|e| win_error("IAudioClient::GetService", &e))?;
    let mut written = write(&render, &pcm, 0, size.min(pcm.len()), layout)?;
    // SAFETY: an initialized client with data queued.
    unsafe { client.Start() }.map_err(|e| win_error("IAudioClient::Start", &e))?;
    let running_ms = called.elapsed().as_millis();
    let deadline = Instant::now() + Duration::from_millis(u64::from(length_ms + STALL_MS));
    loop {
        thread::sleep(POLL);
        // SAFETY: a started client.
        let queued = unsafe { client.GetCurrentPadding() }
            .map_err(|e| win_error("IAudioClient::GetCurrentPadding", &e))?
            as usize;
        if written < pcm.len() {
            let room = size.saturating_sub(queued).min(pcm.len() - written);
            if room > 0 {
                written = write(&render, &pcm, written, room, layout)?;
            }
        } else if queued == 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err(WinError {
                call: "IAudioClient::GetCurrentPadding",
                code: ERROR_TIMEOUT.0,
            });
        }
    }
    let done_ms = called.elapsed().as_millis();
    // The chime has played: a failed stop changes nothing, and releasing the client stops it anyway.
    // SAFETY: a started client.
    let _ = unsafe { client.Stop() };
    Ok(Timing {
        running_ms,
        done_ms,
        converted: flags != 0,
    })
}

/// Writes `count` frames of `pcm` from `from` and returns the new position.
fn write(
    render: &IAudioRenderClient,
    pcm: &[f32],
    from: usize,
    count: usize,
    layout: Layout,
) -> Result<usize, WinError> {
    let frames = u32::try_from(count).map_err(|_| WinError {
        call: "IAudioRenderClient::GetBuffer",
        code: E_UNEXPECTED.0 as u32,
    })?;
    // SAFETY: a live render client; `frames` fits the free part of the buffer.
    let data = unsafe { render.GetBuffer(frames) }
        .map_err(|e| win_error("IAudioRenderClient::GetBuffer", &e))?;
    // SAFETY: `GetBuffer` lent `frames` frames of the stream's format, ours until `ReleaseBuffer`.
    let out = unsafe { std::slice::from_raw_parts_mut(data, count * layout.frame_bytes()) };
    fill(pcm.get(from..from + count).unwrap_or_default(), layout, out);
    // SAFETY: releases exactly what `GetBuffer` lent, all written.
    unsafe { render.ReleaseBuffer(frames, 0) }
        .map_err(|e| win_error("IAudioRenderClient::ReleaseBuffer", &e))?;
    Ok(from + count)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sample {
    F32,
    I16,
    I32,
}

impl Sample {
    fn bytes(self) -> usize {
        match self {
            Sample::I16 => 2,
            Sample::F32 | Sample::I32 => 4,
        }
    }
}

/// A stream format this player writes itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    rate: u32,
    channels: u16,
    sample: Sample,
}

impl Layout {
    fn frame_bytes(self) -> usize {
        usize::from(self.channels) * self.sample.bytes()
    }
}

fn mix_layout(format: *const WAVEFORMATEX) -> Option<Layout> {
    // SAFETY: the pointer from `GetMixFormat`, valid while `MixFormat` lives; the struct is packed.
    let f = unsafe { format.read_unaligned() };
    // `cbSize` 22: the extensible part (Microsoft, WAVEFORMATEXTENSIBLE).
    let sub = (u32::from(f.wFormatTag) == WAVE_FORMAT_EXTENSIBLE && f.cbSize >= 22).then(|| {
        // SAFETY: `cbSize` says the extensible part follows; packed, so read unaligned.
        unsafe { format.cast::<WAVEFORMATEXTENSIBLE>().read_unaligned() }.SubFormat
    });
    layout(
        f.wFormatTag,
        f.nChannels,
        f.nSamplesPerSec,
        f.wBitsPerSample,
        f.nBlockAlign,
        sub,
    )
}

/// The layout of a mix format, or `None` for one the engine should convert to (packed 24-bit, say). For 24 valid
/// bits in a 32-bit container the sample sits in the high bits, so full-scale 32-bit samples fit it.
fn layout(
    tag: u16,
    channels: u16,
    rate: u32,
    bits: u16,
    block_align: u16,
    sub: Option<GUID>,
) -> Option<Layout> {
    let tag = u32::from(tag);
    let (float, pcm) = match sub {
        Some(sub) => (
            sub == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
            sub == KSDATAFORMAT_SUBTYPE_PCM,
        ),
        None if tag == WAVE_FORMAT_EXTENSIBLE => return None,
        None => (tag == WAVE_FORMAT_IEEE_FLOAT, tag == WAVE_FORMAT_PCM),
    };
    let sample = match (float, pcm, bits) {
        (true, _, 32) => Sample::F32,
        (_, true, 16) => Sample::I16,
        (_, true, 32) => Sample::I32,
        _ => return None,
    };
    let layout = Layout {
        rate,
        channels,
        sample,
    };
    let fits = channels > 0
        && (8_000..=384_000).contains(&rate)
        && usize::from(block_align) == layout.frame_bytes();
    fits.then_some(layout)
}

/// The same mono sample in every channel of each frame; frames past `samples` are silence.
fn fill(samples: &[f32], layout: Layout, out: &mut [u8]) {
    out.fill(0);
    let width = layout.sample.bytes();
    let frame = layout.frame_bytes();
    if frame == 0 {
        return;
    }
    for (sample, frame) in samples.iter().zip(out.chunks_exact_mut(frame)) {
        let bytes = encode(*sample, layout.sample);
        for slot in frame.chunks_exact_mut(width) {
            slot.copy_from_slice(bytes.get(..width).unwrap_or_default());
        }
    }
}

fn encode(sample: f32, kind: Sample) -> [u8; 4] {
    let s = sample.clamp(-1.0, 1.0);
    match kind {
        Sample::F32 => s.to_le_bytes(),
        Sample::I16 => {
            let [a, b] = ((s * f32::from(i16::MAX)).round() as i16).to_le_bytes();
            [a, b, 0, 0]
        }
        Sample::I32 => ((f64::from(s) * f64::from(i32::MAX)).round() as i32).to_le_bytes(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::log::MemoryLog;
    use crate::sink::FakeEventSink;

    /// Stands in for the scheme read and the stream.
    #[derive(Default)]
    struct FakeAudio {
        scheme: Option<&'static str>,
        takes: Duration,
        /// While false the stream never ends.
        release: Option<Arc<AtomicBool>>,
        fail: Option<WinError>,
        played: Mutex<Vec<Cue>>,
    }

    impl Audio for FakeAudio {
        fn scheme(&self) -> Option<String> {
            self.scheme.map(str::to_owned)
        }

        fn play(&self, cue: Cue, called: Instant) -> Result<Played, WinError> {
            self.played.lock().unwrap().push(cue);
            if let Some(release) = &self.release {
                while !release.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
            }
            thread::sleep(self.takes);
            match self.fail {
                Some(error) => Err(error),
                None => Ok(Played {
                    form_factor: Some(1),
                    why: PickWhy::Default,
                    running_ms: 0,
                    done_ms: called.elapsed().as_millis(),
                    converted: false,
                }),
            }
        }
    }

    struct Rig {
        player: WasapiPlayer,
        sink: Arc<FakeEventSink>,
        log: Arc<MemoryLog>,
        audio: Arc<FakeAudio>,
    }

    fn rig(audio: FakeAudio, wait_limit: Duration) -> Rig {
        rig_with_sink(audio, wait_limit, FakeEventSink::default())
    }

    fn rig_with_sink(audio: FakeAudio, wait_limit: Duration, sink: FakeEventSink) -> Rig {
        let sink = Arc::new(sink);
        let log = Arc::new(MemoryLog::default());
        let audio = Arc::new(audio);
        let player = WasapiPlayer::with_audio(sink.clone(), log.clone(), audio.clone(), wait_limit);
        Rig {
            player,
            sink,
            log,
            audio,
        }
    }

    /// The sound threads answer late; wait for `n` log lines (each run writes exactly one), at most 5 s.
    fn wait_for_lines(log: &MemoryLog, n: usize) -> Vec<(Level, &'static str, String)> {
        let start = Instant::now();
        loop {
            let lines = log.lines.lock().unwrap().clone();
            if lines.len() >= n || start.elapsed() > Duration::from_secs(5) {
                return lines;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_for_events(sink: &FakeEventSink, n: usize) -> Vec<Event> {
        let start = Instant::now();
        loop {
            let events = sink.events();
            if events.len() >= n || start.elapsed() > Duration::from_secs(5) {
                return events;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn no_sounds_scheme_is_detected() {
        for (scheme, quiet) in [
            (Some(".None"), true),
            (Some(".none"), true),
            (Some(".NONE"), true),
            (Some(".Default"), false),
            (Some(".Current"), false),
            (Some("No Sounds"), false),
            (Some(".None2"), false),
            (Some(""), false),
            (None, false),
        ] {
            assert_eq!(is_no_sounds(scheme), quiet, "{scheme:?}");
        }
    }

    #[test]
    fn play_never_blocks() {
        let r = rig(
            FakeAudio {
                takes: Duration::from_secs(1),
                ..FakeAudio::default()
            },
            WAIT_LIMIT,
        );
        let start = Instant::now();
        r.player.play(Cue::LidClosedHolding);
        let took = start.elapsed();
        assert!(took < Duration::from_millis(50), "play took {took:?}");
        assert_eq!(wait_for_events(&r.sink, 1).len(), 1);
    }

    #[test]
    fn play_and_wait_stops_at_the_limit() {
        assert_eq!(WAIT_LIMIT, Duration::from_secs(3));
        let release = Arc::new(AtomicBool::new(false));
        let r = rig(
            FakeAudio {
                release: Some(release.clone()),
                ..FakeAudio::default()
            },
            Duration::from_millis(200),
        );
        let start = Instant::now();
        let result = r.player.play_and_wait(Cue::GoingToSleep);
        let took = start.elapsed();
        assert_eq!(
            result,
            Err(WinError {
                call: WAIT_CALL,
                code: ERROR_TIMEOUT.0
            })
        );
        assert!(
            took >= Duration::from_millis(200) && took < Duration::from_secs(2),
            "{took:?}"
        );
        release.store(true, Ordering::SeqCst);
        // The late answer is logged, never posted: `play_and_wait` answers its caller, not the loop.
        assert_eq!(wait_for_lines(&r.log, 1).len(), 1);
        assert!(r.sink.events().is_empty());
    }

    #[test]
    fn play_and_wait_returns_once_played() {
        let r = rig(
            FakeAudio {
                takes: Duration::from_millis(100),
                ..FakeAudio::default()
            },
            WAIT_LIMIT,
        );
        let start = Instant::now();
        assert_eq!(
            r.player.play_and_wait(Cue::GoingToSleep),
            Ok(SoundEnd::Played)
        );
        assert!(start.elapsed() >= Duration::from_millis(100));
        assert_eq!(*r.audio.played.lock().unwrap(), [Cue::GoingToSleep]);
        assert!(r.sink.events().is_empty());
        let lines = r.log.lines.lock().unwrap().clone();
        assert_eq!(lines.len(), 1);
        assert_eq!((lines[0].0, lines[0].1), (Level::Info, AREA));
        assert!(
            lines[0]
                .2
                .starts_with("going to sleep chime played on speakers (the default device)"),
            "{}",
            lines[0].2
        );
    }

    #[test]
    fn result_is_posted_as_done_sound() {
        let error = WinError {
            call: "IAudioClient::Initialize",
            code: 0x8889_0004,
        };
        let cases = [
            (FakeAudio::default(), Ok(SoundEnd::Played), Level::Info),
            (
                FakeAudio {
                    scheme: Some(".None"),
                    ..FakeAudio::default()
                },
                Ok(SoundEnd::Skipped),
                Level::Info,
            ),
            (
                FakeAudio {
                    fail: Some(error),
                    ..FakeAudio::default()
                },
                Err(Failure::from(error)),
                Level::Warn,
            ),
        ];
        for (audio, want, level) in cases {
            let skipped = want == Ok(SoundEnd::Skipped);
            let r = rig(audio, WAIT_LIMIT);
            r.player.play(Cue::BatteryFloor);
            assert_eq!(
                wait_for_events(&r.sink, 1),
                [Event::Done(Outcome::Sound(want))]
            );
            let lines = wait_for_lines(&r.log, 1);
            assert_eq!(lines.len(), 1, "{lines:?}");
            assert_eq!(lines[0].0, level, "{lines:?}");
            assert!(lines[0].2.starts_with("battery floor chime"), "{lines:?}");
            // The scheme is read on the sound thread, and No sounds never opens a stream.
            let played = r.audio.played.lock().unwrap().clone();
            assert_eq!(played.is_empty(), skipped);
        }
        assert_eq!(
            failed_text(error),
            "IAudioClient::Initialize failed with 0x88890004"
        );
    }

    #[test]
    fn a_lost_post_is_logged() {
        let r = rig_with_sink(
            FakeAudio::default(),
            WAIT_LIMIT,
            FakeEventSink {
                fail_with: Some(WinError {
                    call: "PostMessageW",
                    code: 1816,
                }),
                ..FakeEventSink::default()
            },
        );
        r.player.play(Cue::LidClosedHolding);
        let lines = wait_for_lines(&r.log, 2);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(
            (lines[1].0, lines[1].2.as_str()),
            (
                Level::Warn,
                "lid closed chime result not posted: PostMessageW failed with 1816"
            )
        );
    }

    #[test]
    fn player_is_send_and_sync() {
        const fn send_and_sync<T: Send + Sync>() {}
        const _: () = send_and_sync::<WasapiPlayer>();
        let player: Arc<dyn SoundPlayer> = Arc::new(WasapiPlayer::new(
            Arc::new(FakeEventSink::default()),
            Arc::new(MemoryLog::default()),
        ));
        drop(player);
    }

    #[test]
    fn mix_formats_are_written_or_left_to_the_engine() {
        let ext = WAVE_FORMAT_EXTENSIBLE as u16;
        let float = WAVE_FORMAT_IEEE_FLOAT as u16;
        let pcm = WAVE_FORMAT_PCM as u16;
        let f = Some(KSDATAFORMAT_SUBTYPE_IEEE_FLOAT);
        let p = Some(KSDATAFORMAT_SUBTYPE_PCM);
        let at = |rate, channels, sample| {
            Some(Layout {
                rate,
                channels,
                sample,
            })
        };
        // (tag, channels, rate, bits, block align, sub format) -> layout
        let table = [
            ((ext, 2, 48_000, 32, 8, f), at(48_000, 2, Sample::F32)),
            ((ext, 6, 44_100, 32, 24, f), at(44_100, 6, Sample::F32)),
            ((ext, 2, 48_000, 16, 4, p), at(48_000, 2, Sample::I16)),
            ((ext, 2, 96_000, 32, 8, p), at(96_000, 2, Sample::I32)),
            ((float, 2, 48_000, 32, 8, None), at(48_000, 2, Sample::F32)),
            ((pcm, 1, 48_000, 16, 2, None), at(48_000, 1, Sample::I16)),
            ((ext, 2, 48_000, 24, 6, p), None),
            ((ext, 2, 48_000, 32, 8, None), None),
            ((ext, 2, 48_000, 64, 16, f), None),
            ((ext, 2, 48_000, 32, 6, f), None),
            ((ext, 0, 48_000, 32, 0, f), None),
            ((pcm, 2, 0, 16, 4, None), None),
        ];
        for ((tag, channels, rate, bits, align, sub), want) in table {
            assert_eq!(
                layout(tag, channels, rate, bits, align, sub),
                want,
                "{tag} {channels} {rate} {bits} {align}"
            );
        }
        assert_eq!(FALLBACK.frame_bytes(), 2);
    }

    #[test]
    fn samples_fill_every_channel() {
        let stereo = |sample| Layout {
            rate: 48_000,
            channels: 2,
            sample,
        };
        let mut out = [0xAAu8; 16];
        fill(&[0.5, -1.0], stereo(Sample::F32), &mut out);
        let floats: Vec<f32> = out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        assert_eq!(floats, [0.5, 0.5, -1.0, -1.0]);

        let mut out = [0xAAu8; 12];
        fill(&[1.0, -2.0], stereo(Sample::I16), &mut out);
        let ints: Vec<i16> = out
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
            .collect();
        // The frame past the samples is silence, and a sample over full scale is clipped.
        assert_eq!(ints, [i16::MAX, i16::MAX, -i16::MAX, -i16::MAX, 0, 0]);

        let mut out = [0xAAu8; 8];
        fill(&[-0.5], stereo(Sample::I32), &mut out);
        let wide: Vec<i32> = out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i32::from_le_bytes(*c))
            .collect();
        assert_eq!(wide, [-(i32::MAX / 2) - 1, -(i32::MAX / 2) - 1]);
    }
}
