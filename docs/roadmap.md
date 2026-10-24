# Roadmap

Pick an item, follow "Adding a command" in [AGENTS.md](../AGENTS.md), tick it here in the same PR.

## Next: commands for the rest of the public API

- [ ] `servers renew <server>` (`POST /api/v1/servers/{id}/renew`), `servers auto-renew <server> on|off` (`PUT .../auto-renew`)
- [ ] `dns create|update|delete|check` (`POST /api/v1/dns`, `PUT|DELETE /api/v1/dns/{id}`, `GET /api/v1/dns/check`)
- [ ] `public-ips list` (`GET /api/v1/public-ips`)
- [ ] `usage` (`GET /api/v1/usage`: servers or containers, CPU / RAM / disk / network)
- [ ] `bills list|show` (`GET /api/v1/billings`, `/billings/{id}`), `invoices list|show`
- [ ] `inbox` (`GET /api/v1/announcements`)
- [ ] `quotas` (`GET /api/v1/quotas`), `pricing` (`GET /api/v1/public/pricing`, no key needed)
- [ ] Ordering: `servers order` (`/market/compute/options`, `/quote`, `/orders`), `orders list|cancel`,
      `containers order` (`/market/containers/options`, `/market/containers`). Show the quote and require `--yes`
      before paying.

## Agent ergonomics

- [ ] `--wait` on server / container actions: poll until `last_action` / `status` settles, with `--timeout`
- [ ] `--fields id,name,status` to trim JSON output (fewer tokens for agents)
- [ ] `stargazer mcp`: an MCP server over stdio exposing the same operations as tools
- [ ] Idempotent creates (`--idempotency-key`, or return the existing item when the name matches)

## Distribution and security

- [ ] Homebrew tap (`Mhon-Cloud/homebrew-tap`) and Scoop bucket, updated by the release workflow
- [ ] Signed releases (cosign / GitHub artifact attestations)
- [ ] `stargazer update` / a "new version available" notice (off in JSON mode)
- [ ] OS keychain for the key, file as fallback
- [ ] Browser login (`stargazer login --web`): needs a small new endpoint in the platform
- [ ] Windows on ARM, Linux armv7 builds

## Housekeeping

- [ ] `stargazer uninstall` (today: `install.sh --uninstall` / `install.ps1 -Uninstall`)
- [ ] A license (the owner's choice; until then the code is all rights reserved)
- [ ] Generate request / response types from `openapi/openapi.json` if hand-written bodies start to drift
