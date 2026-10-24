# Commands

Every command takes the global flags `--output auto|json|table` and `--api-url URL`. Arguments named `<server>` /
`<container>` take an id or a name (containers also take their number, `CT-000001`); `--project` (id or name) narrows
both lists and name lookups.

| Command | API operation | Notes |
|---|---|---|
| `version` | none | `{"version", "os", "arch"}`; works without a config |
| `completions <shell>` | none | bash, zsh, fish, powershell, elvish |
| `login [--api-key-stdin]` | `GET /api/v1/me/credit` (to check the key) | Reads the key from stdin when piped, else asks without echo; saves it 0600 |
| `logout` | none | Removes the saved key (`STARGAZER_API_KEY` is untouched) |
| `whoami` | `GET /api/v1/auth/me` | |
| `credit` | `GET /api/v1/me/credit` | `{"credit": 1250.5, "currency": "THB"}` |
| `projects list` | `GET /api/v1/projects` | `my_role`: owner / admin / staff / guest |
| `zones list` | `GET /api/v1/availability-zones` | |
| `servers list [--project P]` | `GET /api/v1/servers` | |
| `servers start <server>` | `POST /api/v1/servers/{id}/actions` `{"action": "start"}` | Owner, admin, staff |
| `servers stop <server>` | same, `stop` | Owner, admin, staff |
| `servers rename <server> <new-name>` | same, `edit` + `name` | Owner, admin, staff |
| `servers resize <server> [--cores N] [--memory-mb N] [--disk-gb N]` | same, `edit` + sizes | Owner / admin; a Market server is charged one month at the new size (402 without credit, at most once every 2 min) |
| `servers delete <server> --yes` | same, `delete` | Owner / admin; the server must be stopped; unused paid time is refunded |
| `containers list [--project P]` | `GET /api/v1/containers` | |
| `containers start <container>` | `POST /api/v1/containers/{id}/start` | Owner / admin |
| `containers stop <container>` | `POST /api/v1/containers/{id}/stop` | Owner / admin |
| `containers delete <container> --yes` | `DELETE /api/v1/containers/{id}` | Owner / admin; cancels one not created yet |
| `containers logs <container> [--tail N]` | `GET /api/v1/containers/{id}/logs?tail=N` | Not for guests; default 200 lines |
| `dns list [--project P]` | `GET /api/v1/dns` | |

Not covered yet (see [roadmap.md](roadmap.md)): ordering servers / containers, DNS create / edit / delete, public
IPs, usage, bills, announcements, renewals.
