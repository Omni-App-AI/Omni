#!/usr/bin/env bash
# ───────────────────────────────────────────────────────────────
# build.sh — Build and package an Omni extension
#
# Usage:
#   ./build.sh <extension-dir>        Build a specific extension
#   ./build.sh --all                  Build all extensions
#   ./build.sh word-tools --release   Build in release mode (default)
#   ./build.sh word-tools --debug     Build in debug mode
#
# Prerequisites:
#   rustup target add wasm32-wasip1
# ───────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILE="release"

# Parse flags
DIRS=()
for arg in "$@"; do
    case "$arg" in
        --debug)   PROFILE="debug" ;;
        --release) PROFILE="release" ;;
        --all)
            for d in "$SCRIPT_DIR"/*/; do
                [ -f "$d/Cargo.toml" ] && DIRS+=("$(basename "$d")")
            done
            ;;
        *)         DIRS+=("$arg") ;;
    esac
done

if [ ${#DIRS[@]} -eq 0 ]; then
    echo "Usage: $0 <extension-dir|--all> [--debug|--release]"
    echo ""
    echo "Available extensions:"
    for d in "$SCRIPT_DIR"/*/; do
        [ -f "$d/Cargo.toml" ] && echo "  $(basename "$d")"
    done
    exit 1
fi

for DIR_NAME in "${DIRS[@]}"; do
    EXT_DIR="$SCRIPT_DIR/$DIR_NAME"

    if [ ! -f "$EXT_DIR/Cargo.toml" ]; then
        echo "Error: $EXT_DIR/Cargo.toml not found"
        exit 1
    fi

    if [ ! -f "$EXT_DIR/omni-extension.toml" ]; then
        echo "Error: $EXT_DIR/omni-extension.toml not found"
        exit 1
    fi

    echo "╔══════════════════════════════════════════════════╗"
    echo "║  Building: $DIR_NAME ($PROFILE)"
    echo "╚══════════════════════════════════════════════════╝"

    # Build the WASM binary
    CARGO_FLAGS="--manifest-path=$EXT_DIR/Cargo.toml --target=wasm32-wasip1"
    if [ "$PROFILE" = "release" ]; then
        CARGO_FLAGS="$CARGO_FLAGS --release"
    fi

    cargo build $CARGO_FLAGS

    # Determine output file name from Cargo.toml package name
    PKG_NAME=$(grep '^name' "$EXT_DIR/Cargo.toml" | head -1 | sed 's/.*= *"//' | sed 's/".*//' | tr '-' '_')
    if [ "$PROFILE" = "release" ]; then
        WASM_SRC="$EXT_DIR/target/wasm32-wasip1/release/${PKG_NAME}.wasm"
    else
        WASM_SRC="$EXT_DIR/target/wasm32-wasip1/debug/${PKG_NAME}.wasm"
    fi

    if [ ! -f "$WASM_SRC" ]; then
        echo "Error: Expected WASM output not found at $WASM_SRC"
        exit 1
    fi

    # Read entrypoint from manifest
    ENTRYPOINT=$(grep 'entrypoint' "$EXT_DIR/omni-extension.toml" | sed 's/.*= *"//' | sed 's/".*//')
    WASM_DST="$EXT_DIR/$ENTRYPOINT"

    # Copy the compiled WASM to the extension directory
    cp "$WASM_SRC" "$WASM_DST"
    WASM_SIZE=$(wc -c < "$WASM_DST" | tr -d ' ')
    echo "  ✓ Compiled: $ENTRYPOINT ($WASM_SIZE bytes)"

    # Optimize with wasm-opt if available
    if command -v wasm-opt &>/dev/null && [ "$PROFILE" = "release" ]; then
        wasm-opt -Oz "$WASM_DST" -o "$WASM_DST"
        OPT_SIZE=$(wc -c < "$WASM_DST" | tr -d ' ')
        echo "  ✓ Optimized: $OPT_SIZE bytes (wasm-opt -Oz)"
    fi

    # Create a distributable package
    DIST_DIR="$EXT_DIR/dist"
    mkdir -p "$DIST_DIR"

    # Collect files for the package
    PACKAGE_FILES=("$EXT_DIR/omni-extension.toml" "$WASM_DST")

    # Include optional files if they exist
    for extra in README.md LICENSE icon.png; do
        [ -f "$EXT_DIR/$extra" ] && PACKAGE_FILES+=("$EXT_DIR/$extra")
    done

    # Create a tar.gz package (cross-platform)
    PACKAGE_NAME="${DIR_NAME}-$(grep 'version' "$EXT_DIR/omni-extension.toml" | head -1 | sed 's/.*= *"//' | sed 's/".*//')"
    TAR_FILE="$DIST_DIR/${PACKAGE_NAME}.tar.gz"

    tar -czf "$TAR_FILE" -C "$EXT_DIR" \
        omni-extension.toml \
        "$ENTRYPOINT" \
        $(for extra in README.md LICENSE icon.png; do [ -f "$EXT_DIR/$extra" ] && echo "$extra"; done) \
        2>/dev/null || true

    echo "  ✓ Packaged: dist/${PACKAGE_NAME}.tar.gz"
    echo ""
done

echo "Done! To install an extension:"
echo "  1. Copy the extension directory to ~/.omni/extensions/user/"
echo "  2. Or use: omni extension install <path-to-dir>"
