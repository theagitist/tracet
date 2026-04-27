#!/bin/bash
# Launch Tracet in development mode with live reload.
# Works around the broken nvm lazy-load shim by calling node/npm by full path.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Find a usable node binary, preferring the latest installed nvm version.
NODE_BIN=""
for v in $(ls -1 ~/.nvm/versions/node 2>/dev/null | sort -rV); do
    if [ -x "$HOME/.nvm/versions/node/$v/bin/node" ]; then
        NODE_BIN="$HOME/.nvm/versions/node/$v/bin"
        break
    fi
done

# Fall back to system node
if [ -z "$NODE_BIN" ]; then
    if command -v node >/dev/null 2>&1; then
        NODE_BIN="$(dirname "$(command -v node)")"
    else
        echo "ERROR: No node installation found" >&2
        exit 1
    fi
fi

echo "Using node from: $NODE_BIN"
"$NODE_BIN/node" --version

# Put node first on PATH so cargo-tauri's `npm run dev` finds it
export PATH="$NODE_BIN:$PATH"

# Install npm deps if missing
if [ ! -d node_modules ]; then
    echo "Installing npm dependencies..."
    "$NODE_BIN/npm" install
fi

echo "Launching Tracet in dev mode (Ctrl+C to stop)..."
exec cargo tauri dev "$@"
