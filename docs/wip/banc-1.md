# BANC-1 — binaire de banc et matrice de référence

Etat : FAIT sauf validation finale (fmt, clippy OK ; tests workspace en cours). Banc complet 29 s release.


Commande : `tools/heavy.sh cargo run --release -p sim-battle --example battle_bench` (options `--smoke`, `--filter`, `--threads`, `--out`).
Sortie : `out/bench/<date>.json` + résumé texte.

Référence : docs/bench/reference-2026-10-29.{json,txt}, résumé dans ADR 0798. Suite : BANC-2 (--emit-auto-fixture, data_hash) et BANC-3 (bandes), hors de ce lot.
