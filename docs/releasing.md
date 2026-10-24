# Releasing

1. Bump `version` in `Cargo.toml` (semver; before 1.0 a minor bump may break things, say so in the notes), run
   `cargo build` so `Cargo.lock` follows, commit.
2. Wait for **CI** to pass on that commit (it also builds every target, so the release can't hit a broken build).
3. Tag and push: `git tag v0.2.0 && git push origin v0.2.0` (`v0.2.0-rc.1` makes a pre-release, which "latest"
   download URLs skip).
4. `.github/workflows/release.yml` runs: the tag must match `Cargo.toml`, then the full test suite, then the build, then
   it publishes a GitHub release with generated notes, build provenance attestations, and these assets (names have no
   version, so "latest" URLs work):

| Target | Asset |
|---|---|
| Linux x86_64 (static, musl) | `stargazer-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (static, musl) | `stargazer-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `stargazer-x86_64-apple-darwin.tar.gz` |
| macOS Apple silicon | `stargazer-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `stargazer-x86_64-pc-windows-msvc.zip` |

Each has a `<asset>.sha256` next to it; `install.sh` / `install.ps1` refuse an archive whose checksum doesn't match.

5. The last job, **verify-install**, installs that release with `install.sh` (Linux x64, Linux ARM64, macOS) and
   `install.ps1` (Windows) and checks `stargazer version`. If it fails, the release is out but broken: fix it and
   release a patch version (don't move or reuse a tag).
6. Check from a machine of your own:
   `curl -fsSL https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.sh | sh` then `stargazer version`.

Verify a download came from this repo's workflow: `gh attestation verify stargazer-<target>.tar.gz -R Mhon-Cloud/stargazer-cli`.

The macOS binaries aren't notarized yet; a binary downloaded by a browser may need
`xattr -d com.apple.quarantine stargazer` (the install script uses curl, which doesn't set the quarantine flag).
