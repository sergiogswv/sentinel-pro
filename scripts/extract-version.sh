#!/bin/bash
# Extract version from Cargo.toml for use in scripts
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/version = "//' | sed 's/".*//')
echo "$VERSION"
