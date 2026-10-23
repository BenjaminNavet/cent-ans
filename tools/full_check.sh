#!/bin/zsh
# Contrôle complet d'intégration (une fois au merge) : clippy + tests du cœur et du client.
# Usage : tools/full_check.sh [dossier de logs]   (lancer depuis la racine du dépôt/worktree ; smoke et seed bots à part)
root=${0:A:h:h}; cd $root || exit 1
H=$root/tools/heavy.sh
L=${1:-out/check}; mkdir -p $L; rm -f $L/status; export CARGO_INCREMENTAL=0
(cd core && cargo fmt --check && $H cargo clippy --workspace --all-targets -- -D warnings) > $L/core-clippy.log 2>&1; echo "core-clippy=$?" > $L/status
(cd core && $H cargo test --workspace) > $L/core-test.log 2>&1; echo "core-test=$?" >> $L/status
(cd client && source sccache-env.sh && $H cargo clippy --features dev --all-targets -- -D warnings) > $L/client-clippy.log 2>&1; echo "client-clippy=$?" >> $L/status
(cd client && source sccache-env.sh && $H cargo test --features dev) > $L/client-test.log 2>&1; echo "client-test=$?" >> $L/status
echo done >> $L/status
