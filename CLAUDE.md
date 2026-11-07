# Wakeshot

Smart keep-awake for Windows 11: the PC stays awake only while something needs it (an AI agent, a build, Keep awake
from the menu), then sleeps as usual, and it can keep a closed laptop working. Rust with `windows-rs`, MIT.

## Layout

- `crates/wakeshot-core`: the rules. Events in, commands out, no OS code, so it builds and tests on any OS.
- `crates/wakeshot-windows`: every Windows call. Test fakes sit behind its `test-support` feature.
- `crates/wakeshot-tray`: the tray exe (tray, guard and restore modes). `crates/wakeshot-cli`: the `wakeshot` command.
- `docs/`: `product.md` (what it does and why), `architecture.md` (the design), `research.md` and `research/` (facts
  and how they were checked), `brand.md`, `hold-files.md`, `agent-hooks.md`.
- `packaging/`: pack, smoke and end-to-end scripts, winget templates. `site/`: the GitHub Pages page. `media/`: the
  README pictures and the promo video (Remotion, see `media/README.md`).

## Checks

From `CONTRIBUTING.md`. `<t>` is `x86_64-pc-windows-msvc` or `aarch64-pc-windows-msvc`; CI runs both.

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings
cargo clippy --workspace --locked --target <t> -- -D warnings
cargo test --workspace --locked
cargo build --release --workspace --locked --target <t>
```

- The second clippy, without `--all-targets`, checks the libs as they ship, without `test-support`.
- `--locked` fails at once if a build would change `Cargo.lock`.
- Never set `RUSTFLAGS`: it replaces the flags in `.cargo/config.toml` and drops the static CRT.
- ARM64: `ring` calls `clang` by name, so put the Build Tools `VC\Tools\Llvm\x64\bin` folder on `PATH` for the ARM64
  cargo commands only.
- A change to `crates/wakeshot-windows/src/tray/art.rs` is ported to `media/src/art/heldLight.ts` in the same change;
  `node media/scripts/check-art.mjs` must pass.

## Rules

- Comments say why, never what. Every `unsafe` block has a `// SAFETY:` comment (clippy enforces it).
- No behavior change without a test that shows it.
- Tests that write real power settings are `#[ignore]`, work on a throwaway copy of the power plan, and run only with
  Wakeshot and every other keep-awake app quit: they change the same settings (`docs/architecture.md` section 9).
- A test that expects an open or a write to be denied must run unelevated (Medium), or it passes for the wrong reason.
- Docs and comments: plain short English, no em dashes. A fact goes to `docs/research.md` with how and when it was
  checked; what was not verified says so.

## Facts to keep in mind

- Claude Code's `~/.claude/sessions/<pid>.json` status files are undocumented. A file that does not parse is no
  signal and shows "update Wakeshot"; an unknown `status` counts as working while the session process lives and shows
  the same notice. Never read the `.key` files next to them (`docs/research.md`, "How to know Claude Code is working";
  `docs/product.md`, "UX").
- A power request cannot block a lid close; only the lid action Do nothing keeps a closed laptop running (Microsoft
  docs; `docs/research.md`, "Windows power facts").
- Idle sleep on Modern Standby needs `PowerRequestSystemRequired`; `PowerRequestExecutionRequired` alone is not enough
  (Microsoft docs, not tested; same section).
- On battery, Windows ends every power request about 5 minutes after the battery sleep timeout, lid open or closed.
  Battery sleep Never stopped that, so a hold raises the battery timer (same section; `docs/research.md`, "Lid, sleep,
  hibernate and resume" and "Manual power checks: sleep, a closed lid, sign-out and RunOnce").
- A user's own sleep (Start > Sleep) ends every request; after resume it is not honored until it is taken again
  (`docs/research.md`, "Windows power facts" and "Lid, sleep, hibernate and resume", "Resume").
