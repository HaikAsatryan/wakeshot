# The repo's community files: contributing, security, conduct, templates

Phase: 6. Status: ready. Needs Haik: yes: he reads the files in the morning and decides the open items below.
Agent: `wakeshot-worker`. Size: about 60K tokens. Real laptop: no. Build folder: none (no Rust).
Runs after: none. Parallel with: `p6-readme` round 4 (owns `README.md` and links to these files by name) and
`p6-media` round 4 (owns `media/`).

Written by the orchestrator (overnight run, 2026-11-06, Haik asleep). Haik: "Our readme and repo should follow best
practices!" The repo has `LICENSE` (MIT) and `README.md`, but no contributing, security or conduct file and no issue
or PR templates.

## Goal

The usual GitHub community files, lean and in Haik's voice, so a stranger knows how to report a bug, a security
problem or send a PR, and GitHub's community profile shows them.

## Read first

- `CLAUDE.md` (the checks), `docs/plan.md` "How we work" (only to know what is internal), `README.md` top and
  "Credits and license".
- The house way: look in the sibling repos under `C:\Users\haika\code\` (PandaTechAM and Haik's own) for existing
  `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md` and `.github/ISSUE_TEMPLATE/`; port their shape if one exists.
  Read only, no git in them.
- GitHub's current docs on community health files, issue forms (YAML) and private vulnerability reporting.

## Deliverables

1. `CONTRIBUTING.md`: PRs are welcome; Haik reviews the architecture (his words: "I welcome PRs, and only will review
   architecture"); open an issue first for a big change; how to build and run the checks (the commands in
   `CLAUDE.md`, x64 and ARM64; never set `RUSTFLAGS`); tests must pass; real power settings tests need care (say what
   in one or two lines, from `CLAUDE.md`). Short.
2. `SECURITY.md`: supported versions (the latest release), how to report privately (GitHub's private vulnerability
   reporting on this repo), what to expect. No email unless Haik already shows one publicly on his GitHub profile
   (check with `gh api users/HaikAsatryan`; read only).
3. `CODE_OF_CONDUCT.md`: the current Contributor Covenant version (check which is current on its own site), with the
   contact the same private route as `SECURITY.md` or the public profile contact; no new personal data.
4. `.github/ISSUE_TEMPLATE/`: a bug report form (Wakeshot version, Windows version and CPU, what holds, what you saw,
   what you expected, the log path `%LocalAppData%\Wakeshot\logs\wakeshot.log` with a note to remove anything
   private), a feature request form, and `config.yml` that points security reports to `SECURITY.md`.
5. `.github/PULL_REQUEST_TEMPLATE.md`: what and why, the checks run, short.
6. Plain short English, no em dashes, no emojis. Each file as short as it can be and still useful.

## Acceptance checks

- Every file renders on GitHub as intended (issue forms: valid YAML, checked with a parser; field ids unique).
- Every command in `CONTRIBUTING.md` is the same as in `CLAUDE.md`.
- No personal data beyond what Haik already shows publicly; no AI mention; no Nightcrew mention.
- Lines ASCII only.

## Out of scope

- `.github/FUNDING.yml` (waits for Haik's donation account choice).
- `dependabot.yml` and other automation that opens PRs or issues on its own (list it as a suggestion only).
- Any repo setting (private vulnerability reporting must be turned on by Haik: list it in his actions).
- `README.md`, `site/`, `media/`, the workflows.

## Questions for Haik

Each with the worker's default.

## Found outside scope

## Log

## Result
