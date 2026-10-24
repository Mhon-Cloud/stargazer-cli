//! `login`, `logout`, `whoami`, `credit`.

use std::io::{BufRead, IsTerminal};

use serde_json::json;

use crate::api::Client;
use crate::cli::LoginArgs;
use crate::config;
use crate::context::Context;
use crate::error::{CliError, ErrorKind};
use crate::output::{col, print_message, print_object, Column};

/// Every key starts with this (004_profile/api_key_module.go in the platform).
const KEY_PREFIX: &str = "mhc_";

const ME: &[Column] = &[
    col("ID", "user.id"),
    col("Email", "user.email"),
    col("Username", "user.username"),
    col("Status", "user.status"),
    col("Credit", "user.credit"),
    col("First name", "profile.first_name"),
    col("Last name", "profile.last_name"),
];

/// Reads the key from stdin (piped, or `--api-key-stdin`) or asks for it on a terminal without
/// echoing it, checks it against the API, then saves it in the config file.
pub fn login(args: &LoginArgs, ctx: &Context) -> Result<(), CliError> {
    let key = if args.api_key_stdin || !std::io::stdin().is_terminal() {
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| CliError::usage(format!("can't read the key from stdin: {e}")))?;
        line
    } else {
        rpassword::prompt_password("API key (Overview > API in the console): ")
            .map_err(|e| CliError::usage(format!("can't read the key: {e}")))?
    };
    let key = key.trim().to_string();
    if !key.starts_with(KEY_PREFIX) {
        return Err(CliError::usage(format!("that isn't an API key: keys start with {KEY_PREFIX}")));
    }

    // A wrong or revoked key fails here (exit 3) instead of on the next command.
    let credit = Client::new(&ctx.api_url, &key).get("/api/v1/me/credit")?;

    let mut file = config::load()?;
    file.api_key = Some(key);
    let path = config::save(&file)?;
    print_message(
        ctx.format,
        json!({ "logged_in": true, "config": path.display().to_string(), "credit": credit["credit"] }),
        &format!("Signed in. The key is saved in {}.", path.display()),
    );
    Ok(())
}

pub fn logout(ctx: &Context) -> Result<(), CliError> {
    let mut file = config::load()?;
    let had_key = file.api_key.take().is_some();
    if had_key {
        config::save(&file)?;
    }
    let env_note = if std::env::var_os("STARGAZER_API_KEY").is_some() {
        " STARGAZER_API_KEY is still set in this shell."
    } else {
        ""
    };
    print_message(
        ctx.format,
        json!({ "logged_out": had_key }),
        &format!("{}{env_note}", if had_key { "Signed out." } else { "No key was saved." }),
    );
    Ok(())
}

pub fn whoami(ctx: &Context) -> Result<(), CliError> {
    let me = ctx.client()?.get("/api/v1/auth/me")?;
    print_object(ctx.format, &me, ME);
    Ok(())
}

pub fn credit(ctx: &Context) -> Result<(), CliError> {
    let answer = ctx.client()?.get("/api/v1/me/credit")?;
    let amount =
        answer.get("credit").ok_or_else(|| CliError::new(ErrorKind::Internal, "the API's answer has no credit"))?;
    let currency = answer["currency"].as_str().unwrap_or("THB");
    print_message(ctx.format, answer.clone(), &format!("{amount} {currency}"));
    Ok(())
}
