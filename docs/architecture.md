# Architecture

```
main.rs ─► lib::run()
             │ clap parses (cli.rs) ── --help / --version / argument errors handled by clap (exit 2)
             │ Format::resolve(--output)            (output.rs)
             │ version / completions ─► run offline (commands/version.rs)
             │ Context::from_args                   (context.rs: config.rs + env)
             ▼
        commands::dispatch ─► commands/<group>.rs ─► api::Client (ureq, Bearer key) ─► app.mhon.cloud/api/v1
             │                                                   │
             │◄──────────── serde_json::Value ◄──────────────────┘
             ▼
        output::print_list / print_object / print_message   (stdout)
        errors: CliError ─► output::print_error (stderr) ─► exit code (error.rs)
```

## Decisions

- **One crate, library + binary.** `main.rs` only calls `stargazer::run()`, so unit tests reach every module and
  integration tests (`tests/`) run the real binary.
- **Answers stay `serde_json::Value`.** JSON output passes the API's objects through untouched (new API fields show up
  without a release), and tables pick fields by dot path (`output::Column`). Typed structs would add code and break on
  every API change. Request bodies are built with `json!`.
- **Sync HTTP (`ureq`, rustls).** One request at a time is all a command needs; no async runtime keeps start-up near
  1 ms and the binary small. If a command ever needs parallel requests, use threads first.
- **Name lookups list, then act** (`commands/common.rs::find`): one extra GET in exchange for `servers stop web`. Ids
  skip nothing (the list is still fetched, which also checks the item is the caller's).
- **Config is a TOML file, 0600** (`config.rs`). An OS keychain (the `keyring` crate) is on the roadmap; it needs
  D-Bus on Linux, which servers and containers often don't have, so the file stays the fallback.
- **No `--api-key` flag**: keys passed as flags leak into shell history and `ps`.
- **The spec is copied** into `openapi/openapi.json` (refreshed by `scripts/update-openapi.sh`; a weekly workflow
  reports drift). It's the reference for what the CLI may call; code isn't generated from it yet.

## Where the API is

`https://app.mhon.cloud` (the `servers` entry of the spec). The console is `https://console.mhon.cloud`. Override with
`--api-url` / `STARGAZER_API_URL` / `api_url` in the config (e.g. a staging gateway).
