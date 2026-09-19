#!/usr/bin/env bash
# pain ai — Release Artifact Signing Documentation & Helper
# NOTE: The signing private key MUST NEVER be committed to the repository.
#
# 1. To generate a new updater key pair:
#    tauri signer generate -w ~/.pain-ai/pain-ai.key
#
# 2. Set the private key in your secure CI/CD environment secrets:
#    export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.pain-ai/pain-ai.key)"
#    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<optional-password>"
#
# 3. The public key must be placed in `src-tauri/tauri.conf.json`:
#    "plugins": {
#      "updater": {
#        "pubkey": "<YOUR_PUBLIC_KEY>"
#      }
#    }
#
# 4. Sign an artifact manually if needed:
#    tauri signer sign -k ~/.pain-ai/pain-ai.key <path-to-bundle>

set -euo pipefail

echo "[SIGN] pain ai artifact signing guide"
if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  echo "[WARN] TAURI_SIGNING_PRIVATE_KEY is not set in environment."
  echo "[INFO] For local development builds, unsigned packages or dev signatures are used."
  exit 0
fi

echo "[INFO] TAURI_SIGNING_PRIVATE_KEY detected in environment. Ready for release build."
