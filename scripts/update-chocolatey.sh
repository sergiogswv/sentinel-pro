#!/bin/bash
# Update Chocolatey package files

VERSION=$1
CHECKSUM=$2

if [ -z "$VERSION" ] || [ -z "$CHECKSUM" ]; then
    echo "Usage: $0 <version> <sha256_checksum>"
    exit 1
fi

INSTALL_SCRIPT="tools/chocolatey/tools/chocolateyinstall.ps1"
VERIFICATION_FILE="tools/VERIFICATION.txt"

# Update install script
sed -i "s/v[0-9]*\.[0-9]*\.[0-9]*/v$VERSION/g" "$INSTALL_SCRIPT"
sed -i "s/'PLACEHOLDER_CHECKSUM'/'$CHECKSUM'/g" "$INSTALL_SCRIPT"

# Update verification file
sed -i "s/sentinel-pro-[^:]*/sentinel-pro-${VERSION}/g" "$VERIFICATION_FILE"
sed -i "s/: .*/: $CHECKSUM/g" "$VERIFICATION_FILE"
sed -i "s|/v[0-9]*\.[0-9]*\.[0-9]*|/v$VERSION|g" "$VERIFICATION_FILE"

echo "Updated Chocolatey files with version $VERSION"
