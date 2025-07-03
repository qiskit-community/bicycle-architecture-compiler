#!/bin/sh

##
## This script runs: build, rustfmt, clippy, and test suite
##
## If a command succeeds, then output is non-verbose.
##
## If a command fails, then it will be run again with the `--verbose` flag,
## and the output is piped to `less`. In this case, quitting `less` with `q`
## will proceed to the next command.
##

set -e

run_command() {
  local cmd="$1"
  local verbose_cmd="$2"
  if $cmd; then
      echo "Success: $cmd"
  else
      echo "!!! Failed: $cmd"
      $verbose_cmd 2>&1 | less -R
  fi
}

run_command "cargo fmt -- --check" "cargo fmt -- --check --verbose"
run_command "cargo build --release" "cargo build --release --verbose"
run_command "cargo clippy -- -D warnings" "cargo clippy -- -D warnings --verbose"

if command -v cargo-nextest > /dev/null 2>&1; then
    run_command "cargo nextest run --release" "cargo nextest run --release -- --verbose"
else
    run_command "cargo test --release" "cargo test --release -- --verbose"
fi
