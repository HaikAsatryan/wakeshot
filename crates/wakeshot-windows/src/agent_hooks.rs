//! Installs and removes the hook folders Wakeshot gives an agent (a Cursor plugin folder with its hooks, for one), so
//! the agent itself tells Wakeshot when it works (`docs/research/cursor-gemini-signals.md`, "Hooks as a signal, per
//! agent"; the design in `docs/agent-hooks.md`). Not Velopack's install hooks: those are `hooks.rs`, whose uninstall
//! step (`HookEnv::remove_agent_hooks`, a stub for now) is the removal's way in. A stub until that card builds it here.
//!
//! Owner: `p4-agent-hooks-install`.
