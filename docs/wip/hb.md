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
