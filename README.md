# stargazer

The command line for **Stargazer**, the Mhon Cloud console: manage your servers, containers and DNS names, and check
your credit, from a terminal, a script or an AI agent.

```sh
stargazer servers list
stargazer servers stop web
stargazer containers logs api --tail 50
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
(Inventory → Additional Services → CLI). Installed with cargo? `cargo uninstall stargazer-cli`.

## Sign in

Create an API key in the console (**Inventory → Additional Services → CLI**), then:

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
| `stargazer version` | The CLI's version |
| `stargazer login` / `logout` / `whoami` | Save or remove the key, show the account |
| `stargazer credit` | Your credit |
| `stargazer projects list` | Your projects and your role in each |
| `stargazer zones list` | Locations you can order in |
| `stargazer servers list\|start\|stop\|rename\|resize\|delete` | Virtual machines |
| `stargazer containers list\|start\|stop\|delete\|logs` | Containers |
| `stargazer dns list` | DNS names |
| `stargazer completions <shell>` | Shell completion (bash, zsh, fish, powershell, elvish) |

Servers and containers can be named instead of using their id: `stargazer servers start web --project shop`.
Every command has `--help`. Full reference: [docs/commands.md](docs/commands.md).

## For scripts and AI agents

- **Output is JSON when stdout isn't a terminal** (a table on a terminal); force it with `--output json`. Lists are a
  JSON array of the API's objects.
- **Errors** go to stderr as `{"error": {"code": "not_found", "message": "...", "status": 404}}`.
- **Exit codes:** 0 ok · 1 internal · 2 usage · 3 auth · 4 not found · 5 not enough credit · 6 conflict ·
  7 rate limited · 8 network.
- **It never prompts** (except `login` on a terminal). Deleting needs `--yes`.
- Settings by environment: `STARGAZER_API_KEY`, `STARGAZER_OUTPUT`, `STARGAZER_API_URL`, `STARGAZER_CONFIG_DIR`.

Details: [docs/output-and-errors.md](docs/output-and-errors.md).

## Developing

See [AGENTS.md](AGENTS.md) (written for AI agents and people alike): layout, rules, how to add a command, tests.
Next work is listed in [docs/roadmap.md](docs/roadmap.md).
