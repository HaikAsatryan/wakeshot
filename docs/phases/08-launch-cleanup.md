# Phase 8: launch cleanup

Planned 2026-11-07 by `docs/tasks/p8-plan.md` (session wakeshot-4e, Haik asleep). Every call in this file is for
Haik's review; the list is under "Calls for Haik's review".

## Goal

Haik, 2026-11-07 02:39: "you can do cleanup, you can even do code cleanup session if needed (names, structure,
places, comments (I mean deleting idiotic wordy comments). Like full polish of the repo so it would not look like fully
vibe coded repo. Structured and following best practices repo, with good page and so on." 02:40: his yes for the
history squash once the polish is done and every check and CI is green.

So the public repo reads as a normal, well-kept Rust project: code with short "why" comments and no trace of how the
work was planned; docs that explain the product, the design and the facts, not the process; internal planning out of
the repo. Then the orchestrator squashes the history into one commit.

## What stays and what goes

### The repo after Phase 8

```text
.cargo/config.toml     static CRT
.github/               CI, release, Pages, issue and PR templates, Dependabot
assets/                app icon, notice icon
crates/                the four crates and build-resources.rs
docs/
  architecture.md      the design
  product.md           what Wakeshot does and why
  brand.md             colours, icons, lockups
  hold-files.md        the hold file format, for users and scripts
  agent-hooks.md       the hook design for the agents out of v1 (their code stays, unwired)
  research.md          facts and how they were checked, grouped by topic
  research/            longer reports and their pictures
media/                 README pictures and the promo video sources
packaging/             pack, smoke and end-to-end scripts, winget templates
site/                  the GitHub Pages page
CLAUDE.md              short facts for AI coding agents
README.md, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, LICENSE
Cargo.toml, Cargo.lock, rust-toolchain.toml, .gitignore, .gitattributes, .editorconfig
```

### Removed

| Path | Why | Card |
|---|---|---|
| `docs/plan.md`, `docs/phases/`, `docs/tasks/`, `docs/reviews/` | internal planning and review history; the history bundle keeps it | `p8-final-cleanup`; the orchestrator deletes the last three files (`docs/plan.md`, this file, the final card) after it records them |
| `.claude/agents/`, `.claude/skills/` | they drive the internal workflow; no skill a contributor needs (call 2) | `p8-final-cleanup` |
| `spikes/` and the root `exclude = ["spikes"]` | throwaway Phase 0 programs and their logs; their facts are in `research.md`, the code that mattered is ported into the crates (call 1) | `p8-repo-config` |
| the six old-brand reports (`docs/research/brand-*.md`) and the old-art pictures listed in `p8-cleanup.md` | Haik, 2026-11-05: no legacy of the old brand | `p8-cleanup` |
| pictures in `docs/research/img/` that nothing points at after the docs cards | orphans | `p8-final-cleanup` |
| `Owner:` lines, card codes, session names, review finding numbers, "Haik said" | planning history, meaningless to a reader | every polish card, in its own files |

### Kept and rewritten

| File or folder | Card | What changes |
|---|---|---|
| the crates' `.rs` files | eight `p8-polish-*` cards | comments, names, dead code, file places (rules below) |
| `Cargo.toml` files, `rust-toolchain.toml`, `.cargo/config.toml`, `build.rs`, `crates/build-resources.rs`, the exe manifests | `p8-repo-config` | comments, `publish = false`, lints if they pass, the listed cross-crate renames |
| `docs/architecture.md`, `docs/product.md`, `docs/hold-files.md` | `p8-docs-design` | planning history out; section names and numbers kept |
| `docs/research/*.md` (not brand), `docs/agent-hooks.md` | `p8-docs-research-reports` | planning history out; process-only sections cut |
| `docs/research.md` | `p8-docs-research-1`, then `p8-docs-research-2` | history out, headings named by fact, grouped by topic with a contents list |
| `docs/brand.md` | `p8-cleanup` | old-brand pointers and history out |
| `.github/`, `packaging/`, `site/`, community files, `.gitignore`, `.gitattributes`, `.editorconfig` | `p8-repo-hygiene` | comments, internal names, missing best-practice files |
| `CLAUDE.md`, README pointers, `media/` pointers | `p8-final-cleanup` | short `CLAUDE.md`; pointers only in README and `media/` |

### Pointers between files

Code and scripts point at docs about 400 times (architecture sections, decision numbers, product sections, research
sections), and the README about 16 times. The rules:

1. A pointer into a removed path (`docs/plan.md`, `docs/phases/`, `docs/tasks/`, `docs/reviews/`, `spikes/`, the six
   brand reports) is replaced by the fact in a few words, or by a pointer to a kept doc, or dropped. The card that owns
   the file with the pointer does it.
2. A pointer into a kept doc stays as written. Docs cards keep what others cite: `architecture.md` section numbers 1 to
   12 and their subsection names, the "Tray startup" step numbers, the decision numbers 0 to 44 in "Decisions and where
   they live", `product.md`'s section names and its "User settings safety" item numbers. A docs card that renames a
   heading anyway (for example one named after a phase) records old and new in its Result, "Heading renames".
3. A code card records each renamed item in its Result, "Renames".
4. `p8-final-cleanup` applies every recorded rename across the repo, then runs a pointer check that must find zero
   broken pointers.
5. No card adds a new pointer to a `research.md` section during Phase 8: its headings move.

### Not in Phase 8

- `media/`: tonight's media cards own it. `p8-final-cleanup` fixes only its pointers into removed paths and internal
  names in its comments (call 12).
- README: Haik approved it (2026-11-06 and 2026-11-07). Only pointers into removed paths, renamed anchors and facts the
  polish made stale change; not the video spot, not the speed numbers (`p6-end-card-numbers`), not "Why I built it" or
  "What comes next".
- Behavior: no polish card changes what the exes do, write, log or show.
- Public API redesign, crate splits, big module splits: only what a code card lists as a candidate, and at most about
  10 such changes in `p8-repo-config`.
- The squash, the winget submission, the first release (the orchestrator and Haik).
- The Microsoft Store card `p8-store` stays blocked (free signing first) and goes to the archive with the other cards.

## Polish rules

### Code

C1. No behavior change. No string literal changes: user text, log lines, file and registry names, command lines, test
expectations. No signature change of an item used outside your own files. The no-code-change check proves it.

C2. Comments say why, never what. Delete a comment that repeats the code or the item's name. Keep a short reason, a
Windows or agent quirk, a trap, a unit, a contract the caller relies on. One or two lines is the norm. A module doc
says what the module is for and its non-obvious rules, in one short paragraph plus a "Callers may rely on" list where
the module is a seam. Keep every `// SAFETY:` comment (about 590, one per `unsafe` block today; shorten a wordy one,
never drop it): `p8-repo-config` may turn on the lint that requires them. Plain short English, no em dashes, no
emojis.

C3. No planning history in code that stays: no `Owner:` lines, card codes (`p4-cron`), session names, "track note N",
"Question N", review finding numbers, "for Haik's review", "Haik said", "the contracts card", "the integration cards",
phase numbers as process, dates of decisions. A date or version of a fact seen live may stay when it matters; the
agent's version beats the date.

C4. Pointers follow rules 1 to 5 above.

C5. Names follow the naming chapter of the Rust API guidelines (casing, `as_`, `to_`, `into_`, no `get_` on getters,
iterator names, word order). Rename only an item whose every use is in your own files (grep all of `crates/` and
`packaging/`). List other candidates under "Rename candidates" in your Result, for `p8-repo-config`. Record each rename
you made (old, new, file).

C6. Dead code: remove an item nothing uses (grep all crates, examples, tests and `packaging/`; both clippy runs). Keep
the unwired Cursor, Antigravity and VS Code Local chat code (Haik, 2026-11-06: what was built stays for a later
version); trim its comments only.

C7. Places: move a file only inside your own folders, and only when it plainly sits in the wrong module. A file over
about 1,500 lines whose inline `mod tests` is most of it may move those tests, unchanged, to `<module>/tests.rs` (a
scripted cut, not hand edits). Anything bigger (a move across folders, a split, a merge): list it under "Structure
candidates" in your Result.

C8. No new traits, impls, derives, dependencies or features. Keep every test; a test's name says what it proves.

C9. A bug you find goes under "Found outside scope", never into a quiet fix.

C10. The module lists in `lib.rs` and `main.rs` change only for a move rule C7 allows, and only by the card that owns
that file.

### Docs

D1. A doc explains the product, the design or a fact. It does not tell how the work was organized. Remove card codes,
phases as process, session names, "Haik said", "decided by the worker", "for Haik's review", review finding numbers,
pointers into removed paths, and drafts or plans that something newer replaced.

D2. Decisions stay, written as rules in the present tense. Who decided and when goes.

D3. Facts keep how and when they were checked ("tested on the test laptop, 2026-11-03", "Microsoft docs, updated
2025-07-01"). Never drop a "not verified" or "not tested" mark. "Haik's laptop" becomes "the test laptop"; "Haik's
values" becomes "the test laptop's own values".

D4. Keep every heading and numbered item others point at, or record the rename (pointer rule 2).

D5. Plain short English, no em dashes, no emojis. Shorter is better when nothing is lost; never cut a fact the code, the
README or another kept doc relies on.

D6. Haik's style reference videos stay out of the repo. Add no product name that is not in the repo today; the project
memory names one that must never appear.

D7. A docs card never edits `README.md`, `media/` or code.

### Every card

- Never run git, not even read-only. Never read token files. Never set `RUSTFLAGS`.
- Plain short English, no em dashes, no emojis, in every line you write. Rule D6 holds in code comments too.
- Only your own files (the ownership table). A problem elsewhere goes under "Found outside scope".
- Other cards may still append to `docs/research.md` while you work: re-read the part you edit right before each edit.
- The internal-name sweep, on your own files at the end: zero hits for card codes (`\bp[0-9](-[0-9])?-[a-z]`),
  `Owner:`, `\bHaik\b`, `orchestrator`, session names (`wakeshot-[0-9a-f]{2}\b`), `docs/(plan|phases|tasks|reviews)`,
  `track note`, `[Ff]inding [0-9]`, `spikes/` and the six brand report names. Allowed: "Haik Asatryan" as the author
  in the places that name him (`LICENSE`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `build-resources.rs`, the exe manifests,
  the winget templates, `media/CREDITS.md`, the README); test fixture data captured live (never edited); a hit that
  is plain English, not a reference (say which in the Result).

### Checks for a code card

While other cards run, check only your own package and files; the orchestrator runs the whole-workspace checks on x64
and ARM64 when a group closes. Build into your own `CARGO_TARGET_DIR` (`<scratchpad>\<card>\target`) and delete it at
the end. In agent shells cargo may be `%USERPROFILE%\.cargo\bin\cargo.exe`.

1. Before the first edit: copy every owned file to `<scratchpad>\<card>\before\` (paths relative to `crates/`), and save
   `cargo test -p <package> --locked -- --list` (each package you touch).
2. Format: `rustfmt --edition 2024 <file>` on each owned file you changed. Never on a file that declares a module you do
   not own (rustfmt follows `mod` lines): leave `lib.rs` to the card that owns it and format it by hand.
3. `cargo clippy -p <package> --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings`, and the same
   without `--all-targets`.
4. `cargo test -p <package> --locked`. Then the test list again: the same count, names changed only by your listed
   renames.
5. `cargo doc -p <package> --no-deps --locked`: no warning in your files (a comment edit can break an intra-doc link).
6. The no-code-change check: every difference is a listed rename, removal or move.
7. The internal-name sweep (above): zero hits in your files.
8. Comment lines in your files before and after (count lines whose first non-blank characters are `//`), as a number
   for the Result.

A failure in a file you do not own: wait a minute and run again (another card may be mid-edit); if it stays, report
it and go on.

### The no-code-change check

Save this as `<scratchpad>\<card>\code_diff.py` and run
`python -I <scratchpad>\<card>\code_diff.py <scratchpad>\<card>\before <repo>\crates`. It compares only the paths in
`before`, so new files (a rule C7 `tests.rs`) are not checked by it: a C7 move shows as a change in the file the tests
left; list it and show that the moved block's stripped text equals the new file's. It removes comments and whitespace
outside string literals and compares the rest byte for byte. Self-tested 2026-11-07 by `p8-plan`, this exact text,
on a copy of all of `crates/`: deleting 10,865 whole-line comments and adding trailing comments gave "no code
change"; one changed type, one changed letter in a string literal and one removed file were each found.

```python
import os, re, sys

CHAR = re.compile(r"'(?:\\u\{[0-9a-fA-F]{1,6}\}|\\x[0-9a-fA-F]{2}|\\.|[^\\'\n])'")
RAW = re.compile(r'(?:b|c)?r(#*)"')


def strip(src):
    out, i, n = [], 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
            continue
        if src.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            continue
        word_before = i > 0 and (src[i - 1].isalnum() or src[i - 1] == "_")
        m = RAW.match(src, i)
        if m and not word_before:
            end = src.find('"' + m.group(1), m.end())
            end = n if end < 0 else end + 1 + len(m.group(1))
            out.append(src[i:end])
            i = end
            continue
        if c == '"' or (c in "bc" and i + 1 < n and src[i + 1] == '"' and not word_before):
            j = i + (1 if c == '"' else 2)
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            out.append(src[i:j + 1])
            i = j + 1
            continue
        if c == "'":
            m = CHAR.match(src, i)
            if m:
                out.append(m.group(0))
                i = m.end()
                continue
        if not c.isspace():
            out.append(c)
        i += 1
    return "".join(out)


def rs_files(root):
    return {os.path.relpath(os.path.join(d, f), root) for d, _, fs in os.walk(root) for f in fs if f.endswith(".rs")}


before, after = sys.argv[1], sys.argv[2]
changed = False
for rel in sorted(rs_files(before)):
    path = os.path.join(after, rel)
    if not os.path.exists(path):
        print(f"removed or moved: {rel}")
        changed = True
        continue
    x = strip(open(os.path.join(before, rel), encoding="utf-8").read())
    y = strip(open(path, encoding="utf-8").read())
    if x != y:
        k = next((i for i in range(min(len(x), len(y))) if x[i] != y[i]), min(len(x), len(y)))
        print(f"code changed: {rel}: before ...{x[max(0, k - 60):k + 60]}... after ...{y[max(0, k - 60):k + 60]}...")
        changed = True
print("differences listed above" if changed else "no code change")
sys.exit(1 if changed else 0)
```

## Cards

Sizes are estimates; real use runs 2 to 3 times higher.

| Card | Does | Size | Needs Haik | Runs after | Status |
|---|---|---|---|---|---|
| `p8-polish-core-engine` | `wakeshot-core`: state machine, policies, events, commands, files, text | 150K | no | `p6-logs-diagnostics`, any open Rust fix card | ready |
| `p8-polish-core-sources` | `wakeshot-core`: agent sources and the core's integration tests and fixtures | 150K | no | same | ready |
| `p8-polish-windows-power` | `wakeshot-windows`: power layer, recovery, process, session, `power_check` | 150K | no | same | ready |
| `p8-polish-windows-watch` | `wakeshot-windows`: folder watch, agent IO, hooks install, SQLite, Restart Manager | 140K | no | same | ready |
| `p8-polish-core-ownership` | `wakeshot-core`: ownership routines, the simulator, the scenario tests | 150K | no | same | ready |
| `p8-polish-windows-tray` | `wakeshot-windows`: tray icon, art, menu, theme, notices, sounds, `ux_*` examples | 140K | no | same | ready |
| `p8-polish-windows-app` | `wakeshot-windows`: app wiring, store, log, update, install, Velopack hooks | 150K | no | same | ready |
| `p8-polish-bins` | `wakeshot-cli` and `wakeshot-tray`: sources and tests | 120K | no | same | ready |
| `p8-docs-design` | `architecture.md`, `product.md`, `hold-files.md` | 150K | no | `p6-logs-diagnostics` | ready |
| `p8-docs-research-reports` | `docs/research/*.md` (not brand), `agent-hooks.md` | 140K | no | `p6-logs-diagnostics` | running since 04:18 |
| `p8-docs-research-1` | `research.md` from the top through "Update thread checks" | 150K | no | `p6-logs-diagnostics` | running since 04:18 |
| `p8-cleanup` | old-brand history: reports, pictures, `brand.md` | 90K | no | `p6-logs-diagnostics` | done (04:28): 89 files removed (six brand reports, 83 old-art pictures); `brand.md` rewritten with sources; 26 pointers for other cards in its Result |
| `p8-docs-research-2` | `research.md` from "Tray UX: tray icon" to the end; topic groups and contents | 170K | no | `p8-docs-research-1`, `p6-end-card-numbers` | waits |
| `p8-repo-hygiene` | `.github/`, `packaging/`, `site/`, community and editor files | 140K | no | `p6-logs-diagnostics`, `p6-pages`, `p6-end-card-numbers`, `p6-winget-upgrade-restart` | waits |
| `p8-repo-config` | Cargo and build files, `spikes/`, listed renames and moves, whole-workspace checks | 110K | no | the eight `p8-polish-*` cards | waits |
| `p8-final-cleanup` | planning out, short `CLAUDE.md`, every pointer fixed | 160K | no | every other card, `p4-live-uninstall` and the video upload included | waits |

About 2.3M tokens estimated in all.

## Order

Building cards run at most four at a time. Docs cards do not build and run beside them.

- A Rust card that opens later (for example from `p4-live-claude-desktop` once Haik gives his input) does not hold
  the polish back: it runs after the polish card that owns its files and follows the polish rules in what it writes.
- **Group 1**, when `p6-logs-diagnostics` (and any Rust fix card open at that time) is done:
  building `p8-polish-core-engine`, `p8-polish-core-sources`, `p8-polish-windows-power`, `p8-polish-windows-watch`;
  beside them `p8-docs-design`, `p8-docs-research-reports`, `p8-docs-research-1`, `p8-cleanup`. The three docs cards
  that do not touch `architecture.md` or `product.md` may start earlier, if the orchestrator has slots.
- **Group 2**, each the moment a slot frees: building `p8-polish-core-ownership`, `p8-polish-windows-tray`,
  `p8-polish-windows-app`, `p8-polish-bins`; beside them `p8-docs-research-2` (after `p8-docs-research-1` and
  `p6-end-card-numbers`, and when no running card still appends to `research.md`) and `p8-repo-hygiene` (after the
  four cards it names).
- The orchestrator runs the whole-workspace checks on x64 and ARM64 when each group's building cards are done.
- **Alone among building cards**: `p8-repo-config`, after all eight code cards. Docs cards may still run beside it.
- **Alone, last**: `p8-final-cleanup`, after every other card of the night (`p4-live-uninstall`, the video upload and
  the end card numbers included). Right before it, the orchestrator commits everything and makes the history bundle.
- Then the orchestrator: whole-workspace checks, push, CI green, a release dry run (`release.yml` by hand), the squash
  with Haik's given yes, CI green again.
- From `p8-final-cleanup` on, the repo has no `/orchestrate` skill and no agent files: a session started after that
  loads them from the archive, not from the repo. Later work (the field test, fixes) is planned outside the repo.

## File ownership

New files that `p6-logs-diagnostics` or another open card adds go to the card that owns their folder. A new top-level
file in `crates/wakeshot-windows/src/` goes to `p8-polish-windows-app`; in `crates/wakeshot-core/src/` to
`p8-polish-core-engine`; any other new file to the card the orchestrator names.

| Path | Card |
|---|---|
| `crates/wakeshot-core/src/`: `lib.rs`, `command.rs`, `core.rs`, `core/`, `event.rs`, `files.rs`, `holds.rs`, `holds/`, `present.rs`, `process.rs`, `settings.rs`, `state.rs`, `text.rs`, `time.rs`, `view.rs`, `policy/`; `crates/wakeshot-core/proptest-regressions/` | `p8-polish-core-engine` |
| `crates/wakeshot-core/src/`: `ownership/`, `sim.rs`, `testing.rs`, `scenarios/` | `p8-polish-core-ownership` |
| `crates/wakeshot-core/src/`: `sources.rs`, `sources/`; `crates/wakeshot-core/tests/` (fixture READMEs; fixture data is never edited) | `p8-polish-core-sources` |
| `crates/wakeshot-windows/src/`: `lib.rs`, `error.rs`, `sink.rs`, `testing.rs`, `power/`, `recovery.rs`, `process.rs`, `session.rs`; `examples/power_check.rs` | `p8-polish-windows-power` |
| `crates/wakeshot-windows/src/`: `tray.rs`, `tray/`, `notify.rs`, `sound.rs`, `sound/`; `examples/ux_*.rs` | `p8-polish-windows-tray` |
| `crates/wakeshot-windows/src/`: `app.rs`, `app/`, `update.rs`, `install.rs`, `shell.rs`, `hooks.rs`, `child.rs`, `store.rs`, `log.rs`, `io_queue.rs`, `clock.rs`, `localtime.rs` | `p8-polish-windows-app` |
| `crates/wakeshot-windows/src/`: `watch/`, `agent_hooks.rs`, `sqlite.rs`, `lockowner.rs`; `examples/agent-watch.rs` | `p8-polish-windows-watch` |
| `crates/wakeshot-cli/src/`, `crates/wakeshot-cli/tests/`, `crates/wakeshot-tray/src/`, `crates/wakeshot-tray/tests/` | `p8-polish-bins` |
| every `Cargo.toml`, `Cargo.lock` (must not change), `rust-toolchain.toml`, `.cargo/`, `crates/build-resources.rs`, `crates/*/build.rs`, `crates/*/*.exe.manifest`, `spikes/`; cross-crate renames and moves (any `.rs` file, after the code cards) | `p8-repo-config` |
| `docs/architecture.md`, `docs/product.md`, `docs/hold-files.md` | `p8-docs-design` |
| `docs/agent-hooks.md`; `docs/research/*.md` except the six `brand-*.md` | `p8-docs-research-reports` |
| `docs/research.md` | `p8-docs-research-1`, then `p8-docs-research-2` (never at the same time) |
| `docs/brand.md`; `docs/research/brand-*.md`; the old-art pictures in `docs/research/img/brand/` and `docs/research/img/ux/` listed in `p8-cleanup.md` | `p8-cleanup` |
| `.github/` (all), `packaging/`, `site/`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `.gitignore`, `.gitattributes`, `.editorconfig` (new) | `p8-repo-hygiene` |
| `CLAUDE.md`, `.claude/`, `docs/plan.md`, `docs/phases/`, `docs/tasks/`, `docs/reviews/`, orphan pictures in `docs/research/img/`; pointer fixes anywhere, `README.md` and `media/` included (pointers and internal names only) | `p8-final-cleanup` |
| `media/`, the README video spot, the README and site speed numbers | not Phase 8 (tonight's media cards) |

## Calls for Haik's review

Made by `p8-plan`, 2026-11-07, under Haik's go of 02:39. Each can be undone from the history bundle.

1. **`spikes/` goes.** Throwaway Phase 0 programs with about 100 log files; their facts are in `research.md` and the
   code that mattered is in the crates. `research.md` lines that cite a spike say "a throwaway test program, not kept".
   Change it when a Windows fact must be checked again: restore the spike from the bundle.
2. **No Claude skill or agent stays in `.claude/`.** CONTRIBUTING already lists the checks; the agents and the
   `/orchestrate` skill drive the internal workflow and go to the archive. Change it when contributors repeat a
   workflow a skill would save.
3. **`research.md` stays one file**, its sections named by fact (not by card) and grouped by topic under a contents
   list. Splitting it would break about 60 pointers from code, README and site. Change it when one topic outgrows it:
   that topic becomes a report in `docs/research/`.
4. **The old-brand pictures go without new captures.** The notice header colours measured on two of them stay in
   `research.md`, marked "measured on 2026-11-04 captures, not kept (they show the old icon)". No screen session
   tonight. Change it when the notice colours are measured again.
5. **The unwired Cursor, Antigravity and VS Code Local chat code stays**, with its comments trimmed (Haik, 2026-11-06:
   what was built stays for a later version).
6. **Cross-crate renames and module moves are capped**: the code cards only list them; `p8-repo-config` does at most
   about 10 clear wins. Change it when a public API is redesigned on purpose, after v1.
7. **Large test modules may move to `tests.rs`** (files over about 1,500 lines, worker's call, mechanical). Change it
   when a reviewer prefers inline tests: move back with the same script.
8. **Dependabot for GitHub Actions and Cargo**, monthly, grouped, with `velopack` and `idna_adapter` ignored (both
   pinned on purpose). Change it when its pull requests become noise.
9. **No `CHANGELOG.md`**: release notes live in GitHub Releases. Change it when a second release ships and users ask.
10. **`publish = false` in every crate**: none is meant for crates.io, and it stops an accidental publish.
11. **Machine notes leave `CLAUDE.md`** (the test laptop's values, elevated agent shells, the cargo path): to a
    per-person file Claude Code reads but git ignores, if its current docs offer one, else to the archive, listed for
    Haik. The public `CLAUDE.md` keeps repo facts only.
12. **`media/` is not polished beyond pointers and internal names**, because tonight's media cards own it. Change it
    when the media sources change again after v1.
13. **README: pointer fixes only.** Headings named after a phase in the research reports (README anchors such as
    `#phase-4-live-test-2026-11-06`) are renamed and the README anchors follow in `p8-final-cleanup`. The README's
    hidden SmartScreen todo comment stays until the first pre-release, as written.
14. **`Owner:` lines go everywhere.** They served the parallel tracks; git blame and the module docs do that job now.
15. **No behavior change is proven, not assumed**: the comment-stripping compare per file and equal test lists per
    package, plus the release dry run in CI before the squash.

## Facts behind the plan

Measured 2026-11-07 by `p8-plan` with scripts over the working tree (Python, comment lines counted as lines whose first
non-blank characters are `//`, `#` or `/*`):

- Rust: `wakeshot-core` 37.2K lines in `src` plus 6.6K in `tests`; `wakeshot-windows` 53.3K in `src` plus 6.2K in
  `examples`; `wakeshot-tray` 0.4K plus 3.5K tests; `wakeshot-cli` 5.3K plus 0.9K tests.
- About 13.3K comment lines in the crates. Per card group: core engine 1.85K, core ownership and tests 1.96K, core
  sources and tests 1.98K, windows power 1.83K, windows tray 1.58K, windows app 1.91K, windows watch 1.58K, bins 1.03K.
- In the crates: about 430 lines with a card code, 240 `Owner:` lines, 80 lines naming Haik, 400 pointers into docs.
- Docs that stay: `research.md` 6,194 lines (568 KB; 250 lines with card codes, 189 naming Haik, 100 pointers into
  planning files), `architecture.md` 147 KB, `agent-hooks.md` 74 KB, `product.md` 31 KB, 19 reports in
  `docs/research/` (680 KB, six of them old brand), 230 pictures (16 MB, 12.8 MB of it in `img/brand/`).
- No `TODO`, `FIXME` or `dbg!` in the crates; one lint override, `#![allow(dead_code)]` in
  `wakeshot-tray/tests/e2e/mod.rs` (a helper module shared by test binaries); one `get_` function
  (`power/plan.rs`, `get_active`, private).

## Work log

- 2026-11-07 04:18 (orchestrator, session wakeshot-4e): plan accepted as written. Started `p8-docs-research-reports`,
  `p8-cleanup` and `p8-docs-research-1` (no build; they do not touch `architecture.md` or `product.md`). The code
  cards wait for `p6-logs-diagnostics`. Orchestrator decision, for Haik's review: the planning files and `.claude/`
  agents and skill leave the public repo by untracking, not deleting: right before the squash the orchestrator removes
  them from the index and lists them in `.git/info/exclude` (local only, never pushed), so they stay on this laptop and
  later sessions can still run `/orchestrate`; the history bundle and an archive copy keep them too.
  `p8-final-cleanup` therefore fixes pointers and writes the short `CLAUDE.md` but deletes none of them.
- 2026-11-07 04:45 (orchestrator): `p6-logs-diagnostics` done. Group 1 changes, for Haik's review: two Rust fix cards
  run first beside two polish cards that own other files: `p6-winget-upgrade-restart-fix`, `p6-log-silent-decisions`
  (log lines the logs audit found missing), `p8-polish-core-engine`, `p8-polish-windows-power`. `p8-polish-core-sources`
  and `p8-polish-windows-watch` move to group 2 (the log card owns `sources/` and `watch/` until it ends). Every
  build waits about 20 minutes first, so `p6-end-card-numbers` measures idle CPU on a quiet laptop. `p8-docs-design`
  also takes one `product.md` line: `wakeshot diagnostics` is a CLI-only support command, like `run`.
