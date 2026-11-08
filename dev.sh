#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

# Override with PYTHON=python3.12 if needed (Python 3.12+ is required).
"${PYTHON:-python3.13}" tools/bootstrap.py
exec cargo run --locked --bin codegraph -- "$@"
