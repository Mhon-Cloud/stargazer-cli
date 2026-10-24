//! One module per command group. Each `run` takes its parsed arguments and the [`Context`],
//! calls the API through `ctx.client()` and prints with `crate::output`. No command prompts
//! (except `login` on a terminal) and none prints anything but its result on stdout.

mod account;
mod common;
mod containers;
mod dns;
mod projects;
mod servers;
mod version;
mod zones;

use crate::cli::Command;
use crate::context::Context;
use crate::error::CliError;
use crate::output::Format;

/// Commands that need neither the config file nor the API (so they work even with a broken config).
pub fn run_offline(command: &Command, format: Format) -> Option<Result<(), CliError>> {
    match command {
        Command::Version => Some(version::run(format)),
        Command::Completions { shell } => Some(version::completions(*shell)),
        _ => None,
    }
}

pub fn dispatch(command: &Command, ctx: &Context) -> Result<(), CliError> {
    match command {
        Command::Version | Command::Completions { .. } => run_offline(command, ctx.format).expect("offline command"),
        Command::Login(args) => account::login(args, ctx),
        Command::Logout => account::logout(ctx),
        Command::Whoami => account::whoami(ctx),
        Command::Credit => account::credit(ctx),
        Command::Projects(cmd) => projects::run(cmd, ctx),
        Command::Zones(cmd) => zones::run(cmd, ctx),
        Command::Servers(cmd) => servers::run(cmd, ctx),
        Command::Containers(cmd) => containers::run(cmd, ctx),
        Command::Dns(cmd) => dns::run(cmd, ctx),
    }
}
