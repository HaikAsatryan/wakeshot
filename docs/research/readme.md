# What makes a README people read and act on

Research for the README, 2026-11-06 (Yerevan time, UTC+4): a README that leads with what Wakeshot does and how easy
it is to install, then everything else. This file holds the sources, the rules they lead to and the facts behind the
install lines. Marks as in `launch.md`: **verified** (read at the link on the date given), **reported** (a third party
says so; not checked by us), **not verified**.

## Sources

Read 2026-11-06, 10:00 to 10:45, as fetched page text or raw Markdown (`gh api repos/<o>/<r>/readme`).

| Source | What it says that matters here |
|---|---|
| GitHub Docs, [About READMEs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes) | A README tells "What the project does", "Why the project is useful", "How users can get started with the project", "Where users can get help", "Who maintains and contributes". GitHub builds an outline from the headings. "Relative links are easier for users who clone your repository." Content past 500 KiB is cut. **Verified** |
| GitHub Docs, [Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax) | Alerts (`> [!NOTE]` and four more): "Use alerts only when they are crucial for user success", "limit them to one or two per article", never two in a row. `<picture>` is supported. **Verified** |
| [Make a README](https://www.makeareadme.com/) | Sections: name, description, visuals ("screenshots or even a video"), installation (assume the reader "is a novice"), usage, support, roadmap, contributing, license, project status. "while a README can be too long and detailed, too long is better than too short." **Verified** (page source) |
| [standard-readme spec](https://github.com/RichardLitt/standard-readme/blob/main/spec.md) | Order: title, banner, short description (under 120 characters, on its own line), ..., install ("Code block illustrating how to install"), usage, ..., contributing, license last. **Verified** |
| Art of README (Kira, `hackergrrl/art-of-readme`; the repo is gone, read in the [Wayback Machine copy of 2023-12-31](https://web.archive.org/web/20231231175007/https://github.com/hackergrrl/art-of-readme)) | "Cognitive funneling": "the widest end contains the broadest more pertinent details", deeper parts are for readers "interested enough"; "The ideal README is as short as it can be without being any shorter"; its checklist asks for "Caveats and limitations mentioned up-front" and that a README "Doesn't rely on images to relay critical information". **Verified** (archived copy) |
| Nielsen, [How Users Read on the Web](https://www.nngroup.com/articles/how-users-read-on-the-web/) (1997) | "79 percent of our test users always scanned any new page they came across; only 16 percent read word-by-word." Advice: plain headings, lists, one idea per paragraph, lead with the conclusion, half the words. **Verified**. Old, but the scanning finding is the basis of NN/g's later work (not read here) |
| [Open Source Survey 2017](https://opensourcesurvey.org/2017/) (GitHub) | "Incomplete or outdated documentation is a pervasive problem, observed by 93% of respondents"; "64% say an open source license is very important in deciding whether to use a project". **Verified** |
| Prana et al., [Categorizing the Content of GitHub README Files](https://arxiv.org/abs/1802.06997) (2018) | 4,226 sections from 393 random repos: the "What" and "How" are common; "many README files lack information regarding the purpose and status of a repository." **Verified** (abstract) |

READMEs of loved small desktop utilities, read for what works (not copied). Stars from `gh api`, 2026-11-06:

| README | Stars | What it does first |
|---|---|---|
| [KeepingYouAwake](https://github.com/newmarcel/KeepingYouAwake) (macOS keep awake) | 7,034 | One sentence, then Install: one download link and one `brew` line, then one picture. Then plain questions as headings: "Does it work on a MacBook with a closed lid?", "Is this dangerous?". A short story of why the author built it ("I used it for many years ... I decided to build a replacement for myself"). 45 lines |
| [Stats](https://github.com/exelban/stats) (macOS) | 43,161 | Icon, two pictures, one line, then Installation (download, `brew install stats`). FAQ headings below |
| [Ice](https://github.com/jordanbaird/Ice) (macOS) | 29,972 | Icon, one line, banner, one `[!NOTE]` on project status, then Install (download, `brew`) |
| [EarTrumpet](https://github.com/File-New-Project/EarTrumpet) (Windows tray) | 11,523 | Install is a Store link plus `winget install File-New-Project.EarTrumpet` on one line; awards and media first |
| [TranslucentTB](https://github.com/TranslucentTB/TranslucentTB) (Windows) | 20,733 | One line with lightness in a few words: "A lightweight (uses a few MB of RAM and almost no CPU) utility"; then features, pictures, download |
| [Rectangle](https://github.com/rxhanson/Rectangle) (macOS) | 30,248 | Requirements, Installation, How to use; known issues, preferences storage and uninstall near the end |

Also looked at, less useful here: LocalSend, Monitorian, AltTab (a 7-line README that points to a website), PowerToys
(Install in folded `<details>` sections, "open" for the first).

## Rules, with the reason for each

1. **First lines: what it does, for whom, in plain words.** GitHub's own list starts with "what" and "why"; NN/g:
   lead with the conclusion, since most readers scan. One sentence, then one line of what holds the PC awake.
2. **Install in the first screen, as one line to copy.** Every utility above puts Install right after its one-liner
   (KeepingYouAwake, Stats, Ice); standard-readme asks for a code block. For Wakeshot that is
   `winget install HaikAsatryan.Wakeshot`, then the `Setup.exe` download as the second way.
3. **Say the status where the reader acts.** Prana et al.: READMEs often lack it; Make a README puts status at the
   top; GitHub allows one or two alerts. One `[!NOTE]` above the install line while nothing is released.
4. **Show it working early, but never only in pictures.** Make a README: visuals. Art of README: no critical fact
   only in an image. The video sits right under the one-liner; every picture has alt text and the words around it
   say the same.
5. **Funnel from broad to deep.** Art of README: broad first, details for those who read on. Order: what, install,
   what holds the PC awake, how it behaves, what you see, how it compares, the story, then the folded details.
6. **Headings say what is under them; one idea per paragraph; lists.** NN/g. GitHub builds an outline from the
   headings, so each folded section keeps a visible heading above its `<details>`.
7. **Keep the long reference, folded.** Make a README: "too long is better than too short". The power settings,
   limits and uninstall facts are a promise of this project (`architecture.md` decision 41), so they stay in the
   README, each in a `<details>` with a plain title.
8. **Name the limits plainly, near what they limit.** Art of README asks for caveats up front; KeepingYouAwake
   answers "Is this dangerous?" in its own heading. The top says what Wakeshot changes in one line; the full list
   stays below.
9. **Fewer links, and relative where possible.** GitHub: relative links work in clones. A link per table cell made
   the old table heavy: each rival is linked once, in the table head, and the cell sources live in `launch.md`.
10. **A small comparison, low on the page.** No source above asks for one at the top; readers decide on "what" and
    "how", and a big table pushes install down. Five rows, after the reader knows what Wakeshot does.
11. **The story is short and true, in the author's voice.** KeepingYouAwake's "Inspired by Caffeine" does it in four
    sentences. Ours keeps the author's words, with every claim about another tool checked (`docs/research.md`, "Story
    checks").
12. **License and how to help, at the end.** standard-readme puts license last; the 2017 survey says users weigh
    the license, so "MIT" also sits in the install line.

## Facts checked for the new install lines

- **winget picks the CPU's installer.** The manifest has one installer per CPU, `x64` and `arm64`
  (`packaging/winget/templates/HaikAsatryan.Wakeshot.installer.yaml`). Microsoft's `install` page (updated
  2026-07-19) says `show` lists "the installer selected by WinGet for your system", and `--architecture` picks one by
  hand. **Verified** (manifest and docs). How winget ranks x64 against arm64 on an Arm PC: **not verified**.
- **A winget install does not start Wakeshot.** winget's default runs the installer with progress (the `install`
  page: "The default experience shows installer progress"; in manifest terms `silentWithProgress`), which the
  manifest maps to `--silent`; a silent `Setup.exe` does not start the app
  (`docs/research.md`, "Install end to end checks", "Nothing starts the tray": Velopack's `install.rs` and a test;
  and the release install check in the same file). The manifest's `InstallationNotes` say so. **Verified**
  (manifest, docs, source and the earlier test); a real winget install: not tested yet.
- **The winget package and the first release did not exist on 2026-11-06.** `gh release list` empty at 10:40; the
  first winget submission is a hand step (`packaging/winget/README.md`). **Verified**.
- **A `[!NOTE]` alert renders in a repo README.** Ice's README, rendered as GitHub shows it
  (`gh api repos/jordanbaird/Ice/readme -H "Accept: application/vnd.github.html"`), holds `markdown-alert-note`; our
  README through `gh api markdown` in `gfm` mode does too (the `markdown` mode drops it). **Verified** 2026-11-06.
- **The Claude desktop app's Keep computer awake.** Anthropic's docs: a setting of the desktop app, "To prevent
  idle-sleep", and "Closing the laptop lid still puts it to sleep"; nothing on its scope
  (https://code.claude.com/docs/en/desktop-scheduled-tasks, raw page read 2026-11-06). A user's log in
  anthropics/claude-code#94420 (open, 2026-09-15) names it "Keep computer awake while Claude works" and shows "a
  keep-awake hold per Code turn" of the desktop app. **Reported** for the scope; the README says it holds for the
  app's own sessions, and that the terminal CLI holds nothing on Windows (**verified**, `docs/research.md`, "Story
  checks").
