#!/bin/bash
# Update Homebrew formula with new version and checksum

VERSION=$1
CHECKSUM=$2

if [ -z "$VERSION" ] || [ -z "$CHECKSUM" ]; then
    echo "Usage: $0 <version> <sha256_checksum>"
    exit 1
fi

FORMULA_FILE="tools/homebrew/sentinel-pro.rb"

# Update version
sed -i "s/version \".*\"/version \"$VERSION\"/" "$FORMULA_FILE"

# Update checksum
sed -i "s/sha256 \"PLACEHOLDER_SHA256\"/sha256 \"$CHECKSUM\"/" "$FORMULA_FILE"

# Update URL
sed -i "s|sentinel-pro-[^/]*/|sentinel-pro-${VERSION}-|g" "$FORMULA_FILE"

echo "Updated Homebrew formula with version $VERSION"
