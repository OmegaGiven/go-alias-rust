#!/bin/sh
# Builds a macOS .pkg installer for go-alias-rust.
# Run from the repo root after `cargo build --release`.
set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
ROOT=$(pwd)
STAGE=$(mktemp -d)

mkdir -p "$STAGE/usr/local/libexec/go-alias-rust"
mkdir -p "$STAGE/usr/local/share/go-alias-rust/defaults"
mkdir -p "$STAGE/Library/LaunchDaemons"

cp target/release/go_service "$STAGE/usr/local/libexec/go-alias-rust/"
cp -r static "$STAGE/usr/local/share/go-alias-rust/static"
cp packaging/linux/defaults/*.json "$STAGE/usr/local/share/go-alias-rust/defaults/"
cp packaging/macos/com.omegagiven.go-alias-rust.plist "$STAGE/Library/LaunchDaemons/"

pkgbuild \
    --root "$STAGE" \
    --identifier com.omegagiven.go-alias-rust \
    --version "$VERSION" \
    --scripts packaging/macos \
    --install-location / \
    "$ROOT/go-alias-rust-$VERSION.pkg"

rm -rf "$STAGE"
echo "Built go-alias-rust-$VERSION.pkg"
