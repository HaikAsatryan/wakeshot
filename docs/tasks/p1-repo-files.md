# Add the files a public repo needs

Phase: 1. Status: done. Needs Haik: no (he answered Question 1 on 2026-11-03: fully public). Agent:
`wakeshot-worker`. Size: about 60K tokens.
Runs after: `p1-design-review-fixes`. Not the design sign-off: these four files do not depend on the design.
Parallel with: `p1-cargo-workspace` (it writes only `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/`,
`crates/` and one `CLAUDE.md` line), `p1-ci` (only `.github/`), `p1-tray-menu-look`. This card writes none of their
files.

## Goal

Write the four repo files the first public push needs: `LICENSE`, `README.md`, `.gitignore`, `.gitattributes`. The
orchestrator then runs `git init` (workers never run git).

## Read first

- `CLAUDE.md`; `docs/plan.md` "How we work".
- `docs/product.md`: the opening and "Distribution" (what the project is; MIT; public repo).
- `docs/phases/01-design-skeleton.md`: "Questions for Haik" (answer 1: everything public except secrets and build
  output) and "Orchestrator steps" (the secret scan).
- `docs/architecture.md` sections 2 and 10 (workspace layout, build output folders).
- A listing of the repo tree (`Get-ChildItem -Recurse -File`, without `target`), to see what exists.

## Deliverables

1. `LICENSE`: the MIT license text, "Copyright (c) 2026 Haik Asatryan". Take the text from the SPDX list
   (`https://spdx.org/licenses/MIT.html`), not from memory.
2. `README.md`: a short stub, about 15 lines. What Wakeshot is (one or two sentences from `product.md` "The job"), that
   it is in development and not usable yet, the license, and pointers to `docs/plan.md` and `docs/architecture.md`.
   No badges, no install steps, no screenshots. Plain short English, no em dashes, no emojis.
3. `.gitignore`, each pattern with a one-line comment that says why:
   - `target/` at every level (workspace and each spike): build output.
   - `.claude/worktrees/`: Claude Code puts agent worktrees there, and its docs say to ignore the folder; a left
     worktree would be picked up as an embedded repository.
   - `.claude/settings.local.json`: Claude Code's per-person settings file (local permission rules).
   Nothing else: by Haik's answer 1 everything else goes public, tool logs and fixtures included. Secrets are caught by
   the orchestrator's secret scan, not by `.gitignore`.
4. `.gitattributes`: `* text=auto eol=lf`; `*.cmd`, `*.bat` and `*.ps1` as `text eol=crlf` (Windows shells); images,
   icons, WAV and other binaries as `binary`. Check the repo for every file type present and cover each.

## Acceptance checks

- `LICENSE` matches the SPDX MIT text word for word, with the copyright line above.
- A PowerShell script (in your scratch folder) lists every file in the repo that each `.gitignore` pattern matches, and
  the total count of files that stay. Record both in the Log. Every `target` folder is matched; no source file, doc,
  log or image is matched by mistake.
- Every file extension in the repo is covered by a `.gitattributes` rule or is plain text under `text=auto`.
- README: under 20 lines, no em dashes, no emojis (grep).

## Out of scope

- git: no `git init`, no `git check-ignore`, no commits. The orchestrator checks the ignore list with git later.
- Any other file. CI, Cargo files and `CLAUDE.md` belong to other cards. `docs/product.md`: never.
- Removing or redacting anything: the orchestrator's secret scan handles secrets before the push.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

- **The secret scan skips one log.** `git grep --cached -I` skips files with a NUL in the first 8000 bytes (git
  source, `buffer_is_binary`). `spikes/power-settings/out/hold-limited-20261103-021916.log` has one (NULs inside
  Windows event messages), so the content scan in the phase file's "Secret scan" would not read it. The file is a
  power spike log, so a secret in it is unlikely, but the scan does not prove that. Possible fix for the orchestrator:
  one more pass with `-a` limited to logs (`git grep --cached -a -n -E -f <patterns> -- '*.log'`); not tested.
- **`target/` ignores any folder named `target`**, also a source folder such as `crates/x/src/target/`. None exists.
  The card asked for every level; the catch is only that no code folder may be named `target`.
- **The four spike `.gitignore` files** (`/target` each) are now redundant with the root rule. Harmless; Phase 8
  cleanup can drop them.

## Log

- 2026-11-03: card written by `p1-detailed-design`.
- 2026-11-03 15:50: updated by `p1-design-review-fixes` for Haik's answer 1 (fully public) and general review m10.
- 2026-11-03 15:51: started by `wakeshot-worker`. Read CLAUDE.md, plan "How we work", this card, repo listing.
- 2026-11-03 15:52: SPDX MIT downloaded (`spdx.org/licenses/MIT.html` and `license-list-data/text/MIT.txt`, same
  text). Claude Code docs read: worktrees page says "Add `.claude/worktrees/` to your `.gitignore`"; settings page
  says a hand-made `.claude/settings.local.json` needs a `.gitignore` entry. Four spikes have their own `.gitignore`
  with `/target` only.
- 2026-11-03 15:55: file type scan (scratch `scan-types.ps1`, 284 files outside `target`): extensions `.cmd .gitignore
  .json .jsonl .lock .log .manifest .md .png .ps1 .py .rs .toml .txt`, no extensionless file. One log has NUL bytes,
  three logs have lone CRs; git source `convert.c` (`convert_is_binary`) treats both as binary under `text=auto`, so
  they are stored unchanged. Wrote `LICENSE` (script from SPDX `MIT.txt`, only the copyright line changed, LF),
  `README.md`, `.gitignore`, `.gitattributes`. Next: acceptance checks.
- 2026-11-03 15:56: ignore check without git (scratch `check-ignore.ps1`). It reads every `.gitignore` in the tree,
  turns each rule into a regex by gitignore rules (folder-only, anchored or any depth, `*`, `?`; it throws on `!`, `**`
  or escapes, so no rule passes untested), and walks each path the way git does (the first ignored folder wins).
  - 17 made-up paths, all decided as expected: `target/...`, `crates/*/target/...`, `spikes/tray/target/...`,
    `.claude/worktrees/<name>/...` and `.claude/settings.local.json` ignored; `.claude/settings.json`,
    `.claude/agents/*`, `.claude/skills/*`, `docs/target.md`, `src/target.rs`, an image, two logs, `LICENSE` and
    `.gitattributes` kept. Needed because `.claude/worktrees/` and `settings.local.json` do not exist on disk.
  - Real files: 5924 on disk, 5632 ignored, 292 stay (snapshot; the tray card was still adding pictures).
  - Per rule: root `target/` matches `spikes/claude-watch/target` 1752 files, `spikes/lid-sleep/target` 727,
    `spikes/power-settings/target` 1035, `spikes/tray/target` 2118. Each spike's own `/target` rule matches the same
    files. `.claude/worktrees/` and `.claude/settings.local.json`: 0 files (none exist).
  - All four `target` folders ignored. Ignored files outside a `target` folder: none. Full lists:
    `ignored-files.txt` and `kept-files.txt` in the scratch folder.
  - Kept by type: 3 `.cmd`, 1 `.gitattributes`, 5 `.gitignore`, 1 `.json`, 1 `.jsonl`, 4 `.lock`, 92 `.log`,
    1 `.manifest`, 59 `.md`, 73 `.png`, 1 `.ps1`, 1 `.py`, 36 `.rs`, 4 `.toml`, 9 `.txt`, `LICENSE`.
- 2026-11-03 15:57: file checks (scratch `check-files.ps1`):
  - `LICENSE` equals SPDX `MIT.txt` byte for byte with the copyright line replaced; equals the SPDX page text (default
    form, whitespace ignored); "Copyright (c) 2026 Haik Asatryan" once.
  - README: 17 lines, 0 em dashes, 0 en dashes, 0 non-ASCII characters (so no emoji), longest line 113; its three
    links exist.
  - The four files: UTF-8 without BOM, LF only, end with a newline, ASCII only.
  - `.gitattributes` against all 292 kept files with git's own text test (`gather_stats` ported from `convert.c`): no
    file without a rule. `.cmd` (3) and `.ps1` (1) by their CRLF rules, `.png` (73) by `binary`, everything else
    text under `text=auto` except six logs git sees as binary and stores unchanged (`research.md`, "Repo files").
  - Wrote the checked facts to `research.md`, "Repo files".

## Result

Done 2026-11-03 15:57.

Files written: `LICENSE`, `README.md`, `.gitignore`, `.gitattributes` (repo root); `docs/research.md` ("Repo files");
this card.

- `LICENSE`: SPDX MIT text, "Copyright (c) 2026 Haik Asatryan". Check: byte-for-byte match with SPDX `MIT.txt`
  apart from that line. Pass.
- `README.md`: 17 lines. What Wakeshot is, in development and not usable, links to `docs/plan.md`,
  `docs/architecture.md`, `LICENSE`. Check: under 20 lines, no em dash, no emoji (no non-ASCII at all). Pass.
- `.gitignore`: `target/`, `.claude/worktrees/`, `.claude/settings.local.json`, each with a why comment, plus one
  header line saying everything else is public on purpose. Check: every `target` folder ignored (5632 files), nothing
  outside `target` ignored, 292 files stay, 17 made-up paths decided right. Pass. Checked with a PowerShell matcher,
  not git; the orchestrator's `git status --ignored` is the real test.
- `.gitattributes`: `* text=auto eol=lf`; `*.cmd`, `*.bat`, `*.ps1` as `text eol=crlf`; `*.png`, `*.jpg`, `*.jpeg`,
  `*.gif`, `*.ico`, `*.wav` as `binary`. Check: every kept file has a rule; all `text=auto` files are text to git
  except six logs that git stores unchanged. Pass.

Open items for the orchestrator (not verified, no git here): at `git add -A`, git will likely warn that line endings
change in `spikes/morning-shell.log`, `spikes/morning-shell-selftest.log` (CRLF to LF) and
`spikes/morning-shell.cmd`, `spikes/morning-shell.ps1` (LF, CRLF on checkout). Expected, not an error.
