//! The HTTP client for the Mhon Cloud public API. Responses stay `serde_json::Value`: JSON output
//! passes them through untouched (fields the CLI doesn't know about yet aren't lost), and tables
//! pick fields by path (see `output::Column`). The operations are listed in openapi/openapi.json.

use std::time::Duration;

use serde_json::Value;

use crate::error::{CliError, ErrorKind};

/// Where the API lives (the `servers` entry of openapi/openapi.json).
pub const DEFAULT_API_URL: &str = "https://app.mhon.cloud";

const TIMEOUT: Duration = Duration::from_secs(30);

pub struct Client {
    agent: ureq::Agent,
    base: String,
    /// Empty = no Authorization header (the few public operations, e.g. pricing).
    key: String,
}

impl Client {
    pub fn new(base: &str, key: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(TIMEOUT)
            .user_agent(concat!("stargazer-cli/", env!("CARGO_PKG_VERSION")))
            .build();
        Client { agent, base: base.trim_end_matches('/').to_string(), key: key.to_string() }
    }

    /// A client for the operations that need no key.
    pub fn anonymous(base: &str) -> Self {
        Client::new(base, "")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub fn get(&self, path: &str) -> Result<Value, CliError> {
        self.send(self.agent.get(&self.url(path)), None)
    }

    pub fn get_query(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, CliError> {
        let mut req = self.agent.get(&self.url(path));
        for (k, v) in query {
            req = req.query(k, v);
        }
        self.send(req, None)
    }

    pub fn post(&self, path: &str, body: Option<Value>) -> Result<Value, CliError> {
        self.send(self.agent.post(&self.url(path)), body)
    }

    pub fn put(&self, path: &str, body: Value) -> Result<Value, CliError> {
        self.send(self.agent.put(&self.url(path)), Some(body))
    }

    pub fn delete(&self, path: &str) -> Result<Value, CliError> {
        self.send(self.agent.delete(&self.url(path)), None)
    }

    /// DELETE with a JSON body (e.g. a project's confirm_name).
    pub fn delete_with(&self, path: &str, body: Value) -> Result<Value, CliError> {
        self.send(self.agent.delete(&self.url(path)), Some(body))
    }

    /// PUT raw bytes (an upload chunk), with query parameters.
    pub fn put_bytes(&self, path: &str, query: &[(&str, &str)], bytes: &[u8]) -> Result<Value, CliError> {
        let mut req = self.authorize(self.agent.put(&self.url(path))).set("Content-Type", "application/octet-stream");
        for (k, v) in query {
            req = req.query(k, v);
        }
        self.finish(req.send_bytes(bytes))
    }

    fn authorize(&self, req: ureq::Request) -> ureq::Request {
        let req = req.set("Accept", "application/json");
        if self.key.is_empty() {
            req
        } else {
            req.set("Authorization", &format!("Bearer {}", self.key))
        }
    }

    fn send(&self, req: ureq::Request, body: Option<Value>) -> Result<Value, CliError> {
        let req = self.authorize(req);
        let result = match body {
            Some(b) => req.send_json(b),
            None => req.call(),
        };
        self.finish(result)
    }

    fn finish(&self, result: Result<ureq::Response, ureq::Error>) -> Result<Value, CliError> {
        match result {
            Ok(resp) => read_json(resp),
            Err(ureq::Error::Status(status, resp)) => Err(api_error(status, resp)),
            Err(ureq::Error::Transport(t)) => {
                Err(CliError::new(ErrorKind::Network, format!("can't reach {}: {t}", self.base)))
            }
        }
    }
}

fn read_json(resp: ureq::Response) -> Result<Value, CliError> {
    let text = resp
        .into_string()
        .map_err(|e| CliError::new(ErrorKind::Network, format!("reading the API's answer failed: {e}")))?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| {
        CliError::new(ErrorKind::Internal, format!("the API answered with something that isn't JSON: {e}"))
    })
}

/// The gateway answers errors as `{"error": "..."}`; keep its message, it's written for people.
fn api_error(status: u16, resp: ureq::Response) -> CliError {
    let message = resp
        .into_string()
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| format!("the API answered HTTP {status}"));
    CliError::from_status(status, message)
}
