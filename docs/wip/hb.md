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
- [ ] HB3 shader : parcellaire de vue moyenne texturé, canopée, roche (après HB1+HB2).
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

