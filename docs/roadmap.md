# Roadmap

Pick an item, follow "Adding a command" in [AGENTS.md](../AGENTS.md), tick it here in the same PR.

## Done in 0.2

- [x] The Market: `servers create` (quote, `--dry-run`, `--yes`), `servers orders list|cancel`, `containers create`
      (images by name, ports, variables, DNS name, Ubuntu login, startup script, folder uploads), `public-ips
      reserve|quote|bind|unbind|release`, `dns create|update|delete|check`
- [x] `servers show|renew|auto-renew`, `containers show|images|options`, `projects show|create|rename|delete`
- [x] `ssh-keys`, `security-groups`, `backups`, `bills`, `inbox`, `usage`, `quotas`, `pricing`
- [x] `--wait` / `--timeout` (exit 9), `--fields`

## Next: VPCs (when the platform enforces them)

The platform keeps VPCs as data for now (see its CLAUDE.md, "VPCs"). When the owner turns them on, the commands go in
`src/cli/network.rs` and a `src/commands/vpcs.rs`, one `Kind` each in `commands/common.rs`, every operation in
`openapi/contract.json`. Planned shape (AWS-like, matching the console's cards):

```
stargazer vpcs list|show|create|rename|delete           GET/POST /api/v1/vpcs, PUT/DELETE /api/v1/vpcs/{id}
stargazer vpcs subnets add|remove <vpc> ...             POST /api/v1/vpcs/{id}/subnets, DELETE .../subnets/{id}
stargazer vpcs gateway attach|detach|create <vpc>       /api/v1/vpcs/{id}/internet-gateway(s)
stargazer vpcs routes list|add|remove <vpc> <table>     /api/v1/vpcs/{id}/route-tables[/{id}/routes]
stargazer vpcs subnets route-table <vpc> <subnet> <t>   PUT .../subnets/{id}/route-table
```

and `--vpc` / `--subnet` on `servers create` / `containers create` once orders take them (the API ignores them today).
Those routes exist in the gateway but aren't in the public spec yet: add them to `gen_openapi.py` first.

## Later

- [ ] Project members (`projects members list|add|remove`), companies
- [ ] Support tickets (`tickets list|create|reply`)
- [ ] `containers download <container> <path>` (the console's Download: a zip of a mounted folder)
- [ ] `stargazer mcp`: an MCP server over stdio exposing the same operations as tools
- [ ] Idempotent creates (`--idempotency-key`, or return the existing item when the name matches)

## Distribution and security

- [ ] Homebrew tap (`Mhon-Cloud/homebrew-tap`) and Scoop bucket, updated by the release workflow
- [ ] `stargazer update` / a "new version available" notice (off in JSON mode)
- [ ] OS keychain for the key, file as fallback
- [ ] Browser login (`stargazer login --web`): needs a small new endpoint in the platform
- [ ] Windows on ARM, Linux armv7 builds

## Housekeeping

- [ ] `stargazer uninstall` (today: `install.sh --uninstall` / `install.ps1 -Uninstall`)
- [ ] A license (the owner's choice; until then the code is all rights reserved)
- [ ] Generate request / response types from `openapi/openapi.json` if hand-written bodies start to drift
