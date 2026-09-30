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

## HB5 rochers (branche `feat/hb-rocks`, worktree `../gp-hb-rocks`)
État : catalogue `data/art/rock_outcrops.yaml` + schéma + pytest ; génération fal lancée
(`tools/blender_scripts/hb_rock_outcrops.py fal`, brutes `~/dev/cent-ans-raw/hb/rocks/`) ;
couche `game/scripts/map/rock_outcrops.gd` + shader `rock_outcrops.gdshader` écrits, branchés dans
`campaign_map.gd` ; test `game/tests/hb5_rocks_test.gd`.
Prochaine étape : `hb_rock_outcrops.py cleanup` (LOD 400/200/60), import Godot, test hb5 + smoke,
mesures, budget.
