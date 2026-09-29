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

## Mesures (machine partagée, bruit ±1 s)
| étape | wall | total_ms | RSS max | statique |
|---|---|---|---|---|
| référence (8e31dbf) | 13,7-14,0 s | 11 220-11 490 | 2,91-3,20 Go | 2 098 Mo |
| croissance CV1 groupée | 10,8 s | 8 406 | | 2 099 Mo |

Décomposition de référence (wall 14,0) : avant `_ready` (scène + `_ready` des enfants) 1,8 s ;
données 0,9 ; terrain 2,2 ; décor 2,9 (colonies 1,6, rivières 1,1) ; `refresh_all` 4,0 dont
`CampaignLife.refresh` 3,5 (517 maquettes remplacées une à une, chacune suivie de passes sur
toutes les colonies : paires, masquage, hauteurs d'étiquettes) ; 1re image 0,7.
`relief_landcover` (fil de travail) : 2,3 s de décodage seul, mais posé à la 1re image (après
`_ready`), d'où les 7,8-9,7 s affichés.

## État
- [x] mesure de référence
- [x] `SettlementLayer.replace_models` (lot) : croissance 3 375 → 109 ms
- [ ] suite : avant `_ready`, terrain, colonies, rivières, mémoire (relief_shade)
