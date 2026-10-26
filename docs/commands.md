# Commands

Every command takes the global flags `--output auto|json|table`, `--fields a,b.c` (trim JSON output / pick table
columns) and `--api-url URL`. Arguments that name a thing take its **id or a name**: servers by name, containers by name
or number (`CT-000001`), SSH keys by name or fingerprint, public IPs by address, DNS names by name or full name, orders
by number. `--project` (id or name) narrows lists and name lookups; orders go to your personal **Default** project
unless `--project` says otherwise, and to the only location when there's one (`--zone` otherwise).

**What spends credit or can't be undone needs `--yes`**; without it the command does nothing and exits 2 saying what
would happen (the price included). `--wait` (with `--timeout SECONDS`, default 900; exit 9 when it runs out) blocks until
queued work is done. Every operation below is in `openapi/openapi.json` and `openapi/contract.json`.

## Account

| Command | API operation | Notes |
|---|---|---|
| `version` | none | `{"version", "os", "arch"}`; works without a config |
| `completions <shell>` | none | bash, zsh, fish, powershell, elvish |
| `login [--api-key-stdin]` | `GET /api/v1/me/credit` (checks the key) | Piped, or asked for without echo; saved 0600 |
| `logout` | none | Removes the saved key |
| `whoami` | `GET /api/v1/auth/me` | |
| `credit` | `GET /api/v1/me/credit` | |
| `bills list [--from --to]` / `show <id>` | `GET /api/v1/billings[/{id}]` | |
| `bills summary [--range 24h\|7d\|30d]` | `GET /api/v1/billings/summary` | Refunds subtracted |
| `bills invoices` / `invoice <id>` | `GET /api/v1/billings/invoices[/{id}]` | |
| `inbox list [--unread]` / `read <id>` / `dismiss <id>` | `GET /api/v1/announcements`, `POST .../{id}/read`, `DELETE .../{id}` | Order passwords arrive here |
| `usage [--scope servers\|containers] [--of NAME] [--range 1d\|7d\|30d]` | `GET /api/v1/usage` | |
| `quotas` | `GET /api/v1/quotas` | |
| `pricing` | `GET /api/v1/public/pricing` | No key needed |

## Projects and locations

| Command | API operation | Notes |
|---|---|---|
| `projects list` / `show <project>` | `GET /api/v1/projects` | |
| `projects create <name> [--description] [--kind personal\|company --company ID]` | `POST /api/v1/projects` | Free |
| `projects rename <project> <new-name> [--description]` | `PUT /api/v1/projects/{id}` | |
| `projects delete <project> --confirm <name> --yes` | `DELETE /api/v1/projects/{id}` | Deletes everything in it (servers, containers, DNS names...) |
| `zones list` | `GET /api/v1/availability-zones` | |

## Servers (virtual machines)

| Command | API operation | Notes |
|---|---|---|
| `servers list` / `show <server>` | `GET /api/v1/servers` | |
| `servers options` | `GET /api/v1/market/compute/options` | OS images, plans, rates, limits |
| `servers create <name> --os OS (--plan P \| --cpu N --ram GB [--disk ssd:50]...) [--public-ip \| --dns-label L] [--username U] [--password-stdin \| --no-password] [--ssh-key K]... [--backup daily@03:00] [--period 1\|6\|12] [--coupon C] [--dry-run] --yes [--wait]` | `POST /api/v1/market/compute/quote`, `POST /api/v1/market/compute/orders` | Prepaid. `--dry-run` = the quote only. A random password is made (printed once, and in your inbox) unless you give one |
| `servers orders list` | `GET /api/v1/market/compute/orders` | |
| `servers orders cancel <order> --yes` | `POST .../orders/{id}/cancel` | A waiting order only; full refund |
| `servers start\|stop <server> [--wait]` | `POST /api/v1/servers/{id}/actions` | Owner, admin, staff |
| `servers rename <server> <new-name> [--wait]` | same, `edit` + `name` | Owner, admin, staff |
| `servers resize <server> [--cores N] [--memory-mb N] [--disk-gb N] [--yes] [--wait]` | same, `edit` + sizes | Owner / admin; a Market server is charged a month at the new size (needs `--yes`) |
| `servers delete <server> --yes [--wait]` | same, `delete` | Stopped servers only; unused paid time refunded |
| `servers renew <server> --yes` | `POST /api/v1/servers/{id}/renew` | A suspended server: pays a month, starts it |
| `servers auto-renew <server> on\|off` | `PUT /api/v1/servers/{id}/auto-renew` | |

## Containers

| Command | API operation | Notes |
|---|---|---|
| `containers list` / `show <container>` | `GET /api/v1/containers` | |
| `containers images` | none | The images known by name, with ports and required variables |
| `containers options` | `GET /api/v1/market/containers/options` + compute options | Free plan left, plans, rates |
| `containers create <name> --image IMG [--plan P] [--cpu --ram --storage] [--port 80/http]... [-e K=V]... [--command C] [--dns-name N \| --no-dns] [--minecraft-srv BOOL] [--username U --password-stdin --startup-script FILE] [--upload DIR:/path]... [--period] [--yes] [--wait]` | `POST /api/v1/market/containers` (+ uploads) | Free within 1 vCPU / 1 GB / 5 GB (no `--yes`); else prepaid. A DNS name is made by default (web for an HTTP port, else direct, with a Minecraft SRV for 25565). Ubuntu gets a login (random password printed once) |
| `containers start\|stop <container> [--wait]` | `POST /api/v1/containers/{id}/start\|stop` | Owner / admin |
| `containers delete <container> --yes` | `DELETE /api/v1/containers/{id}` | Cancels one not created yet |
| `containers logs <container> [--tail N]` | `GET /api/v1/containers/{id}/logs` | Not for guests |

Uploads: `POST /api/v1/market/containers/uploads` → `PUT .../{id}?offset=N` (raw chunks, resumed after a dropped
connection) → `POST .../{id}/complete`. Folders and regular files only (symlinks are skipped and listed), up to 3
folders of 1 GB each.

## Network

| Command | API operation | Notes |
|---|---|---|
| `dns list` | `GET /api/v1/dns` | |
| `dns check <name>` | `GET /api/v1/dns/check` | |
| `dns create <name> --container C [--port N] [--kind web\|direct] [--minecraft-srv]` | `POST /api/v1/dns` | Free; the container's HTTP port by default |
| `dns update <record> --container C [...]` | `PUT /api/v1/dns/{id}` | Points it elsewhere; the name stays |
| `dns delete <record> --yes` | `DELETE /api/v1/dns/{id}` | |
| `public-ips list` / `quote` | `GET /api/v1/public-ips`, `GET /api/v1/market/public-ip/quote` | |
| `public-ips reserve [--project] [--zone] --yes` | `POST /api/v1/market/public-ip` | Monthly from your credit |
| `public-ips bind <ip> <server>` / `unbind <ip>` | `POST/DELETE /api/v1/public-ips/{id}/bind` | Applied after a restart |
| `public-ips release <ip> --yes` | `DELETE /api/v1/public-ips/{id}` | No refund |

## Security

| Command | API operation | Notes |
|---|---|---|
| `ssh-keys list` | `GET /api/v1/credentials` | |
| `ssh-keys add <name> --file KEY.pub [--project]` | `POST /api/v1/credentials` | `--file -` reads stdin; private keys are refused |
| `ssh-keys rename <key> <new-name>` / `delete <key> --yes` | `PUT/DELETE /api/v1/credentials/{id}` | |
| `ssh-keys install\|uninstall <key> <server>` | `POST /api/v1/credentials/{id}/bindings`, `DELETE .../bindings/{serverId}` | Applied at the next boot |
| `security-groups list` / `show <group>` | `GET /api/v1/security-groups` | |
| `security-groups create <name> [--project]` / `rename` / `delete --yes` | `POST`, `PUT/DELETE /api/v1/security-groups/{id}` | New groups: all outbound, no inbound |
| `security-groups allow <group> [--direction ingress\|egress] [--protocol tcp\|udp\|icmp\|all] [--port 22 \| 8000-8100] [--cidr C \| --peer G]` | `POST .../{id}/rules` | Default from 0.0.0.0/0 |
| `security-groups revoke <group> <rule-id>` | `DELETE .../{id}/rules/{ruleId}` | |
| `security-groups attach\|detach <group> <server>` | `POST .../{id}/attachments`, `DELETE .../attachments/{serverId}` | Applied at once |
| `backups list [--server S]` | `GET /api/v1/backups` | |
| `backups create <server> [--storage S] [--notes N]` | `POST /api/v1/servers/{id}/backups` | |
| `backups restore\|delete <server> <volid> --yes` | `POST .../backups/restore\|delete` | Restore needs the server stopped |

Not covered yet (see [roadmap.md](roadmap.md)): VPCs, companies and project members, support tickets, the VM console.
