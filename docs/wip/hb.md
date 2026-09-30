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
État : 27 matières générées (`fal-ai/flux-2-pro` 1024², 35 appels, 1,10 $), raccordées, empaquetées.
- Catalogue `data/art/ground_materials.yaml` (schéma `ground_materials.schema.json`) : 13 cultures, 5 canopées, 9 sols ; biomes ADR 0143 ; `attempt` = reprise retenue.
- Chaîne `tools/cent_ans_tools/ground_materials.py`, CLI `cent-ans assets ground-materials generate|seamless|pack` (`generate --dry-run` ; fal : `uv run --project tools --with fal-client python -c "from cent_ans_tools.cli import app; app()" assets ground-materials generate`).
  Raccord : aplanissement des dégradés (flou large en miroir), demi-tuile, bandes de couture remplacées entre deux coupes d'erreur minimale fermées (programmation dynamique), puis aplanissement de l'éclairage (flou périodique), luminance moyenne linéaire égalisée à 0,18, normale/rugosité depuis la luminance passe-haut.
- Tableaux `game/assets/textures/terrain/hb_ground_albedo_array.jpg` (grille 3×9 de 1024², 16,6 Mo) et `hb_ground_normal_array.jpg` (3×9 de 512², R,G normale OpenGL, B rugosité, 8 Mo), importés en `CompressedTexture2DArray` comme GA4.
- Manifeste `data/art/ground_materials_pack.json` (id → couche, `mean_linear`) ; chargeur `GroundMaterials.load_arrays()` (`game/scripts/map/ground_materials.gd`) ; non branché dans `terrain.gdshader` (HB3).
- Brutes et planche 2×2 : `~/dev/cent-ans-raw/hb/materials/` (`board_2x2.jpg`, mesures `tiles/report.json`).
Limites : blé et orge gardent des lignes de semis/traces faiblement visibles en 2×2 (orge : la reprise « nadir » donne un motif radial de drone, rejetée) ; seigle légèrement quadrillé ; la teinte d'origine reste dans l'albédo (seule la luminance est égalisée).
Prochaine étape : test headless `hb2_materials_test.gd`, puis jugement de la planche par la session principale.
