# BANC-1 — binaire de banc et matrice de référence

Etat : exemple `core/crates/sim-battle/examples/battle_bench.rs`, matrice `data/rules/battle_bench.json`
(schéma `data/schemas/battle_bench.schema.json`) écrits ; reste à compiler, lancer le banc complet, consigner la matrice.

Commande : `tools/heavy.sh cargo run --release -p sim-battle --example battle_bench` (options `--smoke`, `--filter`, `--threads`, `--out`).
Sortie : `out/bench/<date>.json` + résumé texte.

Prochaine étape : build, tests (`bench_smoke`, `bench_is_deterministic`), banc complet, ADR 0798, référence dans `docs/bench/`.
