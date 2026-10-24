//! What every command gets: the output format, the API URL and the API key, resolved once.
//!
//! Precedence, highest first:
//! - API URL: `--api-url` / `STARGAZER_API_URL`, then the config file, then [`DEFAULT_API_URL`].
//! - API key: `STARGAZER_API_KEY`, then the config file (`stargazer login`). There is no
//!   `--api-key` flag on purpose: flags end up in shell history and process lists.

use crate::api::{Client, DEFAULT_API_URL};
use crate::cli::GlobalArgs;
use crate::config;
use crate::error::{CliError, ErrorKind};
use crate::output::Format;

pub struct Context {
    pub format: Format,
    pub api_url: String,
    pub api_key: Option<String>,
}

impl Context {
    pub fn from_args(global: &GlobalArgs, format: Format) -> Result<Self, CliError> {
        let file = config::load()?;
        let api_url = global
            .api_url
            .clone()
            .filter(|u| !u.is_empty())
            .or(file.api_url)
            .unwrap_or_else(|| DEFAULT_API_URL.to_string());
        let api_key = std::env::var("STARGAZER_API_KEY").ok().filter(|k| !k.is_empty()).or(file.api_key);
        Ok(Context { format, api_url, api_key })
    }

    /// A client with the API key, or an auth error saying how to get one.
    pub fn client(&self) -> Result<Client, CliError> {
        match &self.api_key {
            Some(key) => Ok(Client::new(&self.api_url, key)),
            None => Err(CliError::new(
                ErrorKind::Auth,
                "not signed in: run `stargazer login` or set STARGAZER_API_KEY (create a key in the console: Overview > API)",
            )),
        }
    }
}
