#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

cargo clean

# Override with PYTHON=python3.12 if needed (Python 3.12+ is required).
"${PYTHON:-python3.13}" tools/bootstrap.py
cargo build --locked -p codegraph-engine --bin codegraph-engine
exec cargo run --locked --bin codegraph -- "$@"
