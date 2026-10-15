#!/bin/zsh
# Runs the client tests with cargo-nextest when installed, else with `cargo test` (same tests).
# Usage: client/tests/test.sh [cargo test / nextest arguments]   (default features: dev)
# Env:   NEXTEST_PROFILE=default|ci|smoke   (default: default; ci writes target/nextest/ci/junit.xml)
# Doc-tests are not run by nextest: they are run separately below (the client has none today).
set -u
client=${0:A:h:h}
cd "$client" || exit 1
profile=${NEXTEST_PROFILE:-default}
if cargo nextest --version >/dev/null 2>&1; then
	cargo nextest run --features dev --profile "$profile" "$@" || exit $?
	cargo test --features dev --doc 2>&1 | tail -1
else
	echo "cargo-nextest missing (cargo install cargo-nextest --locked): falling back to cargo test" >&2
	cargo test --features dev "$@"
fi
