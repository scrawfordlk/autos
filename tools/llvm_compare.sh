#!/usr/bin/env bash

set -u

if [ $# -ne 1 ]; then
  echo "Usage: $0 <file.ll>"
  exit 1
fi

ll_file="$1"

# Run autos
cargo run -- -e "$ll_file"
rc_autos=$?

# Run lli
lli "$ll_file"
rc_lli=$?

# Compile and run with clang
tmp_bin=$(mktemp)

clang "$ll_file" -o "$tmp_bin"
clang_rc=$?

if [ "$clang_rc" -ne 0 ]; then
  echo "clang compilation failed with exit code $clang_rc"
  rm -f "$tmp_bin"
  exit 1
fi

"$tmp_bin"
rc_clang=$?

rm -f "$tmp_bin"

# Compare all exit codes
if [ "$rc_autos" -eq "$rc_lli" ] && [ "$rc_lli" -eq "$rc_clang" ]; then
  echo "All exit codes match: $rc_autos"
  exit 0
else
  echo "Exit codes differ:"
  echo "  autos = $rc_autos"
  echo "  lli   = $rc_lli"
  echo "  clang = $rc_clang"
  exit 1
fi
