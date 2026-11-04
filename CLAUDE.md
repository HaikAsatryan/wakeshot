# Wakeshot

Smart keep-awake for Windows 11: the PC stays awake only while something needs it (an AI agent, a build, Keep awake
from the menu), then sleeps as usual. Optional: keep working with the lid closed. Personal project of Haik Asatryan,
free, MIT. Rust with `windows-rs`.

Status: Phases 0 and 1 signed off 2026-11-03; Phases 2 to 6 build as parallel tracks (state in each phase file's card
table). Cargo workspace in `crates/` (`wakeshot-core`, `wakeshot-windows`, `wakeshot-tray`, `wakeshot-cli`): each
track card fills its own modules; the rest are still stubs. Throwaway
spikes in `spikes/` stay outside the workspace, but now build with the root's pinned toolchain (`rust-toolchain.toml`)
and static CRT (`.cargo/config.toml`). The contracts between the parts are in code (`p1-contracts`), with fakes
behind the `test-support` feature. Every module the tracks fill already exists, each file naming its one owner card in
the `Owner:` line of its module doc (`p1-contracts-split`), so no track edits a module list or a `Cargo` file. Repo
public at `HaikAsatryan/wakeshot` since 2026-11-03.

Checks: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings` and
the same without `--all-targets` (the libs as they ship, without `test-support`); `cargo test --workspace --locked`;
`cargo build --release --workspace --locked --target <t>`; `<t>` is `x86_64-pc-windows-msvc` or
`aarch64-pc-windows-msvc`. `--locked` fails at once if a build would change `Cargo.lock`. Never set `RUSTFLAGS`: it
replaces the config flags and drops the static CRT.

Start here: `docs/plan.md` (how we work, phases, status, next step). Then:

- Decisions: `docs/product.md`
- Design: `docs/architecture.md`
- Brand (colours, icons): `docs/brand.md`
- Facts and how they were checked: `docs/research.md`; longer reports in `docs/research/`

## Facts to keep in mind

- Claude Code's `~/.claude/sessions/<pid>.json` status files are undocumented. A file that does not parse is no
  signal, and Wakeshot shows "update Wakeshot". An unknown `status` value counts as working while the session process
  lives, and shows the same notice. Never read the `.key` files next to them.
- Lid close cannot be blocked by a power request. Only the lid action setting (Do nothing) keeps a closed laptop running
  (Microsoft docs; not tested by us).
- Idle sleep on Modern Standby needs `PowerRequestSystemRequired`. `PowerRequestExecutionRequired` alone is not enough
  (Microsoft docs; not tested by us).
- On battery, Windows ends every power request about 5 minutes after the battery sleep timeout, lid open or closed.
  Battery sleep Never stopped it (tested 2026-11-03), so a hold must raise the battery timer.
- A user's own sleep (Start > Sleep) ends every request; after resume it is not honored until it is taken again.

- Agents here run elevated (High, `SeDebugPrivilege` enabled; checked 2026-11-03). A test that expects an open or a
  write to be denied must run under an unelevated (Medium) token, or it passes for the wrong reason.
- In agent shells `cargo` may not be on `PATH`; call `%USERPROFILE%\.cargo\bin\cargo.exe` (checked 2026-11-03).

## Test laptop

- Modern Standby, Balanced plan `381b4222-f694-41f0-9685-ff5bb260df2e`. Haik's own values: lid action Sleep on AC
  and DC, sleep after 5 h on AC and on battery (Haik's own change 2026-11-04 00:31; read 01:39: AC and DC 18000).
  A USB-C dock (DELL) with an external screen is always plugged in; its cable can drop AC for a few
  seconds.
- Nightcrew 0.3.1 was uninstalled 2026-11-03 (values above restored and checked). Nothing of it is left: no exe or
  data folder, no `HKCU\Software\Nightcrew` key, no Run entry, no scheduled task. If it is ever installed again, never
  run it during tests: it changes the same power settings.
