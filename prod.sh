#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"
project_dir="$PWD"

"${PYTHON:-python3.13}" tools/bootstrap.py
"${PYTHON:-python3.13}" tools/package.py

# Keep the previous image until the replacement has been created successfully.
image_dir="$(mktemp -d "${TMPDIR:-/tmp}/codegraph-dmg.XXXXXX")"
trap 'rm -rf "$image_dir"' EXIT
mkdir "$image_dir/contents"
/usr/bin/ditto dist/CodeGraph.app "$image_dir/contents/CodeGraph.app"
ln -s /Applications "$image_dir/contents/Applications"
/usr/bin/hdiutil create -volname CodeGraph -srcfolder "$image_dir/contents" \
    -format UDZO "$image_dir/CodeGraph.dmg"
mv -f "$image_dir/CodeGraph.dmg" "$project_dir/dist/CodeGraph.dmg"

printf '\nInstallable image: %s/dist/CodeGraph.dmg\n' "$project_dir"
