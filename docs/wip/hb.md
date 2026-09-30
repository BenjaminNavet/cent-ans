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
État : **lot terminé** (non fusionné).
- Catalogue `data/art/tree_species.yaml` (23 essences : chêne, hêtre, sapin, érable, châtaignier,
  bouleau, épicéa, pins sylvestre/maritime/parasol/d'Alep/noir, chêne vert, olivier, cyprès,
  peuplier, saule, pommier, mélèze ; arbustes chêne kermès, lentisque, genévrier, arbousier),
  schéma `art_tree_species.schema.json`, compilation `data/art/tree_species.json` (Godot ne lit pas
  le YAML ; `ga3_vegetation_l2.py species`, pytest de synchronisation).
- Génération fal (chaîne GA3 L2 étendue, même cadre/graine/fond) : 2,42 $ (dont reprises chêne
  kermès et bouleau). Atlas GA3 **commun** 8 × 23 cellules 256² (lignes 0-2 inchangées) ; découpe
  des planches par panneau quand une vue est cassée (troncs fins des grands pins).
  Planche locale : `~/dev/cent-ans-raw/ga3/hb4/species_board.jpg`.
- Semis : rôle (cœur, lisière, ripisylve, verger, isolé/bosquet, garrigue) puis essence par
  biome × altitude × fleuve × part de résineux (forest_kind), tirage par peuplements de 3 px.
  Rust `vegetation::species` + `VegetationScatter.set_species` ; miroir GDScript `TreeSpecies` /
  `VegetationTileJob._species_candidate`. Biome : `VegetationMask.biome_at` (`biomes.png`, repli
  biome 2). Tous les paramètres dans le YAML. `--no-hb4-species` : semis V4 (A/B).
- Rendu : ligne d'atlas et classe de saison dans INSTANCE_CUSTOM (r/g + 4 × (n + 1)), nouvelle
  classe « or » (bouleau, érable, peuplier, saule, mélèze) ; même maillage, même matériau :
  **aucun appel de dessin de plus** (MMI d'imposteurs à d = 25 : 39 → 39 ; triangles −3 %).
- Tuile témoin (Orléans, 256², biome 2) : arbres 10 037 → 9 420 (−6 %), densité hors forêt et
  hors ripisylve 0,038 → 0,010 arbre/px² ; steppe 1 459 arbres ; ≥ 5 essences par biome.
- Tests : `hb4_species_test.gd`, `ga3_l2_vegetation_test.gd` (taille d'atlas lue du catalogue),
  smoke, fc2, sz1, sz4b, sz6, settlements OK ; pytest `test_tree_species.py` ; cargo `vegetation`.
Ouvert : jugement visuel en jeu (captures par la session principale) ; `biomes.png` réel (HB1)
non testé ici ; rangs des vergers non alignés (parcelles hachées) ; tailles/teintes à régler sur
capture (arbustes 0,3-0,7, olivier 0,7-1,0).
