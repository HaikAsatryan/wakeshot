# Repo hygiene: workflows, packaging, site, community and editor files

Phase: 8. Status: waits for the four cards below (new 2026-11-07). Needs Haik: no (his go of 2026-11-07 02:39; every
call is for his review).
Agent: `wakeshot-worker`. Size: about 140K tokens.
Runs after: `p6-logs-diagnostics` (it may change the bug report form and the packaging checks), `p6-pages` (owns
`site/` and `pages.yml` now), `p6-end-card-numbers` (the site's numbers), `p6-winget-upgrade-restart` (may change
`packaging/`). Parallel with: every card that does not own these files (group 2; no cargo build).
Build folder: none.

## Goal

Everything around the code looks like a well-kept open source repo: workflows, scripts and community files say why in
few words and carry no planning history, the best-practice files that are missing are added, and nothing changes what
a workflow or a script does.

## Read first

- `CLAUDE.md`.
- `docs/phases/08-launch-cleanup.md`: "What stays and what goes", "Polish rules" (Code rules C1, C2, C3, C9 apply to
  scripts too; Docs; Every card), "File ownership", calls 8 and 9.
- Each owned file's comments (read comments, not whole scripts; `packaging/e2e-update.ps1` and `packaging/e2e.ps1`
  are 73 KB and 55 KB), and each owned markdown file whole.

## Owned files

- `.github/`: `workflows/ci.yml`, `workflows/release.yml`, `workflows/pages.yml`, `ISSUE_TEMPLATE/` (all),
  `PULL_REQUEST_TEMPLATE.md`, and the new `dependabot.yml`.
- `packaging/` (all: `pack.ps1`, `smoke-test.ps1`, `e2e.ps1`, `e2e-update.ps1`, `README.md`, `winget/`).
- `site/` (all), after `p6-pages` ends.
- `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `LICENSE`, `.gitignore`, `.gitattributes`, the new
  `.editorconfig`.

## Deliverables

1. Workflows: comments say why, without "decision 44", "Phase 8", "Haik sets", card names or pointers into
   `docs/tasks/` (`pages.yml` points at `docs/tasks/p6-pages.md`). "The maintainer" where a person must act. No
   change to a key, value, step or trigger.
2. `packaging/` scripts and READMEs: the same rules; wordy comments cut to a line; card codes, "Haik" and pointers into
   removed paths out. No change to what a script does, prints or checks (log lines and CLI output it matches are
   behavior). The winget templates' author fields stay.
3. `site/`: comments without planning history; `style.css` points at `brand-directions-2.md` "Step 2: the colour",
   which `p8-cleanup` removes: point at `docs/brand.md`. No change to the page's visible text or look. Leave the
   layout as it is (the repo-only files sit in `site/` and `pages.yml` removes them at upload); change it when more
   repo-only files join.
4. Community files: facts current (the checks in `CONTRIBUTING.md` match `ci.yml`; `CODE_OF_CONDUCT.md` names a working
   way to reach the maintainer; `LICENSE` year and name). The standard text of the code of conduct keeps its words.
5. `.github/dependabot.yml` (call 8): `github-actions` and `cargo` at `/`, monthly, each ecosystem grouped into one
   pull request, `velopack` and `idna_adapter` ignored (pinned on purpose: `velopack` must match the `vpk` in
   `release.yml`, `idna_adapter` picks a stub). No `npm` for `media/` (the video is final). Use the current GitHub
   docs for the syntax and note the date read.
6. `.gitignore`: a plain reason in place of "(Haik, 2026-11-03)"; add `.claude/scheduled_tasks.lock` (a Claude Code
   runtime file). Tell the orchestrator in the Result: if that file is tracked, it must be untracked at commit.
7. `.gitattributes`: every binary file type in the repo is marked binary (list the types on disk outside `target/`
   and `node_modules/`: `woff2`, `mp4`, `webp` and others if present).
8. `.editorconfig` (new): UTF-8, final newline, trailing whitespace trimmed (not in `.md`, where two spaces can mean a
   break; check the docs use none), LF except `*.ps1`, `*.cmd`, `*.bat` as CRLF (matching `.gitattributes`), indent
   and line length as the files are today (Rust: 4 spaces, rustfmt formats code at its default 100 columns and the
   comments are wrapped by hand at 120; docs: 120 columns; check YAML, JSON, TypeScript and PowerShell indents on
   disk). Comments say why.
9. Result: per file what changed; the calls you made; the `.claude/scheduled_tasks.lock` note.

## Acceptance checks

1. The internal-name sweep (phase file, "Every card") on the owned files: zero hits, except the author's name in
   `LICENSE`, `SECURITY.md`, `CODE_OF_CONDUCT.md` and the winget templates.
2. No em dash in text you wrote or changed.
3. Each PowerShell script parses with no error, and its tokens other than comments and line breaks are equal before
   and after (`[System.Management.Automation.Language.Parser]::ParseFile`, compare each token's kind and text). Copy
   the scripts to `<scratchpad>\p8-repo-hygiene\before\` before the first edit.
4. Each workflow: the YAML data before and after are equal (PyYAML if it is installed, otherwise compare the files with
   comment lines removed). `dependabot.yml` parses.
5. `python -I site/check_page.py site` and `python -I site/check_page.py --final site`: the same outcome as before your
   edits (record both before and after).
6. Every relative link in the owned markdown files resolves to a file that stays.

## Out of scope

Code, every `Cargo.toml`, `rust-toolchain.toml`, `.cargo/` (`p8-repo-config`), `README.md`, `media/`, docs. The site's
visible text and numbers. Git.

## Questions for Haik

None: decide, and log each call in the Log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07: card written by `p8-plan`.

## Result
