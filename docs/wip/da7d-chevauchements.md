# DA7d — Chevauchements des marqueurs de ville (régions denses)

Lot DA7d de `docs/wip/da-direction-artistique.md` (vague DA7). ADR 0066 (section DA7d à ajouter).
Branche `feat/da7d-marker-overlap` (worktree agent). 0 $.

## État
- Squelette : `game/scripts/map/marker_declutter.gd` (placement glouton + grille spatiale),
  `SettlementLayer.screen_occupancy` (mesure), bloc `declutter` de
  `data/map/settlement_markers.json` + schéma, test `game/tests/da7d_overlap_test.gd`
  (`-- --measure` = mesures sans échec).

## Prochaine étape
1. Mesures « avant » (test `--measure`) + captures `docs/img/da7d/*_avant.jpg`.
2. Brancher `MarkerDeclutter` dans `SettlementLayer.declutter()` (marqueurs + noms, épinglés,
   recalcul sur mouvement net de caméra, fondu shader COLOR.b/COLOR.a).
3. Captures après, ADR 0066 § DA7d.
