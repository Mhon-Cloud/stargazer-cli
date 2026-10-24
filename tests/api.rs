//! Commands against a mock API (mockito): requests, name lookups, output and error mapping.

mod common;

use serde_json::{json, Value};

use common::Env;

const SERVERS: &str = r#"{"servers": [
  {"id": "11111111-0000-0000-0000-000000000001", "name": "web", "status": "running", "cpu_cores": 2,
   "project": {"id": "p1", "name": "Shop"}, "zone": {"id": "z1", "name": "BKK-1", "region": "th"}},
  {"id": "11111111-0000-0000-0000-000000000002", "name": "db", "status": "stopped", "cpu_cores": 4,
   "project": {"id": "p2", "name": "Blog"}, "zone": null}
]}"#;

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn servers_list_sends_the_key_and_prints_an_array() {
    let mut api = mockito::Server::new();
    let mock = api
        .mock("GET", "/api/v1/servers")
        .match_header("authorization", "Bearer mhc_test")
        .with_header("content-type", "application/json")
        .with_body(SERVERS)
        .create();

    let env = Env::new();
    let out = env.signed_in(&api.url()).args(["servers", "list", "--project", "shop"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    mock.assert();
    let list = stdout_json(&out);
    assert_eq!(list.as_array().map(Vec::len), Some(1));
    assert_eq!(list[0]["name"], "web");
}

#[test]
fn servers_start_finds_the_id_by_name() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/servers").with_body(SERVERS).create();
    let action = api
        .mock("POST", "/api/v1/servers/11111111-0000-0000-0000-000000000002/actions")
        .match_body(mockito::Matcher::Json(json!({ "action": "start" })))
        .with_status(202)
        .with_body(r#"{"server": {"id": "11111111-0000-0000-0000-000000000002", "name": "db", "last_action": {"action": "start", "status": "pending"}}}"#)
        .create();

    let env = Env::new();
    let out = env.signed_in(&api.url()).args(["servers", "start", "db"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    action.assert();
    assert_eq!(stdout_json(&out)["last_action"]["status"], "pending");
}

#[test]
fn not_enough_credit_exits_5_with_the_apis_message() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/servers").with_body(SERVERS).create();
    api.mock("POST", "/api/v1/servers/11111111-0000-0000-0000-000000000001/actions")
        .with_status(402)
        .with_body(r#"{"error": "not enough credit: the resize costs ฿350.00"}"#)
        .create();

    let env = Env::new();
    let out = env.signed_in(&api.url()).args(["servers", "resize", "web", "--cores", "4"]).output().unwrap();
    assert_eq!(out.status.code(), Some(5));
    let err: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "payment_required");
    assert_eq!(err["error"]["status"], 402);
    assert!(err["error"]["message"].as_str().unwrap().contains("not enough credit"));
}

#[test]
fn unknown_server_is_not_found() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/servers").with_body(SERVERS).create();
    let env = Env::new();
    env.signed_in(&api.url()).args(["servers", "stop", "nope"]).assert().code(4);
}

#[test]
fn credit_passes_the_api_answer_through() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/me/credit").with_body(r#"{"credit": 1250.5, "currency": "THB"}"#).create();
    let env = Env::new();
    let out = env.signed_in(&api.url()).arg("credit").output().unwrap();
    assert!(out.status.success());
    assert_eq!(stdout_json(&out), json!({ "credit": 1250.5, "currency": "THB" }));
}

#[test]
fn login_checks_the_key_then_saves_it_privately() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/me/credit")
        .match_header("authorization", "Bearer mhc_good")
        .with_body(r#"{"credit": 10, "currency": "THB"}"#)
        .create();

    let env = Env::new();
    env.cmd()
        .env("STARGAZER_API_URL", api.url())
        .args(["login", "--api-key-stdin"])
        .write_stdin("mhc_good\n")
        .assert()
        .success();

    let path = env.config.path().join("config.toml");
    assert!(std::fs::read_to_string(&path).unwrap().contains("mhc_good"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn a_revoked_key_is_not_saved() {
    let mut api = mockito::Server::new();
    api.mock("GET", "/api/v1/me/credit").with_status(401).with_body(r#"{"error": "invalid API key"}"#).create();

    let env = Env::new();
    env.cmd()
        .env("STARGAZER_API_URL", api.url())
        .args(["login", "--api-key-stdin"])
        .write_stdin("mhc_revoked\n")
        .assert()
        .code(3);
    assert!(!env.config.path().join("config.toml").exists());
}
