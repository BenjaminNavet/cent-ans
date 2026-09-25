# CM2 — carte de campagne : vue stratégique parchemin, météo, lumière de fin de tour

Branche `worktree-agent-a7241565332549af6` (partie de `main` `ad08ea16`).

## Plan (un commit par lot)
1. Vue stratégique parchemin (fondu selon la distance caméra, zoom maximal).
2. Météo de campagne : `core/` (déterministe : graine, date, province), pont, rendu (pluie, neige,
   brouillard matinal, orages), `campaign_ambience.gd` lit la nouvelle source.
3. Lumière dorée pendant le tour de l'IA (si le temps le permet).

## État
- [ ] 0. Squelette
- [ ] 1. Parchemin
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
Squelette.
