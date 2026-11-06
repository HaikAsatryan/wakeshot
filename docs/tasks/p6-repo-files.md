# The repo's community files: contributing, security, conduct, templates

Phase: 6. Status: done. Needs Haik: yes: he reads the files in the morning and decides the open items below.
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
- No personal data beyond what Haik already shows publicly; no AI mention; no mention of the product Haik keeps out of the repo.
- Lines ASCII only.

## Out of scope

- `.github/FUNDING.yml` (waits for Haik's donation account choice).
- `dependabot.yml` and other automation that opens PRs or issues on its own (list it as a suggestion only).
- Any repo setting (private vulnerability reporting must be turned on by Haik: list it in his actions).
- `README.md`, `site/`, `media/`, the workflows.

## Questions for Haik

Each with the worker's default. None blocks; the files ship with the defaults.

1. Reply time. `SECURITY.md` says "I aim to answer within 7 days". Default: keep it. Change the number or drop the
   promise if a week is too tight.
2. Conduct reports. Your profile shows no email, so `CODE_OF_CONDUCT.md` sends conduct reports through the private
   vulnerability form (title starting "Conduct:") or GitHub's own "Report content". Default: keep. A public contact
   email would be the cleaner route, if you want one.
3. The enforcement ladder. The Covenant's four steps (warning, limited activity, suspension, ban) stay as your policy.
   Default: keep them.

## Found outside scope

- CI's Test step runs `cargo test --workspace --locked --target <t>`; `CLAUDE.md` lists `cargo test --workspace
  --locked` without a target. Both work; the files here use the `CLAUDE.md` form. Harmless, noted only.
- The repo has Dependabot security updates off (`security_and_analysis`). A repo setting, so not changed here; see
  the suggestions in Result.

## Log

- 2026-11-06: started. Read the card, `CLAUDE.md`, `docs/plan.md` "How we work".
- 2026-11-06: house way. `C:\Users\haika\code` holds only `personal-utilities` (no community files). PandaTechAM
  public repos have none (community profile API on six repos; no `PandaTechAM/.github` repo). Haik's own public repos
  do (read with `gh api .../contents`, read only): `armenian-bank-financials` (CONTRIBUTING, YAML issue form with a
  markdown intro, `config.yml` with `blank_issues_enabled: true` and a contact link), `go-vs-dotnet-benchmark` (PR
  template: HTML comment, "What this changes", a checkbox list of checks). Ported that shape.
- 2026-11-06: `gh api users/HaikAsatryan`: `email` null, `blog` empty, name "Haik", location Yerevan. So no email
  anywhere. Repo: private vulnerability reporting `enabled: false`, Discussions off, 0 releases, community profile 42%.
- 2026-11-06: current docs read (GitHub issue forms syntax, form schema, validation errors, config.yml, community
  health files, private vulnerability reporting both sides; contributor-covenant.org). Contributor Covenant 3.0 is
  current; its markdown downloaded to the scratchpad (93 lines, two curly apostrophes, no em dashes, CC BY-SA 4.0).
- 2026-11-06: facts for the files, from the code: the log's start line is `Wakeshot <version> starts`
  (`tray_mode.rs`); `cargo test` skips every test that writes real power settings (all `#[ignore]`, on plan copies;
  `power_check` is an example with `required-features`, so its tests do not run). Labels `bug` and `enhancement`
  exist. `rustup toolchain install` is what CI runs to install the pin (CI green 2026-11-05).
- 2026-11-06: wrote the seven files. `CODE_OF_CONDUCT.md` made by a scratch script from the upstream markdown, so the
  diff against upstream shows only the allowed changes.
- 2026-11-06: checks run (scratch venv: PyYAML 6.0.3, jsonschema 4.26.0, SchemaStore `github-issue-forms.json` and
  `github-issue-config.json`), markdown rendered through GitHub's `POST /markdown` API, links fetched. Facts added to
  `docs/research.md`, "Community files: GitHub's rules and the repo's state".

## Result

**Done.** The repo now has the usual GitHub community files, in plain short English.

Files (all new):

- `CONTRIBUTING.md`: PRs welcome, Haik reviews the architecture, issue first for a big change; build setup (rustup,
  Build Tools, the ARM64 clang note); the five check commands; never `RUSTFLAGS`; tests must pass; a short paragraph
  on real power settings tests; MIT.
- `SECURITY.md`: latest release only; report through private vulnerability reporting (direct link to the form, and
  the "Security and quality" tab); what to send; what is most wanted; what to expect.
- `CODE_OF_CONDUCT.md`: Contributor Covenant 3.0, current on its site. Changes from upstream: the reporting line
  filled in, the note to adopters on the enforcement section removed, two curly apostrophes made straight, the blank
  first and last lines dropped. Upstream's long lines kept as they are.
- `.github/ISSUE_TEMPLATE/bug-report.yml`: version, Windows version, CPU (dropdown), what held, what you saw (with
  battery and lid), what you expected, log (path and a note to remove anything private). Label `bug`.
- `.github/ISSUE_TEMPLATE/feature-request.yml`: the problem, the idea; points at the README's Limits. Label
  `enhancement`.
- `.github/ISSUE_TEMPLATE/config.yml`: blank issues off; a "Security problem" link to the security policy page.
- `.github/PULL_REQUEST_TEMPLATE.md`: what and why; a checkbox per check; real power settings touched or not.

Also: `docs/research.md` (new section, additions only) and this card.

Checks:

| Check | Outcome |
|---|---|
| Issue forms valid YAML, ids unique | Pass. PyYAML parses all three; SchemaStore schemas pass; a script checks GitHub's documented rules (required keys, id characters, unique ids, unique and not too similar labels, a non-markdown field, no `None` option, string options, no `---`). Bug form ids: `version windows cpu holds saw expected log`; feature form: `problem idea`. |
| Renders on GitHub | Markdown: pass, through GitHub's own renderer (`gh api markdown`, gfm, repo context): headings, lists, the code block, 6 task list checkboxes, links as meant; outside links answer 200. Issue forms: GitHub has no render API, so the forms show for real only after the push (Haik's step 3). |
| Commands the same as `CLAUDE.md` | Pass. Every `cargo` command in `CONTRIBUTING.md` and the PR template matches `CLAUDE.md` character for character; the second clippy is its "same without `--all-targets`". Two setup commands are not in `CLAUDE.md`: `rustup toolchain install` (from `ci.yml`) and `powercfg /qh` (to note values before a real test). |
| No personal data, no AI or banned product mention | Pass. Only "Haik" and the repo name; no email. A word scan finds no AI, tool or banned product name. |
| ASCII only | Pass, every line of all seven files; also LF only, one final newline, no trailing spaces, no em dash. |

Decisions (mine, for Haik's review):

- Full Covenant text, not a short link file like `ResultCrafter`'s: readers and GitHub know the standard text, and
  the diff to upstream is tiny. Change it when Haik wants the short form.
- Blank issues off: the bug form asks for what a power bug needs (version, CPU, what held, log). Haik's
  `armenian-bank-financials` keeps them on because its one form covers one case. Change it when a real report fits
  neither form, or when Discussions turns on (then add a Questions link).
- The bug form says "an agent session", not the tool's name, to follow the brief's rule of no AI tools named. Change
  it when Haik wants the tool named.
- The Wakeshot version comes from the log's start line (`Wakeshot 0.1.0 starts`): it is in the code; that Settings >
  Apps shows the version is likely but not verified here.
- The feature form links `README.md#limits`: it breaks if `p6-readme` renames that heading (it was there at 04:20).

Haik's actions, in order:

1. Turn on private vulnerability reporting: repo Settings, sidebar "Security and quality", "Advanced Security", then
   "Enable" next to "Private vulnerability reporting". Until then the links in `SECURITY.md` and the conduct file
   lead nowhere useful.
2. Make sure a report reaches you: on the repo, Watch, then All Activity or Custom with "Security alerts".
3. After the push: open Issues, New issue, and check that both forms and the "Security problem" link show; open the
   "Report a vulnerability" link in `SECURITY.md` and check it opens the form; check Insights, Community standards.
4. Read the three questions above.

Suggestions only (out of scope): `.github/dependabot.yml` for weekly `cargo` and `github-actions` updates, and
turning on Dependabot security updates; both open PRs on their own. `.github/FUNDING.yml` waits for the donation
account.

Open items: the issue forms are checked only by schema and rules until GitHub shows them after the push.
