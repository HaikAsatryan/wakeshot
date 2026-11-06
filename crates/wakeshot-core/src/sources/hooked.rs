//! What the hooked sources share, Cursor's and Antigravity's (`docs/agent-hooks.md`): the state file format, the chat
//! id rule, the event rules and the walk up a process list to the agent's process, so neither source module owns the
//! other's code. The CLI's `hook` command uses it too: the input parse and the state file text belong here, since the
//! core has `serde` and the CLI has no JSON crate of its own (a `Cargo` change otherwise). A stub until that card
//! builds it here.
//!
//! Owner: `p4-cursor-source`.
