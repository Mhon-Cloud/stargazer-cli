#!/bin/sh
# Installs the stargazer CLI from GitHub Releases (macOS and Linux):
#   curl -fsSL https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.sh | sh
# STARGAZER_VERSION=v0.1.0 picks a release (default: the latest); STARGAZER_INSTALL_DIR the folder
# (default: ~/.local/bin). The archive's SHA-256 is checked before anything is installed.
# Uninstall (the binary and the saved key / config):
#   curl -fsSL https://raw.githubusercontent.com/Mhon-Cloud/stargazer-cli/main/install.sh | sh -s -- --uninstall
set -eu

REPO="Mhon-Cloud/stargazer-cli"
VERSION="${STARGAZER_VERSION:-latest}"
INSTALL_DIR="${STARGAZER_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
    echo "stargazer install: $1" >&2
    exit 1
}

if [ "${1:-}" = "--uninstall" ]; then
    bin="$INSTALL_DIR/stargazer"
    found="$(command -v stargazer 2>/dev/null || true)"
    [ -n "$found" ] && [ ! -e "$bin" ] && bin="$found"
    if [ "$(uname -s)" = "Darwin" ]; then
        config="$HOME/Library/Application Support/stargazer"
    else
        config="${XDG_CONFIG_HOME:-$HOME/.config}/stargazer"
    fi
    [ -n "${STARGAZER_CONFIG_DIR:-}" ] && config="$STARGAZER_CONFIG_DIR"
    if [ -e "$bin" ]; then rm -f "$bin" && echo "Removed $bin"; else echo "No stargazer binary at $bin"; fi
    if [ -e "$config" ]; then rm -rf "$config" && echo "Removed $config (your saved API key)"; fi
    echo "Done. Keys stay valid until you revoke them in the console (Inventory > Additional Services > CLI)."
    exit 0
fi

case "$(uname -s)" in
    Linux) os="unknown-linux-musl" ;;
    Darwin) os="apple-darwin" ;;
    *) fail "$(uname -s) isn't supported by this script; download a release from https://github.com/$REPO/releases" ;;
esac
case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) fail "$(uname -m) isn't supported yet; download a release from https://github.com/$REPO/releases" ;;
esac
asset="stargazer-$arch-$os.tar.gz"

if [ "$VERSION" = "latest" ]; then
    base="https://github.com/$REPO/releases/latest/download"
else
    base="https://github.com/$REPO/releases/download/$VERSION"
fi

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2" || fail "can't download $1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2" || fail "can't download $1"
    else
        fail "curl or wget is needed"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $asset ($VERSION)..."
download "$base/$asset" "$tmp/$asset"
download "$base/$asset.sha256" "$tmp/$asset.sha256"

expected="$(cut -d' ' -f1 <"$tmp/$asset.sha256")"
actual="$(sha256 "$tmp/$asset")"
[ "$expected" = "$actual" ] || fail "checksum mismatch for $asset; nothing was installed"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
cp "$tmp/stargazer" "$INSTALL_DIR/stargazer"
chmod 755 "$INSTALL_DIR/stargazer"

echo "Installed $("$INSTALL_DIR/stargazer" version --output table) to $INSTALL_DIR/stargazer"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "Add it to your PATH: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
echo "Next: stargazer login   (create a key in the console: Inventory > Additional Services > CLI)"
