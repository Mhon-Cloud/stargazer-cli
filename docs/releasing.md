# Releasing

1. Bump `version` in `Cargo.toml` (semver; before 1.0 a minor bump may break things, say so in the notes), run
   `cargo build` so `Cargo.lock` follows, commit.
2. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.
3. `.github/workflows/release.yml` checks the tag matches `Cargo.toml`, builds every target, and publishes a GitHub
   release with generated notes and these assets (names have no version, so "latest" URLs work):

| Target | Asset |
|---|---|
| Linux x86_64 (static, musl) | `stargazer-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (static, musl) | `stargazer-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `stargazer-x86_64-apple-darwin.tar.gz` |
| macOS Apple silicon | `stargazer-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `stargazer-x86_64-pc-windows-msvc.zip` |

Each has a `<asset>.sha256` next to it; `install.sh` / `install.ps1` refuse an archive whose checksum doesn't match.

4. Check: `curl -fsSL https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.sh | sh` then
   `stargazer version`.

The macOS binaries aren't notarized yet; a binary downloaded by a browser may need
`xattr -d com.apple.quarantine stargazer` (the install script uses curl, which doesn't set the quarantine flag).
