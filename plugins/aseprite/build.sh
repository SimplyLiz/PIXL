#!/usr/bin/env bash
# Build the PIXL Aseprite extension package.
# Output: pixl-aseprite.aseprite-extension (a zip archive)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

OUT="pixl-aseprite.aseprite-extension"

# Remove old build
rm -f "$OUT"

# Package all plugin files into a zip (renamed to .aseprite-extension)
zip -r "$OUT" \
  package.json \
  plugin.lua \
  LICENSE \
  lib/ \
  commands/ \
  -x "*.DS_Store" -x "build.sh"

echo "Built: $OUT"
echo "Install: double-click the file or drag into Aseprite"
