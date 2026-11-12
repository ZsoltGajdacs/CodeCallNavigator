#!/bin/bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
project_python="${PYTHON:-python3.13}"
if [[ -z "${PYTHON:-}" && -x "$project_dir/.venv-verification/bin/python" ]]; then
    project_python="$project_dir/.venv-verification/bin/python"
fi
exec "$project_python" "$project_dir/tools/verify.py" light "$@"
