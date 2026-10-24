# Output and errors (the contract)

Scripts and AI agents parse what `stargazer` prints, so this page is a promise. Changing it is a breaking change
(a major version once 1.0 is out).

## Choosing the format

| `--output` | Prints |
|---|---|
| `auto` (default) | `table` when stdout is a terminal, `json` otherwise |
| `json` | one JSON document on stdout |
| `table` | aligned columns for people (not stable: don't parse it) |

`STARGAZER_OUTPUT=json` sets it for every command.

## JSON output

- **Exactly one JSON document** on stdout, followed by a newline. Compact when piped, indented on a terminal.
- **Lists are a JSON array** of the API's own objects (`servers list` → `[{...}, ...]`), not wrapped.
- **One item** (an action's result, `whoami`) is the API's object, as the API sent it. Fields the CLI doesn't know
  about are kept, so new API fields appear without a CLI release.
- **Messages** (`login`, `logout`, `version`) are small objects, e.g. `{"logged_out": true}`.
- `containers logs` is `{"logs": "..."}`.
- Nothing else is printed on stdout: no colours, spinners, progress or tips.

## Errors

On failure, nothing is printed on stdout; stderr gets one line:

```json
{"error": {"code": "payment_required", "message": "not enough credit: ...", "status": 402}}
```

`status` is the API's HTTP status, or `null` when the error didn't come from the API. `message` is for people (the
API's own wording when it has one). Branch on **`code` or the exit code**, never on `message`.

In table mode the same error prints as `error: <message>`.

## Exit codes

| Exit | `code` | Meaning | What to do |
|---|---|---|---|
| 0 | | Done | |
| 1 | `internal` | Unexpected: a bug, an API 5xx, an answer that isn't JSON | Retry later; report if it persists |
| 2 | `usage` | Bad arguments, a missing `--yes`, an ambiguous name, or the API refused the input (400) | Fix the command |
| 3 | `auth` | No key, a wrong / revoked key, or not allowed (401 / 403; e.g. a guest stopping a server) | `stargazer login`, or ask the project's owner |
| 4 | `not_found` | No such server / container / ... in your projects (404) | Check `list` |
| 5 | `payment_required` | Not enough credit (402) | Top up in the console |
| 6 | `conflict` | Not possible right now (409): e.g. delete a running server, another action in progress | Wait or change state first |
| 7 | `rate_limited` | Too many requests (429), e.g. a second resize within 2 minutes | Wait and retry |
| 8 | `network` | The API couldn't be reached (DNS, TLS, timeout) | Check the connection / `STARGAZER_API_URL` |

clap's own argument errors (unknown flag, missing argument) also exit with 2, with clap's plain-text message.

## Actions are queued

Server actions (start, stop, rename, resize, delete) are queued by the platform: the command returns as soon as the
action is accepted (HTTP 202), with the server's `last_action` (`pending`). Poll `servers list` to see it finish (a
`--wait` flag is on the roadmap).
