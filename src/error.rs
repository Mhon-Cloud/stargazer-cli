//! Errors and exit codes. Every failure is a [`CliError`]; its [`ErrorKind`] decides the exit
//! code and the `code` field of the JSON error, which scripts and AI agents branch on.
//! Keep the table in docs/output-and-errors.md in step with this file.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Anything unexpected (a bug, a 5xx from the API, a response that isn't JSON).
    Internal,
    /// Bad arguments or input the API refused (400 / 422).
    Usage,
    /// No API key, a wrong one, or not allowed (401 / 403).
    Auth,
    /// The server, container, project... doesn't exist or isn't yours (404).
    NotFound,
    /// Not enough credit (402).
    PaymentRequired,
    /// Not possible in its current state, e.g. delete a running server (409).
    Conflict,
    /// Too many requests (429); try again later.
    RateLimited,
    /// The API couldn't be reached (DNS, TLS, timeout...).
    Network,
}

impl ErrorKind {
    pub fn exit_code(self) -> i32 {
        match self {
            ErrorKind::Internal => 1,
            ErrorKind::Usage => 2,
            ErrorKind::Auth => 3,
            ErrorKind::NotFound => 4,
            ErrorKind::PaymentRequired => 5,
            ErrorKind::Conflict => 6,
            ErrorKind::RateLimited => 7,
            ErrorKind::Network => 8,
        }
    }

    /// The stable machine-readable name, shown as `error.code` in JSON output.
    pub fn code(self) -> &'static str {
        match self {
            ErrorKind::Internal => "internal",
            ErrorKind::Usage => "usage",
            ErrorKind::Auth => "auth",
            ErrorKind::NotFound => "not_found",
            ErrorKind::PaymentRequired => "payment_required",
            ErrorKind::Conflict => "conflict",
            ErrorKind::RateLimited => "rate_limited",
            ErrorKind::Network => "network",
        }
    }
}

#[derive(Debug)]
pub struct CliError {
    pub kind: ErrorKind,
    pub message: String,
    /// The API's HTTP status, when the error came from the API.
    pub status: Option<u16>,
}

impl CliError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        CliError { kind, message: message.into(), status: None }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        CliError::new(ErrorKind::Usage, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        CliError::new(ErrorKind::NotFound, message)
    }

    /// An error answer from the API (`{"error": "..."}` with an HTTP status).
    pub fn from_status(status: u16, message: impl Into<String>) -> Self {
        let kind = match status {
            400 | 422 => ErrorKind::Usage,
            401 | 403 => ErrorKind::Auth,
            402 => ErrorKind::PaymentRequired,
            404 => ErrorKind::NotFound,
            409 => ErrorKind::Conflict,
            429 => ErrorKind::RateLimited,
            _ => ErrorKind::Internal,
        };
        CliError { kind, message: message.into(), status: Some(status) }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_map_to_kinds() {
        assert_eq!(CliError::from_status(402, "x").kind, ErrorKind::PaymentRequired);
        assert_eq!(CliError::from_status(403, "x").kind, ErrorKind::Auth);
        assert_eq!(CliError::from_status(409, "x").kind, ErrorKind::Conflict);
        assert_eq!(CliError::from_status(503, "x").kind, ErrorKind::Internal);
    }

    #[test]
    fn exit_codes_are_distinct() {
        let kinds = [
            ErrorKind::Internal,
            ErrorKind::Usage,
            ErrorKind::Auth,
            ErrorKind::NotFound,
            ErrorKind::PaymentRequired,
            ErrorKind::Conflict,
            ErrorKind::RateLimited,
            ErrorKind::Network,
        ];
        let mut codes: Vec<i32> = kinds.iter().map(|k| k.exit_code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), kinds.len());
    }
}
