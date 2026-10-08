# DN nature : inventaire, catalogue, eau, chaîne imposteurs (branche dn/nature)

## Inventaire de l'existant
**Campagne (arbres)** : 23 essences HB4 (ADR 0143) dans `data/art/tree_species.yaml` (compilé en
`tree_species.json`), une rangée par essence dans l'atlas partagé
`game/assets/textures/vegetation/ga3/ga3_impostors_{albedo,normal}.png` (8 vues x 256 px, 25 deg).
Essences : chêne pédonculé, hêtre, sapin pectiné, sycomore, châtaignier, bouleau, épicéa, pin
sylvestre / maritime / parasol / d'Alep / noir, chêne vert, olivier, cyprès, peuplier noir, saule
blanc, pommier, mélèze, chêne kermès, lentisque, genévrier, arbousier. Poids par biome
(1 océanique, 2 continental, 3 méditerranéen, 4 steppe, 5 boréal, 6 montagnard, 7 semi-aride) et
par rôle (massif, lisière, isolé, verger, ripisylve, garrigue). Cartes de feuilles
`ga3_leaf_cards.png`, touffe `ga3_grass_tuft.png`.
**Bataille** : `battle_trees.gd` génère des arbres procéduraux (profils `SPECIES`, rameaux de vraies
feuilles CC0 ambientCG `leaf_spray_<essence>.png` : chêne, hêtre, frêne, peuplier, saule, fruitier,
haie ; imposteurs pour 6 essences) ; `battle_vegetation.gd` = herbe en MultiMesh (`grass_tufts.png`
FA7, ambientCG) ; sols par biome `ground_biome_mix.json`. Aucune essence résineuse ni
méditerranéenne en bataille.
**Rochers** : 6 rochers HB `models/rocks/hb` (alpine_spire, granite_chaos, limestone_cliff,
mossy_erratic, red_sandstone, stratified_ridge) + 3 GA3 `rock_a..c`, `rock_outcrops.yaml`.
**Eau** : `water.gdshader` (mer), `river_water.gdshader` (fleuves, lacs en mode nappe),
`river_fine.gdshader` (réseau fin de près), `battle_water/sea/foam`, `river_bed/banks`. Détail
tuilable `water_detail.gdshaderinc` (ADR 0141) : les textures `game/assets/textures/water/`
n'avaient jamais été générées (budget NB2) : le détail était éteint.
**dn-ingest** (lot ingest) : sortie `game/assets/models/dn/<categorie>/`, pas encore branchée dans le jeu.

## Trous par biome
- Atlantique / continental : charme, frêne, aulne, tilleul, orme, if, saule têtard, haies (aubépine,
  prunellier, ronce, noisetier), arbre mort / souche (aucun en campagne).
- Méditerranéen : chêne-liège, buis, maquis mixte, lavande/thym (cartes).
- Montagnard / boréal : bruyère, genêt, sous-bois (mousse, fougère), éboulis.
- Steppe / désert : herbes sèches, alfa, tamaris ; palmier dattier + sous-bois d'oasis (aucun).
- Zones humides / littoral : roseaux, massettes, carex, oyat, ajonc, stacks et falaises.
- Bataille : toute la flore hors feuillus communs ; rochers de bataille quasi absents.

## Catalogue
`data/art/dn_catalog_nature.json` : 49 entrées (30 arbres, 11 arbustes, 8 rochers/falaises), format
dn_batch (`region` n'existe pas dans dn_batch : le biome est dans `target`). Classe d'ingest `tree`
(hauteur) ou `rock` (largeur). `dn_catalog_nature_cards.json` : 14 cartes herbes / roseaux /
fleurs, kind `texture`, hors dn_batch (Z-Image local + rembg, ou atlas ambientCG).
Branchement des nouvelles essences : une entrée de `tree_species.yaml` par arbre (biomes, rôles,
hauteur) puis chaîne imposteurs de `docs/pipeline-assets-3d.md` ; les arbustes vont aux rôles
`garrigue` / haies.

## Eau (fait)
`tools/cent_ans_tools/water_procedural.py` : textures tuilables **procédurales** (spectre FFT
périodique, étirées dans le sens du courant), pas de source tierce (ambientCG n'a pas d'eau
tuilable utile). Normales + albédo ; l'alpha de l'albédo porte des stries d'écume. Shader
`river_water` : stries d'écume le long des berges, turbidité (`data/fx/water_detail.json`,
`turbidity` 0,35 grand fleuve / 0,1 rivière) qui teinte vers le limon selon la profondeur. Mer et
océan reçoivent aussi leur détail (force du json, provisoire). Non fait : `river_fine`
(réseau fin) garde son bruit procédural.

## Reste / points ouverts
- Contrôle visuel de l'eau et réglage de `strength`/`scale`/`turbidity` (orchestrateur).
- `river_fine.gdshader` et eau de bataille sans détail tuilable.
- Entrées yaml des nouvelles essences, branchement des glb `models/dn/vegetation` en bataille.
