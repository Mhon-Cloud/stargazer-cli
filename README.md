# stargazer

The command line for **Stargazer**, the Mhon Cloud console: order and manage servers, containers, public IPs, DNS
names, SSH keys, security groups and backups, from a terminal, a script or an AI agent.

```sh
stargazer containers create site --image nginx --upload ./public:/usr/share/nginx/html --wait
stargazer servers create web-1 --os ubuntu24.04 --plan starter --ssh-key laptop --yes --wait
stargazer dns create blog --container site
stargazer servers list
stargazer credit
```

## Install

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.sh | sh
```

It downloads the latest release for your system, checks its SHA-256 and puts `stargazer` in `~/.local/bin`
(`STARGAZER_INSTALL_DIR` to change it, `STARGAZER_VERSION=v0.1.0` for a given version).

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.ps1 | iex
```

**From source** (Rust 1.88+): `cargo install --git https://github.com/Mhon-Cloud/stargazer-cli`

Or download an archive from [Releases](https://github.com/Mhon-Cloud/stargazer-cli/releases).

## Uninstall

```sh
curl -fsSL https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.sh | sh -s -- --uninstall
```

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.ps1))) -Uninstall
```

It removes the binary (and on Windows its PATH entry) and the saved key / config. Revoke the key itself in the console
(Inventory → Interfaces → CLI). Installed with cargo? `cargo uninstall stargazer-cli`.

## Sign in

Create an API key in the console (**Inventory → Interfaces → CLI**), then:

```sh
stargazer login                        # paste the key (it isn't shown)
echo "$KEY" | stargazer login          # or pipe it
export STARGAZER_API_KEY=mhc_...       # or just set it (CI, agents)
```

The key is saved in `~/.config/stargazer/config.toml`, readable only by you. A key can do what you can do in the
console's Inventory and Market, but never top up, withdraw, or change your account.

## Commands

| Command | What it does |
|---|---|
| `stargazer login` / `logout` / `whoami` / `credit` | Sign in, the account, your credit |
| `stargazer servers create\|options\|orders` | Order virtual machines (`--dry-run` prices, `--yes` pays) |
| `stargazer servers list\|show\|start\|stop\|rename\|resize\|delete\|renew\|auto-renew` | Manage them |
| `stargazer containers create\|images\|options` | Order containers (free within 1 vCPU / 1 GB / 5 GB) |
| `stargazer containers list\|show\|start\|stop\|logs\|delete` | Manage them |
| `stargazer dns list\|check\|create\|update\|delete` | `<name>.in.mhon.cloud` names for container ports |
| `stargazer public-ips list\|quote\|reserve\|bind\|unbind\|release` | Reserved public IPv4 addresses |
| `stargazer ssh-keys` / `security-groups` / `backups` | SSH keys, firewall rules, server backups |
| `stargazer projects` / `zones` | Projects (create, rename, delete) and locations |
| `stargazer bills` / `inbox` / `usage` / `quotas` / `pricing` | Bills, messages, usage graphs' data, limits, prices |
| `stargazer completions <shell>` | Shell completion (bash, zsh, fish, powershell, elvish) |

Everything takes names instead of ids: `stargazer servers start web --project shop`. Ordering means you accept the
[Terms of Service](https://mhon.cloud/terms). Every command has `--help`. Full reference: [docs/commands.md](docs/commands.md).
Every command has `--help`. Full reference: [docs/commands.md](docs/commands.md).

## For scripts and AI agents

- **Output is JSON when stdout isn't a terminal** (a table on a terminal); force it with `--output json`. Lists are a
  JSON array of the API's objects.
- **Errors** go to stderr as `{"error": {"code": "not_found", "message": "...", "status": 404}}`.
- **Exit codes:** 0 ok · 1 internal · 2 usage · 3 auth · 4 not found · 5 not enough credit · 6 conflict ·
  7 rate limited · 8 network · 9 timed out (`--wait`).
- **It never prompts** (except `login` on a terminal). Spending credit or destroying something needs `--yes`.
- **`--wait`** blocks until an order / action is done; **`--fields id,name`** trims the JSON.
- Settings by environment: `STARGAZER_API_KEY`, `STARGAZER_OUTPUT`, `STARGAZER_API_URL`, `STARGAZER_CONFIG_DIR`.

Details: [docs/output-and-errors.md](docs/output-and-errors.md).

## Developing

See [AGENTS.md](AGENTS.md) (written for AI agents and people alike): layout, rules, how to add a command, tests.
Next work is listed in [docs/roadmap.md](docs/roadmap.md).
