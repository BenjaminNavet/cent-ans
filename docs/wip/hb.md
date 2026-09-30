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
- Fait : `tools/cent_ans_tools/geo/biomes.py` + `cent-ans geo biomes` → `data/map/biomes.png` (L8 7168×6144, 0,3 Mo) ; légende et seuils `data/map/biomes.yaml` (schéma `biomes.schema.json`) ; `map.json.biomes`.
- Source : Köppen-Geiger 1 km Beck et al. 2023 (CC BY 4.0), période 1901-1930, archive dans `tools/geo/raw/koppen/` (hors dépôt). Règles : océanique ≤ 150 km de l'Atlantique ouvert (Paris, Beauce, Picardie continentaux) ; Dxb ≥ 59,5° N boréal ; BS/BW steppe au nord de 44° N, semi-aride au sud sauf BS côtier méditerranéen (Attique) ; sécheresse `landcover.dryness` + bruit (lisière irrégulière) → steppe pontique / plateau anatolien ; altitude (1150 m à 46° N, −22 m/°, +700 m en climat sec) → montagnard. Lissage gaussien 5 km, îlots < 300 km² fondus, pixels côtiers isolés retirés. Repli par règles sans source (testé).
- Parts des terres : océanique 4,5 %, continental 32,9, méditerranéen 9,5, steppe 8,5, boréal 17,2, montagnard 5,6, semi-aride 21,8.
- Carte de couleur : section `biomes` de `colormap_style.yaml` (palettes, garrigue en taches, forêts gardées près des rivières en steppe, cultures/prés par biome tirés par bloc, multiplicateurs bocage/openfield/vigne/présence), poids fondus sur 25 km. Re-cuite (pic mémoire ~11,6 Go).
- Tests : `tools/tests/test_biomes.py` (38, dont 21 lieux réels : Athènes/Péloponnèse = 3, Rostov/Volgograd = 4…), `test_colormap.py` inchangé et vert.
- Suite (retours) : lisière forêt-steppe = score (limite historique 35 % + Köppen BSk/Dfa/Dfb flouté 60 km 65 %, à l'est de 24-28° E) + bruit 300/80/25 km → lisière sinueuse, bosquets en îlots (écart à la droite ~1,1° contre 0,36°) ; carte de couleur : `dry_max` par biome (méditerranéen 0,35, semi-aride 0,5, montagnard 0,6), zones arides de `dryness` gardées → Tell olive, hauts plateaux intermédiaires, Sahara sable. Tests 56 verts.
- Points ouverts : terrasses méditerranéennes non dessinées ; Dobroudja semi-aride (BSk intérieur au sud de 44° N).
