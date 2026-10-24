//! Test helpers: run the real binary in a clean environment (its own config folder, no key from
//! the developer's shell), so tests never touch a real account or the developer's config.

use assert_cmd::Command;

pub struct Env {
    pub config: tempfile::TempDir,
}

impl Env {
    pub fn new() -> Self {
        Env { config: tempfile::tempdir().expect("temp dir") }
    }

    /// `stargazer` with no key and output forced to JSON (as an agent would see it).
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("stargazer").expect("binary");
        cmd.env("STARGAZER_CONFIG_DIR", self.config.path())
            .env_remove("STARGAZER_API_KEY")
            .env_remove("STARGAZER_API_URL")
            .env_remove("STARGAZER_OUTPUT")
            .arg("--output")
            .arg("json");
        cmd
    }

    /// `stargazer` signed in with a test key against `api_url` (a mockito server).
    pub fn signed_in(&self, api_url: &str) -> Command {
        let mut cmd = self.cmd();
        cmd.env("STARGAZER_API_KEY", "mhc_test").env("STARGAZER_API_URL", api_url);
        cmd
    }
}
