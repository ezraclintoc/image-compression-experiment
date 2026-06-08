#!/usr/bin/env bash
set -euo pipefail

PNG_DIR="$(dirname "$0")/images/png"
PPM_DIR="$(dirname "$0")/images/ppm"

if ! command -v magick &>/dev/null; then
    echo "Error: ImageMagick not found. Install it first." >&2
    exit 1
fi

find "$PNG_DIR" -type f -name "*.png" | while read -r src; do
    rel="${src#"$PNG_DIR"/}"
    dst="$PPM_DIR/${rel%.png}.ppm"
    mkdir -p "$(dirname "$dst")"
    magick "$src" "$dst"
    echo "Converted: $rel"
done

echo "Done."
