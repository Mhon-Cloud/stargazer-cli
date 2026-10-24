//! `stargazer version` and `stargazer completions <shell>`: no API, no config.

use clap::CommandFactory;
use serde_json::json;

use crate::cli::Cli;
use crate::error::CliError;
use crate::output::{print_message, Format};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(format: Format) -> Result<(), CliError> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    print_message(
        format,
        json!({ "version": VERSION, "os": os, "arch": arch }),
        &format!("stargazer {VERSION} ({os} {arch})"),
    );
    Ok(())
}

pub fn completions(shell: clap_complete::Shell) -> Result<(), CliError> {
    let mut command = Cli::command();
    clap_complete::generate(shell, &mut command, "stargazer", &mut std::io::stdout());
    Ok(())
}
