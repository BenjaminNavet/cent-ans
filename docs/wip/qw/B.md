# QW-B — règles du joueur dans le code (10/10)

État : fait.
- TRELLIS 2 retiré : `dn_batch.py` (backend `fal2`, fonctions, CLI) rejeté par `validate_options` ; `dn_reingest.py`, `ga3_fal_decor.py`, `ga3_fal_figure.py` sans trellis-2 ; doc `pipeline-assets-3d.md` : « INTERDIT ». Journal de dépenses historique inchangé.
- Repli local : désactivé par défaut, `--local-fallback` l'active ; `--no-local-fallback` reste un no-op déprécié (anciennes commandes des notes).
- Meilleur-de-N refusé si `--image-backend fal` avec `--seeds`/`seeds` > 1 (catalogue compris) ; autorisé en local. `docs/wip/dn/local-retry.md` annoté ; `pipeline-assets-3d.md` §repli réécrit.
- Tests : `tools/tests/test_dn_batch_options.py`.
