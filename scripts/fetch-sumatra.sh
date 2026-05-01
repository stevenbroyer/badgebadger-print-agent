#!/usr/bin/env bash
# Downloads SumatraPDF.exe into src-tauri/resources/ so `pnpm tauri build`
# bundles it into the Windows MSI. Sister script to fetch-sumatra.ps1 —
# the bash version exists for cross-builds running on macOS/Linux.

set -euo pipefail

VERSION="3.5.2"
URL="https://www.sumatrapdfreader.org/dl/rel/${VERSION}/SumatraPDF-${VERSION}-64.exe"
DEST="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources/SumatraPDF.exe"

if [[ -f "$DEST" ]]; then
  echo "SumatraPDF already present at $DEST — skipping."
  exit 0
fi

mkdir -p "$(dirname "$DEST")"
echo "Fetching SumatraPDF ${VERSION} → ${DEST}"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$URL" -o "$DEST"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$URL" -O "$DEST"
else
  echo "neither curl nor wget is available; install one or fetch manually" >&2
  exit 1
fi
echo "Done."
