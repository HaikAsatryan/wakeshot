//! Wakeshot's three chimes, made in code at the stream's own rate, so there is no WAV resource and no resampling.
//!
//! `media/scripts/make-sounds.mjs` reads the `Shape` constants with regular expressions: keep their layout.

use std::f32::consts::PI;

use wakeshot_core::view::Cue;

/// A codec that wakes on the first sample may swallow the start of a short sound; this silence takes that hit
/// instead of the tone. The chime tested at two real lid closes had it; whether it is needed was not tested (research,
/// "Lid-close sound").
pub(super) const LEAD_MS: u32 = 150;

/// The stream stops once the engine has taken every frame. What is still in the device buffer then is this silence,
/// so no tone is cut.
pub(super) const TAIL_MS: u32 = 100;

/// One tone of a chime.
#[derive(Clone, Copy, Debug)]
pub(super) struct Note {
    /// From the end of the lead silence.
    pub start_ms: u32,
    pub length_ms: u32,
    pub hz: f32,
    /// Exponential fall per second: a higher value sounds more struck, a lower one more held.
    pub decay: f32,
    /// Level of the second harmonic: it makes a low tone carry on small laptop speakers, which are weak below about
    /// 1 kHz (the reason the lid chime sits at 1 to 1.6 kHz).
    pub overtone: f32,
}

/// A whole chime: its notes, its loudest sample and how softly each note starts and ends.
#[derive(Clone, Copy, Debug)]
pub(super) struct Shape {
    pub notes: &'static [Note],
    pub peak_dbfs: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

/// Rising, so it reads as "still going". The chime tested at two real lid closes, with its peak set to exactly
/// -6 dBFS.
const LID_CLOSED_HOLDING: Shape = Shape {
    notes: &[
        Note {
            start_ms: 0,
            length_ms: 160,
            hz: 1046.5,
            decay: 8.0,
            overtone: 0.0,
        },
        Note {
            start_ms: 160,
            length_ms: 340,
            hz: 1568.0,
            decay: 5.0,
            overtone: 0.0,
        },
    ],
    peak_dbfs: -6.0,
    attack_ms: 5.0,
    release_ms: 10.0,
};

/// The lid chime turned around, so the pair reads as "stays on" and "goes off". Softer: 3 dB lower and gentler
/// edges, since it may play at night with nobody near. Short, because a lid opened while it plays still hibernates.
const GOING_TO_SLEEP: Shape = Shape {
    notes: &[
        Note {
            start_ms: 0,
            length_ms: 170,
            hz: 1568.0,
            decay: 6.0,
            overtone: 0.0,
        },
        Note {
            start_ms: 170,
            length_ms: 380,
            hz: 1046.5,
            decay: 4.0,
            overtone: 0.0,
        },
    ],
    peak_dbfs: -9.0,
    attack_ms: 15.0,
    release_ms: 30.0,
};

/// Two short low pulses on one pitch: a warning, unlike the two-note chimes.
const BATTERY_FLOOR: Shape = Shape {
    notes: &[
        Note {
            start_ms: 0,
            length_ms: 130,
            hz: 659.25,
            decay: 10.0,
            overtone: 0.35,
        },
        Note {
            start_ms: 230,
            length_ms: 130,
            hz: 659.25,
            decay: 10.0,
            overtone: 0.35,
        },
    ],
    peak_dbfs: -6.0,
    attack_ms: 5.0,
    release_ms: 15.0,
};

pub(super) fn shape(cue: Cue) -> &'static Shape {
    match cue {
        Cue::LidClosedHolding => &LID_CLOSED_HOLDING,
        Cue::GoingToSleep => &GOING_TO_SLEEP,
        Cue::BatteryFloor => &BATTERY_FLOOR,
    }
}

/// The whole chime with its lead and tail silence.
pub(super) fn length_ms(cue: Cue) -> u32 {
    let notes_end = shape(cue)
        .notes
        .iter()
        .map(|n| n.start_ms + n.length_ms)
        .max()
        .unwrap_or(0);
    LEAD_MS + notes_end + TAIL_MS
}

/// Frames for `ms` at `rate`; u64 so no rate overflows it.
pub(super) fn frames(ms: u32, rate: u32) -> usize {
    usize::try_from(u64::from(ms) * u64::from(rate) / 1000).unwrap_or(usize::MAX)
}

/// The chime as mono samples, full scale 1.0, at `rate` frames per second.
pub(super) fn render(cue: Cue, rate: u32) -> Vec<f32> {
    let shape = shape(cue);
    let mut out = vec![0.0f32; frames(length_ms(cue), rate)];
    let rate_f = rate as f32;
    let attack = shape.attack_ms / 1000.0;
    let release = shape.release_ms / 1000.0;
    for note in shape.notes {
        let first = frames(LEAD_MS + note.start_ms, rate);
        let count = frames(note.length_ms, rate);
        let length = note.length_ms as f32 / 1000.0;
        // Iterators, not indexes: the tray builds with `panic = "abort"`, so a slip here must not end the process.
        for (i, sample) in out.iter_mut().skip(first).take(count).enumerate() {
            let t = i as f32 / rate_f;
            let envelope =
                fade(t / attack) * fade((length - t) / release) * (-note.decay * t).exp();
            let phase = 2.0 * PI * note.hz * t;
            *sample += (phase.sin() + note.overtone * (2.0 * phase).sin()) * envelope;
        }
    }
    let loudest = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if loudest > 0.0 {
        let scale = 10f32.powf(shape.peak_dbfs / 20.0) / loudest;
        for sample in &mut out {
            *sample *= scale;
        }
    }
    out
}

/// A raised-cosine ramp from 0 at `x <= 0` to 1 at `x >= 1`: its slope is zero at both ends, so neither the start
/// nor the end of a note clicks.
fn fade(x: f32) -> f32 {
    if x <= 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        0.5 - 0.5 * (PI * x).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUES: [Cue; 3] = [Cue::LidClosedHolding, Cue::GoingToSleep, Cue::BatteryFloor];
    /// The two rates a shared-mode mix format has on most PCs.
    const RATES: [u32; 2] = [48_000, 44_100];

    fn db(x: f32) -> f32 {
        20.0 * x.log10()
    }

    fn loudest(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    #[test]
    fn chimes_have_lead_silence_fades_and_peak() {
        for cue in CUES {
            let shape = shape(cue);
            for rate in RATES {
                let pcm = render(cue, rate);
                let peak = loudest(&pcm);
                let ms = |n: u32| frames(n, rate);
                assert!(
                    (db(peak) - shape.peak_dbfs).abs() < 0.05,
                    "{cue:?} at {rate}: peak {} dBFS",
                    db(peak)
                );
                assert!(
                    pcm[..ms(LEAD_MS)].iter().all(|s| *s == 0.0),
                    "{cue:?}: lead silence"
                );
                assert!(
                    pcm[pcm.len() - ms(TAIL_MS)..].iter().all(|s| *s == 0.0),
                    "{cue:?}: tail silence"
                );
                // A note that starts or ends away from zero clicks.
                for note in shape.notes {
                    let first = ms(LEAD_MS + note.start_ms);
                    let last = first + ms(note.length_ms) - 1;
                    for edge in [first, last] {
                        assert!(
                            pcm[edge].abs() < 0.02 * peak,
                            "{cue:?} at {rate}: note edge {edge} is {}",
                            pcm[edge]
                        );
                    }
                    let ramp_in = loudest(&pcm[first..first + ms(1)]);
                    let ramp_out = loudest(&pcm[last + 1 - ms(1)..=last]);
                    assert!(ramp_in < 0.25 * peak, "{cue:?}: fade in, {ramp_in}");
                    assert!(ramp_out < 0.25 * peak, "{cue:?}: fade out, {ramp_out}");
                }
                // A jump from the chime's level to silence would be a step of 0.35 or more.
                let step = pcm
                    .windows(2)
                    .map(|w| (w[1] - w[0]).abs())
                    .fold(0.0f32, f32::max);
                assert!(step < 0.15, "{cue:?} at {rate}: step {step}");
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Direction {
        Rising,
        Falling,
        Level,
    }

    /// The pitch heard in each note, from its zero crossings: a sine crosses zero twice per cycle, and the overtone
    /// used here adds no crossing (`sin x (1 + 2a cos x)` with `a < 0.5`).
    fn heard_hz(cue: Cue, rate: u32) -> Vec<f32> {
        let pcm = render(cue, rate);
        shape(cue)
            .notes
            .iter()
            .map(|note| {
                let first = frames(LEAD_MS + note.start_ms, rate);
                let part = &pcm[first..first + frames(note.length_ms, rate)];
                let signs: Vec<bool> = part
                    .iter()
                    .filter(|s| **s != 0.0)
                    .map(|s| *s > 0.0)
                    .collect();
                let crossings = signs.windows(2).filter(|w| w[0] != w[1]).count();
                crossings as f32 / 2.0 / (note.length_ms as f32 / 1000.0)
            })
            .collect()
    }

    fn direction(hz: &[f32]) -> Direction {
        let (first, last) = (hz[0], hz[hz.len() - 1]);
        if last > first * 1.2 {
            Direction::Rising
        } else if last < first / 1.2 {
            Direction::Falling
        } else {
            Direction::Level
        }
    }

    /// True when the notes are separate pulses with silence between them.
    fn pulsed(cue: Cue) -> bool {
        shape(cue)
            .notes
            .windows(2)
            .all(|w| w[1].start_ms >= w[0].start_ms + w[0].length_ms + 50)
    }

    #[test]
    fn chimes_differ_in_pitch_direction() {
        for rate in RATES {
            for cue in CUES {
                for (heard, note) in heard_hz(cue, rate).iter().zip(shape(cue).notes) {
                    assert!(
                        (heard / note.hz - 1.0).abs() < 0.03,
                        "{cue:?} at {rate}: heard {heard} Hz, made {} Hz",
                        note.hz
                    );
                }
            }
            let lid = heard_hz(Cue::LidClosedHolding, rate);
            let sleep = heard_hz(Cue::GoingToSleep, rate);
            let floor = heard_hz(Cue::BatteryFloor, rate);
            assert_eq!(direction(&lid), Direction::Rising, "{lid:?}");
            assert_eq!(direction(&sleep), Direction::Falling, "{sleep:?}");
            assert_eq!(direction(&floor), Direction::Level, "{floor:?}");
            // Low: below both other chimes' lowest note.
            let lowest = |hz: &[f32]| hz.iter().fold(f32::MAX, |m, h| m.min(*h));
            assert!(lowest(&floor) < lowest(&lid) / 1.2 && lowest(&floor) < lowest(&sleep) / 1.2);
        }
        assert!(!pulsed(Cue::LidClosedHolding) && !pulsed(Cue::GoingToSleep));
        assert!(pulsed(Cue::BatteryFloor));
        assert_eq!(shape(Cue::BatteryFloor).notes.len(), 2);
    }

    #[test]
    fn chimes_fit_the_wait_limit() {
        // The slowest stream start seen on the test laptop (research, "Lid-close sound").
        const START_MS: u32 = 800;
        for cue in CUES {
            let ms = length_ms(cue);
            assert!(ms < 1200, "{cue:?}: {ms} ms");
            assert_eq!(render(cue, 48_000).len(), frames(ms, 48_000));
            let wait_ms = u32::try_from(crate::sound::player::WAIT_LIMIT.as_millis()).unwrap();
            assert!(START_MS + ms < wait_ms, "{cue:?}: {ms} ms");
        }
        // The going to sleep chime is the one a hibernate waits for: no longer than the lid chime.
        assert!(length_ms(Cue::GoingToSleep) <= length_ms(Cue::LidClosedHolding) + 100);
    }

    #[test]
    fn the_lid_chime_is_the_tested_one() {
        assert_eq!(length_ms(Cue::LidClosedHolding), 750);
        let hz: Vec<f32> = LID_CLOSED_HOLDING.notes.iter().map(|n| n.hz).collect();
        assert_eq!(hz, [1046.5, 1568.0]);
    }
}
