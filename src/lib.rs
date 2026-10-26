//! The `stargazer` command line: a thin client of the Mhon Cloud public API
//! (`openapi/openapi.json`). `main.rs` only calls [`run`], so tests can drive the whole CLI.

pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod error;
pub mod output;

use clap::Parser;

use crate::cli::Cli;
use crate::context::Context;
use crate::output::Format;

/// Parses the arguments, runs the command and returns the process exit code
/// (see `error::ErrorKind::exit_code` and docs/output-and-errors.md).
pub fn run() -> i32 {
    // clap prints --help / --version / usage errors itself (usage errors exit with 2).
    let cli = Cli::parse();
    let format = Format::resolve(cli.global.output);
    output::set_fields(cli.global.fields.clone());
    let result = match commands::run_offline(&cli.command, format) {
        Some(result) => result,
        None => Context::from_args(&cli.global, format).and_then(|ctx| commands::dispatch(&cli.command, &ctx)),
    };
    match result {
        Ok(()) => 0,
        Err(err) => {
            output::print_error(format, &err);
            err.kind.exit_code()
        }
    }
}
