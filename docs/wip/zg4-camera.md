# ZG4 — caméra rapprochée et exagération verticale dynamique (ADR 0036)

Branche `zg4-camera` (depuis `integration/zoom`, worktree d'agent). Liens symboliques non
versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib : cargo avec
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`, copie dans `game/bin/`.
Aucun changement Rust.

## État
- [x] `CloseCameraProfile` (`game/scripts/map/close_camera_profile.gd`, `game/resources/close_camera.tres`) :
  distance min par étage + champ adouci (distance exacte aux tuiles), exagération (courbe, paliers
  de 4 % + hystérésis), tangage rasant, visée relevée, near/far, garde au sol, crêtes.
- [x] `MapData.set_vertical_scale` + paramètre global `campaign_vertical_scale` (terrain, quadtree,
  fleuves, maquettes).
- [x] `TerrainBuilder.set_vertical_scale` : signal `vertical_scale_changed`, recalages après 180 ms de
  stabilité, étalés (`rescale_budget_ms`), `rescaling_vertical` ; AABB du quadtree remises à l'échelle.
- [x] `CampaignCamera` : `relief`, `ground_height`, `min_distance_at` par étage, point visé au sol,
  garde au sol + crêtes, tangage rasant, near/far.
- [x] Armées : sol fin + `reground()` ; échelle proportionnelle sous 12 unités ; plaques lointaines masquées.
- [x] `LandmarkModel` : hauteurs en mètres (pas de cuisson au changement d'échelle), cuisson étalée.
- [x] `ZoomTiers` : paliers VALLEY / SITE ; frontières, brouillard, étiquettes, routes commerciales,
  maquettes, rubans, ponts, moulins, navires ; arbres `campaign_prop_scale` ; pluie ; atmosphère de près.
- [x] Test `game/tests/zg4_camera_test.gd` (OK), ZG2 OK, fumée : même plantage qu'au départ
  (« Message queue out of memory », étape campagne, cause étrangère), aucune nouvelle erreur.
- [x] Banc : parcours « descente » (`--bench-descent-only`), mesures ci-dessous.
- [x] Docs `godot-map.md`, addendum ADR 0036.
- [x] Captures `docs/img/zg4/` (20 JPEG), mesures du banc complet (`docs/godot-map.md`).
- [ ] Fusion de `integration/zoom` (correctif 3a3c8a95 du quadtree) : `git merge` refusé par le
  classificateur de permissions dans ce worktree → à faire par l'orchestrateur (pas de conflit attendu :
  `relief_quadtree.gd` modifié à des endroits différents).

## Prochaine étape
Lot terminé, en attente de fusion (fusionner d'abord `integration/zoom` dans `zg4-camera`).
Suites : ZG5b (routes/fleuves à l'échelle réelle ; lit creusé 719 m/px visible de près), ZG6/VH4
(villes à l'échelle réelle, masquées au palier site), recalages des colonies/ponts encore ~250 ms par banc.

## Mesures (descente seule, `--bench-descent-only`, 1 440 × 900, charge notée)
| Essai | charge (1 min) | i/s | médiane ms | p99 ms | > 50 ms | recalages |
|---|---|---|---|---|---|---|
| dynamique, recalage à chaque palier (1ʳᵉ version) | 20-30 | 17,2 | 27,0 | 273 | 161 | 1 596 émissions, 1,2 s ; écouteur colonies 1,23 s |
| statique `--static-exaggeration` (1) | 34 | 35,6 | 24,7 | 122 | 53 | — |
| dynamique, recalage après 180 ms de stabilité | 16-20 | 25,5 | 27,8 | 257 | 99 | 534 émissions, 150 ms (max 4,1 ms/image) |
| dynamique, sans aucun recalage de calque | 17-19 | 26,9 | 30,2 | 154 | 105 | — |
| statique (2) | 16-18 | 26,4 | 31,6 | 171 | 109 | — |
| dynamique (3) | 15-16 | 24,9 | 29,5 | 266 | 116 | 497 émissions, 125 ms (max 3,5 ms/image) |

Maquettes (`LandmarkModel`) : ~35 cuissons par descente, ≈ 60 ms de calcul chacune, **≤ 4,3 ms par image**
(étalées) au lieu de 50-300 ms d'un bloc (ZG2) ; plus aucune cuisson déclenchée par l'échelle seule.
