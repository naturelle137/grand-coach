#!/usr/bin/env bash
# SPDX-License-Identifier: EUPL-1.2
# Run godot/tests/interaction.gd (DE and EN) headless with a pinned Godot.
# Usage: scripts/godot-headless.sh   (set GODOT=/path/to/godot to use your own;
# it must match the pinned version, otherwise the pinned build is downloaded
# into a temporary directory and its SHA-512 is verified first).
set -euo pipefail

VERSION=4.4.1-stable
# From https://github.com/godotengine/godot-builds/releases/download/4.4.1-stable/SHA512-SUMS.txt
SHA512=ef4e76880a514257175544952c61191106fdef3095b909bafed9fcbeb230c3e5533920a0f3012882dd4bbde83028a67549825794e2d2c3cf76eba7918b71370e
ARCHIVE=Godot_v${VERSION}_linux.x86_64.zip
URL=https://github.com/godotengine/godot-builds/releases/download/${VERSION}/${ARCHIVE}

cd "$(dirname "$0")/.."
grep -q "PackedStringArray(\"${VERSION%.*}\")" godot/project.godot || {
    echo "godot/project.godot features do not match pinned Godot $VERSION" >&2
    exit 1
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

godot=${GODOT:-}
if [ -z "$godot" ]; then
    curl -fsSL --retry 3 -o "$work/$ARCHIVE" "$URL"
    echo "$SHA512  $work/$ARCHIVE" | sha512sum -c -
    unzip -q "$work/$ARCHIVE" -d "$work/bin"
    godot=$work/bin/Godot_v${VERSION}_linux.x86_64
    chmod +x "$godot"
fi

cargo build --locked -p gui-godot
"$godot" --headless --editor --path godot --import --quit >"$work/import.log" 2>&1 || {
    cat "$work/import.log"
    exit 1
}
status=0
"$godot" --headless --path godot --script res://tests/interaction.gd >"$work/run.log" 2>&1 || status=$?
cat "$work/run.log"
# A failed assert prints SCRIPT ERROR but leaves the exit code at 0.
if [ "$status" -ne 0 ] || grep -q 'SCRIPT ERROR' "$work/run.log" ||
    ! grep -q '^Godot interaction: German and English' "$work/run.log"; then
    echo "Godot interaction test failed (exit $status)" >&2
    exit 1
fi
