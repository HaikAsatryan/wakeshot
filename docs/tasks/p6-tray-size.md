# Why the release tray exe doubled, and whether it can shrink

Phase: 6. Status: done. Needs Haik: no (a dependency change waits for the orchestrator's go). Agent:
`wakeshot-worker`. Size: about 50K tokens. Written by the orchestrator (session `wakeshot-12`, 2026-11-04) from the
`p6-install-e2e` rerun (sizes) and docs pass round 4 "Found outside scope" 8.
Runs after: `p6-int-update-wiring` (done). Parallel with: stop 1 runs beside any card (read only, own target folder);
stop 2, if any, changes `Cargo.toml` and `Cargo.lock` and runs alone among cards that build.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-tray-size\target`.

## Goal

The release x64 tray exe grew from 2,026,496 bytes to 3,952,640 bytes when `p6-int-update-wiring` started using the
update feed (`url`, `idna`, `icu_*` and an HTTPS stack now built). Size is a selling point (the README compares it with
rivals). Find what adds the bytes, and whether a smaller build keeps everything the app uses.

## Read first

- `docs/tasks/p6-int-update-wiring.md` (Result), `docs/tasks/p6-velopack-dependency.md` (Result), `crates/wakeshot-tray/
  src/velo.rs`, `crates/wakeshot-tray/Cargo.toml`, root `Cargo.toml` (profile, `velopack` pin).
- The `velopack` crate's features at the pinned version (its `Cargo.toml` in the cargo registry, and its docs).

## Stop 1 (read only)

1. Build release x64 in your own target folder. Measure: `cargo tree -p wakeshot-tray -e features -i <crate>` for
   `url`, `idna`, `icu_*`, the TLS and HTTP crates; and the size each adds, with `cargo bloat` if it is installed
   (do not install tools; if it is missing, compare sizes of builds with a feature turned off, in your own target
   folder, with any `Cargo.toml` change made only in a scratch copy of the workspace outside the repo).
2. Which `velopack` features (or our own code in `velo.rs`) pull each, and which of them the app needs (the GitHub
   update source over HTTPS, the local test feed in debug builds).
3. Options, each with the size it saves and what it costs (lost function, a TLS choice, risk). The release profile
   too (`opt-level`, `lto`, `codegen-units`, `panic`, `strip`), if not set already.
4. Write the findings and your pick in the card Log and Result, then end with a short report that starts "Stop 1
   ready". Change no repo file but this card.

## Stop 2 (only after the orchestrator's go)

The picked change in `Cargo.toml`, `crates/wakeshot-tray/Cargo.toml` and `Cargo.lock`, with the full checks: `cargo fmt
--all --check`; clippy both forms, both targets (the tray's ARM64 needs clang: CI only); `cargo test --workspace
--locked`; the release builds; the dry-run smoke test (`crates/wakeshot-tray/tests/smoke.rs`).

## Out of scope

- `velo.rs` logic, the update schedule, packaging scripts.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. A release exe holds the build machine's paths as panic locations: the local baseline has 204 strings with
   `C:\Users\haika\.cargo\registry\...` (19,099 bytes). Shipped exes come from `.github/workflows/release.yml`, so they
   hold the runner's paths, not Haik's name. Cargo's `trim-paths` would drop them, but it is still unstable in Cargo
   1.99.1 (checked: the manifest fails to parse with "feature `trim-paths` is required"). Change it when `trim-paths`
   is stable, or if a release is ever built on the laptop.
2. No test runs a release exe. The dry-run smoke test skips itself in a release build (a release exe ignores
   `WAKESHOT_DATA_DIR`), so its `--release` run checks nothing; stop 1's advice to run it was wrong. The CLI's
   `crates/wakeshot-cli/tests/run.rs` has no such guard: under `cargo test --release` its runs use the real
   `%LocalAppData%\Wakeshot` (it happened here, see the Log at 06:58). Fix: the same `cfg!(debug_assertions)` skip as
   `smoke.rs`, `recovery.rs` and `tests/e2e/mod.rs` (`tests/exe.rs` needs none: help and argument errors only).
3. `docs/architecture.md` section 11 (Dependencies) has no row for the new direct dependency `idna_adapter` (1.0.0,
   tray only, pinned in the root `[workspace.dependencies]`, never named in code: it picks the no-Unicode IDNA back
   end for velopack's `url`). Its `velopack` row still says `Cargo.lock` 80 to 181 packages (now 160).
   `docs/phases/06-ship.md` line 29 and `docs/phases/01-design-skeleton.md` line 501 say velopack brings 113
   packages; now 93 on x64 (`cargo tree -e normal`, besides itself). Not edited: outside this card's files.
4. The dry-run icon's `HKCU\Control Panel\NotifyIconSettings\15983757285637097514` (only `IconSnapshot`) was there
   before this card's smoke run, from an earlier run; this run added no entry, so it was left alone.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 06:13: worker started stop 1 (read only). Status in progress.
- 2026-11-04 06:33: stop 1 measured. All builds release x64, `--locked` where the lock did not change, no `RUSTFLAGS`,
  in `<scratchpad>\p6-tray-size\target` (repo baseline, untouched repo) and `...\target-map` (a scratch copy of the
  workspace in `...\ws`, whose own `.cargo\config.toml` adds `/MAP` to the linker to get a per-crate breakdown; the map
  flag changes no byte: both baselines are 3,953,152). `cargo bloat` is not installed, so the map file and builds with
  one change each stand in for it.
- 2026-11-04 06:33: velopack features read from the registry source (`velopack-1.2.161/Cargo.toml`) and the crates.io
  API: `default = []`, `file-logging`, `public-utils`, the same in the newest version (`1.3.39-g8d2844f`, pre-release).
  None of them touches HTTP, TLS or `url`: velopack takes `ureq = "3.4"` with default features (`rustls` with `ring`
  and `webpki-roots`, `gzip`) and `url = "2.5"` as plain dependencies. Our own code adds none of them; `velo.rs` only
  calls velopack. So no feature switch in our `Cargo.toml` can turn them off (features only add).
- 2026-11-04 06:33: decided by the worker, for Haik's review: the pick for stop 2 is `opt-level = "z"` in the root
  release profile plus `idna_adapter = "=1.0.0"` in `crates/wakeshot-tray/Cargo.toml` (3,953,152 to 2,705,920 bytes,
  minus 31.6%). SChannel through a patched velopack (to 1,865,728) is not in stop 2: it needs a vendored velopack, outside
  stop 2's three files; proposed as its own card. Reasons and triggers in Result, "Decided by the worker".
- 2026-11-04 06:46: the orchestrator's go for stop 2 with the pick (opt-level "z" and the IDNA stub); SChannel stays a
  proposal for Haik. No other card builds. Started: sizes before, then the change, then the full checks.
- 2026-11-04 06:58: change made (root `Cargo.toml`, tray `Cargo.toml`, `Cargo.lock` by one resolve, no `cargo
  update`); fmt, clippy (x64 both forms, ARM64 core, windows and cli both forms), `cargo test --workspace --locked`, the
  release x64 build and both smoke runs passed. The smoke run with `--release` only prints "skipped: a release exe
  ignores WAKESHOT_DATA_DIR": by design it never runs the release exe.
- 2026-11-04 06:58: **my mistake, cleaned up.** As an extra check of the `"z"` code I ran `cargo test --workspace
  --release --locked --target x86_64-pc-windows-msvc`. The CLI's `tests/run.rs` has no release guard, and a release
  `wakeshot.exe` ignores `WAKESHOT_DATA_DIR`, so its runs used the real `%LocalAppData%\Wakeshot`: the folder was
  created at 06:57:41 by these runs (it did not exist before; creation time and log lines match the tests), with two
  logs and one leftover hold file `holds\run-23792.json` (pid 23792 already gone). No tray ran ("Wakeshot is not
  running"), so nothing held the PC awake, and the CLI writes no power setting. 3 tests failed for the same reason, and
  cargo stopped there. I copied the folder to `<scratchpad>\p6-tray-size\real-data-folder-copy`, deleted
  `%LocalAppData%\Wakeshot` (checked gone 06:58), and found no process of mine left. Not rerun: the release test run is
  invalid by design for these tests.
- 2026-11-04 07:00: `docs/research.md` section "Tray size" added; build folder deleted (checked gone); the real data
  folder still gone. Status done.

## Result

**Done 2026-11-04 07:00.** Stop 1 (read only) found the cause and the options; stop 2 shipped the pick on the
orchestrator's go (an orchestrator decision, for Haik's review). The SChannel option stays a proposal for Haik.

### Stop 2: what changed

| File | Change |
|---|---|
| `Cargo.toml` | `[workspace.dependencies]`: `idna_adapter = "=1.0.0"` with a why comment. `[profile.release]`: `opt-level = "z"` with a why comment |
| `crates/wakeshot-tray/Cargo.toml` | `idna_adapter.workspace = true` with a why comment (the pin works only as some package's dependency, and the tray is the one with velopack); the velopack comment's "113 packages" became "over 90" (93 on x64 now, `cargo tree -e normal`) |
| `Cargo.lock` | written by one resolve (`cargo metadata`), no `cargo update`: `idna_adapter` 1.2.2 to 1.0.0, 21 ICU packages removed (`icu_*`, `zerovec`, `yoke`, `tinystr` and their kin), the tray's new edge; 181 to 160 packages, no other version changed (diffed); identical to stop 1's scratch lock |
| `docs/research.md` | new section "Tray size" |

Sizes, release x64, `cargo build --release --workspace --locked`:

| Exe | Before | After | Change |
|---|---|---|---|
| `wakeshot-tray.exe` | 3,954,176 | 2,706,432 | minus 1,247,744 (31.6%) |
| `wakeshot.exe` (CLI) | 668,672 | 557,056 | minus 111,616 (16.7%; `opt-level` only, the CLI has no velopack) |

### Stop 2: checks

| Check | Outcome |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings`, and without `--all-targets` | exit 0, both |
| `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --locked --target aarch64-pc-windows-msvc -- -D warnings`, with and without `--all-targets` | exit 0, both |
| ARM64 tray | **CI only**: no clang on this laptop (`ring`); not run |
| `cargo test --workspace --locked` | exit 0; 14 suites, 1,287 passed, 0 failed, 52 ignored |
| `cargo build --release --workspace --locked --target x86_64-pc-windows-msvc` | exit 0; sizes above |
| CI's CRT check (`dumpbin /dependents`) on both new exes | passed; no Visual C++ runtime import |
| Smoke, debug: `cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test smoke -- --ignored --test-threads=1` | 1 passed; the dry-run tray started, held, went idle, ended with code 0; no new `NotifyIconSettings` entry or AUMID key |
| Smoke, `--release` | exit 0, but the test skips itself in a release build ("a release exe ignores WAKESHOT_DATA_DIR"), so it checks nothing |
| Extra, my choice: `cargo test --workspace --release --locked --target x86_64-pc-windows-msvc` | **invalid by design, and it touched the real data folder**: 3 CLI `run.rs` tests failed because a release `wakeshot.exe` ignores `WAKESHOT_DATA_DIR`; the folder it made was copied to scratch and deleted (Log 06:58) |
| Build folder | deleted, checked gone |

No power setting was read or written by this card; no install, hook, Run or RunOnce value touched.

Open items: the "Found outside scope" list (the CLI test release guard, the dependency rows in `docs/architecture.md`
and the phase files, the old dry-run icon entry). The SChannel card if Haik wants it (stop 1 below). Scratch kept in
`<scratchpad>\p6-tray-size\`: `exes\` (every measured exe and map), `ws\` (the scratch workspace with the patched
velopack), `tlscheck\`, `real-data-folder-copy\`, scripts and lock files.

### Stop 1 (read only, 06:13 to 06:34)

### What adds the bytes

Baseline: release x64 tray, the repo as it is: **3,953,152 bytes** (the card's 3,952,640 plus 512 from commits since).
Bytes per crate from the linker map (`/MAP`, sizes from symbol address gaps; generic code counts for the crate that
defines it, so rows are rough; the build diffs in the next table are the firm numbers).

| Group | Crates (code bytes) | Data bytes | Pulled by | The app needs it? |
|---|---|---|---|---|
| TLS | rustls 363,568; ring C and asm 176,352; ring 96,302; webpki 44,208; rustls_pki_types 6,576 | ring 191,980 (151,552 of it one P-256 table); likely webpki-roots about 75,000 (anonymous data that leaves with rustls; not verified by name) | `ureq` default feature `rustls` (`_ring`, `rustls-webpki-roots`), from `velopack` | HTTPS yes; this TLS stack no (SChannel does it, see C) |
| HTTP | ureq 138,080; http 38,400; ureq_proto 25,136; bytes 5,632; httparse 3,536; base64 1,808 | small | `velopack` `download.rs` | yes: GitHub API and assets |
| URL and IDNA | url 45,136; idna 15,760; icu_normalizer 8,608; icu_collections 1,504 | likely ICU data about 107,000 (not verified by name) | `velopack` takes `url` with defaults; `url` takes `idna`, which takes `idna_adapter` 1.2.2 (ICU4X) | `url` yes, for one ASCII URL; Unicode domain names no |
| gzip and zip | zlib_rs 19,760; flate2 17,952; zip 11,216; crc32fast 5,552 | crc32fast 16,384; zlib_rs 9,216 | `ureq` `gzip`; `velopack` `zip` `deflate` | kept |
| velopack and its parsers | velopack 117,888; serde_json 69,280 (ours use it too); xml 60,224; sha1 and sha2 17,497; glob 14,192; semver 10,448 | | `velopack` | yes |
| Ours | wakeshot_windows 477,488; wakeshot_core 241,392; wakeshot_tray 48,480 | | | yes |

`regex` (a `velopack` dependency) is built but leaves no code in the exe. Our `velo.rs` turns on no feature: velopack's
only features are `file-logging` and `public-utils`, both off, and none of them reaches HTTP, TLS or `url` (registry
`Cargo.toml` and the crates.io API, also for the newest `1.3.39-g8d2844f`). Features only add, so our `Cargo.toml`
cannot turn off velopack's `ureq` defaults or its `url`.

### Options (release x64, bytes)

| Option | Exe | Saved | Deflated (what a package holds) | Cost |
|---|---|---|---|---|
| Baseline | 3,953,152 | | 2,036,364 | |
| A1: `opt-level = "s"` | 3,053,568 | 899,584 (22.8%) | not measured | slower code; no lost function |
| A2: `opt-level = "z"` | 2,827,776 | 1,125,376 (28.5%) | 1,531,453 | slower code (no loop vectorizing); no lost function |
| B: IDNA stub, `idna_adapter = "=1.0.0"` | 3,820,544 | 132,608 (3.4%) | 1,981,712 | `url` rejects non-ASCII host names (see below); 21 crates leave the build |
| C: SChannel, patched velopack | 2,866,688 | 1,086,464 (27.5%) | 1,439,224 | a vendored velopack to keep (see below) |
| **A2 + B (pick)** | **2,705,920** | **1,247,232 (31.6%)** | 1,482,480 | as A2 and B |
| A2 + B + C | 1,865,728 | 2,087,424 (52.8%) | 991,355 | all three; ARM64: 1,699,840 |

The release profile already has `lto = true`, `codegen-units = 1`, `panic = "abort"` and `strip = true`; only
`opt-level` was left at its default (3). `trim-paths` (about 19 KB of path strings) is unstable in Cargo 1.99.1.
IDNA through unicode-rs (`idna_adapter` 1.1.x) was not built: its README says it gives a larger binary than ICU4X.

What each costs, in detail:

- **A2, `opt-level = "z"`**: the code is smaller and slower. Speed not measured; the tray's work is small file reads,
  JSON parsing and, once per update, hashing a package of a few MB, all while idle. Risk low. `p6-media`'s idle CPU
  number should come from a build with this change.
- **B, IDNA stub**: per the `idna_adapter` README, `url` then rejects non-ASCII host names and does not check
  Punycode labels. Wakeshot parses one URL with `url`: the constant `https://github.com/HaikAsatryan/wakeshot` in
  `GithubSource::new`. Asset and redirect URLs go to `ureq` (`http::Uri`, not `url`), and the debug feed
  (`FileSource`) takes a path. Checked: `idna` 1.1.0 `src/uts46.rs` passes lowercase ASCII labels through without
  the Unicode back end (`is_passthrough_ascii_label`); the 7 `velo` unit tests pass with the stub (in the scratch copy,
  debug), and they build the GitHub source, which panics if the URL does not parse. Not checked at run time: the
  `host_str()` value, beyond reading the passthrough code. A direct dependency in the tray's `Cargo.toml`, so a later
  `cargo update` cannot undo it; the lock then differs from the stub lock only by the tray's new edge (checked).
- **C, SChannel**: `[patch.crates-io]` with velopack 1.2.161 vendored in the repo (24 `.rs` files, 5,754 lines; the
  crate ships no license file, so add upstream's MIT text), with two changes: `ureq` with `default-features = false,
  features = ["gzip", "native-tls-no-default"]`, and `get_download_agent` setting `TlsProvider::NativeTls` and
  `RootCerts::PlatformVerifier`. The patch must be redone at each velopack bump (pinned exactly anyway). The
  `vendor` folder must go in the workspace `exclude`, or `clippy --workspace` lints velopack. TLS is then Windows'
  own: root store and TLS fixes come with Windows Update, and corporate TLS-inspection proxies work; the built-in
  Mozilla root list goes. `ring` leaves the build, so **the ARM64 tray builds without clang** (checked: ARM64 release
  build passed with no `clang` on PATH; `cargo tree -e build -i cc` for ARM64 prints nothing). Run-time check with a
  scratch program on the same `ureq` setup: GET the GitHub releases API for `HaikAsatryan/wakeshot`: 200 (an empty
  list, no release yet); for `velopack/velopack`: 200; HEAD of a release asset through the github.com redirect: 200.
  Lock: 17 entries in (`native-tls`, `schannel`, `der`, ...; `openssl`, `security-framework` and `core-foundation`
  only for other OSes, never built here), 9 out (`ring`, `rustls`, `rustls-webpki`, `webpki-roots`, `untrusted`,
  `subtle`, `getrandom` 0.2, `wasi`, `windows-sys` 0.52).

### Decided by the worker, for Haik's review

| # | Choice | Why | Change it when |
|---|---|---|---|
| 1 | Stop 2 does A2 + B: `opt-level = "z"` in the root `[profile.release]`, `idna_adapter = "=1.0.0"` (with a why comment) in `crates/wakeshot-tray/Cargo.toml` | 31.6% smaller with no lost function and no fork, in stop 2's three files | see 3 |
| 2 | `"z"`, not `"s"` | 225,792 bytes smaller here; the tray's code is not speed bound | a measured hot path is too slow under `"z"` |
| 3 | C (SChannel) is not in stop 2; proposed as its own card with a review of the TLS path | it adds a vendored velopack outside stop 2's files and changes the trust store of the update path | the orchestrator wants the extra 840,192 bytes (to 1.87 MB, under the 2,026,496 from before the update wiring) or ARM64 tray builds without clang; or velopack ships a feature for native TLS (then no patch: ask upstream) |
| 4 | Pin B as a direct dependency, not a lock-only `cargo update --precise` | a lock-only pin is lost at the next `cargo update` | never likely |

### Checks run

- Release x64 build of the repo as it is, own target folder, `--locked`: exit 0, 3,953,152 bytes, 103 s.
- `cargo tree -p wakeshot-tray -e normal,features -i` for `url`, `idna`, `idna_adapter`, `icu_normalizer`,
  `icu_properties`, `ureq`, `rustls`, `ring`, `webpki-roots`, `flate2`, `zip`, `regex`: every one comes from
  `velopack` default features of its own dependencies; none from a `wakeshot-*` crate.
- The option builds above, in a scratch copy of the workspace; `cargo update` ran only there.

### Stop 1's advice for stop 2

- The card's checks, plus the smoke test once with `--release`. Wrong in part: that run skips itself (stop 2 checks
  above; "Found outside scope" 2).
- Stop 1's second build folder (`target-map`) was deleted at stop 1; the main one at stop 2.
