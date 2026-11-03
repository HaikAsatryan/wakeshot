# Phase 1 design: general review

Reviewer: `wakeshot-reviewer`, 2026-11-03. Read only. Scope: `docs/architecture.md`, the summary, questions and git
steps in `docs/phases/01-design-skeleton.md`, and the cards `p1-repo-files`, `p1-cargo-workspace`, `p1-ci`,
`p1-privacy-check`, `p1-privacy-fixes`, against `docs/product.md` and `docs/plan.md`. Restore paths, hold facts and
fact sources are left to the power reviewer (`p1-design-power.md`).

Counts: 1 blocker, 4 major, 13 minor.

Spot checks that passed (crates.io API, 2026-11-03): `windows` 0.62.2 is the newest (2025-10-06, MSRV 1.82);
`windows-core` is at 0.100.0 (2026-09-03) while `windows` is not; `velopack` newest stable 1.2.161, newest overall
1.3.39-g8d2844f (pre-release). The architecture's version table matches.

## Blocker

### B1. Files can change after the privacy check and still go public

- Where: `phases/01-design-skeleton.md:119-131` (steps 4 to 8); card table rows `p1-lid-sound`, `p1-tray-menu-look`.
- What fails: the privacy check reads the tree at step 4. Nothing stops other cards from writing to the tree between
  then and `git add -A` at step 6. `p1-privacy-fixes` asks for `p1-lid-sound` and `p1-tray-menu-look` to be "done or
  paused" only while it runs. Gate C compares only the file count, not names or content. Example: Haik does his two
  lid closes, the lid sound worker resumes and writes its results to `docs/research.md` (its card log already holds
  an audio endpoint ID, a class 2 item in the privacy card). The file count stays the same, Gate C passes, and
  unchecked text is published. A swap (one file removed, one added) also passes. Publishing cannot be undone, and the
  squash before launch does not take anything back from GitHub or from clones.
- Fix: check exactly what gets committed. After the privacy fixes, `git add -A`, save `git ls-files -s` (path plus
  blob hash) to the scratchpad, and run the privacy re-check on that staged state. At Gate C, `git status --porcelain`
  must be empty and `git ls-files -s` must equal the saved list. Also write in the steps: no card runs from step 3
  until the push, and a paused card stays paused until then.

## Major

### M1. Worktree isolation does not fit this repo's git flow

- Where: `plan.md:46`; `.claude/skills/orchestrate/SKILL.md:21-22`; `phases/01-design-skeleton.md:115-131`.
- What fails (Claude Code docs, "Run parallel sessions with worktrees", read 2026-11-03):
  1. Between `git init` (step 3) and the first commit (step 6) there is no commit to branch from. The privacy
     reviewer, the fixes worker and the re-check reviewer run in exactly this window. The rule says give them
     worktrees. They would fail or see an empty checkout (exact error not tested).
  2. A worktree holds only tracked files. Anything `.gitignore` keeps local is missing there, including the Claude
     fixture log the design needs (M2) and every `out/` log the docs cite as evidence.
  3. Subagent worktrees branch from the remote default branch unless `worktree.baseRef` is `"head"`. The plan
     commits after each card but pushes only after a phase review, so a worktree agent in Phase 2 starts from the
     last push and misses the cards committed since.
  4. No step says how a worker's edits in its worktree get back to the main checkout. Workers never run git, so
     their changes sit uncommitted in `.claude/worktrees/<name>/`.
- Fix: in the phase file, say no worktree isolation until the first push (no agent runs git, and the Phase 1 cards
  write disjoint files). Before Phase 2, pick one: set `worktree.baseRef` to `"head"` in the project settings and
  add a merge-back step to the skill, or keep worktrees for read-only reviewers only. Add `.worktreeinclude` for any
  ignored file agents must read.

### M2. Question 1 keeps the Claude fixture log local, but the design builds parser tests from it

- Where: `phases/01-design-skeleton.md:55-59`; `architecture.md:578-579`; `p1-repo-files.md:29-31`;
  `spikes/claude-watch/fixtures/README.md:10` and its "Where each shape is" table.
- What fails: Question 1 calls `spikes/claude-watch/fixtures/live-tests.log` a raw tool log and ignores it. It is
  the redacted fixture: the README says it was cut to 401 lines and redacted, and maps every Claude shape to its line
  numbers. Section 9 builds the Phase 2 and 4 parser fixtures "from `spikes/claude-watch/fixtures/`". If it is
  ignored, the public README points at a missing file, and any worktree agent (M1) cannot read it.
- Fix: in Question 1, list it with the redacted fixtures that go public after the privacy check. Remove it from the
  `p1-repo-files` ignore list. If Haik wants it local, change section 9 to say where Phase 2 gets the shapes.

### M3. The privacy check card does not fit one reviewer

- Where: `p1-privacy-check.md:12-13` (Goal: "read every file git would add"), `:41-43` (Method: grep, then read
  fixtures, logs and images).
- What fails: by the default answer to Question 1 the public list is about 109 text files, 1.36 MB (docs about
  900 KB: tasks 290 KB, reviews 200 KB, research 190 KB, phases 70 KB, top docs 150 KB; spikes 450 KB), plus 33
  images and the new tray menu pictures. That is roughly 350K to 400K tokens to read, more than twice the 150K budget.
  The Goal says read every file; the Method says grep most of them. Classes 3 and 4 (screenshots, project or customer
  names, transcript lines) cannot be found by grep alone.
- Fix: split into three reviewers with disjoint lists: `docs/` text; `spikes/` and `.claude/` and root files; all
  images. Each writes its own section of `docs/reviews/p1-privacy.md` (or its own file). Make the Goal say: grep every
  file, read in full the files in high-risk groups (task cards, reviews, research, fixtures, logs, images).

### M4. Every public commit will carry a personal Gmail address, and nobody asks Haik

- Where: `phases/01-design-skeleton.md:115-116` (step 3 checks only that the name is Haik's), `:124-125`, `:129-130`.
- What fails: `~/.gitconfig` sets `user.name = HaikAsatryan` and a `@gmail.com` address. Every commit pushed to the
  public repo shows that address, for good. Haik's rule is "nothing personal published". This may be fine with him,
  but it is his call, and Gate B does not show it.
- Fix: add to Gate B: show the author name and email that the first commit will use. Offer the GitHub noreply
  address as the other option (`git config user.email <id>+HaikAsatryan@users.noreply.github.com`, repo-local).

## Minor

### m1. The reason for two bin crates is wrong

- Where: `architecture.md:36-37`.
- What fails: it says each exe needs its own resources "from its own `build.rs`". `embed-resource` 3.0.12 has
  `compile_for`, "Likewise, but only for select binaries" (docs.rs), so one package with two `[[bin]]` targets can
  embed a different `.rc` file in each.
- Fix: either one bin package with two bins (one `Cargo.toml` and one `build.rs` fewer), or keep two crates and give
  a true reason. Change it when the two exes need different dependencies.

### m2. The HKCU journal copy is more than the product asks for

- Where: `architecture.md:368-369`, `:459`, `:487`, `:493`.
- What fails: `product.md` asks for the original "(atomic file)". The registry copy adds a second store, a `seq`
  merge rule, a corrupt-file branch and uninstall cleanup. Atomic write with write-through already covers a torn write.
- Fix: file only. Change it when a lost or corrupt journal shows up in a test or a report. (Restore safety itself is
  the power reviewer's.)

### m3. The panic hook restore duplicates the guard and breaks the thread rule

- Where: `architecture.md:311`, `:554`; thread table `:290-293`.
- What fails: the thread table says only the IO thread writes power settings. A panic hook runs on whatever thread
  panicked, possibly while the IO thread is mid-write. The guard already restores when the tray dies, and abort
  follows at once.
- Fix: the hook logs and aborts; the guard restores. Change it when a test shows the guard missing a panic exit.

### m4. `wakeshot restore` with a tray running is a second settings writer

- Where: `architecture.md:285`; summary `phases/01-design-skeleton.md:14-16` ("They talk only through small JSON
  files").
- What fails: the CLI writes power settings while the tray owns them, and the tray learns about it up to 10 s later
  as a user change. The user then gets the "you changed a setting" notice per key, not the "Restore my power
  settings" notice, and a tray apply or plan switch in that window races with it. It also contradicts the summary.
- Fix: with a tray running, the CLI writes a request file (or a field in `config.json`) and the tray runs the same
  path as the menu. With no tray running, the CLI runs the whole routine.

### m5. Decisions the worker made that Haik does not see as questions

- Where: `p1-detailed-design.md:162-163`; `architecture.md:185`, `:272-275`, `:180-183`, `:346-352`;
  `phases/01-design-skeleton.md:70-72`.
- What fails: Haik signs off from the phase file. These are not in it:
  - Design values listed only in the card's Result: Claude timer margin 2 minutes, safety tick 30 s, 4 s wait at
    sign-out, log 1 MB times three, reason cut at 60 characters. The margin is user-visible.
  - Battery floor "at or under" (design) vs "below" (`product.md`).
  - A second signed-in user gets requests only (no lid feature), because of the machine-wide settings mutex.
  - The lid lock waits for the sound to end, so a closed laptop stays unlocked about 1 s longer.
  - Battery sleep Never: `product.md` still says "Never, or a value Settings cannot set" is open for Phase 1. The
    Phase 0 close recommended Never and the summary states it, but Question 5 does not say that Never is what makes
    the crash gap drain the battery at the sign-in screen.
- Fix: add one short "Values I set" list to the summary, and add the Never trade-off to Question 5.

### m6. When the lid lock fires: design and lid sound card disagree

- Where: `architecture.md:88`, `:181` (lock "when the sound reports done", `Done(Played)` "when the sound ends");
  `p1-lid-sound.md:163-164` ("the lock waits until the stream runs (at most 1 s)").
- What fails: start vs end of the sound gives a different lock delay and a different `SoundPlayer` contract.
- Fix: say "lock when the sound has started, or the cap passes"; `p1-lid-sound` sets the cap. Change it when a lid
  test shows the lock cuts a started sound.

### m7. Some boundary types and owners are too vague to build from

- Where: `architecture.md:124`, `:133`, `:126`, `:294-295`, `:360-362`, `:283`.
- What fails: `Outcome` (the type every fallback rests on), `HoldFacts`, `Notice` variants and `PlanValues` are named
  only. No command asks the IO thread for the 10 s or per-tick plan reads, yet the loop owns the timers. Process exit
  waits sit in both the watch thread and the system callbacks rows. Nothing says which thread watches `config.json`
  to produce `ConfigChanged`.
- Fix: define `Outcome` and `HoldFacts` fields; say the IO thread keeps its own 10 s timer, or add a `ReadPlan`
  command; give exit waits one owner; name the data folder watch.

### m8. Keep awake phase differs from `plan.md`

- Where: `architecture.md:226` and `:247` (phases 2, 5); `plan.md:63` (Phase 4 lists Keep awake).
- Fix: use the same phase in both.

### m9. A hold file without `procStart` can hold for a stranger

- Where: `architecture.md:469-471`.
- What fails: Wakeshot reads the start time when it first sees the file. If the tray was not running when the
  writer died and the pid was reused, the new process is taken as the writer, and the file holds while it lives.
- Fix: count the process only if it started before the file's last write time. No new field needed.

### m10. `.gitignore` misses Claude Code's worktree folder

- Where: `p1-repo-files.md:29-32` ("Nothing else").
- What fails: Claude Code puts worktrees in `.claude/worktrees/<name>/` inside the repo, and its docs say to ignore
  that folder. `.claude/` goes public by default, so a later `git add -A` picks up any worktree left on disk as an
  embedded repository.
- Fix: add `.claude/worktrees/` and `.claude/settings.local.json` (per-person permission rules), each with a
  comment why.

### m11. Root Cargo config also changes the spike builds of the parallel cards

- Where: `p1-cargo-workspace.md:4-7`, `:31-33`.
- What fails: cargo reads `.cargo/config.toml` and rustup reads `rust-toolchain.toml` from parent folders. Once they
  exist at the root, every spike build (`p1-lid-sound`, `p1-tray-menu-look`) gets `+crt-static` and a full rebuild.
  The card handles `exclude` but not this. Low risk, but spike sizes and behavior then differ from what Phase 0
  recorded.
- Fix: one line in the card and in the spikes' READMEs: spikes now build with the static CRT.

### m12. Git step order and the pull rule

- Where: `phases/01-design-skeleton.md:114-118`; `SKILL.md:33-34`.
- What fails: step 3 starts "Once `.gitignore` exists", which can be read as right after `p1-repo-files`, before the
  workspace, CI and general review. From `git init` until the push the branch has no upstream, so every orchestrator
  run hits "no upstream: abort and tell Haik".
- Fix: say step 3 runs after step 2 ends. Tell the orchestrator to skip `git pull --rebase` until step 9.

### m13. Card lines and gates that do not line up

- Where: `phases/01-design-skeleton.md:98`, `:120-122`; `p1-tray-menu-look.md:5-7`; `p1-privacy-fixes.md:6-7`;
  `p1-tray-menu-look.md:43-46`.
- What fails:
  - Gate B needs the general review fixed, but no row or card does the fixes.
  - `p1-tray-menu-look` allows any card that does not use the screen or sound in parallel; `p1-privacy-fixes`
    forbids it. The skill follows "Parallel with" lines, so they must agree.
  - Later commits (step 11) have no check against AI lines. This reviewer's own session got a harness instruction to
    end commit messages with a `Claude-Session:` line, although `~/.claude/settings.json` sets `attribution.commit`
    to empty. Gate C checks only the first commit.
  - Outside the cards under review: `p1-tray-menu-look` pictures the Wi-Fi quick settings flyout, which lists
    nearby network names (a privacy class), and says nothing about cropping them.
- Fix: add a fix row for the general review; make the two parallel lines agree; add a local `commit-msg` hook in
  `.git/hooks` (not committed) that rejects `Claude`, `Co-Authored-By` and `Generated with`; add a crop note to
  `p1-tray-menu-look` deliverable 1.

## Checked and fine

- Every `product.md` decision maps to a row in the table (rows 0 to 44); none is missing.
- `exclude = ["spikes"]` in the first write of the root `Cargo.toml` protects parallel spike builds; the
  `cargo metadata` check proves it.
- `p1-cargo-workspace` and `p1-ci` fit one worker each; their acceptance checks are concrete (PE machine field,
  `dumpbin`, `cargo tree`).
- The `p1-ci` action pinning rule and the read-only token are sound. `upload-artifact` needs no extra permission.
