# Clean research.md, part 1: from the top through "Update thread checks"

Phase: 8. Status: done (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p6-logs-diagnostics`. Parallel with: every card that does not own `docs/research.md` (group 1; no build).
Never at the same time as `p8-docs-research-2`. Other open cards may append sections at the end of the file while you
work: they are not yours; re-read the part you edit right before each edit.
Build folder: none.

## Goal

The first half of `docs/research.md` keeps every fact and how it was checked, and loses the planning history: each
section is named by its fact, not by the card that checked it, and points at nothing that Phase 8 removes.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Pointers between files", "Polish rules" (Docs,
  Every card), "File ownership", call 1 (`spikes/` goes) and call 3 (`research.md` stays one file).
- Your range of `docs/research.md`: from the top through the section "Update thread checks" (about lines 1 to 2,990
  today, 270 KB). Read it in parts, section by section.
- What points at your sections: grep the repo outside `docs/tasks/`, `docs/reviews/` and `docs/phases/` for
  `research.md` (code cites sections by quoted name, for example `docs/research.md` "Process module checks"; the README
  links `#windows-power-facts` and `#story-checks-powertoys-awake-and-claudes-own-keep-awake`).

## Deliverables

1. Every section in your range cleaned by rules D1 to D6:
   - No card codes, sessions, "for card `docs/tasks/...`", "Haik said", pointers into `docs/plan.md`, `docs/phases/`,
     `docs/tasks/`, `docs/reviews/`. Keep the date, the machine ("the test laptop", Windows build), the command or test
     that checked it, the source and its date, and every "not verified" or "not tested" mark.
   - Pointers into `spikes/` (removed by `p8-repo-config`): keep what was run and what it showed; say "a throwaway
     test program (not kept in the repo)" instead of the path.
   - Tests in the crates stay as evidence by name (`store::tests`, `atomic_write_replaces_a_file_a_reader_has_open`).
2. Headings named by fact: "Store checks" becomes, for example, "Files, the registry and named mutexes". Keep the
   README's two anchors unchanged. Record every rename (old heading, old anchor, new heading, new anchor) in the Result;
   `p8-final-cleanup` fixes the pointers.
3. Sections that only record planning (the track plan checks, "Contracts split checks", "Track plan fixes checks",
   "Design verify checks" and the like): remove them when they hold no fact about Windows, an agent, a tool or the
   product; move a fact they do hold into the section of its topic. List each removed section and where its facts went.
4. Do not reorder sections and do not touch the file's opening paragraph: `p8-docs-research-2` groups the whole file by
   topic and writes the intro and contents list. Do not edit sections from "Tray UX: tray icon" on.
5. Result: "Heading renames"; removed sections and where their facts went; sections you left as they are and why;
   size of your range before and after.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on your range: zero hits (`spikes/` included).
2. No em dash (U+2014) in your range.
3. Every section name or anchor in your range that a file outside `docs/tasks/`, `docs/reviews/`, `docs/phases/` cites
   still exists, or is in "Heading renames". Show the grep.
4. "not verified" and "not tested" counts in your range: none lost (count before and after, minus those inside
   sections you removed, each listed).
5. Every relative link and picture path in your range resolves to a file that stays.

## Out of scope

The rest of `docs/research.md`, every other file, code, the README. Git.

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

- Pointers to headings this card renamed, outside `docs/research.md` (for `p8-final-cleanup`; old name, then files,
  from the check 3 grep at 04:43; code cards running tonight may already have changed some comments):
  - "Power settings spike": `crates/wakeshot-core/src/policy/floor.rs`.
  - "Lid and sleep spike": `crates/wakeshot-windows/examples/power_check.rs`, `src/power/hibernate.rs` (2),
    `src/power/requests.rs`.
  - "Spike launch and stop checks": `examples/power_check.rs`, `packaging/e2e.ps1`, `packaging/e2e-update.ps1`.
  - "Phase 1 design checks": `docs/architecture.md`.
  - "Ship track planning checks": `crates/wakeshot-windows/src/update.rs`.
  - "Log writer checks", "Store checks": `docs/architecture.md`.
  - "Process module checks": `crates/wakeshot-windows/src/process.rs`, `docs/architecture.md`.
  - "Power rules checks": `crates/wakeshot-core/src/policy/lid.rs`, `docs/architecture.md`.
  - "Journal seam checks": `src/power/journal.rs`. "System events checks": `src/power/system.rs`.
  - "Recovery checks": `examples/power_check.rs`, `src/recovery.rs`.
  - "Cron checks": `crates/wakeshot-core/src/sources/cron.rs`, `docs/architecture.md`,
    `docs/research/claude-code-signals.md`.
  - "Claude transcript checks": `crates/wakeshot-core/src/sources/claude/transcript.rs`,
    `docs/research/claude-code-signals.md` (2).
  - "Folder watch checks": `src/watch/dir.rs` (2). "Path value checks": `src/install.rs`.
  - "Power track real checks": `crates/wakeshot-tray/tests/e2e_real.rs` (2), `docs/architecture.md` (5).
  - "CLI argument checks": `crates/wakeshot-cli/src/args.rs`.
  - "Claude UI stop notice checks": `docs/research/claude-code-signals.md`.
  - `spikes/*/README.md` also cite three of them; they go with `spikes/`.
- Pointers to renamed headings in the second half of `docs/research.md` (for `p8-docs-research-2`; line numbers of
  04:43): 3462 "Contracts build", 3999 and 4005 "Lid and sleep spike", 4454 "Power settings spike".
- Two "Haik Asatryan" hits stay in the range: the Velopack section's `vpk pack --packAuthors "Haik Asatryan"` command
  and the `Publisher` value Windows shows. They are the author name as package data; the phase file's allowed list
  does not name `research.md`, so the final sweep should allow these two.

## Log

- 2026-11-07: card written by `p8-plan`.
- 2026-11-07 04:17: started (session wakeshot-4e). Range today: lines 1 to 2989 of 6293, file 577,198 bytes.
- 2026-11-07 04:22: method: edit a scratch copy of the range, then splice it into `docs/research.md` in one write at the end (re-read right before, tail from "Tray UX: tray icon" checked unchanged). Cleaned the top through "Lid and sleep spike" in the copy.
- 2026-11-07 04:34: copy cleaned through "Harness checks" (about two thirds). Moved the four facts of "Design verify checks" into their topic sections; dropped two process-only bullets (secret scan dry run, commit-msg pattern) and the "Spikes outside the workspace" bullet.
- 2026-11-07 04:42: whole range cleaned in the copy (59 headings renamed, "Design verify checks" removed, reflowed at 120 columns). Spliced into `docs/research.md` at 04:42: live head equal to the starting copy first, tail (309,489 bytes, with the appends made meanwhile) re-read equal after.
- 2026-11-07 04:43: acceptance checks run on the live file (all pass; see Result). Calls for Haik's review, made
  under the phase file's rules: (1) 59 of 78 headings renamed by fact, cited ones included, because the card asks for
  fact names and `p8-final-cleanup` applies recorded renames; the README's two anchors and 17 headings that already
  name their fact stay. (2) Planning-named sections that hold real facts were renamed, not split up and moved: topic
  grouping is `p8-docs-research-2`'s job. Only "Design verify checks" was removed. (3) Dropped as process-only: the
  secret scan dry run, the commit-msg pattern check, "Spikes outside the workspace", "No `match` lists every variant",
  "Whole-workspace checks after the changes", and pointers to review findings and checklist items. (4) "Haik" became
  "the tester", "by hand", "a real right-click" or "the test laptop's own values"; "the orchestrator" became "an
  (elevated) agent session"; "this laptop" and "on this machine" became "the test laptop" (50 places). (5) The manual
  test blocks keep their letters (C, D, E, F, H, I), now defined once in "Morning results", since many lines cite
  them. (6) Two "at the time" lines got a fact checked tonight: clang is in the Build Tools now (listed 2026-11-07),
  and `app/io_thread.rs` sets the session-end `Ack` (read 2026-11-07).

## Result

Done 2026-11-07 04:44. `docs/research.md` from the top through "The update schedule and thread" (old "Update thread
checks") is cleaned by rules D1 to D6; nothing from "Tray UX: tray icon" on was touched, and the opening paragraph is
byte for byte the same.

What changed:

- Every card code, session name, "for card", "Haik said", review finding, checklist item and pointer into
  `docs/plan.md`, `docs/phases/`, `docs/tasks/`, `docs/reviews/` is gone. Dates, the machine, the commands, the test
  names, the sources with their dates and every "not verified" or "not tested" mark stay.
- Every `spikes/` pointer now says what was run and what it showed, with "a throwaway test program (not kept in the
  repo)" once per section; log file names stay as evidence names.
- 59 headings renamed by fact (table below). Internal references in the range follow the new names.
- One section removed (below). Lines reflowed to 120 columns where an edit made them long.

Heading renames (old heading, old anchor, new heading, new anchor):

| Old heading | Old anchor | New heading | New anchor |
|---|---|---|---|
| Power settings spike | `#power-settings-spike` | Power settings without admin: writes, change detection and the battery rule | `#power-settings-without-admin-writes-change-detection-and-the-battery-rule` |
| Lid and sleep spike | `#lid-and-sleep-spike` | Lid, sleep, hibernate and resume | `#lid-sleep-hibernate-and-resume` |
| Spike launch and stop checks | `#spike-launch-and-stop-checks` | Medium launches through Explorer and console stop events | `#medium-launches-through-explorer-and-console-stop-events` |
| Agent reach on this laptop | `#agent-reach-on-this-laptop` | Agent reach on the test laptop | `#agent-reach-on-the-test-laptop` |
| Phase 1 design checks | `#phase-1-design-checks` | Crates, runners and Windows APIs read for the design | `#crates-runners-and-windows-apis-read-for-the-design` |
| Repo files | `#repo-files` | Licence text and git file rules | `#licence-text-and-git-file-rules` |
| Contracts build | `#contracts-build` | Dependency weight and the test-support feature | `#dependency-weight-and-the-test-support-feature` |
| Sources track plan checks | `#sources-track-plan-checks` | Cron syntax, Restart Manager types and child process rules | `#cron-syntax-restart-manager-types-and-child-process-rules` |
| Power track plan checks | `#power-track-plan-checks` | `windows` crate features for the power layer | `#windows-crate-features-for-the-power-layer` |
| Ship track planning checks | `#ship-track-planning-checks` | clang for `ring` on ARM64, and the Velopack update API | `#clang-for-ring-on-arm64-and-the-velopack-update-api` |
| Tray UX track plan checks | `#tray-ux-track-plan-checks` | `windows` crate features for the tray | `#windows-crate-features-for-the-tray` |
| Contracts split checks | `#contracts-split-checks` | Workspace members, `windows` features and the core's tree | `#workspace-members-windows-features-and-the-cores-tree` |
| Track plan fixes checks | `#track-plan-fixes-checks` | rustfmt on one file, and cargo test filters | `#rustfmt-on-one-file-and-cargo-test-filters` |
| Close contracts checks | `#close-contracts-checks` | `GetCommandLineW`'s feature, a lock file line and a workspace copy | `#getcommandlinews-feature-a-lock-file-line-and-a-workspace-copy` |
| Log writer checks | `#log-writer-checks` | Log appends, rolls and the UTC offset | `#log-appends-rolls-and-the-utc-offset` |
| Restore routine checks | `#restore-routine-checks` | Restore routine: a plan switch mid-run and a stop at any call | `#restore-routine-a-plan-switch-mid-run-and-a-stop-at-any-call` |
| Process module checks | `#process-module-checks` | Process opens, the system process list and exit waits | `#process-opens-the-system-process-list-and-exit-waits` |
| Store checks | `#store-checks` | Files, the registry and named mutexes | `#files-the-registry-and-named-mutexes` |
| Hold file checks | `#hold-file-checks` | Hold files: process start times, dates and the doc's examples | `#hold-files-process-start-times-dates-and-the-docs-examples` |
| Tray view checks | `#tray-view-checks` | Tooltip and balloon text limits | `#tooltip-and-balloon-text-limits` |
| Power rules checks | `#power-rules-checks` | Short AC drops on the dock | `#short-ac-drops-on-the-dock` |
| Journal seam checks | `#journal-seam-checks` | Journal file and mutex | `#journal-file-and-mutex` |
| Power plan checks | `#power-plan-checks` | Plan copies, the plan list and setting GUIDs | `#plan-copies-the-plan-list-and-setting-guids` |
| Apply and ownership check checks | `#apply-and-ownership-check-checks` | Apply: entry takeover and stop walks | `#apply-entry-takeover-and-stop-walks` |
| Core state machine checks | `#core-state-machine-checks` | proptest persistence, `#[expect]` and the safety tick in tests | `#proptest-persistence-expect-and-the-safety-tick-in-tests` |
| Power request checks | `#power-request-checks` | Power request counts and handles | `#power-request-counts-and-handles` |
| System events checks | `#system-events-checks` | Power notifications: unregister, threads and payloads | `#power-notifications-unregister-threads-and-payloads` |
| Session checks | `#session-checks` | Session messages, end of session and the console session | `#session-messages-end-of-session-and-the-console-session` |
| Power IO checks | `#power-io-checks` | Power executor stops and journal set-asides | `#power-executor-stops-and-journal-set-asides` |
| Watch reads checks | `#watch-reads-checks` | Reading agent files: sharing, name trimming and file ids | `#reading-agent-files-sharing-name-trimming-and-file-ids` |
| Core settings checks | `#core-settings-checks` | A PowerShell quirk and the settings scenarios | `#a-powershell-quirk-and-the-settings-scenarios` |
| Watch data checks | `#watch-data-checks` | Hold file write times and a hold file held open | `#hold-file-write-times-and-a-hold-file-held-open` |
| Hibernate thread checks | `#hibernate-thread-checks` | Hibernate calls and the shutdown privilege | `#hibernate-calls-and-the-shutdown-privilege` |
| Recovery checks | `#recovery-checks` | Recovery guard: stop event, Explorer parent and exe modes | `#recovery-guard-stop-event-explorer-parent-and-exe-modes` |
| Claude status file checks | `#claude-status-file-checks` | Claude Code status files against live writes | `#claude-code-status-files-against-live-writes` |
| Core lid checks | `#core-lid-checks` | Lid rules: the plan read lag and planted mutants | `#lid-rules-the-plan-read-lag-and-planted-mutants` |
| Stop point checks | `#stop-point-checks` | Stops at any call on a real plan copy | `#stops-at-any-call-on-a-real-plan-copy` |
| Cron checks | `#cron-checks` | Claude Code cron: parser, search and fire times | `#claude-code-cron-parser-search-and-fire-times` |
| Dispatcher checks | `#dispatcher-checks` | Tray loop dispatcher: re-entry, the session-end wait and deadlines | `#tray-loop-dispatcher-re-entry-the-session-end-wait-and-deadlines` |
| Journal property test checks | `#journal-property-test-checks` | Ownership properties: false user changes and fault coverage | `#ownership-properties-false-user-changes-and-fault-coverage` |
| Claude transcript checks | `#claude-transcript-checks` | Claude Code transcripts: cron carry, resume restore and notifications | `#claude-code-transcripts-cron-carry-resume-restore-and-notifications` |
| Harness checks | `#harness-checks` | The power_check harness on the test laptop | `#the-power_check-harness-on-the-test-laptop` |
| Core property test checks | `#core-property-test-checks` | Core properties in the simulated world | `#core-properties-in-the-simulated-world` |
| Tray words checks | `#tray-words-checks` | Notice text limits, menu prefixes and the tooltip | `#notice-text-limits-menu-prefixes-and-the-tooltip` |
| Tray startup checks | `#tray-startup-checks` | Tray startup: the panic hook, mutex tests and an unknown lock state | `#tray-startup-the-panic-hook-mutex-tests-and-an-unknown-lock-state` |
| Loop window checks | `#loop-window-checks` | The loop window: post limit, timers and end of session | `#the-loop-window-post-limit-timers-and-end-of-session` |
| Power review fix checks | `#power-review-fix-checks` | Lid and lock fixes: tests first, strict properties and mutants | `#lid-and-lock-fixes-tests-first-strict-properties-and-mutants` |
| Folder watch checks | `#folder-watch-checks` | Folder watch with `ReadDirectoryChangesW` | `#folder-watch-with-readdirectorychangesw` |
| CLI command checks | `#cli-command-checks` | Opening files from the CLI, and its commands | `#opening-files-from-the-cli-and-its-commands` |
| Claude parsing review fix checks | `#claude-parsing-review-fix-checks` | Claude Code task stops, Monitor timeouts and fire lines | `#claude-code-task-stops-monitor-timeouts-and-fire-lines` |
| Watch thread checks | `#watch-thread-checks` | The watch thread: latency, handles and a denied listing | `#the-watch-thread-latency-handles-and-a-denied-listing` |
| Power track real checks | `#power-track-real-checks` | Real checks of the power layer on the test laptop | `#real-checks-of-the-power-layer-on-the-test-laptop` |
| CLI argument checks | `#cli-argument-checks` | Command line splitting and the built CLI | `#command-line-splitting-and-the-built-cli` |
| Claude UI stop notice checks | `#claude-ui-stop-notice-checks` | Claude Code's UI stop notice in the transcript | `#claude-codes-ui-stop-notice-in-the-transcript` |
| Launch research checks | `#launch-research-checks` | Launch facts: rivals, winget, the Store and README media | `#launch-facts-rivals-winget-the-store-and-readme-media` |
| CLI run checks | `#cli-run-checks` | `wakeshot run`: finding, quoting and stopping the child | `#wakeshot-run-finding-quoting-and-stopping-the-child` |
| Path value checks | `#path-value-checks` | Registry values and the user Path | `#registry-values-and-the-user-path` |
| IO thread checks | `#io-thread-checks` | A Windows wait timeout against Rust's clock | `#a-windows-wait-timeout-against-rusts-clock` |
| Update thread checks | `#update-thread-checks` | The update schedule and thread | `#the-update-schedule-and-thread` |

Removed sections and where their facts went:

- "Design verify checks" (`#design-verify-checks`): "End of the plan list" merged into "The plan list ends with 259
  here" in "Plan copies, the plan list and setting GUIDs"; "Console notices" into "Session messages, end of session
  and the console session"; the `WaitForSingleObject` quote of "Abandoned mutex" into the "Abandoned mutex" bullet of
  "Files, the registry and named mutexes"; "Panic hook" into "Tray startup: the panic hook, mutex tests and an unknown
  lock state". The commit-msg pattern bullet was dropped: it tested a commit hook of the internal workflow that is
  not in the repo. No "not verified" or "not tested" mark was lost: the two in it moved with their bullets.
- Bullets dropped as process-only (sections kept): "Secret scan patterns, dry run" (patterns in a removed phase file,
  hits in removed spike files), "Spikes outside the workspace", "No `match` in `crates/` lists every `Event` or
  `Notice` variant", "Whole-workspace checks after the changes", and the "Odd files" bullet rewritten as a general
  fact about logs of Windows tools.

Sections left with their heading, and why: "The problem", "Claude Code today", "How to know Claude Code is working",
"Reading elevated Claude Code sessions from an unelevated process", "Competitors", "Insomnia review (code read
2026-11-02)", "Distribution", "Name research", "Toolchain", "Tray, menu and notifications", "Lid-close sound",
"Workspace build", "CI workflow", "Velopack checks for Wakeshot" (its point numbers kept), "Tray UX: theme reads",
"Tray UX: menu painting", "Tray UX: icon art": each already names its fact. "Windows power facts" and "Story checks:
PowerToys Awake and Claude's own keep awake": the README's two anchors. Their bodies were cleaned like the rest.

Size of the range: 273,474 bytes and 2,989 lines before; 268,467 bytes and 2,938 lines after.

Acceptance checks (run 04:43 on the live file, range = lines before "## Tray UX: tray icon"; the live range is byte
for byte the cleaned copy):

1. Internal-name sweep (card codes, `Owner:`, `\bHaik\b`, `orchestrator`, session names, planning paths,
   `track note`, `[Ff]inding [0-9]`, `spikes/`, the six brand report names): pass. Two hits left, both "Haik Asatryan"
   as package data in "Velopack checks for Wakeshot" (`--packAuthors`, `Publisher`), allowed as the author name (see
   Found outside scope).
2. Em dash (U+2014) in the range: 0. Pass.
3. Cited names and anchors: pass. A script (`check3.py` in the scratchpad) greps every kept file outside
   `docs/tasks/`, `docs/reviews/`, `docs/phases/` for each old heading of the range, quoted (line breaks and comment
   markers flattened) or as an anchor. Cited and still existing: "Windows power facts" (README anchor 2, launch.md),
   "Reading elevated Claude Code sessions from an unelevated process", "Distribution", "Tray, menu and notifications",
   "Lid-close sound", "Workspace build", "CI workflow", "Velopack checks for Wakeshot", "Tray UX: theme reads", "Tray
   UX: menu painting", "Story checks: ..." (README anchor), "Tray UX: icon art". Cited and renamed: the 20 names
   listed under Found outside scope ("Contracts build" only in the second half), each in the table above. Bold labels cited by code and docs ("Live theme switch
   through Settings", "`LoadIconMetric` follows the system DPI", "Menu text in grey", "First add lands in the
   overflow", "Limits: Kernel-Power 125", "Live checks", "Electron", Velopack points 1, 4, 7, 9, 10) all still exist.
4. Marks, counted case-insensitive over text with line breaks flattened: "not verified" 77 before, 76 after; "not
   tested" 43 before, 44 after. The one "not verified" gone was a quote of another doc's old mark ("This answers
   architecture section 8's "not verified (Phase 3)""), not a mark on a fact here: the fact it sat beside has a test.
   The extra "not tested" is new ("could not test, and not tested later" for wake timers). None lost with the removed
   section.
5. Relative links and picture paths: pass. The range has no Markdown links; every picture name and pattern (in
   `docs/research/img/` and `img/ux/menu-render-*.png`) and doc path (`docs/research/*.md`,
   `docs/hold-files.md`, `docs/brand.md`, `assets/wakeshot.ico`) resolves to a file that stays. The tooltip picture
   path was corrected to `docs/research/img/tray-tooltip-dark.png`.

Files changed: `docs/research.md` (the range only), this card.

Open items: the pointer fixes under Found outside scope (`p8-final-cleanup` for code, scripts and docs;
`p8-docs-research-2` for four lines in its half).
