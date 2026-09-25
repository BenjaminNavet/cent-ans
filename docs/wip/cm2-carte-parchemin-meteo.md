# CM2 — carte de campagne : vue stratégique parchemin, météo, lumière de fin de tour

Branche `worktree-agent-a7241565332549af6` (partie de `main` `ad08ea16`).

## Plan (un commit par lot)
1. Vue stratégique parchemin (fondu selon la distance caméra, zoom maximal).
2. Météo de campagne : `core/` (déterministe : graine, date, province), pont, rendu (pluie, neige,
   brouillard matinal, orages), `campaign_ambience.gd` lit la nouvelle source.
3. Lumière dorée pendant le tour de l'IA (si le temps le permet).

## État
- [x] 0. Squelette
- [x] 1. Parchemin : `strategic_view.gd` (poids 1080 → 1440, global `campaign_parchment`),
  `parchment_common/map/sea.gdshaderinc` (crochets `// CM2` dans terrain, water, river_water),
  `terrain_parchment.gdshader` (substitué au poids 1 : le rendu 3D n'est plus payé),
  `parchment_overlay.gd` (noms, vignettes pré-rendues, jetons, navires et monstres),
  `parchment_decor.gd` (roses, navires, monstres placés par la distance à la côte)
- [ ] 2. Météo
- [ ] 3. Lumière de fin de tour
- [ ] Mesures FPS (3 zooms), captures `docs/audit/captures/cm2/`, fusion de main, smoke

## Mesures de référence (avant CM2, Vulkan, 1600×900, `--fps-probe`, charge ≈ 30)
| Distance | i/s | GPU ms |
|---|---|---|
| 400 | 50,1 | 19,94 |
| 1000 | 98,5 | 6,38 |
| 1500 | 65,4 | 10,28 |

## Prochaine étape
Lot 2 : météo dans `core/` (module `weather.rs`), pont, rendu, AU1.
