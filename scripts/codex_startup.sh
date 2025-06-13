#!/usr/bin/env bash
# Quick startup script for the OpenAI Codex environment.
# Fetches dependencies and performs a lightweight compilation
# to prevent timeouts during container startup.

set -euo pipefail

echo "Fetching dependencies..."
cargo fetch --locked

# Run a fast compilation step instead of a full release build
# This keeps startup time under the 10 minute limit.

echo "Running cargo check..."
cargo check --all-features

echo "Codex startup finished."
