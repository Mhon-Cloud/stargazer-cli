# Working on stargazer-cli (for AI agents and contributors)

`stargazer` is the command line for **Stargazer**, Mhon Cloud's console (https://console.mhon.cloud). It is a thin
client of the platform's **public API** (`https://app.mhon.cloud/api/v1`, described in `openapi/openapi.json`).
Many users drive it **through AI agents**, so its output and errors are a contract: read
[docs/output-and-errors.md](docs/output-and-errors.md) before changing anything a command prints.

## Rules that don't bend

1. **The API spec is the allowlist.** Only call operations listed in `openapi/openapi.json`. The gateway refuses
   everything else for API keys anyway (admin, top-up / withdrawal, account, key management). Don't add those.
2. **JSON mode is a contract.** With `--output json` (the default when stdout isn't a terminal) a command prints
   **exactly one JSON document on stdout** and nothing else: no colours, spinners, progress or hints. Lists are a JSON
   array of the API's own objects, passed through untouched. Errors go to **stderr** as
   `{"error": {"code", "message", "status"}}`.
3. **Exit codes are stable** (`src/error.rs`): 0 ok, 1 internal, 2 usage, 3 auth, 4 not found, 5 not enough credit,
   6 conflict, 7 rate limited, 8 network. Never reuse or renumber one; add new ones at the end.
4. **Never prompt** except `login` on a terminal. Destructive commands need `--yes` and fail with a usage error (exit 2)
   without it. An agent must never hang waiting for input.
5. **Keys stay secret.** No `--api-key` flag (shell history). The key comes from `STARGAZER_API_KEY` or the config file
   (written 0600). Never print it, log it, or put it in an error message.
6. **Names work where ids do.** Actions take an id or a name (servers: name; containers: name or number), resolved with
   `commands::common::find`; ambiguous names are a usage error that asks for the id or `--project`.
7. **Help text is short and plain.** `--help` is how agents learn the CLI: one line per command saying what it does.

## Layout

```
.
├── AGENTS.md                 this file (CLAUDE.md points here)
├── README.md                 for users: install, sign in, examples
├── Cargo.toml / Cargo.lock   one crate: library `stargazer` + binary `stargazer`
├── rustfmt.toml              max_width 120
├── install.sh / install.ps1  installers (download a release, check its SHA-256)
├── openapi/openapi.json      the platform's public API spec (copy; refreshed by the daily openapi-sync PR)
├── openapi/contract.json     what the CLI needs from the API (checked by tests/contract.rs and by the platform)
├── scripts/                  maintenance scripts
├── docs/
│   ├── architecture.md       how the code is put together, and why
│   ├── commands.md           every command and the API operation behind it
│   ├── output-and-errors.md  the JSON / exit-code contract
│   ├── releasing.md          how to cut a release
│   └── roadmap.md            what's next (pick work from here)
├── src/
│   ├── main.rs               calls stargazer::run()
│   ├── lib.rs                run(): parse, build the Context, dispatch, print errors, exit code
│   ├── cli.rs                every command / flag / help text (clap derive)
│   ├── context.rs            output format + API URL + key, resolved once
│   ├── config.rs             ~/.config/stargazer/config.toml (0600)
│   ├── api.rs                HTTP client (ureq); answers stay serde_json::Value
│   ├── error.rs              CliError, ErrorKind, exit codes
│   ├── output.rs             tables, JSON, errors
│   └── commands/             one file per command group
│       ├── mod.rs            dispatch
│       ├── common.rs         list extraction, find by id / name, project filter
│       ├── account.rs        login, logout, whoami, credit
│       ├── projects.rs  zones.rs  servers.rs  containers.rs  dns.rs
│       └── version.rs        version, completions (no API, no config)
├── tests/
│   ├── common/mod.rs         runs the real binary in a clean env (temp config, no key)
│   ├── cli.rs                offline behaviour
│   ├── api.rs                against a mock API (mockito)
│   └── contract.rs           contract.json = the code's calls, and the spec has them
└── .github/
    ├── dependabot.yml        weekly dependency / action update PRs
    └── workflows/
        ├── test.yml          fmt, clippy, shellcheck, tests (Linux / macOS / Windows), MSRV, cargo audit
        ├── build.yml         release binaries for 5 targets, smoke-tested, packaged + SHA-256
        ├── ci.yml            push / PR: test -> build (binaries kept 7 days as run artifacts)
        ├── release.yml       tag v*: version check -> test -> build -> publish -> verify-install
        └── openapi-sync.yml  daily: live spec changed -> PR with the diff + contract check
```

## Keeping up with the API

- **`openapi/contract.json`** lists every operation the CLI calls with the request fields it sends and the response
  fields it needs. `tests/contract.rs` fails when the code calls something not listed, or the spec copy lacks it.
- **The platform checks it before deploying** the gateway (`apidocs/check_api.py`, run by its `redeploy.sh`, reads this
  file from `main`): a change that would break the CLI stops the deploy. Keep `main`'s contract.json current.
- **`openapi-sync.yml`** (daily): the live spec changed → PR `openapi-sync` with the new copy, the operations added /
  removed / changed (marked when the CLI uses them) and the contract result. New operation → a roadmap item.

## CI/CD

| Stage | When | What |
|---|---|---|
| **Test** (`test.yml`) | every push to main, every PR, every release | `cargo fmt --check`, clippy `-D warnings`, `shellcheck`, `cargo test` on Linux, macOS and Windows, the MSRV build (1.88), `cargo audit` |
| **Build** (`build.yml`) | after Test passes | `--release` binaries for every target (`cross` for Linux musl); each runnable one is smoke-tested (`--help`, `version` matches Cargo.toml, no key = exit 3); packaged exactly like a release |
| **Release** (`release.yml`) | a `v*` tag | tag = Cargo.toml version, Test, Build, then publish a GitHub release with provenance attestations |
| **Verify** (`release.yml`) | after publishing | `install.sh` on Linux x64 / ARM64 and macOS, `install.ps1` on Windows, against the release just published |

A PR is mergeable when CI is green. Never tag a commit whose CI failed. How to cut a release: docs/releasing.md.

## Adding a command

1. Find the operation in `openapi/openapi.json` (if it isn't there, the platform must add it first: it lives in
   `management_page/backend/api_gateway/apidocs/gen_openapi.py` in the platform repo, then refresh the copy here).
2. Add the subcommand and its flags in `src/cli.rs` (doc comments = help text; `visible_alias = "ls"` on lists).
3. Implement it in the matching `src/commands/<group>.rs` (new group = new file + a line in `commands/mod.rs`):
   call `ctx.client()?`, print with `output::print_list` / `print_object` / `print_message`. Pick table columns with
   `col("HEADER", "dot.path")`; JSON output stays the API's object.
4. Test it in `tests/api.rs` with a mockito mock: the request (method, path, body), the JSON output, and at least one
   error status mapping to the right exit code. Never call the real API from tests.
5. Add the operation (and the fields you rely on) to `openapi/contract.json`: `tests/contract.rs` fails until you do.
6. Document it in `docs/commands.md` (and the README's table when it's a headline command).

## Build, lint, test

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

No Rust on the machine? Use Docker (as uid 1001 on the Mhon Cloud dev host, so files keep their owner):

```sh
docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -e CARGO_HOME=/tmp/cargo -v "$PWD":/src -w /src rust:1-alpine \
  sh -c 'cargo test --locked'
```

(`rustfmt` / `clippy` aren't in that image: run them as root in a throwaway container after
`rustup component add rustfmt clippy`, then `chown` the files back.)

Try it against the real API with your own key: `STARGAZER_API_KEY=mhc_... cargo run -- servers list`.
Read-only commands are safe; actions act on real machines and may move real credit.

## Style

- Comments say *why*, like the platform repo: a module doc comment, then only what isn't obvious.
- User-facing words: plain and specific ("stop the server before deleting it"), no apologies, no jargon the console
  doesn't use. The API's own error messages are already written for people: pass them through.
- Keep dependencies few (start-up time and binary size matter for agents calling it in loops). Prefer sync code; no
  async runtime unless a command really needs concurrency.
