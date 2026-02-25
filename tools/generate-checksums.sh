#!/bin/bash
# Generate SHA256 checksums for all binaries in a directory
# Usage: ./tools/generate-checksums.sh ./target/release

if [ -z "$1" ]; then
    echo "Usage: $0 <binary_directory>"
    exit 1
fi

BINARY_DIR="$1"

for binary in "$BINARY_DIR"/sentinel*; do
    if [ -f "$binary" ]; then
        sha256sum "$binary" >> "$BINARY_DIR/SHA256SUMS"
    fi
done

echo "Checksums written to $BINARY_DIR/SHA256SUMS"
