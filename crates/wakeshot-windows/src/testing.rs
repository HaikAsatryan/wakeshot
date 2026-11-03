//! Fakes for this crate's boundary traits, so the core, the loop and the IO thread are tested without Windows doing
//! anything. Built for this crate's tests and for other crates through the `test-support` feature (dev-dependencies
//! only). Each fake lives beside its trait; this module gathers them in one place.
//!
//! Owner: contracts card. The plan seam's fake is the core's `ownership::fake::FakePlans`.

pub use crate::log::MemoryLog;
pub use crate::notify::FakeNotifier;
pub use crate::process::FakeLiveness;
pub use crate::sink::FakeEventSink;
pub use crate::sound::FakeSoundPlayer;

use wakeshot_core::view::MenuModel;

use crate::tray::{Anchor, MenuView};

/// Records each menu shown, so a test can check the model and then post the pick itself.
///
/// Owner: tray track (Phase 5). Stands in for the owner-drawn Win32 menu.
#[derive(Debug, Default)]
pub struct FakeMenuView {
    pub shown: Vec<(MenuModel, Anchor)>,
}

impl MenuView for FakeMenuView {
    fn show(&mut self, menu: &MenuModel, anchor: Anchor) {
        self.shown.push((menu.clone(), anchor));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use wakeshot_core::command::{Failure, Outcome, SoundEnd};
    use wakeshot_core::event::Event;
    use wakeshot_core::view::{Cue, KeepAwake, MenuCommand, MenuItem, Notice};

    use super::*;
    use crate::WinError;
    use crate::log::{Level, Log};
    use crate::notify::Notifier;
    use crate::sink::EventSink;
    use crate::sound::SoundPlayer;

    #[test]
    fn fake_menu_view_stands_in_for_the_menu() {
        let pick = MenuCommand::KeepAwake(KeepAwake::Minutes(30));
        let model = MenuModel {
            items: vec![
                MenuItem::Label("Holding awake: Claude Code".into()),
                MenuItem::Separator,
                MenuItem::Action {
                    text: "30 minutes".into(),
                    command: pick,
                    checked: false,
                    enabled: true,
                },
            ],
        };
        let mut fake = FakeMenuView::default();
        let view: &mut dyn MenuView = &mut fake;
        view.show(&model, Anchor { x: 1800, y: 1180 });

        let (shown, anchor) = &fake.shown[0];
        assert_eq!((shown, anchor.x), (&model, 1800));
        let posted: Vec<Event> = shown
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { command, .. } => Some(Event::Menu(*command)),
                _ => None,
            })
            .collect();
        assert_eq!(posted, [Event::Menu(pick)]);
    }

    #[test]
    fn fake_notifier_stands_in_for_balloons() {
        let mut fake = FakeNotifier::default();
        let outcome = |n: &dyn Notifier| {
            Outcome::Notified(n.show(&Notice::RestoreFailed).map_err(Failure::from))
        };

        assert_eq!(outcome(&fake), Outcome::Notified(Ok(())));
        assert_eq!(*fake.shown.borrow(), [Notice::RestoreFailed]);

        fake.fail_with = Some(WinError {
            call: "Shell_NotifyIconW",
            code: 0x8000_4005,
        });
        assert_eq!(
            outcome(&fake),
            Outcome::Notified(Err(Failure {
                call: "Shell_NotifyIconW",
                code: 0x8000_4005,
            }))
        );
    }

    /// The player posts its result through the sink, as the real one does from its own thread.
    #[test]
    fn fake_sound_player_posts_through_the_sink() {
        let sink = Arc::new(FakeEventSink::default());
        let fake = FakeSoundPlayer {
            sink: Some(sink.clone()),
            ..FakeSoundPlayer::default()
        };
        let player: &dyn SoundPlayer = &fake;
        player.play(Cue::LidClosedHolding);
        assert_eq!(
            player.play_and_wait(Cue::GoingToSleep),
            Ok(SoundEnd::Played)
        );
        assert_eq!(
            *fake.played.lock().unwrap(),
            [Cue::LidClosedHolding, Cue::GoingToSleep]
        );
        assert_eq!(
            sink.events(),
            [Event::Done(Outcome::Sound(Ok(SoundEnd::Played)))]
        );

        let error = WinError {
            call: "IAudioClient::Start",
            code: 0x8889_0004,
        };
        let failing = FakeSoundPlayer {
            wait_error: Some(error),
            ..FakeSoundPlayer::default()
        };
        assert_eq!(failing.play_and_wait(Cue::GoingToSleep), Err(error));
    }

    #[test]
    fn fake_event_sink_records_or_fails() {
        let sink = FakeEventSink::default();
        let poster: &dyn EventSink = &sink;
        poster.post(Event::Tick).unwrap();
        assert_eq!(sink.events(), [Event::Tick]);

        let full = WinError {
            call: "PostMessageW",
            code: 1816,
        };
        let failing = FakeEventSink {
            fail_with: Some(full),
            ..FakeEventSink::default()
        };
        assert_eq!(failing.post(Event::Tick), Err(full));
        assert!(failing.events().is_empty());
    }

    #[test]
    fn memory_log_keeps_every_line() {
        let log = MemoryLog::default();
        let writer: &dyn Log = &log;
        writer.line(Level::Warn, "power", "journal set aside");
        assert_eq!(
            *log.lines.lock().unwrap(),
            [(Level::Warn, "power", "journal set aside".to_owned())]
        );
    }
}
