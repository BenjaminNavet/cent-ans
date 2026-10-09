# TX veg/build/eau — végétation, bâtiments, eau générés en jeu (lots T3 + T4 + eau)

Branche `tx-veg-build` (depuis 79e60b9ee + cherry-pick d2d289623). Spec : § 3 et § 4 de
`docs/superpowers/specs/2026-10-09-textures-regionales-design.md`. ADR 0241.

## État (tout fait, non fusionné)
- Fabrique : `cards.py` (détourage -> cartes RGBA), `tiles.py` (eau), `regions.py`, packs dans les catalogues.
- Bâtiments : paquet 80 couches (33 Mo), `building_regions.json` `materials`, `BuildingMaterials` + shaders
  `building_atlas` / `town_building` (région par matériau en bataille, par ville en campagne).
- Cartes au sol : `GroundCards` + `GroundClutter` (45 % des touffes). Herbe de bataille : `BattleGrassGroups`.
- Écorces/feuilles : `TreeTextures`, champ `textures` des essences, shaders battle_tree_*.
- Eau : `water_detail.json` (`tx`, `basins`), `water.gdshader` (détail par bassin), 7 surfaces 2k (20 Mo).
- Tests : pytest `test_tx_veg_build_water.py`, Godot `tx_veg_build_test.gd`, smoke OK.

## Points ouverts
- `BattleGrassGroups.biome_for` à remplacer par `BattleGroundTextures.biome_for` après fusion des sols de bataille.
- Maquettes lointaines (`town_far`, `maquette_kit`) et faubourgs partagés : atlas par défaut.
- Niveau 2k « haute » des paquets (hi/) non généré. Cartes au sol : caméra de campagne limitée à >= 7 u, le semis
  `GroundClutter` (portée 2,6 u) n'apparaît donc qu'avec la caméra de capture ; à régler côté caméra.
