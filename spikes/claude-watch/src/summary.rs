use chrono::{DateTime, Local, TimeZone};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// String values under these keys are short ids, enums or schedules. Every other string can hold user text
/// (prompts, commands, output) and is logged only as its length.
const KEEP_STRING_KEYS: &[&str] = &[
    "type",
    "subtype",
    "name",
    "id",
    "taskId",
    "task_id",
    "tool_use_id",
    "cron",
    "humanSchedule",
    "status",
    "taskType",
    "kind",
    "taskKind",
    "cronKind",
    "operation",
    "level",
    "stop_reason",
    "stopReason",
    "jobId",
    "agentId",
    "shellId",
    "backgroundTaskId",
    "sessionId",
    "createdBySessionId",
    "procStart",
    "createdByProcStart",
    "permissionMode",
    "subagent_type",
    "model",
];

/// Tools whose calls are about timers or background work even without `run_in_background`.
const WATCHED_TOOLS: &[&str] = &[
    "CronCreate",
    "CronDelete",
    "CronList",
    "ScheduleWakeup",
    "Monitor",
    "TaskStop",
    "TaskOutput",
    "KillShell",
    "BashOutput",
];

/// Keys every transcript line carries; they add nothing to a change log.
const COMMON_KEYS: &[&str] = &[
    "parentUuid",
    "logicalParentUuid",
    "uuid",
    "sessionId",
    "cwd",
    "version",
    "gitBranch",
    "userType",
    "isSidechain",
    "timestamp",
    "slug",
    "entrypoint",
    "requestId",
    "promptId",
    "message",
    "toolUseResult",
    "type",
    "subtype",
];

const KIND_BODY_MAX: usize = 160;
const MAX_DEPTH: usize = 5;
const MAX_ITEMS: usize = 12;

pub fn local_ms(ms: i64) -> String {
    match Local.timestamp_millis_opt(ms).single() {
        Some(t) => t.format("%H:%M:%S%.3f").to_string(),
        None => ms.to_string(),
    }
}

pub fn local_iso(iso: &str) -> String {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(t) => t.with_timezone(&Local).format("%H:%M:%S%.3f").to_string(),
        Err(_) => iso.to_string(),
    }
}

fn is_time_key(key: &str) -> bool {
    key.ends_with("At") || key.ends_with("For")
}

/// One value for the log: epoch-ms times get the local clock time next to them.
pub fn show(key: &str, v: &Value) -> String {
    match epoch_ms(key, v) {
        Some(ms) => format!("{v} ({})", local_ms(ms)),
        None => v.to_string(),
    }
}

/// `nextFireAt` comes as a float with a fraction of a millisecond (seen live), so read numbers as f64.
fn epoch_ms(key: &str, v: &Value) -> Option<i64> {
    let ms = v.as_f64()?;
    (is_time_key(key) && ms > 1e12).then_some(ms as i64)
}

pub fn redact(v: &Value) -> Value {
    redact_at(v, None, 0)
}

fn redact_at(v: &Value, key: Option<&str>, depth: usize) -> Value {
    match v {
        Value::String(s) => {
            let keep =
                key.is_some_and(|k| KEEP_STRING_KEYS.contains(&k)) && s.chars().count() <= 120;
            if keep {
                v.clone()
            } else {
                Value::String(format!("<str {}>", s.chars().count()))
            }
        }
        Value::Number(_) => match key {
            Some(k) if epoch_ms(k, v).is_some() => Value::String(show(k, v)),
            _ => v.clone(),
        },
        Value::Array(items) => {
            if depth >= MAX_DEPTH {
                return Value::String(format!("<array {}>", items.len()));
            }
            let mut out: Vec<Value> = items
                .iter()
                .take(MAX_ITEMS)
                .map(|i| redact_at(i, key, depth + 1))
                .collect();
            if items.len() > MAX_ITEMS {
                out.push(Value::String(format!(
                    "<+{} more>",
                    items.len() - MAX_ITEMS
                )));
            }
            Value::Array(out)
        }
        Value::Object(map) => {
            if depth >= MAX_DEPTH {
                return Value::String(format!("<object {} keys>", map.len()));
            }
            Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), redact_at(v, Some(k), depth + 1)))
                    .collect(),
            )
        }
        _ => v.clone(),
    }
}

fn without_common(line: &Value) -> Value {
    match line.as_object() {
        Some(map) => Value::Object(
            map.iter()
                .filter(|(k, _)| !COMMON_KEYS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Map<String, Value>>(),
        ),
        None => Value::Null,
    }
}

fn str_at<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn content_items(line: &Value) -> &[Value] {
    line.get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// The text of a user line, when its content is a plain string or text items.
fn user_text(line: &Value) -> String {
    match line.get("message").and_then(|m| m.get("content")) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter(|i| str_at(i, "type") == "text")
            .map(|i| str_at(i, "text"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// The short value of `<tag>value</tag>` inside system-made text, such as `<status>completed</status>`.
fn tag_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let len = text[start..].find(&close)?;
    let value = text[start..start + len].trim();
    (value.len() <= 40).then(|| value.to_string())
}

/// Turns transcript lines into log lines about timers and background work.
#[derive(Default)]
pub struct Summarizer {
    /// tool_use id to tool name, for the calls whose results matter.
    watched: HashMap<String, String>,
}

impl Summarizer {
    /// `kinds` also returns a short kind line for every other line, with no text.
    pub fn line(&mut self, line: &Value, kinds: bool) -> Option<String> {
        let ty = str_at(line, "type");
        let mut parts: Vec<String> = Vec::new();
        let mut relevant = false;
        match ty {
            "assistant" => {
                for item in content_items(line) {
                    let item_type = str_at(item, "type");
                    if item_type != "tool_use" {
                        if kinds {
                            parts.push(item_type.to_string());
                        }
                        continue;
                    }
                    let name = str_at(item, "name");
                    let id = str_at(item, "id");
                    let input = item.get("input").unwrap_or(&Value::Null);
                    let background =
                        input.get("run_in_background").and_then(Value::as_bool) == Some(true);
                    if WATCHED_TOOLS.contains(&name) || background {
                        self.watched.insert(id.to_string(), name.to_string());
                        relevant = true;
                        parts.push(format!("tool_use {name} id={id} input={}", redact(input)));
                    } else if kinds {
                        parts.push(format!("tool_use {name}"));
                    }
                }
            }
            "user" => {
                for item in content_items(line) {
                    if str_at(item, "type") != "tool_result" {
                        continue;
                    }
                    let id = str_at(item, "tool_use_id");
                    if let Some(name) = self.watched.get(id) {
                        relevant = true;
                        let is_error = item
                            .get("is_error")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        let result = redact(line.get("toolUseResult").unwrap_or(&Value::Null));
                        parts.push(format!(
                            "tool_result {name} id={id} is_error={is_error} result={result}"
                        ));
                    } else if kinds {
                        parts.push("tool_result".to_string());
                    }
                }
                let text = user_text(line);
                // Starts-with, not contains: the `/loop` skill text quotes the tag in its instructions (seen live).
                if text.trim_start().starts_with("<task-notification>") {
                    relevant = true;
                    let status = tag_value(&text, "status").unwrap_or_else(|| "?".to_string());
                    let task = tag_value(&text, "task-id").unwrap_or_else(|| "?".to_string());
                    parts.push(format!("task-notification task-id={task} status={status}"));
                } else if kinds && parts.is_empty() {
                    let origin = line.get("origin").map(|o| str_at(o, "kind")).unwrap_or("");
                    let meta = line.get("isMeta").and_then(Value::as_bool).unwrap_or(false);
                    parts.push(format!(
                        "text <str {}> origin={origin} meta={meta}",
                        text.chars().count()
                    ));
                }
            }
            "system" => {
                let sub = str_at(line, "subtype");
                relevant = matches!(sub, "scheduled_task_fire" | "turn_duration");
                if relevant {
                    parts.push(format!("{sub} {}", redact(&without_common(line))));
                } else if kinds {
                    parts.push(sub.to_string());
                }
            }
            "attachment" => {
                let attachment = line.get("attachment").unwrap_or(&Value::Null);
                let at = str_at(attachment, "type");
                // `session_cron_carry` (seen live after /clear) lists the timers still armed in the process.
                relevant = matches!(at, "task_status" | "session_cron_carry");
                if relevant {
                    parts.push(format!("{at} {}", redact(attachment)));
                } else if kinds {
                    parts.push(at.to_string());
                }
            }
            _ => {
                if kinds {
                    // Small bookkeeping lines (mode, permission-mode, queue-operation) are worth seeing whole.
                    let body = redact(&without_common(line)).to_string();
                    parts.push(if body.len() <= KIND_BODY_MAX {
                        body
                    } else {
                        String::new()
                    });
                }
            }
        }
        if relevant || (kinds && !parts.is_empty()) {
            let mark = if relevant { "*" } else { " " };
            Some(format!("{mark} {ty}: {}", parts.join("; ")))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redact_keeps_ids_and_hides_text() {
        let v = json!({"cron": "*/5 * * * *", "prompt": "secret plan", "recurring": true, "nextFireAt": 1_762_200_000_000_i64});
        let r = redact(&v);
        assert_eq!(r["cron"], "*/5 * * * *");
        assert_eq!(r["prompt"], "<str 11>");
        assert_eq!(r["recurring"], true);
        assert!(
            r["nextFireAt"]
                .as_str()
                .unwrap()
                .starts_with("1762200000000 (")
        );
    }

    #[test]
    fn fractional_fire_time_is_still_a_time() {
        // Shape seen live from CronCreate in 2.1.316.
        let v = json!({"nextFireAt": 1_793_655_541_131.567_f64});
        let shown = redact(&v)["nextFireAt"].as_str().unwrap().to_string();
        assert!(shown.starts_with("1793655541131.567 ("), "{shown}");
    }

    #[test]
    fn cron_create_and_its_result_are_reported_without_the_prompt() {
        let mut s = Summarizer::default();
        let call = json!({"type": "assistant", "message": {"content": [
            {"type": "text", "text": "ok"},
            {"type": "tool_use", "id": "toolu_1", "name": "CronCreate",
             "input": {"cron": "*/5 * * * *", "prompt": "say the time", "recurring": true}}]}});
        let out = s.line(&call, false).unwrap();
        assert!(out.contains("tool_use CronCreate id=toolu_1"));
        assert!(!out.contains("say the time"));

        let result = json!({"type": "user",
            "message": {"content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "Scheduled"}]},
            "toolUseResult": {"id": "a1b2c3d4", "recurring": true, "nextFireAt": 1_762_200_000_000_i64}});
        let out = s.line(&result, false).unwrap();
        assert!(out.contains("tool_result CronCreate id=toolu_1"));
        assert!(out.contains("\"id\":\"a1b2c3d4\""));
    }

    #[test]
    fn prompt_that_names_a_tool_is_not_a_tool_call() {
        let mut s = Summarizer::default();
        let prompt = json!({"type": "user", "message": {"content": "please call CronCreate and ScheduleWakeup"}});
        assert_eq!(s.line(&prompt, false), None);
        let kinds = s.line(&prompt, true).unwrap();
        assert!(!kinds.contains("CronCreate"));
    }

    #[test]
    fn background_shell_and_fire_lines_are_reported() {
        let mut s = Summarizer::default();
        let call = json!({"type": "assistant", "message": {"content": [
            {"type": "tool_use", "id": "toolu_2", "name": "PowerShell",
             "input": {"command": "Start-Sleep 180", "run_in_background": true}}]}});
        let out = s.line(&call, false).unwrap();
        assert!(out.contains("tool_use PowerShell"));
        assert!(!out.contains("Start-Sleep"));

        let fire = json!({"type": "system", "subtype": "scheduled_task_fire", "taskId": "a1b2c3d4",
            "cron": "*/5 * * * *", "prompt": "say the time", "timestamp": "2026-11-03T10:00:00.000Z"});
        let out = s.line(&fire, false).unwrap();
        assert!(out.contains("scheduled_task_fire"));
        assert!(out.contains("a1b2c3d4"));
        assert!(!out.contains("say the time"));
    }

    #[test]
    fn task_notification_status_is_read_from_the_tags() {
        let mut s = Summarizer::default();
        let note = json!({"type": "user", "message": {"content":
            "<task-notification><task-id>b123</task-id><status>completed</status><summary>x</summary></task-notification>"}});
        let out = s.line(&note, false).unwrap();
        assert!(out.contains("task-id=b123 status=completed"));
    }
}
