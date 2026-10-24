//! Commands that need no API: help, version, completions, and the errors raised before any request.

mod common;

use predicates::prelude::*;
use serde_json::Value;

use common::Env;

#[test]
fn help_lists_the_commands() {
    Env::new()
        .cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("servers").and(predicate::str::contains("Exit codes")));
}

#[test]
fn version_is_json_for_agents() {
    let out = Env::new().cmd().arg("version").assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).expect("json");
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn version_flag_works() {
    Env::new().cmd().arg("--version").assert().success().stdout(predicate::str::starts_with("stargazer "));
}

#[test]
fn completions_print_a_script() {
    Env::new().cmd().args(["completions", "bash"]).assert().success().stdout(predicate::str::contains("stargazer"));
}

#[test]
fn no_key_is_an_auth_error() {
    let assert = Env::new().cmd().args(["servers", "list"]).assert().code(3);
    let err: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json error");
    assert_eq!(err["error"]["code"], "auth");
}

#[test]
fn delete_needs_yes() {
    // Refused before any request: the URL can't be reached and isn't needed.
    let env = Env::new();
    let assert = env.signed_in("http://127.0.0.1:9").args(["servers", "delete", "web"]).assert().code(2);
    let err: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json error");
    assert_eq!(err["error"]["code"], "usage");
}

#[test]
fn login_refuses_what_isnt_a_key() {
    Env::new().cmd().args(["login", "--api-key-stdin"]).write_stdin("not-a-key\n").assert().code(2);
}
