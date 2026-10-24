#!/bin/sh
# Refreshes openapi/openapi.json from the live API (the platform publishes it without a key).
# Run it when the platform adds operations, then review the diff: it's the list of what the CLI may call.
set -eu
cd "$(dirname "$0")/.."
url="${STARGAZER_API_URL:-https://app.mhon.cloud}/api/v1/openapi.json"
curl -fsSL "$url" -o openapi/openapi.json.new
mv openapi/openapi.json.new openapi/openapi.json
echo "Updated openapi/openapi.json from $url"
git --no-pager diff --stat -- openapi/openapi.json || true
