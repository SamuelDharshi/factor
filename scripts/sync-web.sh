#!/usr/bin/env bash
# Copy deployed addresses into the web app (web/lib/deployments.json).
set -euo pipefail
cd "$(dirname "$0")/.."
[ -s deployments/testnet.json ] || { echo "run make deploy dao first"; exit 1; }
jq '{network, factor, musdc, dao, registry, vault_code_hash}' deployments/testnet.json > web/lib/deployments.json
echo "web/lib/deployments.json updated:"; cat web/lib/deployments.json
