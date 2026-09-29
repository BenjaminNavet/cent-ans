# OMR-R2 — chargement et mémoire de la carte de campagne

Lot R2 de `docs/wip/omr.md`. Worktree `../gp-omr-r2`, branche `feat/omr-r2`.
Cible : chargement réel ≤ 7,5 s (OM : 11,4 s), RSS max ≤ 2,2 Go (OM : 2,89 Go).

## Protocole
- Sonde `game/tests/r2_load_probe.gd` (réglages de test, `user://settings.cfg` intact) :
  `CENT_ANS_RELIEF_DIR=/Users/jean_hubert/dev/game_project/data/map /usr/bin/time -l godot --headless --path game --script res://tests/r2_load_probe.gd`
  `wall_ms` = instanciation → `load_ok` et `ReliefLandcover.pending()` faux ; RSS = `time -l`.
- dylib : `core/target/i1/libcent_ans.dylib` (aucun changement de `core/` depuis b9e8a069b).

## État
- [ ] mesure de référence
