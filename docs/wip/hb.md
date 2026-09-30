# HB — habillage de la carte par biomes

ADR 0143. Branche `feat/hb`, worktree `../gp-hb` (dylib copiée, lien `data/map/pyramid`). Autonomie totale ; budget fal.ai 8 $.
Demandes du joueur (30/09) : champs, forêts, roches, rivières ; fal.ai ; biomes (océanique, continental, méditerranéen, steppe…) ; essences variées (sapins, hêtres, érables, pins, pommiers…).

## Lots
Vague 1 (agents, chacun sa branche `feat/hb-*` et son worktree `../gp-hb-*`) :
- [x] HB1 biomes (fusionné aef687bc4 ; suite demandée : lisière de steppe rectiligne, Tell maghrébin) : `cent-ans geo biomes` → `data/map/biomes.png` + `biomes.yaml` ; palettes par biome dans `colormap_style.yaml` ; re-cuisson colormap.
- [ ] HB2 matières de sol fal.ai : textures tuilables → tableau `game/assets/textures/terrain/hb_*` + catalogue `data/art/ground_materials.yaml`.
- [ ] HB4 essences : ~16 imposteurs (chaîne GA3 L2) + `data/art/tree_species.yaml` ; répartition par biome dans `vegetation.gd` (lit `biomes.png`).
- [ ] HB5 rochers : affleurements fal.ai + couche de pose.
Session principale :
- [x] HB6 relief : `relief_exaggeration.tres` gain local −0,4 (loin) / −0,1 (près) = collines aplanies au-dessus du fond, écrasement des montagnes dès la vue stratégique (`mountain_squash_far` 1). Caméra inchangée (`exaggeration_far` 4,31 : les échelles cuites en dépendent ; le baisser inverse l'écrasement). Tests ZG8 réécrits selon l'ADR 0143.
- [x] Brouillard de guerre : assombri (`fog_tone` 0,78, désaturation 0,4, voile 0,1) au lieu du voile clair qui pâlissait tout le territoire non vu.
- [x] HB3 shader : `hb_ground.gdshaderinc` + `HbGround` (`hb_ground.gd`), mélange par biome `data/art/ground_biome_mix.json` (parcelles, lanières, haies, présence agricole, cultures pondérées, sols sauvages, canopée, roche, teinte). Pièges corrigés : valeurs de table > 1 bornées (table normalisée dans [0, 1]) ; rotation des lanières sur coordonnées absolues = tourbillons (angle constant par région, rotation autour de l'origine de région) ; parcelles réelles (230-900 m) illisibles : `hb_cell_scale` 3. Diagnostic `hb_debug` 1-13 (1 biome, 2 parcelles, 4 couleur brute, 5 couches, 8 grille carte, 9-13 sondes après chaque passe) sortie directe dans ALBEDO.
- [x] Réglages de lisibilité : teinte de faction de près 0,10 → 0,04, brume météo 0,07, brouillard de guerre désaturation 0,25 / ton 0,72 ; steppe re-teintée vert-doré (colormap re-cuite).
- [x] Visite des biomes (captures `--param=fog_enabled=false --param=weather_enabled=false`) : Normandie parcelles, Provence verte et en damier, Grèce garrigue, steppe dorée, Finlande boréale, Alpes alpages + roche + neige.
- [ ] HB7 rivières (agent, `feat/hb-rivers`, captures autorisées ≤ 15) : rivières = traits gris d’un pixel ; `sky_reflect` 0,25 et `major_min_px` 3,2 déjà posés.
- [ ] HB8 captures, banc, docs.

## HB1 — biomes (branche `feat/hb-biomes`, worktree `../gp-hb-biomes`)
- Fait : `tools/cent_ans_tools/geo/biomes.py` + `cent-ans geo biomes` → `data/map/biomes.png` (L8 7168×6144, 0,3 Mo) ; légende et seuils `data/map/biomes.yaml` (schéma `biomes.schema.json`) ; `map.json.biomes`.
- Source : Köppen-Geiger 1 km Beck et al. 2023 (CC BY 4.0), période 1901-1930, archive dans `tools/geo/raw/koppen/` (hors dépôt). Règles : océanique ≤ 150 km de l'Atlantique ouvert (Paris, Beauce, Picardie continentaux) ; Dxb ≥ 59,5° N boréal ; BS/BW steppe au nord de 44° N, semi-aride au sud sauf BS côtier méditerranéen (Attique) ; sécheresse `landcover.dryness` + bruit (lisière irrégulière) → steppe pontique / plateau anatolien ; altitude (1150 m à 46° N, −22 m/°, +700 m en climat sec) → montagnard. Lissage gaussien 5 km, îlots < 300 km² fondus, pixels côtiers isolés retirés. Repli par règles sans source (testé).
- Parts des terres : océanique 4,5 %, continental 32,9, méditerranéen 9,5, steppe 8,5, boréal 17,2, montagnard 5,6, semi-aride 21,8.
- Carte de couleur : section `biomes` de `colormap_style.yaml` (palettes, garrigue en taches, forêts gardées près des rivières en steppe, cultures/prés par biome tirés par bloc, multiplicateurs bocage/openfield/vigne/présence), poids fondus sur 25 km. Re-cuite (pic mémoire ~11,6 Go).
- Tests : `tools/tests/test_biomes.py` (38, dont 21 lieux réels : Athènes/Péloponnèse = 3, Rostov/Volgograd = 4…), `test_colormap.py` inchangé et vert.
- Points ouverts : lisière ouest de la steppe droite vers 26-28° E (héritée de `landcover.dryness`) ; terrasses méditerranéennes non dessinées ; Dobroudja semi-aride (BSk intérieur au sud de 44° N).

## HB5 rochers (branche `feat/hb-rocks`, worktree `../gp-hb-rocks`)
État : terminé, non fusionné.
- Modèles (0,30 $, aucune reprise) : `game/assets/models/rocks/hb/<id>_lod{0,1,2}.glb`, falaise
  calcaire, chaos de granit, barre stratifiée, aiguille alpine, grès rouge, blocs erratiques
  moussus. `tools/blender_scripts/hb_rock_outcrops.py fal|cleanup` (brutes
  `~/dev/cent-ans-raw/hb/rocks/`) ; formes à aiguilles (collapse Blender calé à ~1 200 triangles)
  par `hb_rock_lods.py` (remaillage voxel grossier par LOD, `voxel_divs`). Triangles : 278-542 /
  137-231 / 45-60.
- Catalogue `data/art/rock_outcrops.yaml` (syntaxe de flux, lu par Godot sans les lignes `#`) +
  `data/schemas/art_rock_outcrops.schema.json` + `tools/tests/test_art_rock_outcrops_schema.py`.
- Couche `RockOutcrops` (`game/scripts/map/rock_outcrops.gd`, shader `rock_outcrops.gdshader`),
  branchée dans `campaign_map.gd` après `GroundClutter`. Tuiles de 256 u (une par tuile de
  terrain), semis dans le `WorkerThreadPool` (pré-examen par blocs de 16 u), un MultiMesh par
  modèle et par tuile ; biome lu dans `biomes.png` (repli altitude / position tant que HB1 n'est
  pas fusionné) ; pied au plus bas de l'emprise sur la surface affichée, recalage sur
  `chunk_surface_changed` ; taille réelle de près, ×2,6 au-delà de d = 110, hauteur ×
  exagération^0,5 ; falaises et barres posées le long des courbes de niveau ; effacement
  950 → 1200. `--no-outcrops` pour les A/B.
- Test `game/tests/hb5_rocks_test.gd` OK (Mont-Blanc 223, Écrins 257, Sancy 59, Cantal 51 dans
  r = 40 ; Beauce et golfe de Gascogne 0 ; 60 pieds vérifiés, écart max 0,8 u) ; smoke OK.
- Mesures (Alpes, test headless) : d = 60 : 3 572 instances, 529 k triangles, 25 appels ;
  d = 250 : 6 631 / 366 k / 87 ; d = 400 : 8 976 / 498 k / 137 (plafond 900 k jamais atteint).
  `--fps-probe` 1280 × 720 A/B (machine chargée, FPS non significatifs) : d = 250 +147 k
  primitives, +52 appels ; d = 400 +223 k, +80 appels. Semis d'une tuile alpine ≤ 0,8 s (fil de
  travail), pose ≤ 0,6 ms (fil principal).
Ouvert : jugement visuel (aucune capture faite) ; réglage de densité et du grossissement une fois
`biomes.png` (HB1) fusionné ; ombres limitées au LOD0.

## HB2 — matières de sol fal.ai (branche `feat/hb-materials`, worktree `../gp-hb-mat`)
État : 27 matières générées (`fal-ai/flux-2-pro` 1024², 35 appels, 1,10 $), raccordées, empaquetées.
- Catalogue `data/art/ground_materials.yaml` (schéma `ground_materials.schema.json`) : 13 cultures, 5 canopées, 9 sols ; biomes ADR 0143 ; `attempt` = reprise retenue.
- Chaîne `tools/cent_ans_tools/ground_materials.py`, CLI `cent-ans assets ground-materials generate|seamless|pack` (`generate --dry-run` ; fal : `uv run --project tools --with fal-client python -c "from cent_ans_tools.cli import app; app()" assets ground-materials generate`).
  Raccord : aplanissement des dégradés (flou large en miroir), demi-tuile, bandes de couture remplacées entre deux coupes d'erreur minimale fermées (programmation dynamique), puis aplanissement de l'éclairage (flou périodique), luminance moyenne linéaire égalisée à 0,18, normale/rugosité depuis la luminance passe-haut.
- Tableaux `game/assets/textures/terrain/hb_ground_albedo_array.jpg` (grille 3×9 de 1024², 16,6 Mo) et `hb_ground_normal_array.jpg` (3×9 de 512², R,G normale OpenGL, B rugosité, 8 Mo), importés en `CompressedTexture2DArray` comme GA4.
- Manifeste `data/art/ground_materials_pack.json` (id → couche, `mean_linear`) ; chargeur `GroundMaterials.load_arrays()` (`game/scripts/map/ground_materials.gd`) ; non branché dans `terrain.gdshader` (HB3).
- Brutes et planche 2×2 : `~/dev/cent-ans-raw/hb/materials/` (`board_2x2.jpg`, mesures `tiles/report.json`).
Limites : blé et orge gardent des lignes de semis/traces faiblement visibles en 2×2 (orge : la reprise « nadir » donne un motif radial de drone, rejetée) ; seigle légèrement quadrillé ; la teinte d'origine reste dans l'albédo (seule la luminance est égalisée).
Test headless `hb2_materials_test.gd` OK (27 couches, DXT1). Prochaine étape : jugement de la planche par la session principale.
- Suite (retours) : lisière forêt-steppe = score (limite historique 35 % + Köppen BSk/Dfa/Dfb flouté 60 km 65 %, à l'est de 24-28° E) + bruit 300/80/25 km → lisière sinueuse, bosquets en îlots (écart à la droite ~1,1° contre 0,36°) ; carte de couleur : `dry_max` par biome (méditerranéen 0,35, semi-aride 0,5, montagnard 0,6), zones arides de `dryness` gardées → Tell olive, hauts plateaux intermédiaires, Sahara sable. Tests 56 verts.
- Points ouverts : terrasses méditerranéennes non dessinées ; Dobroudja semi-aride (BSk intérieur au sud de 44° N).
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
### HB5 retouche visuelle (après fusion dans feat/hb, biomes.png présent)
- Couleur ramenée à la roche locale (`color` / `albedo_mean` par modèle, `max_albedo` 0,42),
  fondu tramé sous 4 px écran, groupes (bandes le long des courbes de niveau, amas), bruit de
  regroupement seuillé, atténuation sous forêt dense, base cisaillée sur la pente affichée
  (`INSTANCE_CUSTOM.zw`), grossissement lointain 5,5 (d ≥ 70), ombres jusqu'au LOD1, premier
  affichage attendu (comme `Vegetation`), `--outcrops-log`.
- Diagnostic (A/B en pixels, `--no-outcrops`) : les affleurements étaient bien dessinés mais de la
  taille des arbres grossis et de la couleur du sol (écart moyen < 15/255) : camouflés.
- Dernier état NON revu à l'image (budget de captures épuisé) : densité relevée
  (`max_probability` 0,6) après un essai trop clairsemé. Test hb5 OK (Mont-Blanc 228, Écrins 289,
  Sancy 30, Cantal 15 ; Beauce, mer 0) ; d = 60 : 2 828 visibles, 416 k triangles, 17 appels.
Ouvert : lisibilité alpine toujours insuffisante sur la dernière planche vue ; piste plus sûre :
roche au shader de terrain (HB3) sur pentes raides, les instances en appoint.

## HB7 — lisibilité des rivières (branche `feat/hb-rivers`, worktree `../gp-hb-rivers`)
- **Cause principale** : `world_per_px` des rubans (`river_water.gdshader`, `river_fine.gdshader`) prend `PROJECTION_MATRIX[1][1]`, **négatif sous Metal/Vulkan** (retournement de Y) : la largeur écran minimale (`min_px`) n'a jamais agi sur le Mac, rubans à leur largeur réelle (sous-pixel au-delà de ~50 unités) et `thin_fade` au plancher → trait gris. Corrigé par `abs()`. Même motif (non corrigé, hors lot) dans `road_line`, `road_fine`, `terrain_line`, `sea_lane`, `settlement_icon`.
- Causes secondaires : bord du ruban moyen normalisé par la largeur réelle (cœur seul opaque) ; palier fin sans réglage (`min_px` 1,2, élargissement dessiné en berge grise) ; eau trop sombre (fond gris-bleu 0,25/0,43/0,52, ciel gris).
- Corrections : élargissement = eau dans les deux paliers ; liseré/fondu du bord en pixels (dérivées écran) ; palette bleu-vert plus claire, reflet de ciel bleu ; `river_display.json` : `major_min_px` 4, facteurs par importance jusqu'à ×1,75 (Seine ≈ 7 px), `fine` (min_px 4, facteur par ordre de Strahler, mêmes couleurs) ; schéma complété (`sky_reflect` manquait).
- Mer d'Azov (demande du coordinateur) : `terrain.gdshader` peignait en « lac » sombre (et berge de vase) l'eau raster dont le MNT est ≥ 0 m → taches et bandes en escalier. Seuil `lake_level_m` = 2 m (aussi dans `water.gdshader`) et test `sea_nearby` (eau sous 0 m à 4 px alentour → mer). Pixels sombres dans la vue ÷ 6.
- Planches : `docs/research/hb7_rivieres_avant_apres.jpg` (gauche avant, droite après : Rouen 600/250/80, Orléans, Lyon, Vienne 250), `docs/research/hb7_azov_avant_apres.jpg`.
- Tests : `game/tests/hb7_river_width_test.gd` (largeur ≥ 6 px d'un fleuve majeur de 20 à 1200, relais fin/moyen, abs(), eau bleue) ; rc3, ss_lakes, zg5b, smoke verts.
- Reste : `git merge feat/hb` refusé par le classifieur d'autorisations (à faire par la session principale ; conflits attendus sur `river_display.json` et `terrain.gdshader` mineurs) ; bande d'eau peinte encore un peu plus sombre le long de la côte nord d'Azov (maillage LOD au-dessus du plan d'eau).
