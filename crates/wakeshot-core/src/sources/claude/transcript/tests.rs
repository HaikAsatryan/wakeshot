use super::*;
use crate::holds::file::format_expires;
use crate::sources::cron::FixedOffset;
use proptest::prelude::*;
use serde_json::{Value, json};

// A Monitor's one due time covers its run only while the core holds a timer at least this long before it.
const _: () = assert!(crate::policy::sessions::TIMER_LEAD_MS >= MONITOR_MAX_MS);

/// The test laptop's zone at the live tests: UTC+4, no daylight saving.
const ZONE: FixedOffset = FixedOffset(240);
/// 2026-11-03T14:19:23.530Z, when the live `/loop 1m` job was made.
const T: i64 = 1_793_715_563_530;
const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const DAY: i64 = 24 * HOUR;
const SID: &str = "5e550004-0000-4000-8000-000000000004";

fn iso(ms: i64) -> String {
    format_expires(ms).replace('Z', &format!(".{:03}Z", ms.rem_euclid(1000)))
}

fn line(mut fields: Value, at: i64) -> String {
    fields["timestamp"] = json!(iso(at));
    fields["sessionId"] = json!(SID);
    fields["entrypoint"] = json!("cli");
    fields["version"] = json!("2.1.317");
    fields.to_string()
}

fn call(id: &str, name: &str, input: Value, at: i64) -> String {
    line(
        json!({"isSidechain": false, "type": "assistant", "message": {"model": "claude-haiku-5-5",
            "role": "assistant", "content": [{"type": "tool_use", "id": id, "name": name, "input": input,
            "caller": {"type": "direct"}}], "stop_reason": "tool_use"}}),
        at,
    )
}

fn result(id: &str, result: Value, at: i64) -> String {
    line(
        json!({"type": "user", "message": {"role": "user", "content": [{"tool_use_id": id,
            "type": "tool_result", "content": "<str 192>"}]}, "toolUseResult": result}),
        at,
    )
}

fn failed(id: &str, at: i64) -> String {
    line(
        json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result",
            "content": "<str 40>", "is_error": true, "tool_use_id": id}]},
            "toolUseResult": "Error: <str 33>"}),
        at,
    )
}

fn fire(task: &str, cron: &str, kind: Option<&str>, at: i64) -> String {
    let mut fields = json!({"type": "system", "subtype": "scheduled_task_fire", "content": "<str 37>",
        "isMeta": false, "taskId": task, "cron": cron, "prompt": "<str 29>"});
    if let Some(kind) = kind {
        fields["taskKind"] = json!(kind);
        fields["cronKind"] = json!(kind);
    }
    line(fields, at)
}

fn user_text(text: &str, at: i64) -> String {
    line(
        json!({"type": "user", "message": {"role": "user", "content": text},
            "origin": {"kind": "task-notification"}}),
        at,
    )
}

fn attachment(attachment: Value, at: i64) -> String {
    line(json!({"type": "attachment", "attachment": attachment}), at)
}

fn notification(ids: &[&str], status: Option<&str>) -> String {
    let mut text = String::from("<task-notification>");
    for id in ids {
        text += &format!("\n<task-id>{id}</task-id>");
    }
    text += "\n<tool-use-id>toolu_01LuABWMxFXYBKsiunRt9TjE</tool-use-id>\n<output-file><str 117></output-file>";
    if let Some(status) = status {
        text += &format!("\n<status>{status}</status>");
    }
    text + "\n<summary><str 92></summary>\n</task-notification>"
}

fn create(tool: &str, cron: &str, recurring: bool, at: i64) -> String {
    call(
        tool,
        "CronCreate",
        json!({"cron": cron, "prompt": "<str 29>", "recurring": recurring}),
        at,
    )
}

fn created(tool: &str, id: &str, recurring: bool, next: f64, at: i64) -> String {
    result(
        tool,
        json!({"id": id, "humanSchedule": "Every minute", "recurring": recurring, "durable": false,
            "nextFireAt": next}),
        at,
    )
}

fn wakeup(tool: &str, delay: u32, at: i64) -> String {
    call(
        tool,
        "ScheduleWakeup",
        json!({"delaySeconds": delay, "noop": false, "reason": "<str 74>", "prompt": "<str 204>"}),
        at,
    )
}

fn woken(tool: &str, scheduled_for: i64, at: i64) -> String {
    result(
        tool,
        json!({"scheduledFor": scheduled_for, "clampedDelaySeconds": 60, "wasClamped": false}),
        at,
    )
}

fn shell(tool: &str, task: &str, at: i64) -> [String; 2] {
    [
        call(
            tool,
            "Bash",
            json!({"command": "<str 36>", "run_in_background": true}),
            at,
        ),
        result(
            tool,
            json!({"stdout": "", "backgroundTaskId": task}),
            at + 1,
        ),
    ]
}

fn carried(tasks: Value, other: Option<Value>, wakeup: Option<Value>, ended: &[&str]) -> Value {
    let mut carry = json!({"type": "session_cron_carry", "tasks": tasks});
    if let Some(other) = other {
        carry["otherArmed"] = other;
    }
    if let Some(wakeup) = wakeup {
        carry["wakeup"] = wakeup;
    }
    if !ended.is_empty() {
        carry["ended"] = json!(ended);
    }
    carry
}

fn job(id: &str, cron: &str, created: i64, recurring: bool) -> Value {
    json!({"id": id, "cron": cron, "prompt": "<str 29>", "createdAt": created, "recurring": recurring})
}

fn replay(lines: &[String]) -> Replay {
    let mut replay = Replay::default();
    for line in lines {
        replay.feed(format!("{line}\n").as_bytes(), &ZONE);
    }
    replay
}

/// Everything counts: no line is older than the process.
fn timers(replay: &Replay) -> Vec<i64> {
    replay.timers(i64::MIN, &ZONE)
}

fn tasks(replay: &Replay) -> u32 {
    replay.open_tasks(i64::MIN)
}

fn ends(replay: &Replay) -> Vec<i64> {
    replay.task_timers(i64::MIN)
}

fn next_fire(id: &str, cron: &str, recurring: bool, from_ms: i64) -> i64 {
    let cron = Cron::parse(cron).unwrap();
    ClaudeScheduler::DEFAULT
        .next_fire_ms(
            &ClaudeJob {
                id,
                cron: &cron,
                recurring,
                from_ms,
            },
            &ZONE,
        )
        .unwrap()
}

/// The latest fire any value of Claude Code's remote flag allows, the second due time of a computed recurring time.
fn latest(id: &str, cron: &str, from_ms: i64) -> i64 {
    let cron = Cron::parse(cron).unwrap();
    ClaudeScheduler::latest_recurring_ms(
        &ClaudeJob {
            id,
            cron: &cron,
            recurring: true,
            from_ms,
        },
        &ZONE,
    )
    .unwrap()
}

/// Live fixture 03, lines 5 and 6: the float `nextFireAt` is due at its ceiling.
#[test]
fn cron_create_arms_its_exact_next_fire() {
    let r = replay(&[
        create("toolu_1", "*/1 * * * *", true, T),
        created(
            "toolu_1",
            "bd9c33c4",
            true,
            1_793_715_622_219.941_2,
            T + 664,
        ),
    ]);
    assert_eq!(timers(&r), [1_793_715_622_220]);
    assert_eq!(r.problem(), None);
    assert_eq!(tasks(&r), 0);
}

#[test]
fn a_failed_or_durable_cron_create_arms_nothing() {
    let r = replay(&[
        create("toolu_1", "*/1 * * * *", true, T),
        failed("toolu_1", T + 1),
        create("toolu_2", "*/1 * * * *", true, T + 2),
        result(
            "toolu_2",
            json!({"id": "aaaa0002", "humanSchedule": "Every minute", "recurring": true, "durable": true,
                "nextFireAt": (T + MIN) as f64}),
            T + 3,
        ),
        // A result for a call never seen is not a job: it has no cron.
        created("toolu_3", "aaaa0003", true, (T + MIN) as f64, T + 4),
    ]);
    assert_eq!(timers(&r), Vec::<i64>::new());
    assert_eq!(r.problem(), None);
}

#[test]
fn a_recurring_fire_moves_the_job_on_and_a_one_shot_fire_ends_it() {
    let fired_at = T + 58_744;
    let r = replay(&[
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "bd9c33c4", true, (T + 58_690) as f64, T + 664),
        fire("bd9c33c4", "*/1 * * * *", None, fired_at),
    ]);
    let next = next_fire("bd9c33c4", "*/1 * * * *", true, fired_at);
    // Live fixture 03: the next fire came 107 ms after this.
    assert_eq!(next, 1_793_715_682_220);
    assert_eq!(
        timers(&r),
        [next, latest("bd9c33c4", "*/1 * * * *", fired_at)]
    );

    let mut lines = vec![
        create("toolu_2", "23 18 3 11 *", false, T),
        created("toolu_2", "12d31851", false, 1_793_715_780_000.0, T + 1),
    ];
    assert_eq!(timers(&replay(&lines)), [1_793_715_780_000]);
    lines.push(fire("12d31851", "23 18 3 11 *", None, 1_793_715_780_849));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
}

#[test]
fn a_recurring_job_7_days_old_fires_once_more_then_is_gone() {
    let mut lines = vec![
        create("toolu_1", "0 * * * *", true, T),
        created("toolu_1", "00000000", true, (T + HOUR) as f64, T),
    ];
    let before = T + 7 * DAY - 10 * MIN;
    lines.push(fire("00000000", "0 * * * *", None, before));
    assert_eq!(
        timers(&replay(&lines))[0],
        next_fire("00000000", "0 * * * *", true, before)
    );
    lines.push(fire("00000000", "0 * * * *", None, T + 7 * DAY + 50 * MIN));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
}

#[test]
fn cron_delete_ends_the_job_unless_it_failed() {
    let mut lines = vec![
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1),
        call("toolu_2", "CronDelete", json!({"id": "bd9c33c4"}), T + 2),
        failed("toolu_2", T + 3),
        call("toolu_3", "CronDelete", json!({"id": "ffffffff"}), T + 4),
        result("toolu_3", json!({"id": "ffffffff"}), T + 5),
    ];
    assert_eq!(timers(&replay(&lines)), [T + MIN]);
    lines.push(call(
        "toolu_4",
        "CronDelete",
        json!({"id": "bd9c33c4"}),
        T + 6,
    ));
    lines.push(result("toolu_4", json!({"id": "bd9c33c4"}), T + 7));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
}

/// Live fixture 04: armed at line 6, fired at line 10, stopped at line 16.
#[test]
fn a_wakeup_holds_its_time_until_a_loop_fire_or_stop() {
    let due = 1_793_715_660_000;
    let mut lines = vec![wakeup("toolu_1", 60, T), woken("toolu_1", due, T + 1)];
    assert_eq!(timers(&replay(&lines)), [due]);
    lines.push(fire("b19b19ca", "21 18 * * *", Some("loop"), due + 951));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());

    // A fire takes every wakeup due within 90 s of it, early ones included, and no later one.
    let lines = [
        wakeup("toolu_1", 60, T),
        woken("toolu_1", due, T + 1),
        wakeup("toolu_2", 600, T + 2),
        woken("toolu_2", due + 10 * MIN, T + 3),
        fire("c0ffee00", "0 18 * * *", Some("loop"), due - 89_000),
    ];
    assert_eq!(timers(&replay(&lines)), [due + 10 * MIN]);

    let stop = [
        wakeup("toolu_1", 60, T),
        woken("toolu_1", due, T + 1),
        call("toolu_3", "ScheduleWakeup", json!({"stop": true}), T + 2),
        result(
            "toolu_3",
            json!({"scheduledFor": 0, "clampedDelaySeconds": 0, "wasClamped": false, "stopped": true,
                "cancelledWakeups": 1}),
            T + 3,
        ),
    ];
    assert_eq!(timers(&replay(&stop)), Vec::<i64>::new());

    // `scheduledFor: 0` without `stopped`: nothing was armed.
    let none = replay(&[wakeup("toolu_1", 60, T), woken("toolu_1", 0, T + 1)]);
    assert_eq!(timers(&none), Vec::<i64>::new());
    assert_eq!(none.problem(), None);
}

#[test]
fn a_wakeup_is_forgotten_an_hour_after_its_time() {
    let due = T + 2 * MIN;
    let mut lines = vec![wakeup("toolu_1", 120, T), woken("toolu_1", due, T + 1)];
    lines.push(fire("aaaa0001", "*/1 * * * *", None, due + HOUR - 1));
    assert_eq!(timers(&replay(&lines)), [due]);
    lines.push(fire("aaaa0001", "*/1 * * * *", None, due + HOUR));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
}

/// Live fixture 06: after `/clear`, a known job keeps its last fire.
#[test]
fn a_carry_line_is_the_armed_set() {
    let fired_at = T + 65_000;
    let lines = [
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "f7152559", true, (T + 64_000) as f64, T + 1),
        create("toolu_2", "*/5 * * * *", true, T + 2),
        created("toolu_2", "aaaa0002", true, (T + 285_000) as f64, T + 3),
        create("toolu_3", "0 9 * * *", true, T + 4),
        created("toolu_3", "aaaa0003", true, (T + 20 * HOUR) as f64, T + 5),
        fire("f7152559", "*/1 * * * *", None, fired_at),
        wakeup("toolu_4", 600, T + 6),
        woken("toolu_4", T + 10 * MIN, T + 7),
        attachment(
            carried(
                json!([job("aaaa0003", "0 9 * * *", T + 5, true)]),
                Some(json!([
                    job("f7152559", "*/1 * * * *", T + 1, true),
                    job("bbbb0001", "30 18 3 11 *", T - MIN, false),
                ])),
                None,
                &["aaaa0003"],
            ),
            T + 2 * MIN,
        ),
    ];
    let r = replay(&lines);
    // `aaaa0002` is not listed and `aaaa0003` ended; the wakeup stays, since the line names none.
    let mut want = vec![
        next_fire("f7152559", "*/1 * * * *", true, fired_at),
        latest("f7152559", "*/1 * * * *", fired_at),
        next_fire("bbbb0001", "30 18 3 11 *", false, T - MIN),
        T + 10 * MIN,
    ];
    want.sort_unstable();
    assert_eq!(timers(&r), want);
    assert_eq!(r.problem(), None);

    // A carried wakeup replaces the ones before it.
    let mut with_wakeup = lines.to_vec();
    with_wakeup.push(attachment(
        carried(
            json!([]),
            None,
            Some(json!({"prompt": "<str 29>", "scheduledFor": T + 20 * MIN, "reason": "<str 9>"})),
            &[],
        ),
        T + 3 * MIN,
    ));
    assert_eq!(timers(&replay(&with_wakeup)), [T + 20 * MIN]);
}

/// A job known only from a carry line (Wakeshot started after the `/clear`) is due at its first fire after it.
#[test]
fn a_job_known_only_from_a_carry_is_due_at_its_first_fire_after_the_line() {
    // Live fixture 06, `.2` alone: the job fired 282 ms after this.
    let carry = attachment(
        carried(
            json!([]),
            Some(json!([job(
                "f7152559",
                "*/1 * * * *",
                1_793_715_621_070,
                true
            )])),
            None,
            &[],
        ),
        1_793_715_733_332,
    );
    assert_eq!(timers(&replay(&[carry]))[0], 1_793_715_748_955);

    // An hourly job made days ago whose run has passed but whose late fire has not: that fire.
    let id = "ffffffff";
    let run = next_fire(id, "0 * * * *", true, T);
    let pending = run - MIN;
    assert!(pending > T + MIN);
    let carry = attachment(
        carried(
            json!([job(id, "0 * * * *", T - 3 * DAY, true)]),
            None,
            None,
            &[],
        ),
        pending,
    );
    assert_eq!(timers(&replay(&[carry]))[0], run);
}

#[test]
fn background_tasks_open_and_any_status_closes() {
    let opened = [
        call(
            "toolu_1",
            "PowerShell",
            json!({"command": "<str 33>", "run_in_background": true}),
            T,
        ),
        result(
            "toolu_1",
            json!({"stdout": "", "stderr": "", "interrupted": false, "backgroundTaskId": "b1irxlyqr"}),
            T + 1,
        ),
    ];
    assert_eq!(tasks(&replay(&opened)), 1);
    for status in [
        "completed",
        "failed",
        "killed",
        "stopped",
        "a status not seen yet",
    ] {
        let mut lines = opened.to_vec();
        lines.push(user_text(
            &notification(&["b1irxlyqr"], Some(status)),
            T + 2,
        ));
        assert_eq!(tasks(&replay(&lines)), 0, "{status}");
    }
    // Another task's notification closes nothing here, and a failed call opens nothing.
    let mut lines = opened.to_vec();
    lines.push(user_text(
        &notification(&["b0ther000"], Some("completed")),
        T + 2,
    ));
    assert_eq!(tasks(&replay(&lines)), 1);
    assert_eq!(tasks(&replay(&[failed("toolu_9", T)])), 0);
}

fn monitor(tool: &str, task: &str, timeout_ms: u32, persistent: bool, at: i64) -> [String; 2] {
    [
        call(
            tool,
            "Monitor",
            json!({"command": "<str 95>", "timeout_ms": timeout_ms, "persistent": persistent}),
            at,
        ),
        result(
            tool,
            json!({"taskId": task, "timeoutMs": if persistent { 0 } else { timeout_ms },
                "persistent": persistent}),
            at + 1,
        ),
    ]
}

/// Live fixture 02: an event notification has no `<status>`. Claude Code ends a Monitor that is not persistent at
/// its timeout with one more event and no `<status>`, so it is a timer at its timeout, not an open task; a
/// `<status>` before that ends it.
#[test]
fn a_monitor_holds_until_its_timeout_and_its_events_close_nothing() {
    let event = |at| {
        user_text(
            "<task-notification>\n<task-id>bu1gw9xxc</task-id>\n<summary><str 70></summary>\n<event><str 5></event><str 123></task-notification>",
            at,
        )
    };
    let mut lines = monitor("toolu_1", "bu1gw9xxc", 120_000, false, T).to_vec();
    lines.push(event(T + 2));
    let r = replay(&lines);
    assert_eq!((tasks(&r), ends(&r)), (0, vec![T + 1 + 120_000]));
    assert_eq!(r.problem(), None);
    // The timeout's own event changes nothing; the time passing ends it.
    lines.push(event(T + 1 + 120_000));
    assert_eq!(ends(&replay(&lines)), [T + 1 + 120_000]);
    let mut done = lines.clone();
    done.push(user_text(
        &notification(&["bu1gw9xxc"], Some("completed")),
        T + 3,
    ));
    assert_eq!(ends(&replay(&done)), Vec::<i64>::new());
    // Forgotten an hour after its time, like a wakeup.
    lines.push(fire(
        "aaaa0001",
        "*/1 * * * *",
        None,
        T + 1 + 120_000 + HOUR,
    ));
    assert_eq!(ends(&replay(&lines)), Vec::<i64>::new());

    // A persistent Monitor has no end of its own: an open task until its notification or a stop.
    let mut lines = monitor("toolu_2", "bpersist1", 0, true, T).to_vec();
    lines.push(event(T + 2));
    let r = replay(&lines);
    assert_eq!((tasks(&r), ends(&r)), (1, vec![]));
    lines.push(user_text(
        &notification(&["bpersist1"], Some("killed")),
        T + 3,
    ));
    assert_eq!(tasks(&replay(&lines)), 0);

    // A `taskId` from another tool opens nothing.
    let other = [
        call("toolu_2", "TaskCreate", json!({"subject": "<str 9>"}), T),
        result("toolu_2", json!({"taskId": "7"}), T + 1),
    ];
    assert_eq!(tasks(&replay(&other)), 0);
    assert_eq!(ends(&replay(&other)), Vec::<i64>::new());
}

/// A Monitor result of a shape 2.1.317 does not write never holds for the life of the process.
#[test]
fn a_monitor_of_another_shape_still_ends() {
    let r = replay(&[
        call("toolu_1", "Monitor", json!({}), T),
        result("toolu_1", json!({"taskId": "bmon00001"}), T + 1),
    ]);
    assert_eq!((tasks(&r), ends(&r)), (0, vec![T + 1 + HOUR]));
    assert_eq!(r.problem(), Some((TRANSCRIPT, "Monitor result")));
    let r = replay(&[
        call("toolu_1", "Monitor", json!({}), T),
        result(
            "toolu_1",
            json!({"taskId": "bmon00001", "timeoutMs": 0, "persistent": false}),
            T + 1,
        ),
    ]);
    assert_eq!((tasks(&r), ends(&r)), (0, vec![T + 1 + HOUR]));
    // Longer than 2.1.317 allows: a due time every hour from its start, so the core's hour before each covers it.
    let r = replay(&[
        call("toolu_1", "Monitor", json!({}), T),
        result(
            "toolu_1",
            json!({"taskId": "bmon00001", "timeoutMs": 150 * MIN, "persistent": false}),
            T + 1,
        ),
    ]);
    assert_eq!(
        ends(&r),
        [T + 1 + HOUR, T + 1 + 2 * HOUR, T + 1 + 150 * MIN]
    );
    assert_eq!(r.problem(), Some((TRANSCRIPT, "Monitor result")));
    // At most a day of them.
    let r = replay(&[
        call("toolu_1", "Monitor", json!({}), T),
        result(
            "toolu_1",
            json!({"taskId": "bmon00001", "timeoutMs": 100 * DAY, "persistent": false}),
            T + 1,
        ),
    ]);
    assert_eq!(ends(&r).len(), MONITOR_STEPS as usize);
    // A Monitor of an earlier process is gone with it.
    let r = replay(&monitor("toolu_1", "bu1gw9xxc", 120_000, false, T));
    assert_eq!(r.task_timers(T + 2), Vec::<i64>::new());
}

fn stop(tool: &str, name: &str, input: Value, stopped: &str, at: i64) -> [String; 2] {
    [
        call(tool, name, input, at),
        result(
            tool,
            json!({"message": "<str 57>", "task_id": stopped, "task_type": "local_bash",
                "command": "<str 36>"}),
            at + 1,
        ),
    ]
}

/// Claude Code writes no notification for a task `TaskStop` stops; its result is the end.
#[test]
fn task_stop_closes_the_task_it_names() {
    let mut opened = shell("toolu_1", "b1irxlyqr", T).to_vec();
    opened.extend(shell("toolu_2", "b0000000b", T));
    opened.extend(monitor("toolu_3", "bu1gw9xxc", 120_000, false, T));
    opened.extend(monitor("toolu_4", "bpersist1", 0, true, T));
    let r = replay(&opened);
    assert_eq!((tasks(&r), ends(&r).len()), (3, 1));
    for name in ["TaskStop", "KillShell", "KillBash"] {
        let mut lines = opened.clone();
        lines.extend(stop(
            "toolu_9",
            name,
            json!({"task_id": "b1irxlyqr"}),
            "b1irxlyqr",
            T + 5,
        ));
        assert_eq!(tasks(&replay(&lines)), 2, "{name}");
    }
    // The result names the task; the input's `task_id` or older `shell_id` stands in when it does not.
    let mut lines = opened.clone();
    lines.extend(stop(
        "toolu_9",
        "TaskStop",
        json!({"task_id": "bu1gw9xxc"}),
        "bu1gw9xxc",
        T + 5,
    ));
    lines.extend(stop(
        "toolu_10",
        "TaskStop",
        json!({"task_id": "bpersist1"}),
        "bpersist1",
        T + 6,
    ));
    lines.push(call(
        "toolu_11",
        "KillShell",
        json!({"shell_id": "b0000000b"}),
        T + 7,
    ));
    lines.push(result("toolu_11", json!("Shell stopped"), T + 8));
    let r = replay(&lines);
    assert_eq!((tasks(&r), ends(&r).len()), (1, 0));
    assert_eq!(r.problem(), None);
    // A failed stop closes nothing; a stop of a task not known changes nothing.
    let mut lines = opened.clone();
    lines.push(call(
        "toolu_9",
        "TaskStop",
        json!({"task_id": "b1irxlyqr"}),
        T + 5,
    ));
    lines.push(failed("toolu_9", T + 6));
    lines.extend(stop(
        "toolu_10",
        "TaskStop",
        json!({"task_id": "a0ther000"}),
        "a0ther000",
        T + 7,
    ));
    assert_eq!(tasks(&replay(&lines)), 3);
    // Neither the result nor the input names a task: named, nothing closed.
    let mut lines = opened;
    lines.push(call("toolu_9", "TaskStop", json!({}), T + 5));
    lines.push(result("toolu_9", json!({"message": "<str 9>"}), T + 6));
    let r = replay(&lines);
    assert_eq!(tasks(&r), 3);
    assert_eq!(r.problem(), Some((TRANSCRIPT, "TaskStop result")));
}

/// A background shell whose result has no `backgroundTaskId` would stop holding with no sign, so it is named. A
/// foreground shell or a background subagent is not.
#[test]
fn a_background_shell_result_without_its_task_id_is_named() {
    let named = replay(&[
        call(
            "toolu_1",
            "PowerShell",
            json!({"command": "<str 33>", "run_in_background": true}),
            T,
        ),
        result(
            "toolu_1",
            json!({"stdout": "", "bgTaskId": "b1irxlyqr"}),
            T + 1,
        ),
    ]);
    assert_eq!(
        named.problem(),
        Some((TRANSCRIPT, "background shell result"))
    );
    assert_eq!(tasks(&named), 0);
    let quiet = replay(&[
        shell("toolu_1", "b1irxlyqr", T)[0].clone(),
        failed("toolu_1", T + 1),
        call("toolu_2", "Bash", json!({"command": "<str 9>"}), T + 2),
        result("toolu_2", json!({"stdout": "<str 3>"}), T + 3),
        call(
            "toolu_3",
            "Agent",
            json!({"prompt": "<str 9>", "run_in_background": true}),
            T + 4,
        ),
        result(
            "toolu_3",
            json!({"isAsync": true, "status": "async_launched", "agentId": "a1b2c3"}),
            T + 5,
        ),
    ]);
    assert_eq!(quiet.problem(), None);
    assert_eq!(tasks(&replay(&shell("toolu_1", "b1irxlyqr", T))), 1);
}

/// Claude Code's own reader takes a notification after its header, and several joined after it.
#[test]
fn a_notification_after_the_header_and_joined_ones_close_their_tasks() {
    const DWE: &str = "[SYSTEM NOTIFICATION - NOT USER INPUT]\nThis is an automated background-task event, NOT a message from the user.\nDo NOT interpret this as user acknowledgement.\nNo human input has been received since the last genuine user message.\n\n";
    let mut opened = Vec::new();
    for (n, task) in ["b0000000a", "b0000000b", "b0000000c"].iter().enumerate() {
        opened.extend(shell(&format!("toolu_{n}"), task, T));
    }
    let one = notification(&["b0000000a"], Some("completed"));
    let two = notification(&["b0000000b"], Some("killed"));
    let event = "<task-notification>\n<task-id>b0000000c</task-id>\n<summary><str 9></summary>\n<event><str 5></event>\n</task-notification>";
    for (text, open) in [
        (format!("{DWE}{one}"), 2),
        (format!("{DWE}{one}\n\n{two}"), 1),
        (format!("{DWE}{event}\n\n{one}\n\n{two}"), 1),
        (
            format!("<system-reminder>\n{DWE}{one}\n</system-reminder>"),
            2,
        ),
        // Without the header only the first one counts, as Claude Code reads it.
        (format!("{one}\n\n{two}"), 2),
        // Text that only quotes the header or a notification closes nothing.
        (format!("<str 9> {DWE}{one}"), 3),
        (format!("{DWE}<str 9>\n\n{one}"), 3),
    ] {
        let mut lines = opened.clone();
        lines.push(user_text(&text, T + 5));
        assert_eq!(tasks(&replay(&lines)), open, "{text}");
        let mut lines = opened.clone();
        lines.push(attachment(
            json!({"type": "queued_command", "prompt": text}),
            T + 5,
        ));
        assert_eq!(tasks(&replay(&lines)), open, "queued: {text}");
    }
}

/// A computed recurring time also comes as the latest any flag value allows, and the real fire, early or late,
/// falls inside the core's window of one of them.
#[test]
fn a_computed_recurring_time_covers_every_flag_value() {
    let lead = crate::policy::sessions::TIMER_LEAD_MS;
    let margin = crate::policy::sessions::TIMER_MARGIN_MS;
    for (id, cron) in [
        ("bd9c33c4", "*/1 * * * *"),
        ("ffffffff", "*/5 * * * *"),
        ("80000000", "0 * * * *"),
        ("ffffffff", "0 9 * * *"),
        ("00000001", "*/15 * * * *"),
    ] {
        let lines = [
            create("toolu_1", cron, true, T),
            created("toolu_1", id, true, (T + MIN) as f64, T + 1),
            fire(id, cron, None, T + MIN),
        ];
        let due = timers(&replay(&lines));
        let parsed = Cron::parse(cron).unwrap();
        let job = ClaudeJob {
            id,
            cron: &parsed,
            recurring: true,
            from_ms: T + MIN,
        };
        for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
            for cap in [0, MIN, 30 * MIN] {
                for cache_lead in [0, 1, 15_000, MIN] {
                    let flag = ClaudeScheduler {
                        recurring_frac: frac,
                        recurring_cap_ms: cap,
                        cache_lead_ms: cache_lead,
                        ..ClaudeScheduler::DEFAULT
                    };
                    let real = flag.next_fire_ms(&job, &ZONE).unwrap();
                    assert!(
                        due.iter().any(|&d| d - lead <= real && real < d + margin),
                        "{cron} {frac} {cap} {cache_lead}: {real} {due:?}"
                    );
                }
            }
        }
    }
}

/// A `nextFireAt` the built-in values do not give is kept as a drift, for a live check.
#[test]
fn a_next_fire_the_built_in_scheduler_does_not_give_is_a_drift() {
    let right = next_fire("bd9c33c4", "*/1 * * * *", true, T + 664);
    let r = replay(&[
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "bd9c33c4", true, right as f64, T + 664),
    ]);
    assert_eq!(r.scheduler_drift_ms(), None);
    let r = replay(&[
        create("toolu_1", "*/1 * * * *", true, T),
        created(
            "toolu_1",
            "bd9c33c4",
            true,
            (right + 20_000) as f64,
            T + 664,
        ),
    ]);
    assert_eq!(r.scheduler_drift_ms(), Some(20_000));
    assert_eq!(timers(&r), [right + 20_000]);
    assert_eq!(r.problem(), None);
}

/// A job dropped at the built-in 7 day age comes back when Claude Code fires it again, and then only its fire lines
/// end it.
#[test]
fn a_job_fired_after_its_age_comes_back() {
    let mut lines = vec![
        create("toolu_1", "0 * * * *", true, T),
        created("toolu_1", "00000000", true, (T + HOUR) as f64, T),
        fire("00000000", "0 * * * *", None, T + 7 * DAY + 50 * MIN),
    ];
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
    let again = T + 7 * DAY + 110 * MIN;
    lines.push(fire("00000000", "0 * * * *", None, again));
    let r = replay(&lines);
    assert_eq!(
        timers(&r)[0],
        next_fire("00000000", "0 * * * *", true, again)
    );
    // It no longer ends at its age, nor at a resume.
    lines.push(fire("00000000", "0 * * * *", None, again + HOUR));
    assert_eq!(
        timers(&replay(&lines)),
        [next_fire("00000000", "0 * * * *", true, again + HOUR)]
    );
    let resumed = again + 2 * HOUR;
    assert_eq!(
        replay(&lines).timers(resumed, &ZONE),
        [next_fire("00000000", "0 * * * *", true, resumed)]
    );
    // A delete still ends it.
    lines.push(call(
        "toolu_2",
        "CronDelete",
        json!({"id": "00000000"}),
        again + 2 * HOUR,
    ));
    lines.push(result(
        "toolu_2",
        json!({"id": "00000000"}),
        again + 2 * HOUR + 1,
    ));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());
}

/// Claude Code 2.1.317 marks the shells a dead process left in one notification with one `<task-id>` each.
#[test]
fn a_batch_notification_closes_every_listed_task() {
    let mut lines = Vec::new();
    for (n, task) in ["b0000000a", "b0000000b", "b0000000c"].iter().enumerate() {
        lines.extend(shell(&format!("toolu_{n}"), task, T));
    }
    lines.push(user_text(
        "<task-notification>\n<task-id>b0000000a</task-id>\n<task-id>b0000000b</task-id>\n<status>stopped</status>\n<summary><str 40></summary>\n<note><str 300></note>\n</task-notification>",
        T + 5,
    ));
    assert_eq!(tasks(&replay(&lines)), 1);
}

/// Live fixture 07: a notification that came during a busy turn is only an attachment.
#[test]
fn a_queued_command_notification_closes_its_task() {
    let mut lines = shell("toolu_1", "ad8c97644", T).to_vec();
    lines.push(attachment(
        json!({"type": "queued_command", "prompt": notification(&["ad8c97644"], Some("completed")),
            "source_uuid": "5e550029-0000-4000-8000-000000000029", "commandMode": "<str 17>"}),
        T + 2,
    ));
    assert_eq!(tasks(&replay(&lines)), 0);
}

fn queued(operation: &str, content: Option<&str>, at: i64) -> String {
    let mut fields = json!({"type": "queue-operation", "operation": operation});
    if let Some(content) = content {
        fields["content"] = json!(content);
    }
    line(fields, at)
}

/// Claude Code writes a notification as a `queue-operation` `enqueue` line when it queues it. A stop from the UI
/// queues its `killed` notice as passive (2.1.317, `tUr`), so the user line may come only at the next prompt; the
/// queued line ends the task at once. Not checked live for a UI stop.
#[test]
fn a_queued_notice_ends_its_task_at_once() {
    let killed = notification(&["b1irxlyqr"], Some("killed"));
    let opened = shell("toolu_1", "b1irxlyqr", T);
    let mut lines = opened.to_vec();
    lines.push(queued("enqueue", Some(&killed), T + 2));
    let r = replay(&lines);
    assert_eq!(tasks(&r), 0);
    assert_eq!(r.problem(), None);
    // Its later delivery changes nothing.
    lines.push(queued("dequeue", None, T + 3));
    lines.push(user_text(&killed, T + 4));
    assert_eq!(tasks(&replay(&lines)), 0);

    // A Monitor's timer ends too, persistent or not, and so does a notice after Claude Code's header.
    let header = format!("[SYSTEM NOTIFICATION - NOT USER INPUT]\n<str 64>\n\n{killed}");
    for persistent in [false, true] {
        let mut lines = monitor("toolu_2", "b1irxlyqr", 120_000, persistent, T).to_vec();
        lines.push(queued("enqueue", Some(&header), T + 2));
        let r = replay(&lines);
        assert_eq!((tasks(&r), ends(&r)), (0, vec![]), "{persistent}");
    }

    // Other operations, a notice with no `<status>`, a notice not at the start, a typed prompt and no content end
    // nothing and name nothing.
    let event = "<task-notification>\n<task-id>b1irxlyqr</task-id>\n<summary><str 70></summary>\n<event><str 5></event><str 123></task-notification>";
    let quoted = format!("<str 29> {killed}");
    for (operation, content) in [
        ("remove", Some(killed.as_str())),
        ("dequeue", None),
        ("enqueue", Some(event)),
        ("enqueue", Some(quoted.as_str())),
        ("enqueue", Some("<str 29>")),
        ("enqueue", None),
    ] {
        let mut lines = opened.to_vec();
        lines.push(queued(operation, content, T + 2));
        let r = replay(&lines);
        assert_eq!(tasks(&r), 1, "{operation} {content:?}");
        assert_eq!(r.problem(), None);
    }
    let mut lines = opened.to_vec();
    lines.push(line(
        json!({"type": "queue-operation", "operation": 5, "content": killed}),
        T + 2,
    ));
    lines.push(line(
        json!({"type": "queue-operation", "operation": "enqueue", "content": [killed]}),
        T + 2,
    ));
    assert_eq!(tasks(&replay(&lines)), 1);
}

#[test]
fn only_a_notification_at_the_start_of_the_text_counts() {
    let closing = notification(&["b1irxlyqr"], Some("completed"));
    let mut lines = shell("toolu_1", "b1irxlyqr", T).to_vec();
    lines.push(user_text(&format!("<str 8206> {closing}"), T + 2));
    lines.push(user_text(&format!(" {closing}"), T + 3));
    // A `<status>` after the `<summary>` is the summary's text.
    lines.push(user_text(
        "<task-notification>\n<task-id>b1irxlyqr</task-id>\n<summary><str 8></summary>\n<status>completed</status>\n</task-notification>",
        T + 4,
    ));
    assert_eq!(tasks(&replay(&lines)), 1);
    // The same text as a text item of a list closes it.
    lines.push(line(
        json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": closing}]}}),
        T + 5,
    ));
    assert_eq!(tasks(&replay(&lines)), 0);
}

/// Live fixtures 09 to 11 and 13: lines older than the process are an earlier process's (resume).
#[test]
fn lines_of_an_earlier_process_follow_the_resume_rules() {
    let resumed = T + HOUR;
    let mut lines = vec![
        create("toolu_1", "*/5 * * * *", true, T),
        created("toolu_1", "f9471e8f", true, (T + 285_000) as f64, T + 1),
        create("toolu_2", "0 9 * * *", true, T - 8 * DAY),
        created(
            "toolu_2",
            "aaaa0002",
            true,
            (T - 7 * DAY) as f64,
            T - 8 * DAY,
        ),
        create("toolu_3", "50 18 3 11 *", false, T + 2),
        created("toolu_3", "aaaa0003", false, (T + 30 * MIN) as f64, T + 3),
        create("toolu_4", "40 19 3 11 *", false, T + 4),
        created("toolu_4", "aaaa0004", false, (T + 80 * MIN) as f64, T + 5),
        wakeup("toolu_5", 3_600, T + 6),
        woken("toolu_5", T + 2 * HOUR, T + 7),
    ];
    lines.extend(shell("toolu_6", "b01yw8588", T + 8));
    let r = replay(&lines);
    assert_eq!(r.open_tasks(T), 1);
    assert_eq!(r.open_tasks(resumed), 0);
    let mut old = vec![
        T + 285_000,
        T - 7 * DAY,
        T + 30 * MIN,
        T + 80 * MIN,
        T + 2 * HOUR,
    ];
    old.sort_unstable();
    assert_eq!(r.timers(T - 8 * DAY, &ZONE), old);
    // At the resume: the `*/5` job counts from it; the 8-day-old job, the missed one-shot and the wakeup are gone.
    assert_eq!(
        r.timers(resumed, &ZONE),
        [
            resumed + 285_000,
            latest("f9471e8f", "*/5 * * * *", resumed),
            T + 80 * MIN
        ]
    );
    // A fire of this process counts from the fire again.
    let fired = resumed + 285_500;
    lines.push(fire("f9471e8f", "*/5 * * * *", None, fired));
    assert_eq!(
        replay(&lines).timers(resumed, &ZONE),
        [
            fired + 285_000,
            latest("f9471e8f", "*/5 * * * *", fired),
            T + 80 * MIN
        ]
    );
}

/// An unknown cron syntax holds only for the exact time it came with.
#[test]
fn a_cron_that_does_not_parse_holds_only_for_its_exact_time() {
    let mut lines = vec![
        create("toolu_1", "@hourly", true, T),
        created("toolu_1", "aaaa0001", true, (T + HOUR) as f64, T + 1),
    ];
    let r = replay(&lines);
    assert_eq!(timers(&r), [T + HOUR]);
    assert_eq!(r.problem(), Some((CRON, "@hourly")));
    // Nor does it count at a resume, or after its fire.
    assert_eq!(r.timers(T + 2, &ZONE), Vec::<i64>::new());
    lines.push(fire("aaaa0001", "@hourly", None, T + HOUR + 500));
    assert_eq!(timers(&replay(&lines)), Vec::<i64>::new());

    let long = "0 0 1 1 * ".repeat(10) + "\u{0531}";
    let r = replay(&[
        create("toolu_2", &long, true, T),
        created("toolu_2", "aaaa0002", true, (T + HOUR) as f64, T + 1),
    ]);
    assert_eq!(
        r.problem(),
        Some((CRON, "0 0 1 1 * 0 0 1 1 * 0 0 1 1 * 0 "))
    );
}

/// A line of a known kind without a field it needs is skipped and named, never guessed.
#[test]
fn a_known_kind_with_a_missing_field_is_skipped_and_named() {
    let made = || create("toolu_1", "*/1 * * * *", true, T);
    let cases: Vec<(Vec<String>, &str)> = vec![
        (
            vec![
                made(),
                result("toolu_1", json!({"humanSchedule": "Every minute", "nextFireAt": 1.0}), T),
            ],
            "CronCreate result",
        ),
        (vec![made(), result("toolu_1", json!("Created"), T)], "CronCreate result"),
        (
            vec![call("toolu_1", "CronCreate", json!({"prompt": "<str 29>"}), T)],
            "CronCreate",
        ),
        (vec![call("toolu_1", "CronDelete", json!({}), T)], "CronDelete"),
        (vec![call("toolu_1", "CronDelete", json!({"id": "a b"}), T)], "CronDelete"),
        (
            vec![line(
                json!({"type": "assistant", "message": {"content": [{"type": "tool_use",
                    "name": "ScheduleWakeup", "input": {}}]}}),
                T,
            )],
            "ScheduleWakeup",
        ),
        (
            vec![
                wakeup("toolu_1", 60, T),
                result("toolu_1", json!({"clampedDelaySeconds": 60}), T),
            ],
            "ScheduleWakeup result",
        ),
        (
            vec![
                call("toolu_1", "Monitor", json!({}), T),
                result("toolu_1", json!({"timeoutMs": 1}), T),
            ],
            "Monitor result",
        ),
        (
            vec![line(
                json!({"type": "system", "subtype": "scheduled_task_fire", "cron": "*/1 * * * *"}),
                T,
            )],
            "scheduled_task_fire",
        ),
        (
            vec![
                json!({"type": "system", "subtype": "scheduled_task_fire", "taskId": "bd9c33c4"})
                    .to_string(),
            ],
            "scheduled_task_fire",
        ),
        (
            vec![attachment(
                json!({"type": "session_cron_carry", "otherArmed": []}),
                T,
            )],
            "session_cron_carry",
        ),
        (
            vec![attachment(
                carried(json!([{"id": "aaaa0001", "cron": "*/1 * * * *"}]), None, None, &[]),
                T,
            )],
            "session_cron_carry",
        ),
        (
            vec![attachment(carried(json!(["aaaa0001"]), None, None, &[]), T)],
            "session_cron_carry",
        ),
        (
            vec![attachment(
                carried(json!([]), None, Some(json!({"prompt": "<str 1>"})), &[]),
                T,
            )],
            "session_cron_carry",
        ),
        (
            vec![
                made(),
                json!({"type": "user", "message": {"content": [{"type": "tool_result",
                    "tool_use_id": "toolu_1"}]}, "toolUseResult": {"id": "aaaa0001", "nextFireAt": 1.0}})
                .to_string(),
            ],
            "timestamp",
        ),
        (
            vec![
                shell("toolu_1", "b1", T)[0].clone(),
                result("toolu_1", json!({"backgroundTaskId": "b 1"}), T),
            ],
            "task id",
        ),
    ];
    for (lines, kind) in cases {
        let r = replay(&lines);
        assert_eq!(r.problem(), Some((TRANSCRIPT, kind)), "{lines:?}");
        assert_eq!(timers(&r), Vec::<i64>::new(), "{lines:?}");
        assert_eq!(tasks(&r), 0, "{lines:?}");
    }
    // A missing `nextFireAt` is named too, but the job still counts from its creation.
    let r = replay(&[
        made(),
        result(
            "toolu_1",
            json!({"id": "bd9c33c4", "recurring": true}),
            T + 1,
        ),
    ]);
    assert_eq!(r.problem(), Some((TRANSCRIPT, "CronCreate result")));
    assert_eq!(
        timers(&r),
        [
            next_fire("bd9c33c4", "*/1 * * * *", true, T + 1),
            latest("bd9c33c4", "*/1 * * * *", T + 1)
        ]
    );
}

/// Transcripts have many line kinds, and the names read here mean other things in some of them.
#[test]
fn other_kinds_and_shapes_are_ignored_silently() {
    let lines = [
        line(
            json!({"type": "tool_use_summary", "subtype": "scheduled_task_fire", "taskId": 5}),
            T,
        ),
        attachment(
            json!({"type": "task_status", "taskType": "local_bash", "status": "running",
                "tasks": {"b1": 1}, "prompt": 7}),
            T,
        ),
        call(
            "toolu_1",
            "TaskUpdate",
            json!({"id": 5, "cron": ["x"], "recurring": "yes"}),
            T,
        ),
        result("toolu_1", json!(["tool_use", 1, null, {"id": {}}]), T),
        json!({"type": "user", "message": {"role": "user", "content": {"type": "tool_use"}},
            "toolUseResult": null, "timestamp": 5})
        .to_string(),
        line(json!({"type": "assistant", "message": "tool_use"}), T),
        line(
            json!({"type": "queue-operation", "operation": "remove",
                "content": notification(&["b1irxlyqr"], Some("completed"))}),
            T,
        ),
        json!({"type": "progress", "data": {"tool_use": [1, 2, {"nested": [[[]]]}]}}).to_string(),
        "{\"type\":\"user\"}".to_owned(),
        String::new(),
        "not json, no marker".to_owned(),
    ];
    let r = replay(&lines);
    assert_eq!(r.problem(), None);
    assert_eq!(timers(&r), Vec::<i64>::new());
    assert_eq!(tasks(&r), 0);
}

#[test]
fn a_broken_line_that_may_matter_is_named() {
    let whole = fire("bd9c33c4", "*/1 * * * *", None, T);
    let cut = whole.find("scheduled_task_fire").unwrap() + 24;
    let r = replay(&[whole[..cut].to_owned(), whole.clone()]);
    assert_eq!(r.problem(), Some((TRANSCRIPT, "line")));
    // A broken line that names nothing that matters says nothing.
    let r = replay(&["{\"type\":\"last-prompt\",\"lastPrompt\":\"<str".to_owned()]);
    assert_eq!(r.problem(), None);
    // Not an object.
    let r = replay(&["\"scheduled_task_fire\"".to_owned()]);
    assert_eq!(r.problem(), Some((TRANSCRIPT, "line")));
}

#[test]
fn the_first_problem_is_kept() {
    let r = replay(&[
        call("toolu_1", "CronDelete", json!({}), T),
        call("toolu_2", "CronCreate", json!({}), T),
    ]);
    assert_eq!(r.problem(), Some((TRANSCRIPT, "CronDelete")));
}

#[test]
fn at_most_50_jobs() {
    let mut lines = Vec::new();
    for n in 0..60 {
        let tool = format!("toolu_{n}");
        lines.push(create(&tool, "0 9 * * *", true, T + n));
        lines.push(created(
            &tool,
            &format!("{n:08x}"),
            true,
            (T + DAY + n) as f64,
            T + n,
        ));
    }
    assert_eq!(timers(&replay(&lines)).len(), MAX_JOBS);
}

#[test]
fn a_partial_line_waits_for_its_newline() {
    let made = format!(
        "{}\n{}",
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1)
    );
    let mut r = Replay::default();
    r.feed(made.as_bytes(), &ZONE);
    assert_eq!(timers(&r), Vec::<i64>::new());
    r.feed(b"\n", &ZONE);
    assert_eq!(timers(&r), [T + MIN]);

    // After `/clear` the new file starts clean.
    let mut r = Replay::default();
    r.feed(
        b"{\"type\":\"system\",\"subtype\":\"scheduled_task_f",
        &ZONE,
    );
    r.start_file();
    r.feed(format!("{made}\n").as_bytes(), &ZONE);
    assert_eq!(timers(&r), [T + MIN]);
    assert_eq!(r.problem(), None);
}

#[test]
fn a_line_too_long_is_skipped() {
    let mut r = Replay::default();
    let mut huge = b"{\"type\":\"user\",\"tool_use\":\"".to_vec();
    huge.resize(MAX_LINE + 10, b'a');
    r.feed(&huge, &ZONE);
    r.feed(b"aaaa\"}\n", &ZONE);
    let made = format!(
        "{}\n{}\n",
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1)
    );
    r.feed(made.as_bytes(), &ZONE);
    assert_eq!(timers(&r), [T + MIN]);
    assert_eq!(r.problem(), None);
    assert!(r.partial.capacity() < 1 << 20);
}

/// No prompt, text, output or summary is kept. Each one here is the same marked text, and the kept state shows none
/// of it.
#[test]
fn the_replay_keeps_no_text() {
    let secret = "SECRET";
    let lines = [
        call(
            "toolu_1",
            "CronCreate",
            json!({"cron": "*/1 * * * *", "prompt": secret, "recurring": true}),
            T,
        ),
        created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1),
        call(
            "toolu_2",
            "Bash",
            json!({"command": secret, "run_in_background": true}),
            T,
        ),
        result(
            "toolu_2",
            json!({"stdout": secret, "stderr": secret, "backgroundTaskId": "b1irxlyqr"}),
            T,
        ),
        user_text(
            &format!(
                "<task-notification>\n<task-id>b0ther000</task-id>\n<output-file>{secret}</output-file>\n<status>completed</status>\n<summary>{secret}</summary>\n</task-notification>"
            ),
            T,
        ),
        user_text(secret, T),
        line(
            json!({"type": "assistant", "message": {"content": [{"type": "text", "text": secret},
                {"type": "thinking", "thinking": secret}]}}),
            T,
        ),
        attachment(
            carried(
                json!([{"id": "aaaa0001", "cron": "0 9 * * *", "prompt": secret, "createdAt": T,
                    "recurring": true}]),
                Some(json!([job("bd9c33c4", "*/1 * * * *", T, true)])),
                Some(json!({"prompt": secret, "scheduledFor": T + HOUR, "reason": secret})),
                &[],
            ),
            T + 2,
        ),
        attachment(json!({"type": "queued_command", "prompt": secret}), T + 3),
        fire("aaaa0001", "0 9 * * *", None, T + 4).replace("<str 29>", secret),
        queued(
            "enqueue",
            Some(&format!(
                "<task-notification>\n<task-id>b0ther001</task-id>\n<output-file>{secret}</output-file>\n<status>killed</status>\n<summary>{secret}</summary>\n</task-notification>"
            )),
            T + 4,
        ),
        queued(
            "enqueue",
            Some(&format!("<task-notification>{secret}")),
            T + 4,
        ),
    ];
    let mut r = replay(&lines);
    assert_eq!(r.problem(), None);
    assert_eq!(timers(&r).len(), 3);
    assert_eq!(tasks(&r), 1);
    // A partial line waiting for its newline is held, but not shown.
    let held = user_text(secret, T + 5);
    let end = held.find(secret).unwrap() + secret.len();
    r.feed(&held.as_bytes()[..end], &ZONE);
    assert!(contains(&r.partial, secret.as_bytes()));
    let kept = format!("{r:?}");
    assert!(!kept.contains(secret), "{kept}");
    for id in ["bd9c33c4", "aaaa0001", "b1irxlyqr"] {
        assert!(kept.contains(id), "{kept}");
    }
}

fn sample() -> Vec<String> {
    let mut lines = vec![
        create("toolu_1", "*/1 * * * *", true, T),
        created("toolu_1", "bd9c33c4", true, (T + MIN) as f64, T + 1),
        fire("bd9c33c4", "*/1 * * * *", None, T + MIN),
        wakeup("toolu_2", 60, T + MIN),
        woken("toolu_2", T + 3 * MIN, T + MIN + 1),
        call("toolu_3", "CronDelete", json!({}), T + 2 * MIN),
        attachment(
            carried(
                json!([job("bd9c33c4", "*/1 * * * *", T + 1, true)]),
                None,
                None,
                &[],
            ),
            T + 2 * MIN,
        ),
        "{\"type\":\"user\",\"tool_use\":broken".to_owned(),
    ];
    lines.extend(shell("toolu_4", "b1irxlyqr", T));
    lines.extend(shell("toolu_5", "b0000000b", T + 3));
    lines.extend(shell("toolu_6", "b0000000c", T + 4));
    lines.push(user_text(
        &notification(&["b1irxlyqr"], Some("completed")),
        T + 2 * MIN,
    ));
    lines.push(queued(
        "enqueue",
        Some(&notification(&["b0000000c"], Some("killed"))),
        T + 2 * MIN,
    ));
    lines
}

proptest! {
    /// The bytes may come in any pieces; the state is that of the whole file.
    #[test]
    fn any_split_gives_the_same_state(cuts in proptest::collection::vec(any::<prop::sample::Index>(), 0..12)) {
        let text: String = sample().iter().map(|line| format!("{line}\r\n")).collect();
        let bytes = text.as_bytes();
        let whole = replay(&sample());
        let mut at: Vec<usize> = cuts.iter().map(|cut| cut.index(bytes.len() + 1)).collect();
        at.sort_unstable();
        let mut pieces = Replay::default();
        let mut start = 0;
        for end in at.into_iter().chain([bytes.len()]) {
            pieces.feed(&bytes[start..end], &ZONE);
            start = end;
        }
        prop_assert_eq!(timers(&pieces), timers(&whole));
        prop_assert_eq!(tasks(&pieces), tasks(&whole));
        prop_assert_eq!(pieces.problem(), whole.problem());
        prop_assert_eq!(format!("{:?}", pieces.jobs), format!("{:?}", whole.jobs));
    }

    #[test]
    fn any_bytes_feed_without_a_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let mut r = Replay::default();
        r.feed(&bytes, &ZONE);
        r.feed(b"\n", &ZONE);
        let _ = (r.timers(i64::MIN, &ZONE), r.open_tasks(0), r.problem());
    }

    /// A line that matters, with any one value changed, reads without a panic and keeps only plain ids.
    #[test]
    fn changed_values_never_panic(value in prop_oneof![
        Just(json!(null)), any::<i64>().prop_map(|n| json!(n)), any::<f64>().prop_map(|n| json!(n)),
        "\\PC{0,80}".prop_map(|s| json!(s)), Just(json!([])), Just(json!({})), Just(json!(true)),
    ], which in 0usize..7) {
        let mut made = json!({"id": "bd9c33c4", "humanSchedule": "Every minute", "recurring": true,
            "durable": false, "nextFireAt": 1.0});
        let mut input = json!({"cron": "*/1 * * * *", "prompt": "<str 29>", "recurring": true});
        match which {
            0..=3 => made[["id", "recurring", "durable", "nextFireAt"][which]] = value.clone(),
            _ => input[["cron", "recurring", "prompt"][which - 4]] = value.clone(),
        }
        let entry = json!({"id": value.clone(), "cron": value.clone(), "createdAt": value.clone(),
            "recurring": value.clone()});
        let r = replay(&[
            call("toolu_1", "CronCreate", input, T),
            result("toolu_1", made, T + 1),
            attachment(carried(json!([value]), Some(json!([entry])), None, &[]), T + 2),
        ]);
        for id in r.jobs.keys() {
            prop_assert!(is_plain_id(id));
        }
        let _ = r.timers(i64::MIN, &ZONE);
    }
}
