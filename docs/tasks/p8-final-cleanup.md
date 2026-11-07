# Planning out, a short CLAUDE.md, every pointer fixed

Phase: 8. Status: waits for every other card of the night (new 2026-11-07). Needs Haik: no (Phase 8 as he defined it
on 2026-11-03, and his go of 2026-11-07 02:39; every call is for his review).
Agent: `wakeshot-worker`. Size: about 160K tokens.
Runs after: every other Phase 8 card, and every other card still open tonight (`p4-live-uninstall`, the video upload,
`p6-end-card-numbers`, `p6-logs-diagnostics`, `p6-pages`, `p6-winget-upgrade-restart`, and `p4-live-claude-desktop`
unless it waits for Haik). Runs alone: no other agent edits the repo while it works. Right before it starts, the
orchestrator commits everything and makes the history bundle, so nothing removed here is lost.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p8-final-cleanup\target` only if a check needs a build; delete it when
done.

## Goal

The repo holds no internal planning, `CLAUDE.md` is a short set of facts for AI coding agents, and every pointer in
the repo (README, docs, code comments, scripts, `site/`, `media/`) leads to a file and heading that exist.

## Read first

- `CLAUDE.md` (the one you replace).
- `docs/phases/08-launch-cleanup.md`, whole.
- The Results of every Phase 8 card: their "Heading renames", "Renames", pointer lists (`p8-cleanup`), sections left
  for you (`p8-docs-research-2`), "Found outside scope".
- The current Claude Code docs on memory files (`CLAUDE.md`, a per-person project file that git ignores, imports).
  Note what you read and the date.

## Deliverables

1. Remove `docs/phases/` (all but `08-launch-cleanup.md`), `docs/tasks/` (all but this card), `docs/reviews/`,
   `.claude/agents/`, `.claude/skills/`. Keep `docs/plan.md`, the phase file and this card: the orchestrator records
   your Result in them, then deletes the three in its last commit before the squash. Leave Claude Code's runtime
   files in `.claude/` (git ignores them).
2. `CLAUDE.md`, at most about 60 lines, all checked against the code:
   - what Wakeshot is, in two sentences; the layout (the four crates, `docs/`, `packaging/`, `site/`, `media/`);
   - the checks (the exact commands from `CONTRIBUTING.md`; never set `RUSTFLAGS`; `--locked`; the ARM64 clang note);
   - the rules an agent must follow here: comments say why; no behavior change without a test; tests that write real
     power settings are `#[ignore]`, work on a plan copy, and run only with Wakeshot and every other keep-awake app
     quit; a test that expects a denied open must run unelevated;
   - the facts to keep in mind from today's `CLAUDE.md` (Claude Code's undocumented session files and the "update
     Wakeshot" rule; lid close and power requests; Modern Standby needs `PowerRequestSystemRequired`; on battery the
     requests end about 5 minutes after the battery sleep timeout; a user's own sleep ends every request), each with
     its pointer into `docs/`;
   - no planning, no orchestrator, no sessions, no task cards.
3. Machine notes (call 11): the test laptop's plan and values, the dock, "agents here run elevated" with the Git Bash
   privilege note, the cargo path. If the current Claude Code docs offer a per-person project file that git ignores and
   that is not deprecated, write them there and add its name to `.gitignore` (one line). Otherwise write them to the
   archive folder the orchestrator names, and list them in the Result for Haik.
4. Apply every recorded rename across the repo: heading renames (anchors and quoted section names) and item renames,
   in `README.md`, `CONTRIBUTING.md`, `docs/`, `crates/` (comments only), `packaging/`, `.github/`, `site/`, `media/`.
5. The pointer check (write it in your scratch folder): every relative link and picture path in the kept markdown and
   HTML files; every `docs/<name>.md` or `research/<name>.md` mention with a quoted section name or an anchor, in any
   kept file; `architecture.md` "section N" (1 to 12) and "decision N" (0 to 44); `product.md` "User settings safety"
   N. GitHub anchor rules: lowercase, punctuation dropped except hyphens, spaces to hyphens, repeated headings get
   `-1`, `-2` in order. It must report zero misses, the three planning files left out.
6. `README.md`: only pointers into removed paths, renamed anchors, and facts the polish made stale (a renamed path or
   command). Not the video spot, not the speed numbers, not "Why I built it" or "What comes next". List every README
   change in the Result.
7. `media/` (call 12): only pointers into removed paths and internal names in comments (`README.md`, `src/fonts.ts`,
   `src/theme.ts`, scripts). No change to code, data or outputs.
8. Sections another card appended after the docs cards ran (listed in `p8-docs-research-2`'s Result, and any newer):
   clean them by rules D1 to D6.
9. Orphan pictures: delete each file in `docs/research/img/` whose name no kept file mentions (search markdown, HTML,
   code and scripts: `art.rs` writes `icon-sheet-*.png`, `media/` reads them).
10. The internal-name sweep (phase file, "Every card") over the whole repo, leaving out `target/`, `node_modules/`, test
    fixture data and the three planning files: zero hits outside the allowed places. Fix what is left in docs and
    comments; list each fix in code with its file.
11. Result: what was removed (file counts, bytes); `CLAUDE.md` line count; renames applied; the pointer check output;
    the sweep output; README and `media/` changes; where the machine notes went; what the orchestrator still does.

## Acceptance checks

1. Under `docs/plan.md`, `docs/phases/`, `docs/tasks/` only the three planning files remain; `docs/reviews/`,
   `.claude/agents/`, `.claude/skills/` are gone.
2. The pointer check: zero misses (paste its last lines).
3. The internal-name sweep: zero hits outside the allowed places (paste the command and its count).
4. `cargo fmt --all --check` passes and `cargo metadata --locked --format-version 1` succeeds. If you changed a Rust
   comment, also `cargo doc --workspace --no-deps --locked` with no warning in the files you changed.
5. `python -I site/check_page.py site`: the same outcome as `p8-repo-hygiene` recorded.
6. If node is installed: `node media/scripts/check-art.mjs` passes.
7. `CLAUDE.md` is at most about 60 lines, and every pointer in it resolves.
8. No em dash in any file you changed.

## Out of scope

Behavior, string literals, the README's approved text, the video and its numbers, `media/` code and data. Git, the
history bundle and the squash (the orchestrator).

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
