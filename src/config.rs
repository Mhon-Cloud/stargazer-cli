//! The config file: `<config dir>/stargazer/config.toml` (e.g. `~/.config/stargazer/config.toml`
//! on Linux), or `$STARGAZER_CONFIG_DIR/config.toml`. It holds the API key, so it's written
//! readable by its owner only (0600 on Unix).

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{CliError, ErrorKind};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

pub fn dir() -> Result<PathBuf, CliError> {
    if let Some(d) = std::env::var_os("STARGAZER_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Ok(PathBuf::from(d));
    }
    dirs::config_dir()
        .map(|d| d.join("stargazer"))
        .ok_or_else(|| CliError::new(ErrorKind::Internal, "can't find a config folder; set STARGAZER_CONFIG_DIR"))
}

pub fn path() -> Result<PathBuf, CliError> {
    Ok(dir()?.join("config.toml"))
}

pub fn load() -> Result<Config, CliError> {
    let path = path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| CliError::new(ErrorKind::Internal, format!("can't read {}: {e}", path.display())))?;
    toml::from_str(&text)
        .map_err(|e| CliError::new(ErrorKind::Internal, format!("{} isn't valid TOML: {e}", path.display())))
}

pub fn save(config: &Config) -> Result<PathBuf, CliError> {
    let path = path()?;
    let fail = |e: std::io::Error| CliError::new(ErrorKind::Internal, format!("can't write {}: {e}", path.display()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(fail)?;
    }
    let text = toml::to_string(config)
        .map_err(|e| CliError::new(ErrorKind::Internal, format!("can't encode the config: {e}")))?;

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(fail)?;
    file.write_all(text.as_bytes()).map_err(fail)?;
    #[cfg(unix)]
    {
        // A file made before (with other permissions) is tightened too.
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(fail)?;
    }
    Ok(path)
}
