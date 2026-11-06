//! VS Code chat sessions (Copilot and any other participant in the chat panel; architecture section 7): the session
//! files under `workspaceStorage\*\chatSessions\` and `globalStorage\emptyWindowChatSessions\`, read whole through
//! `WantedFile::Recent` (`docs/research/copilot-vscode-signals.md`, "Recommended detection rules"). Rule 7 (a request
//! older than every running `Code.exe` is a crash leftover) fits `Liveness::ImageSince`. A stub until that card
//! builds the source here.
//!
//! Owner: `p4-vscode-source`.
