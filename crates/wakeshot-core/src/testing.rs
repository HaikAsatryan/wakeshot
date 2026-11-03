//! Fakes for the core's boundaries, so each track tests its side without the other. Built for this crate's tests and
//! for other crates through the `test-support` feature (dev-dependencies only).
//!
//! Owner: contracts card.

use std::cell::Cell;

use crate::sources::{AgentSource, Fed, Feed, FileId, SessionFacts, SourceId, WantedFile};
use crate::time::{Clock, Now};

/// A clock the test moves by hand. `run`, `suspend` and `set_wall` move the two clocks apart the way a real PC does,
/// so suspends and clock changes are covered (architecture section 9).
///
/// Owner: core track (Phase 2). Stands in for `wakeshot_windows::clock::SystemClock`.
#[derive(Debug)]
pub struct FakeClock {
    now: Cell<Now>,
}

impl FakeClock {
    pub fn new(start: Now) -> Self {
        Self {
            now: Cell::new(start),
        }
    }

    /// The PC runs for `ms`: both clocks move.
    pub fn run(&self, ms: u64) {
        let t = self.now.get();
        self.now.set(Now {
            wall_ms: t
                .wall_ms
                .checked_add_unsigned(ms)
                .expect("wall clock overflow"),
            mono_ms: t.mono_ms + ms,
        });
    }

    /// The PC sleeps or hibernates for `ms`: only the wall clock moves.
    pub fn suspend(&self, ms: u64) {
        let t = self.now.get();
        self.now.set(Now {
            wall_ms: t
                .wall_ms
                .checked_add_unsigned(ms)
                .expect("wall clock overflow"),
            ..t
        });
    }

    /// Someone changes the clock: the wall clock jumps, the PC's run time does not.
    pub fn set_wall(&self, wall_ms: i64) {
        let t = self.now.get();
        self.now.set(Now { wall_ms, ..t });
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Now {
        self.now.get()
    }
}

/// A source whose wants and sessions the test sets, and which records every feed. A file listed in `retry` gets
/// `Fed::RetryOnce` once and is then taken off the list; every other feed gets `Fed::Taken`.
///
/// Owner: sources track (Phase 4). Stands in for a parser on the watch thread's side, and for the watch thread on
/// the core's side (its `sessions` become `Event::Sources`).
#[derive(Debug)]
pub struct FakeSource {
    pub id: SourceId,
    pub wanted: Vec<WantedFile>,
    pub sessions: Vec<SessionFacts>,
    pub fed: Vec<(FileId, Feed)>,
    pub retry: Vec<FileId>,
}

impl FakeSource {
    pub fn new(id: SourceId) -> Self {
        Self {
            id,
            wanted: Vec::new(),
            sessions: Vec::new(),
            fed: Vec::new(),
            retry: Vec::new(),
        }
    }
}

impl AgentSource for FakeSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn wanted(&self) -> Vec<WantedFile> {
        self.wanted.clone()
    }

    fn feed(&mut self, file: &FileId, data: Feed) -> Fed {
        self.fed.push((file.clone(), data));
        match self.retry.iter().position(|f| f == file) {
            Some(i) => {
                self.retry.remove(i);
                Fed::RetryOnce
            }
            None => Fed::Taken,
        }
    }

    fn sessions(&self) -> Vec<SessionFacts> {
        self.sessions.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::event::Event;
    use crate::process::{Alive, FileTime};
    use crate::sources::{Activity, FormatHealth, Liveness, SessionKey};

    const START: Now = Now {
        wall_ms: 1_793_750_400_000,
        mono_ms: 5_000,
    };

    /// What a loop does with any clock: read it before each event.
    fn read_twice(clock: &dyn Clock, between: impl FnOnce()) -> (Now, Now) {
        let a = clock.now();
        between();
        (a, clock.now())
    }

    #[test]
    fn fake_clock_stands_in_for_the_real_clock() {
        let clock = FakeClock::new(START);

        let (a, b) = read_twice(&clock, || clock.run(30_000));
        assert_eq!(
            (b.wall_ms - a.wall_ms, b.mono_ms - a.mono_ms),
            (30_000, 30_000)
        );

        let (a, b) = read_twice(&clock, || clock.suspend(3_600_000));
        assert_eq!((b.wall_ms - a.wall_ms, b.mono_ms), (3_600_000, a.mono_ms));

        let (a, b) = read_twice(&clock, || clock.set_wall(START.wall_ms - 60_000));
        assert_eq!((b.wall_ms, b.mono_ms), (START.wall_ms - 60_000, a.mono_ms));
    }

    fn status_name(name: &str) -> bool {
        name.strip_suffix(".json")
            .is_some_and(|stem| !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_digit()))
    }

    /// What the watch thread does with any source: filter names, feed (again once when asked), post the snapshot with
    /// liveness.
    fn watch_once(source: &mut dyn AgentSource, folder: &[(&str, &[u8])]) -> Event {
        for want in source.wanted() {
            if let WantedFile::Folder { dir, accept } = want {
                for (name, bytes) in folder {
                    if accept(name) {
                        let id = FileId(dir.join(name));
                        if source.feed(&id, Feed::Whole(bytes.to_vec())) == Fed::RetryOnce {
                            source.feed(&id, Feed::Whole(bytes.to_vec()));
                        }
                    }
                }
            }
        }
        Event::Sources {
            source: source.id(),
            sessions: source
                .sessions()
                .into_iter()
                .map(|s| (s, Alive::Yes))
                .collect(),
        }
    }

    #[test]
    fn fake_source_stands_in_for_a_parser() {
        let dir = PathBuf::from(r"C:\Users\x\.claude\sessions");
        let session = SessionFacts {
            key: SessionKey {
                source: SourceId::ClaudeCode,
                id: "a1".into(),
            },
            liveness: Liveness::Process {
                pid: 20636,
                start: FileTime(134_381_211_391_959_336),
            },
            activity: Activity::Working,
            timers: vec![],
            open_tasks: 0,
            label: "Claude Code",
            format: FormatHealth::Ok,
        };
        let mut source = FakeSource::new(SourceId::ClaudeCode);
        source.wanted = vec![WantedFile::Folder {
            dir: dir.clone(),
            accept: status_name,
        }];
        source.sessions = vec![session.clone()];
        let status = FileId(dir.join("20636.json"));
        source.retry = vec![status.clone()];

        let event = watch_once(
            &mut source,
            &[("20636.json", b"{}"), ("20636.abc.key", b"secret")],
        );

        let fed = (status, Feed::Whole(b"{}".to_vec()));
        assert_eq!(source.fed, [fed.clone(), fed]);
        assert!(source.retry.is_empty());
        assert_eq!(
            event,
            Event::Sources {
                source: SourceId::ClaudeCode,
                sessions: vec![(session, Alive::Yes)],
            }
        );
    }
}
