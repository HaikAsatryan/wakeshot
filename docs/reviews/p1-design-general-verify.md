# Phase 1 design: verify the general review fixes

Reviewer: `wakeshot-reviewer`, 2026-11-03 about 16:00. Read only; no git, no registry or power writes, no screen.
Scope: the 18 findings of `p1-design-general.md` as handled by `p1-design-review-fixes`, the git steps, the three
build cards, and new choices that are Haik's. The power review fixes are left to the other verifier.

Counts: 0 blocker, 1 major, 9 minor.

## Answer for the orchestrator (point 4)

- **No open question (9 to 12) changes what `p1-repo-files`, `p1-cargo-workspace` or `p1-ci` build.** All three
  run before `git init`, so Question 9 (worktrees) cannot apply to them. Questions 10 to 12 are Phase 2 and 3 design.
  The `.claude/worktrees/` ignore line is right whatever Haik answers to Question 9. Nothing else that is still open
  (menu look, pack id dot, cron crate) touches the skeleton.
- **One choice was made without asking Haik:** the first public push no longer waits for his yes (major 1 below). It
  touches only the git steps, which come after the build cards. So the build cards can start now. `git init` and the
  push must wait for Haik's real sign-off, as his 15:45 note says ("local only: no repo, no push").
- Two smaller items for the sign-off list, no decision needed: network names in the new tray menu pictures (minor 6)
  and the chime on unplug with the lid closed, which `product.md` does not list (minor 7).

## Each finding

"Closed": the risk is gone. "Closed, with a note": closed, plus a minor finding below.

| # | Card says | Verdict | Checked at |
|---|---|---|---|
| B1 | Changed | Closed, with notes (minor 1, 3). The scan reads the staged blobs; nothing is committed before it; Gate C needs a clean status and an index equal to the saved `git ls-files -s`. A clean status means HEAD, index and tree match, so the push sends exactly the scanned tree | phase `:168-199` |
| M1 | Changed | Closed for Phase 1: no repo exists until step 4, and only a secret fix card can run after it. `plan.md:48` and `SKILL.md:21` still say worktrees (minor 8) | phase `:107-113`, `:163` |
| M2 | Fixed | Fixed. The fixture log is public; it is not in the ignore list; section 9 says so | `p1-repo-files.md:31-37`; architecture `:723-725` |
| M3 | Changed | Closed. Under answer 1 only secrets matter, and a grep scan fits any budget | dropped cards' headers |
| M4 | Fixed | Fixed: step 4 checks the identity, step 7 checks the commit | phase `:172-173`, `:192-193`, `:100-101` |
| m1 | Changed | Closed. The reason is true: Cargo dependencies belong to a package, not to one bin | architecture `:37-41` |
| m2 | Fixed | Fixed | architecture `:609-610` |
| m3 | Fixed | Fixed | architecture `:373-374`, `:694` |
| m4 | Fixed | Fixed; the summary's "only through small JSON files" now holds | architecture `:341-343`, `:261` |
| m5 | Fixed | Fixed. Every value the review named is in "Values I set"; Question 5 has the Never trade-off | phase `:55-80`, `:92-94` |
| m6 | Changed | Closed: lock at once, `SoundPlayer::play` never blocks. `p1-lid-sound.md:177-178` (done card) still says the lock waits (minor 8) | architecture `:91`, `:231-235` |
| m7 | Fixed | Fixed: types defined, `ReadPlan`, exit waits on the watch thread, data folder watch named | architecture `:153`, `:162-194`, `:352` |
| m8 | Fixed | Fixed: rows 11 and 32 say phases 2, 4, 5 | architecture `:282`, `:303` |
| m9 | Fixed | Fixed | architecture `:588-591` |
| m10 | Fixed | Fixed | `p1-repo-files.md:33-35` |
| m11 | Changed | Closed, with a note (minor 5) | `p1-cargo-workspace.md:9-12`; architecture `:768-770` |
| m12 | Fixed | Fixed in the phase file; the `SKILL.md:33` pull rule is unchanged (minor 8) | phase `:168-171` |
| m13 | Fixed in part | Fix row and parallel lines: fixed. Hook: fixed, with gaps (minor 4). Crop: not applied (minor 6) | phase `:141`, `:176-183` |

Own checks, read only, 2026-11-03 about 15:58:

- The ten scan patterns and the generic key pattern, run with GNU `grep -rnIE` over the tree without `target/`, hit
  only the three known fine files the phase file lists (31 lines in `live-tests.log`, `watch.rs:523`, the manifest's
  `publicKeyToken`). No unredacted `session_` id or account UUID field anywhere.
- No `.key`, `.pem`, `.env`, credentials, exe or dll file, and no nested `.git` folder, outside `target/`. No text
  file with NUL bytes, so `-I` skips no text file today. 69 PNG files are not scanned.

## Major

### 1. The first public push no longer waits for Haik's yes

- Where: `phases/01-design-skeleton.md:162-163` ("Haik's 'go fully public' (15:29) is the go for the first push"),
  `:165` (Gate A), `:309-313` (work log 15:45). The old steps had Gate B: "Haik says yes to going public after a short
  summary ... Publishing cannot be undone" (the fix worker's copy, `phase-before.md:122-124`).
- What breaks: the fix removed Haik's last yes before an act that cannot be undone, and wrote it only in the steps'
  intro, not in the summary, "Values I set" or the questions he reads to sign off. Gate A reads as already met ("His
  answers ... came 15:29"). Haik's 15:45 ahead sign-off covers local build cards only ("no repo, no push"), and that
  limit lives only in the work log. Scenario: Haik is away past 17:30; the orchestrator is cleared, reads the steps,
  finishes the cards, the review and its fix card, and runs steps 3 to 9. The repo goes public before Haik has read
  the sign-off list. His reading of "go fully public" may well allow this, but it is his call (his rule: anything
  hard to undo).
- Fix: Gate A text: "Haik signs off after reading the sign-off list; the 15:45 ahead sign-off allows only the local
  build cards." Put one line in the sign-off list: "After the scan, the first push runs without asking you again,
  unless the scan finds something I cannot judge. Say if you want a last yes." No change to the build cards.

## Minor

### 1. The freeze covers cards, not the orchestrator's own writes or running processes

- Where: phase `:168-171` (freeze), `:189-191` (step 6: record fine hits "in the work log"), `:197-199` (Gate C);
  `SKILL.md:29` (work log and `plan.md` Status at every stop).
- What breaks: the work log is a repo file. Writing it after the step 5 scan makes Gate C fail ("back to step 5"); a
  second pass writes the log again and fails again. Each pass also makes another "Initial commit". The same happens
  with a running spike that logs into the repo: the orchestrator's away hold writes
  `spikes/power-settings/out/hold-limited-20261103-154319.log` (updated 15:52) and `out/journal.txt` until about
  18:13. Gate C catches every case, so nothing unscanned is pushed; the flow just loops.
- Fix: from step 3 to the push, the orchestrator writes notes only to its scratchpad and copies them into the work log
  after the push. No spike process that writes into the repo runs in that window. On a Gate C failure after step 7,
  re-scan and amend the unpushed commit (or name the second commit plainly).

### 2. From Phase 2 on, a secret can enter a local commit, and the steps have no way out

- Where: phase `:202-203` (step 11: scan before each push, commit after each card), `:189-191` (step 6 remedy);
  `p1-repo-files.md:36-37` ("Nothing else ... not by `.gitignore`").
- What breaks: commits come before any scan, so a `.key` file or a token in a doc can be committed. The pre-push scan
  of `origin/main..HEAD` then hits, and keeps hitting after a worker deletes it, because the old commit still holds
  it. That fails safe, but no step says how to get it out of an unpushed commit (a history rewrite, which needs
  Haik's go under his git rule). If the orchestrator instead re-runs the step 5 index scan, as step 6 says, the scan
  passes while the secret is still in an unpushed commit, and the push publishes it. For the first push the brief's
  "never staged" does not hold either: `git add -A` stages a `.key` file; the scan stops it before the commit, so it
  never leaves the PC.
- Fix: run the step 5 scan (paths and `git grep --cached`) before every commit, not only before the push. Write the
  remedy for a hit in an unpushed commit. Optional, cheap: in `p1-repo-files`, also ignore `*.key`, `*.pem`, `*.pfx`,
  `*.p12`, `.env*`, `id_rsa*` and `id_ed25519*` (comment: secrets; the scan still runs). Not Haik's call: answer 1
  keeps secrets out.

### 3. The scan has no check that it works

- Where: phase `:188` ("Not run yet by anyone: the first run is its test"), `:231-234` (known fine hits).
- What breaks: a `git grep` that matches nothing because of a wrong flag, a bad pattern file or a regex difference
  looks the same as a clean tree. Then a real secret passes.
- Fix: step 5 must reproduce the known fine hits (`live-tests.log`, `watch.rs`, the manifest). Zero hits means the
  scan is broken: stop.

### 4. The commit-msg hook misses two forms

- Where: phase `:179`.
- What breaks (tested with `grep -qiE` and the hook's pattern on sample messages): a bare
  `https://claude.ai/code/session_...` line passes; so does a `Claude-Session:` trailer with leading spaces. The
  harness adds the bare link form to pull requests today; a future commit form may match it.
- Fix: `'^[[:space:]]*(co-authored-by|claude-session):|generated with|anthropic\.com|claude\.ai/code'`.

### 5. The toolchain pin makes a parallel spike build download a toolchain

- Where: `p1-cargo-workspace.md:9-12`, `:37-38`.
- What breaks: only `stable-x86_64-pc-windows-msvc` is installed (`rustup toolchain list`, 15:58). rustup treats
  channel `1.99.1` as a different toolchain, even though stable is 1.99.1. Once `rust-toolchain.toml` lands, the next
  cargo call under the repo, including a `p1-tray-menu-look` spike build, needs that toolchain. rustup either fails or
  downloads it inside that worker's build. Two workers may start the same install at once; whether rustup 1.29.2 is
  safe with that: not verified.
- Fix: in the card, run `rustup toolchain install 1.99.1 --component rustfmt clippy --target x86_64-pc-windows-msvc
  aarch64-pc-windows-msvc` before writing `rust-toolchain.toml`, and log the time.

### 6. New tray menu pictures have no crop step; the phase file says they do

- Where: phase `:235-236` ("the menu card already has a crop step"); `p1-tray-menu-look.md:43-46` (deliverable 1:
  Wi-Fi quick settings flyout, Chrome, Telegram) vs `:73-77` (deliverable 7, the only crop step, for three old
  pictures).
- What breaks: images are not scanned. The Wi-Fi flyout lists nearby network names, which hint at where Haik lives.
  These are not secrets under answer 1, but the phase file's claim is wrong, and one card crops personal content in
  old pictures but not in new ones.
- Fix: the orchestrator adds one line to deliverable 1: crop network and account names. Cheap and safe, so no Haik
  decision needed; tell him in the sign-off list.

### 7. `product.md` says no sound when AC is lost; the design plays the chime then

- Where: `product.md:83-85` ("No sound for: ... AC lost or back"); architecture `:236-239`; summary phase `:33-35`.
- What breaks: unplugging a closed, docked laptop whose own AC lid action is Do nothing makes Wakeshot's DC Do nothing
  take effect; the design then locks and plays the lid chime. The summary tells Haik, `product.md` contradicts it.
- Fix: in `product.md`: "AC lost or back, except when it makes a closed laptop keep running (then as a lid close)".
  Default as designed; one line in the sign-off list.

### 8. Stale lines left

- `plan.md:48` and `SKILL.md:21`: worktree isolation once the repo exists, against the Question 9 default.
  `SKILL.md:33`: `git pull --rebase` and "no upstream: abort", against step 3. `product.md:3`: "Phase 0 (facts)
  running". `CLAUDE.md` status. All in the fix card's "Found outside scope". Not listed there:
  `p1-lid-sound.md:177-178` says the lock waits until the stream runs.
- No harm in Phase 1: no repo exists while the build cards run, and the pull rule only stalls the orchestrator. Fix
  after Haik answers Question 9, before Phase 2.

### 9. "The README says so" has no owner

- Where: `product.md:125`, `:177`; phase `:92-93` (Question 5), `:116-117` (Question 10), `:121-122` (Question 11);
  architecture decision 9 (credit lines).
- What breaks: the crash gap and its paths, one user per PC, `winget upgrade` killing `wakeshot run`, and credit lines
  for ported code are all promised for the README. `p1-repo-files` rightly writes a stub without them, and `plan.md`
  Phase 6 says only "README". They can be forgotten.
- Fix: one line in `plan.md` Phase 6 or architecture decision 41 listing what the v1 README must say. No change to
  `p1-repo-files`.

## Build cards (point 3)

- Buildable: every file and section they name exists (`research.md` "Toolchain", "Crates", "Manifest needed",
  "Distribution", "Phase 1 design checks"; `research/distribution.md` section 4; architecture sections 2, 10, 11;
  the tray spike's `build.rs` and manifest). Each has concrete checks.
- Size: 60K, 120K and 80K look right for the reads, builds and outputs they ask for.
- Order: "Runs after" and "Parallel with" agree between the three cards, the phase table and `p1-tray-menu-look`
  ("any Phase 1 card that does not use the screen or sound"). No two write the same file; only `p1-cargo-workspace`
  writes `CLAUDE.md`. The one shared effect is the root toolchain and CRT on spike builds (minor 5).

Verdict: the general review is closed; the build cards can start now, but `git init` and the first push must wait for
Haik's own sign-off, which should name the dropped last yes (major 1).
