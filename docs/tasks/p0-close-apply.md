# Apply part 2 of the Phase 0 close list

Phase: 0. Status: done. Needs Haik: no (his yes is recorded). Agent: `wakeshot-worker`.
Runs after: `p0-close-recheck` (done) and Haik's yes (2026-11-03, recorded under "Questions from the recheck" in
`docs/reviews/p0-close-proposals.md`).
Parallel with: `p0-morning-trim` (it edits `docs/phases/00-morning-steps.md` and `docs/tasks/p0-spike-*.md` only).

## Goal

Make the design docs match Haik's decisions and the Phase 0 facts found so far, by applying part 2 of
`docs/reviews/p0-close-proposals.md` exactly as tagged.

## Read first

- `docs/reviews/p0-close-proposals.md`: part 1, part 2 (items 1 to 33 and their recheck tags), "Questions from the
  recheck" with Haik's answer.
- Each target file at the lines an item names. Line numbers may have moved: find each line by its old text.

## Deliverables

1. Every item tagged Keep or Change, and new items 26 to 31 and 33, applied to its file. Change items use the
   recheck's new wording. Drop items: no edit.
2. Item 34 (new, from recheck question 3): `docs/product.md`, battery floor rule. At the floor: release every request,
   restore settings and notify at once; hibernate a closed laptop only after the same few minutes with no input as
   item 28.
3. Recheck question 1's answer reaches `CLAUDE.md`: today "Treat any unknown shape as no signal". New meaning: a file
   we cannot parse gives no signal; a readable file with an unknown `status` value counts as working while the
   session process lives, and Wakeshot shows "update Wakeshot". Keep the "never read `.key` files" part.
4. Item 25 (live-test fixtures), as reworded by the recheck, with this source: the orchestrator copied the old
   scratchpad's `live` and `ccstate` folders to `C:\Users\haika\.claude\wakeshot-keep\2026-11-03-live-tests\`. Read
   from there. Redact prompts, user names and personal paths. Never read or copy a `.key` file. Fixtures go to
   `spikes/claude-watch/fixtures/` with a short README on where each came from.
5. Skip, and say so in the Result: items for `docs/phases/00-facts.md` (24 and any other; give the exact edit, the
   orchestrator applies it), item 32 (applied when the lid card closes), and every part 3 item (waits for the
   morning results).
6. Below each applied item in the proposals file, add one line: "Applied 2026-11-03." Do not change anything else in
   that file.

## Acceptance checks

- Every part 2 item has either "Applied 2026-11-03" or a reason in the Result for why not.
- No edited line still says the lid-closed time limit, the heat guard, or a time cap on `busy` or `shell` exist as
  product rules.
- No fixture contains a prompt text, a user name, a personal path or any content from a `.key` file (grep and
  record the grep).
- `CLAUDE.md` stays short and its other facts are unchanged.

## Out of scope

- `docs/phases/00-morning-steps.md`, `docs/tasks/p0-spike-*.md`, `docs/phases/00-facts.md`.
- Any spike run, power setting or process.

## Questions for Haik

Each with the worker's default.

1. `spikes/claude-watch/fixtures/raw-transcript-lines.jsonl` holds two lines from your own transcript (2026-11-02),
   every text field replaced; only ids, times, enum values and the notification tag names are left. The repo goes
   public in Phase 1. Default: keep them, they are the only raw shape of a task notification we have.

## Found outside scope

Problems seen but not fixed.

- `docs/tasks/p0-morning-trim.md:139` and `:141` cite `product.md:56` (battery floor) and `product.md:53` (no lock
  if Never). After this card those lines are `product.md:66` and `product.md:63`.
- Signals rule 11 (`claude-code-signals.md`, transcript growth as a fallback when the status file is missing or
  unknown) still stands. Its "unknown" case now contradicts the new rule (a file that does not parse is no signal; an
  unknown `status` counts as working). The "missing file" case is for Phase 4. No part 2 item touches it.
- Signals rule 5's confidence cell still lists "tasks restored after resume under a new session id" as a miss. The
  new pid keying covers `/clear` (same pid); a `--resume` in a new process is not covered. Phase 4 can restate it.
- `00-facts.md:47` still says WSL is "Open: do we read WSL homes?". Item 6 decided it (not in v1).
- `CLAUDE.md:11` says "Decisions and open questions: `docs/product.md`", but `product.md` has no Open section now.
  Left as is, because this card keeps CLAUDE.md's other facts unchanged.

## Log

- 2026-11-03: card written by the orchestrator.
- 2026-11-03: worker started. Read CLAUDE.md, plan.md "How we work", the card and the proposals file.
- 2026-11-03: kept folder listed: `live` (watch.log, watch.stdout.txt, agents.json, section.md, fallback.md,
  splice.py, claude-watch.exe) and `ccstate` (driver state). No `.key` file. The lock file is not in the copy, but
  watch.log has its content in SIDE FILE lines. watch.log transcript lines are the watcher's summaries, not raw JSONL;
  the raw task notification text is not in it.
- 2026-11-03: `product.md` done: items 1, 2, 4, 5 (Change), 6, 8, 9, 10, 11, 26, 27, 28, 34. Item 27's "(line 31)"
  written as "(the `waiting` rule above)", because the line moved. Extra: the UX line on "update Wakeshot" now also
  names the unknown `status` value (recheck question 1), so CLAUDE.md and the rules table point at a line that says it.
- 2026-11-03: `architecture.md` done: items 12, 13, 15, 16, 17, 18, 19, 20. Item 18's child pid bullet went into the
  `holds` row; item 12's "part 3 item 4" written in plain words.
- 2026-11-03: `claude-code-signals.md` item 21 done (rows merged into one row per rule number, so "rule 3" stays one
  rule; the `updatedAt` note is a line under the table). Rule 10's "transcripts too" is not backed by line 182, so it
  is marked not verified. `CLAUDE.md` items 22 and 23 done; `product.md:97` written without a line number, because
  part 3 edits lines above it.
- 2026-11-03: orchestrator asked for one more `CLAUDE.md` edit: the Nightcrew line. Checked read-only: both Nightcrew
  folders, `HKCU:\Software\Nightcrew`, a Run entry and a scheduled task are all gone. Line now says nothing is left;
  dropped the `--uninstall` tip (no exe left), kept "never run it during tests" for a reinstall. Fact added to
  `research.md:154` (same line, so no line numbers move).
- 2026-11-03: items 29, 30, 31 done. `research.md:192` kept on one line so later `research.md` line refs stay right.
- 2026-11-03: item 25 fixtures written to `spikes/claude-watch/fixtures/`: `live-tests.log` (401 of 1078 watcher
  lines, test sessions only, redacted), `agents.json` (redacted), `raw-transcript-lines.jsonl` (one background shell
  result and its task notification from Haik's own wakeshot project transcripts, every text field replaced), README.
  Scripts in my scratch folder printed only redacted text. Item 25 names Haik's transcripts as a second source, so I
  read them through that script only; no other file there was opened. `cctest*` folders in `~/.claude/projects`: 0.
- 2026-11-03: `claude-code-signals.md` Sources line fixed (watcher log kept, where) and one new fact (raw task
  notification shape, `origin.kind`). Proposals file: "Applied 2026-11-03." added below 27 items and below recheck
  question 3 (as item 34); a diff against a scratch copy showed only those 28 lines plus blank lines added.
- 2026-11-03: acceptance checks run (outcomes in Result). Card done.

## Result

Done 2026-11-03.

Applied (each has "Applied 2026-11-03." in the proposals file): 1, 2, 4, 5, 6, 8 to 13, 15 to 23, 25 to 31, and
recheck question 3 as item 34. Small wording changes, all logged above: line numbers replaced where the line had
moved ("(line 31)" in item 27, `product.md:97`), "part 3 item 4" and "decision 3" written in plain words, rule 10's
"transcripts too" marked not verified, rows of item 21 merged into one row per rule.

Extra: `product.md` UX line on "update Wakeshot" now names the unknown `status` value (recheck question 1); the
Nightcrew line in `CLAUDE.md` (orchestrator's request); one new fact in `claude-code-signals.md` (raw task
notification shape, `origin.kind`).

Not applied:

- 3, 7, 14: tagged Drop.
- 24 and 33: `docs/phases/00-facts.md`, the orchestrator applies them. Exact edits, find each line by its text:
  - `- Rejected by earlier decision: "hold 2h" menu item (...the CLI covers it).` Append: ` (superseded: Keep awake
    now has 2 hours, product.md UX)`.
  - `- Every "still working" signal needs a time limit. ... laptop got hot in a bag.` Append: ` (superseded:
    decision 3, signals are checked against facts)`.
  - `- Docked detection: also check external displays (LidGuard WMI rule), not only the user's lid action.` Append:
    ` (no v1 rule uses it after decisions 1 to 3)`.
  - `- Heat guard candidate: thermal zone reads without admin here (about 50 C). Verify in spikes.` Append: ` (not
    adopted: decision 2)`.
  - Optional, same file: `- Claude Code inside WSL is invisible to Windows-side tools. Open: do we read WSL homes?`
    Append: ` (decided: not in v1, product.md)`.
- 32: applied when `p0-spike-lid-sleep` closes.
- Part 3, items 1 to 8: wait for the morning results.

Files changed: `CLAUDE.md`, `docs/product.md`, `docs/architecture.md`, `docs/research.md` (lines 154 and 192, each
kept on one line so no `research.md` line ref moves), `docs/research/claude-code-signals.md`,
`docs/research/rivals-nightcrew-lidguard.md`, `docs/research/rivals-others.md`, `docs/reviews/p0-close-proposals.md`
(only the "Applied" lines). New: `spikes/claude-watch/fixtures/` with `README.md`, `live-tests.log`, `agents.json`,
`raw-transcript-lines.jsonl`.

Checks:

- Every part 2 item is marked Applied or listed above with a reason: pass.
- Grep of the edited files for "time limit", "heat guard", "cap", "lid-closed limit": every edited hit says there is
  none ("no time limit and no heat guard", "No time cap", "no cap", "should not exist", "Not adopted"). Pass.
- Fixture grep (PowerShell `Select-String` over the fixtures folder): `haika|haik|asatryan|gmail|@|session_01|p16s|
  \.key|1828f2de|pandatech` hits only README line 20 ("No `.key` file was read or copied"); real UUIDs 0; user paths
  not `USER` 0; plain strings over 40 characters in the raw lines 0, and in transcript summaries 0 apart from
  `humanSchedule`. No `.key` file was opened. Pass.
- `CLAUDE.md`: 31 lines (was 28). Only the unknown-shape line, the two "Microsoft docs; not tested by us" marks and the
  Nightcrew line changed. Pass.

Open items: the 00-facts edits above (orchestrator), item 32 (lid card close), part 3 (morning results), the
question for Haik on the raw lines.
