#!/bin/zsh
cd /Users/jean_hubert/dev/gp-bm-merge8/client
source ./sccache-env.sh
unset CENT_ANS_MUTE
out=$1
cargo clippy --features dev --all-targets -- -D warnings > $out.clippy 2>&1; echo "clippy=$?" > $out.rc
cargo test --features dev > $out.test 2>&1; echo "test=$?" >> $out.rc
