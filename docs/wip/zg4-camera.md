# ZG4 — caméra rapprochée et exagération verticale dynamique (ADR 0036)

Branche `zg4-camera` (depuis `integration/zoom`, worktree d'agent). Liens symboliques non
versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib : cargo avec
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`, copie dans `game/bin/`.

## État
- [x] `CloseCameraProfile` (`game/scripts/map/close_camera_profile.gd`, `game/resources/close_camera.tres`) :
  distance min par étage + champ adouci (distance exacte aux tuiles), exagération (courbe, paliers
  de 4 % + hystérésis), tangage rasant, near/far, garde au sol.
- [x] `MapData.set_vertical_scale` + paramètre global `campaign_vertical_scale` (terrain, quadtree,
  fleuves, maquettes).
- [x] `TerrainBuilder.set_vertical_scale` : signal `vertical_scale_changed`, recalages étalés
  (`rescale_budget_ms`), `rescaling_vertical` ; AABB du quadtree remises à l'échelle.
- [x] `CampaignCamera` : `relief`, `ground_height`, `min_distance_at` par étage, point visé au sol,
  garde au sol, tangage rasant, near/far.
- [x] Armées : sol fin + `reground()` ; échelle proportionnelle sous 12 unités.
- [x] `LandmarkModel` : hauteurs en mètres (pas de cuisson au changement d'échelle), cuisson étalée.
- [x] `ZoomTiers` : paliers VALLEY / SITE, poids, `border_alpha`, `fog_alpha`.
- [x] Test `game/tests/zg4_camera_test.gd` (OK), ZG2 OK.
- [ ] Brancher les paliers (frontières, brouillard, étiquettes) + atmosphère de près.
- [ ] Mesure des écouteurs (`--bench-listeners`), réglage quadtree vue rasante, parcours « descente ».
- [ ] Captures `docs/img/zg4/`, docs `godot-map.md`, addendum ADR.

## Prochaine étape
Brancher `ZoomTiers` vallée/site dans `campaign_map._process` (matériau terrain : alphas des
frontières et du voile ; `SettlementLayer` : portée des étiquettes), puis atmosphère
(`campaign_atmosphere.gd` : brouillard/DOF/ombres de près), puis banc.

## Mesures
(à venir ; noter `uptime` à chaque mesure)
