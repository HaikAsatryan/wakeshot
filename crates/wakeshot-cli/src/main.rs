//! The `wakeshot` command (architecture section 4, "CLI"). This dispatch is the contracts card's and stays as it is;
//! each command's file has its own owner.
//!
//! Owner: contracts card.

mod args;
mod automatic;
mod command;
mod keep_awake;
mod open_log;
mod restore;
mod run;
mod status;

use std::process::ExitCode;

use command::Command;

fn main() -> ExitCode {
    let argv: Vec<_> = std::env::args_os().skip(1).collect();
    let (global, command) = args::parse(&argv);
    match command {
        Command::Run => run::main(&global),
        Command::KeepAwake { choice, screen } => keep_awake::main(&global, choice, screen),
        Command::Automatic(on) => automatic::main(&global, on),
        Command::Status => status::main(&global),
        Command::Restore => restore::main(&global),
        Command::Log => open_log::main(&global),
        Command::Help => args::help(),
        Command::Bad(why) => args::bad(&why),
    }
}
