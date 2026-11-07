<!-- CONTRIBUTING.md has the details. For a big change, open an issue first. -->

## What and why

<!-- What this changes and why, in a few sentences. Link the issue if there is one. -->

## Checks

<!-- <t> is x86_64-pc-windows-msvc or aarch64-pc-windows-msvc. Say which you ran. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --locked --target <t> -- -D warnings`
- [ ] `cargo clippy --workspace --locked --target <t> -- -D warnings`
- [ ] `cargo test --workspace --locked`, with tests for what this changes
- [ ] `cargo build --release --workspace --locked --target <t>`
- [ ] Real power settings: not touched, or tested by hand as CONTRIBUTING.md says (say how)
