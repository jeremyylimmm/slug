#!/usr/bin/env bash
set -euo pipefail

# Run from the directory this script lives in, regardless of where it's invoked.
cd "$(dirname "$0")"

# Use the project's virtualenv where the `slime` Rust extension is installed.
source .venv/bin/activate

exec python3 slug.py
