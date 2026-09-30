# HB — habillage de la carte par biomes

ADR 0143. Branche `feat/hb`, worktree `../gp-hb` (dylib copiée, lien `data/map/pyramid`). Autonomie totale ; budget fal.ai 8 $.
Demandes du joueur (30/09) : champs, forêts, roches, rivières ; fal.ai ; biomes (océanique, continental, méditerranéen, steppe…) ; essences variées (sapins, hêtres, érables, pins, pommiers…).

## Lots
Vague 1 (agents, chacun sa branche `feat/hb-*` et son worktree `../gp-hb-*`) :
- [ ] HB1 biomes : `cent-ans geo biomes` → `data/map/biomes.png` + `biomes.yaml` ; palettes par biome dans `colormap_style.yaml` ; re-cuisson colormap.
- [ ] HB2 matières de sol fal.ai : textures tuilables → tableau `game/assets/textures/terrain/hb_*` + catalogue `data/art/ground_materials.yaml`.
- [ ] HB4 essences : ~16 imposteurs (chaîne GA3 L2) + `data/art/tree_species.yaml` ; répartition par biome dans `vegetation.gd` (lit `biomes.png`).
- [ ] HB5 rochers : affleurements fal.ai + couche de pose.
Session principale :
- [ ] HB6 relief : exagération lointaine.
- [ ] HB3 shader : parcellaire de vue moyenne texturé, canopée, roche (après HB1+HB2).
- [ ] HB7 rivières : vérification de lisibilité (RC).
- [ ] HB8 captures, banc, docs.

## HB1 — biomes (branche `feat/hb-biomes`, worktree `../gp-hb-biomes`)
- État : `geo/biomes.py` implémenté (Köppen Beck 2023 1901-1930 + règles + lissage) ; `biomes.png` cuit (0,3 Mo) ; parts : océanique 4,5 %, continental 32,9, méditerranéen 9,5, steppe 8,5, boréal 17,2, montagnard 5,6, semi-aride 21,8. Source dans `tools/geo/raw/koppen/` (lien `tools/geo/raw` vers le dépôt principal).
- Palettes par biome : section `biomes` de `colormap_style.yaml` (+ schéma), `colormap.py` fond les palettes (poids gaussiens `transition_km`), garrigue en taches, forêts gardées près des rivières (steppe), cultures par biome tirées par bloc.
- Prochaine étape : tests `test_biomes.py`, re-cuisson colormap.
