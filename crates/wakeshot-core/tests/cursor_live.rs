//! Cursor's table replayed on the live hook inputs in `fixtures/cursor/live/` (`p4-live-cursor`, editor 3.27.12 and
//! CLI 2026.11.05-86047df): each input reads, and each live sequence leaves the chat files Cursor's real turns call
//! for: a turn's own `stop` ends it, `completed` or `aborted`, and a running chat's closed tab keeps it working until
//! that `stop`.
//!
//! Owner: `p4-live-cursor`.

use std::collections::BTreeMap;
use std::path::Path;

use wakeshot_core::process::{FileTime, ProcessRef};
use wakeshot_core::sources::cursor::AGENT;
use wakeshot_core::sources::hooked::{self, FileRead, Op, State};

const R2: &str = "5d0c9e2a-7b41-4f3e-9a8d-1c6b2e4f7a90";
const A: &str = "6e1d0f3b-8c52-4a4f-8b9e-2d7c3f5a8b01";
const B: &str = "7f2e1a4c-9d63-4b5a-9caf-3e8d4a6b9c12";
const C: &str = "8a3f2b5d-ae74-4c6b-8db0-4f9e5b7cad23";

/// Every live input with its event.
const INPUTS: [(&str, &str); 11] = [
    ("r2-1-before-submit-prompt.json", "beforeSubmitPrompt"),
    ("r2-2-stop-completed.json", "stop"),
    ("r2-3-before-submit-prompt.json", "beforeSubmitPrompt"),
    ("r2-4-stop-aborted.json", "stop"),
    ("r3-1-session-end-idle.json", "sessionEnd"),
    ("r3-2-before-submit-prompt-a.json", "beforeSubmitPrompt"),
    ("r3-3-before-submit-prompt-b.json", "beforeSubmitPrompt"),
    ("r3-4-session-end-running-b.json", "sessionEnd"),
    ("r3-5-stop-completed-a.json", "stop"),
    ("r3-6-stop-completed-b.json", "stop"),
    ("cli-session-end.json", "sessionEnd"),
];

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cursor")
        .join("live")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The extension host of the test window, as the walk found it.
const HOST: ProcessRef = ProcessRef {
    pid: 4120,
    start: FileTime(134_384_524_042_912_323),
};

/// The chat files of `agents\cursor\` as the hook leaves them: chat id to file text.
#[derive(Default)]
struct Folder(BTreeMap<String, Vec<u8>>);

impl Folder {
    /// What `hook cursor <event>` does with one input at Medium, after the walk found `HOST` (`hook.rs`, `apply`).
    fn run(&mut self, name: &str, event: &str) -> Op {
        let rule = AGENT.event(event).unwrap();
        let input = hooked::parse_input(&AGENT, event, &fixture(name)[..])
            .input
            .unwrap_or_else(|problem| panic!("{name}: {problem}"));
        let op = hooked::resolve(rule, &input).unwrap();
        match op {
            Op::Write(state) => {
                let text = hooked::state_text(HOST, &input.turn, "Cursor", state);
                self.0.insert(input.chat, text);
            }
            Op::DeleteIfTurn => {
                if self
                    .0
                    .get(&input.chat)
                    .is_some_and(|bytes| hooked::names_turn(bytes, &input.turn))
                {
                    self.0.remove(&input.chat);
                }
            }
            Op::Delete => {
                self.0.remove(&input.chat);
            }
            Op::Problem(problem) => panic!("{name}: {problem:?}"),
        }
        op
    }

    fn chats(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }

    fn turn(&self, chat: &str) -> String {
        match hooked::read_state(&self.0[chat]) {
            FileRead::V1(file) => file.generation,
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn every_live_input_reads_with_its_ids_and_version() {
    for (name, event) in INPUTS {
        let parsed = hooked::parse_input(&AGENT, event, &fixture(name)[..]);
        let input = parsed.input.unwrap_or_else(|p| panic!("{name}: {p}"));
        assert_eq!(
            hooked::chat_id(&input.chat).as_deref(),
            Some(input.chat.as_str()),
            "{name}"
        );
        let version = if name.starts_with("cli") {
            "2026.11.05-86047df"
        } else {
            "3.27.12"
        };
        assert_eq!(parsed.version.as_deref(), Some(version), "{name}");
    }
}

#[test]
fn a_turns_own_stop_ends_it_completed_or_cancelled_with_the_stop_button() {
    let mut folder = Folder::default();
    assert_eq!(
        folder.run("r2-1-before-submit-prompt.json", "beforeSubmitPrompt"),
        Op::Write(State::Working)
    );
    assert_eq!(folder.chats(), [R2]);
    assert_eq!(
        folder.run("r2-2-stop-completed.json", "stop"),
        Op::DeleteIfTurn
    );
    assert!(
        folder.chats().is_empty(),
        "the prompt and its stop carry one turn id"
    );
    folder.run("r2-3-before-submit-prompt.json", "beforeSubmitPrompt");
    assert_eq!(folder.chats(), [R2]);
    folder.run("r2-4-stop-aborted.json", "stop");
    assert!(
        folder.chats().is_empty(),
        "the Stop button's stop names the turn it ended"
    );
}

#[test]
fn a_running_chats_closed_tab_keeps_it_working_until_its_own_stop() {
    let mut folder = Folder::default();
    // An empty chat closed: nothing to end.
    assert_eq!(
        folder.run("r3-1-session-end-idle.json", "sessionEnd"),
        Op::Delete
    );
    folder.run("r3-2-before-submit-prompt-a.json", "beforeSubmitPrompt");
    folder.run("r3-3-before-submit-prompt-b.json", "beforeSubmitPrompt");
    assert_eq!(folder.chats(), [A, B]);
    let b_before = folder.0[B].clone();
    // Ctrl+F4 on B while it ran: Cursor says `final_status` `generating`, and B's turn went on for 50 s.
    assert_eq!(
        folder.run("r3-4-session-end-running-b.json", "sessionEnd"),
        Op::Write(State::Working)
    );
    assert_eq!(folder.chats(), [A, B], "the closed tab's turn still runs");
    assert_eq!(
        folder.0[B], b_before,
        "the same file, so the hook skips the write"
    );
    // A ran on behind Ctrl+N; each background turn ends with its own stop.
    folder.run("r3-5-stop-completed-a.json", "stop");
    assert_eq!(folder.chats(), [B]);
    assert_eq!(folder.turn(B), "b4dafa68-5e7c-4d9b-9f3a-2c4e6b8dad54");
    folder.run("r3-6-stop-completed-b.json", "stop");
    assert!(folder.chats().is_empty());
    assert!(!folder.chats().contains(&C));
}

#[test]
fn the_clis_session_end_ends_its_chat() {
    let mut folder = Folder::default();
    assert_eq!(folder.run("cli-session-end.json", "sessionEnd"), Op::Delete);
    assert!(folder.chats().is_empty());
}

#[test]
fn the_live_chat_file_is_a_version_1_working_file() {
    match hooked::read_state(&fixture("state-working.json")) {
        FileRead::V1(file) => {
            assert_eq!(
                (file.pid, file.label.as_str(), file.state),
                (4120, "Cursor", State::Working)
            );
            assert_eq!(file.generation, "e1a7c3d5-2b4f-4a6e-8c0d-9f1b3e5a7c21");
        }
        other => panic!("{other:?}"),
    }
}
