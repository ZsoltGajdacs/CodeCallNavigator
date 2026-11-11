#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"
project_dir="$PWD"

cargo clean

"${PYTHON:-python3.13}" tools/bootstrap.py
"${PYTHON:-python3.13}" tools/package.py

# Keep the previous image until the replacement has been created successfully.
image_dir="$(mktemp -d "${TMPDIR:-/tmp}/codecallnavigator-dmg.XXXXXX")"
trap 'rm -rf "$image_dir"' EXIT
mkdir "$image_dir/contents"
/usr/bin/ditto dist/CodeCallNavigator.app "$image_dir/contents/CodeCallNavigator.app"
ln -s /Applications "$image_dir/contents/Applications"
/usr/bin/hdiutil create -volname CodeCallNavigator -srcfolder "$image_dir/contents" \
    -format UDZO "$image_dir/CodeCallNavigator.dmg"
mv -f "$image_dir/CodeCallNavigator.dmg" "$project_dir/dist/CodeCallNavigator.dmg"

printf '\nInstallable image: %s/dist/CodeCallNavigator.dmg\n' "$project_dir"
