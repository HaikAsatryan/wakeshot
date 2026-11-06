# Contributing

PRs are welcome. I review the architecture of each one: where the change lives, how it talks to the rest, and what it
does to your power settings. I do not review every line, so the checks and tests below carry the rest.

For a big change, open an issue first, so we agree on the design before you write the code. The design is in
[docs/architecture.md](docs/architecture.md), the decisions behind it in [docs/product.md](docs/product.md).

Bugs and ideas go through the [issue forms](https://github.com/HaikAsatryan/wakeshot/issues/new/choose). A security
problem goes through [SECURITY.md](SECURITY.md), never a public issue.

## Build

On Windows 11, x64 or ARM64:

1. Install [rustup](https://rustup.rs/) and the Visual Studio Build Tools with "Desktop development with C++".
2. Run `rustup toolchain install` in the repo root. It installs the pinned Rust and both targets from
   `rust-toolchain.toml`.
3. For ARM64, also add the Build Tools parts "C++ ARM64 build tools" and "C++ Clang Compiler for Windows". `ring`
   calls `clang` by name, so put the Build Tools `VC\Tools\Llvm\x64\bin` folder on `PATH` for the ARM64 cargo
   commands only.

## Checks

Run these before you open a PR. `<t>` is `x86_64-pc-windows-msvc` or `aarch64-pc-windows-msvc`; CI runs both.

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings
cargo clippy --workspace --locked --target <t> -- -D warnings
cargo test --workspace --locked
cargo build --release --workspace --locked --target <t>
```

- All tests must pass, and new code comes with its tests.
- The second clippy, without `--all-targets`, checks the libs as they ship, without the `test-support` fakes.
- `--locked` fails at once if a build would change `Cargo.lock`.
- Never set `RUSTFLAGS`. It replaces the flags in `.cargo/config.toml` and drops the static CRT, so the exes would need
  the Visual C++ runtime.
- Comments say why, not what.

## Tests that touch real power settings

`cargo test --workspace --locked` runs on fakes and never writes your power settings. The tests that do are
`#[ignore]` and work on a throwaway copy of the power plan. Before you run one with `--ignored`, quit Wakeshot from its
tray and close any other keep-awake app, write down your values (`powercfg /qh`), and check them after. A test that
expects access to be denied must run in a terminal that is not elevated, or it passes for the wrong reason.

## License

Your contribution ships under the [MIT license](LICENSE), like the rest of Wakeshot.
