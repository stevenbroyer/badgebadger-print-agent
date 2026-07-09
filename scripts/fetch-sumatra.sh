#!/usr/bin/env bash
# Downloads SumatraPDF.exe into src-tauri/resources/ so `pnpm tauri build`
# bundles it into the Windows MSI. Sister script to fetch-sumatra.ps1 —
# the bash version exists for cross-builds running on macOS/Linux.

set -euo pipefail

VERSION="3.5.2"
# SHA-256 of the official SumatraPDF-3.5.2-64.exe — keep in sync with
# fetch-sumatra.ps1. Pinning stops a tampered/corrupted download from being
# bundled into every operator's install. Bump when VERSION changes;
# cross-check against sumatrapdfreader.org.
EXPECTED_SHA="290e4aa7ed64c728138711c011e89aab7aa48dbc1ae430371dc2be4100b92bf0"
URL="https://www.sumatrapdfreader.org/dl/rel/${VERSION}/SumatraPDF-${VERSION}-64.exe"
DEST="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources/SumatraPDF.exe"

sha256_of() {
  if command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | cut -d' ' -f1
  else sha256sum "$1" | cut -d' ' -f1; fi
}

assert_sha() {
  local actual; actual="$(sha256_of "$1")"
  if [[ "$actual" != "$EXPECTED_SHA" ]]; then
    rm -f "$1"
    echo "SumatraPDF SHA-256 mismatch: expected $EXPECTED_SHA got $actual — refusing to bundle an unverified binary." >&2
    exit 1
  fi
  echo "SHA-256 verified: $actual"
}

if [[ -f "$DEST" ]]; then
  assert_sha "$DEST"
  echo "SumatraPDF already present and verified at $DEST — skipping."
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
assert_sha "$DEST"
echo "Done."
