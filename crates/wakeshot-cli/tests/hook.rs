//! The built `wakeshot.exe hook`, as an agent's hook runs it: `{}` on stdout, nothing on stderr, exit 0, for every form
//! (`docs/agent-hooks.md`, "The hook command", step 6 and "The `wakeshot hook` command form is frozen").
//!
//! Owner: `p4-cursor-source` (first version from `p4-contracts-r3`).

use std::process::{Command, Output};

fn wakeshot(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wakeshot"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn every_hook_form_answers_an_empty_object_with_exit_0() {
    for args in [
        &["hook", "cursor", "beforeSubmitPrompt"][..],
        &["hook", "cursor", "stop"],
        &["hook", "cursor", "sessionEnd"],
        &["hook", "cursor", "remove"],
        &["hook", "antigravity", "PreInvocation"],
        &["--dry-run", "hook", "cursor", "stop"],
        &[
            "hook",
            "cursor",
            "beforeSubmitPrompt",
            "--checked",
            "a",
            "b",
        ],
        // Forms no version knows: still the answer, never help or an error.
        &["hook"],
        &["hook", "--help"],
        &["hook", "cursor", "stop", "--dry-run"],
        &["hook", "nobody", "nothing", "extra"],
        &["HOOK", "Cursor", "Stop"],
    ] {
        let out = wakeshot(args);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert_eq!(out.stdout, b"{}", "{args:?}");
        assert!(out.stderr.is_empty(), "{args:?}");
    }
}
