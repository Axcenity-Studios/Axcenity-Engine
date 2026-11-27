#!/usr/bin/env bash
# Runs each test of one test binary in its own process (as nextest does),
# three at a time, so the editor, render and player binaries don't grow past
# the machine's memory (QA-019). For machines where nextest can't be installed.
# Usage: run-each.sh <test binary> <results file> <crate folder under crates/>
# The results file gets one "ok", "IGNORED" or "FAIL(code)" line a test, then
# ALLDONE; failing tests' last 40 lines go to <results file>.fail.
set -u
bin=$(realpath "$1"); out=$(realpath -m "$2"); dir=$3
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$root/crates/$dir"
export RUST_MIN_STACK=16777216 RUST_BACKTRACE=0
: > "$out"; : > "$out.fail"
one() {
  t=$1
  TMPDIR=$(mktemp -d); export TMPDIR
  log=$TMPDIR.log
  timeout 900 "$bin" --exact "$t" --test-threads=1 > "$log" 2>&1
  code=$?
  if [ $code -eq 0 ]; then
    if grep -q "1 ignored" "$log"; then echo "IGNORED $t"; else echo "ok $t"; fi
  else
    echo "FAIL($code) $t"; { echo "=== $t"; tail -40 "$log"; } >> "$out.fail"
  fi
  rm -rf "$TMPDIR" "$log"
}
export -f one; export bin out
"$bin" --list 2>/dev/null | grep ': test$' | sed 's/: test$//' \
  | xargs -P 3 -I{} bash -c 'one "$@"' _ {} >> "$out"
echo ALLDONE >> "$out"
