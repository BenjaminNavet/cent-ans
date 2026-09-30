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

## HB2 — matières de sol fal.ai (branche `feat/hb-materials`, worktree `../gp-hb-mat`)
État : squelette (catalogue `data/art/ground_materials.yaml` 27 matières, schémas, module `tools/cent_ans_tools/ground_materials.py`, CLI `cent-ans assets ground-materials generate|seamless|pack`, chargeur `game/scripts/map/ground_materials.gd`, test `game/tests/hb2_materials_test.gd`).
Prochaine étape : tests pytest, génération flux-2-pro (≈ 0,03 $/image), raccord, pack, test headless.
Brutes : `~/dev/cent-ans-raw/hb/materials/` (planche `board_2x2.jpg`, mesures `tiles/report.json`).
