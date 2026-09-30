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

## HB4 — essences et répartition par biome (branche `feat/hb-trees`, worktree `../gp-hb-trees`)
État :
- [x] Catalogue `data/art/tree_species.yaml` (23 essences dont 4 arbustes de maquis/garrigue) + schéma
  `art_tree_species.schema.json` + compilation `data/art/tree_species.json` (`ga3_vegetation_l2.py species`).
- [x] Script GA3 L2 étendu (`fal` sur tout le catalogue, `atlas` une ligne par essence, `board`).
- [ ] Génération fal (en cours), atlas commun, planche locale `~/dev/cent-ans-raw/ga3/hb4/species_board.jpg`.
- [ ] Semis : `TreeSpecies` (GDScript) + miroir Rust (`vegetation` crate, `set_species`), grille grossière biome.
- [ ] Shader : ligne d'atlas et classe de saison par instance (INSTANCE_CUSTOM r/g + 4 × (n + 1)).
- [ ] Tests `hb4_species_test.gd`, pytest `test_tree_species.py`, smoke, ga3_l2 ; budget.
Prochaine étape : Rust + branchement `vegetation.gd`.
