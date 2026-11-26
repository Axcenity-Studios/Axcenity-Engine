#!/bin/sh
# Dumps what the engine draws of a project, for the Unity port to rebuild.
#
#   dump.sh <project folder> <output folder> [sequence, relative to the project]
#
# The build goes in the engine's target/shot-build, shared with the
# projects' own still-capture tool so the engine is compiled once.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
ENGINE=$(cd "$HERE/../../../.." && pwd)
BUILD="$ENGINE/target/shot-build"
mkdir -p "$BUILD/dump-crate"
sed -e "s|@ENGINE@|$ENGINE|g" -e "s|@SOURCE@|$HERE/src|g" "$HERE/Cargo.toml.in" > "$BUILD/dump-crate/Cargo.toml"
cp "$ENGINE/Cargo.lock" "$ENGINE/rust-toolchain.toml" "$BUILD/dump-crate/"
(cd "$BUILD/dump-crate" && CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$BUILD/cargo}" cargo build)
"${CARGO_TARGET_DIR:-$BUILD/cargo}/debug/homestead-dump" "$@"
