# GA — Assets générés et CC0 haute résolution (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-ga-assets-generes-design.md` (726a31bf, approuvée).
Branche d'intégration : `feat/ga` (worktree `../game_project-ga`). Un worktree par lot,
fusion `--ff-only` dans `feat/ga`, puis `feat/ga` → `main` après jugement du joueur.
Budget : plafond 15 $, section « GA » de `docs/budget.md`.

## Consigne de reprise

> Lis ce fichier et `git log --oneline feat/ga -20`, fais les vérifications d'avant reprise,
> puis continue à la première case non cochée. N'ouvre pas les images sauf étape de jugement.

## Vérifications d'avant reprise

- `git worktree list` : ne toucher qu'aux worktrees `ga*`.
- Autres chantiers actifs (TW2, FE) : fichiers partagés à surveiller —
  `game/shaders/battle_soldier_skinned.gdshader`, `game/scripts/battle/battle_terrain.gd`,
  `game/shaders/terrain.gdshader`, `docs/budget.md`. Rebaser le lot sur `main` avant fusion.
- Solde OpenRouter suffisant (sonde 1 image) avant tout lot payant.

## Lots

### GA0 — Squelette (session principale, ~15 min)
- [x] Branche `feat/ga` + worktree.
- [x] `tools/cent_ans_tools/material_gen.py` : API vide (`generate`, `make_tileable`,
      `derive_maps`, `contact_sheet`) + tests pytest désactivés (`skip`).
- [x] `data/art/materials.yaml` (vide typé) + `data/schemas/materials.schema.json`.
- [x] Section « GA » dans `docs/budget.md`. Commit `wip: GA0 skeleton`.

### GA1 — Matières des figurines (IA, ≤ 4 $, agent `cent-ans-dev`)
Fichiers : `material_gen.py`, `data/art/materials.yaml`,
`tools/blender_scripts/battle_fine_tiles.py` (assemblage), `game/assets/models/battle_fine/textures/`,
`game/shaders/battle_soldier_skinned.gdshader` (sous `#ifdef FG3_BAKED` seulement),
`game/scripts/battle/battle_skinned.gd` (chargement), ADR `0104-albedo-de-detail-des-figurines.md`.
1. [x] Chaîne : `make_tileable` (décalage ½ + fondu des coutures), `derive_maps` (hauteur =
       luminance passe-haut → normale OpenGL → rugosité), `contact_sheet`. Tests : écart des
       bords ≤ 4/255, dimensions, canaux. Commit. CLI : `uv run --project tools cent-ans assets
       materials --out <scratch> [--only id]… [--sheet planche.png]` (brute réutilisée si présente).
2. [x] Sonde : 2 matières (laine, mailles) → planche ; jugement en session principale (1 capture).
       Sonde faite (0,08 $) : `docs/research/ga1_probe_sheet.png` ; validée (raccords invisibles) ; corrections : mailles moins rugueuses,
       laine feutrée irrégulière (height_strength −30 %), police accentuée de la planche.
3. [x] Lot : 12 matières (laine, lin, futaine, gambison, mailles, cuir, plates, bois, peau,
       cheveux, robe claire, robe foncée), 512², budget consigné.
4. [x] Nouveau `fine_detail_albedo` (Texture2DArray, centré en luminance moyenne 0,5) multiplié
       à la couleur de sommet ; tuiles RG/B/A remplacées ; même `FG3_TILE_SIZE`. Drapeau `--no-ga1`.
5. [x] Tests : mémoire (modèle `fg3_maps_test.gd`, ajout ≤ 4 Mo), smoke ; A/B `--closeup`
       et standard (≤ +5 %). ADR 0104. Commit.

### GA2 — Sol de bataille (CC0, 0 $, agent `cent-ans-mech`)
Fichiers : `game/assets/textures/battle/build_textures.py`, `README.md`, tableaux du sol,
`game/scripts/battle/battle_terrain.gd`, shader du sol de bataille, données des couches
(`data/fx/` ou `battle_layers`), ADR `0105-textures-2k-et-macro-variation.md`.
1. [x] Mesure mémoire : 13 couches 2k BC7. Albédo 2k (13 × 2048² × 1 o/texel × 4/3 mipmaps)
       ≈ 69,3 Mo ; normales 2k ≈ 69,3 Mo → total 2k/2k ≈ 138,6 Mo > 120 Mo. Normales en 1k
       (13 × 1024² × 1 o/texel × 4/3) ≈ 17,3 Mo → total albédo 2k + normale 1k ≈ 86,6 Mo (sous
       le plafond). Décision : albédo 2k, normale 1k pour les 13 couches.
2. [x] Poly Haven 2k (9 couches existantes + 4 nouvelles) : identifiants dans `data/fx/
       battle_ground_layers.json` (schéma `fx_battle_ground_layers.schema.json`) et `README.md`.
       Nouvelles : `leafy_grass` (prairie fleurie — pas de texture « prairie fleurie » dédiée en
       CC0 chez Poly Haven ; substitut le plus proche, à revoir si une meilleure source apparaît),
       `grassy_cobblestone` (herbe piétinée), `withered_grass` (chaume/éteules), `farm_furrows`
       (labour frais, distinct de `farm_soil` déjà utilisé pour le labour ambiant/procédural).
3. [x] Câblage (données) : `BattleTerrain.ground_layers()`/`ground_role_index()` lisent
       `data/fx/battle_ground_layers.json` ; uniformes `layer_count`, `layer_tile_size[]`,
       `idx_flowering_meadow/trodden_grass/stubble/fresh_plough` posés dans `_build_material`.
       Occupation : labour frais = parcelles décor EP6 `decor_kind==1` (le labour ambiant garde
       `farm_soil`) ; chaume = `decor_kind==6` ; herbe piétinée = halo `splat_b.g` des chemins/
       routes déjà cuit ; prairie fleurie = part des taches de prairie grasse (`patch`/`patch2`
       du lot V4b). `layer_size()` du shader lit `layer_tile_size[]` (plus de constantes en dur).
4. [x] Macro-variation dédiée (4 octaves 50/90/140/200 m, teinte + luminance, faible amplitude),
       appliquée à l'albédo final, sous `ga2_on`. `--no-ga2` la coupe (compromis : ne restaure pas
       d'anciennes textures 1k, cf. ADR 0105 « conséquences »).
5. [ ] Tests mémoire + smoke ; A/B ≤ +5 %. ADR 0105 écrit (à confirmer avec les mesures). Commit.

### GA5 — Bâtiments (CC0, ≤ 1 $, agent `cent-ans-mech`, après GA2)
- [ ] Textures bâtiments de bataille et `textures/buildings` en 2k Poly Haven ; variantes
      torchis et colombages ; `SOURCE.md`/`README.md` ; tests mémoire ; ajout à l'ADR 0105.

### GA4 — Campagne (CC0 + IA ponctuelle, ≤ 2 $, agent `cent-ans-dev`, après GA2)
Fichiers : `game/assets/textures/terrain/`, `game/scripts/map/terrain_builder.gd`,
`game/shaders/terrain.gdshader`, `game/shaders/water.gdshader`.
- [ ] Plaines et couches terrain en 2k ; macro-variation (réutiliser GA2).
- [ ] Eau : normales CC0 animées + couleur de profondeur.
- [ ] A/B banc carte PB1 (cache de relief relié, cf. PO6). Vignettes IA seulement sur décision.

### GA3 — Décor 3D statique (image-vers-3D, ≤ 8 $)
1. [ ] Recherche (session principale) : service hébergé TRELLIS payant à l'appel, conditions
       de droits sur les sorties, prix. Si aucun ne convient : lot abandonné, noter ici.
2. [ ] Sonde : maison à colombages (image gpt-image 3/4 fond neutre → TRELLIS →
       `tools/blender_scripts/ga3_cleanup.py` : décimation, LOD0/1/2, UV, cuisson albédo 1024²)
       → `.glb` dans `game/assets/models/props_ga/`. Planche ; **go/no-go du joueur**.
3. [ ] Lot : maison paysanne, église de village, moulin, chariot, tente, palissade, puits,
       trébuchet, bélier (LOD0 ≤ 8 k tri bâtiment, ≤ 3 k objet). Branchement dans le décor
       de bataille (données `battle_decor`). ADR `0106-pipeline-image-vers-3d.md`.

### GA6 — Clôture
- [ ] Planche avant/après globale (réutiliser `po_shot.gd`, vues 04, 08, 09 + gros plan).
- [ ] Jugement du joueur, puis `feat/ga` → `main` (`--ff-only`), suppression des worktrees.

## Vagues
1. GA0 (principal) → vague 1 : GA1 ∥ GA2 → vague 2 : GA5 ∥ GA4 ∥ recherche GA3 → sonde GA3
   (jugement) → GA3 → GA6. Au plus 3 agents à la fois (autres chantiers actifs).

## Journal
- 28/09 : spec approuvée (726a31bf), plan écrit.
- 28/09 : GA0 fait (squelette, section budget GA). Suite : vague 1 (GA1 ∥ GA2).
- 28/09 : GA1 étape 1 faite (worktree `../game_project-ga1`, branche `feat/ga1`) : chaîne
  tuilable + cartes dérivées + planche + CLI `assets materials`. Suite : sonde laine/mailles.
- 28/09 : GA1 sonde laine + mailles (gpt-5-image-mini, 2 × 0,04 $ = 0,08 $, estimation 0,02 $/image :
  prévoir ≈ 0,50 $ pour les 12). Brutes 1024² hors dépôt (scratch de l'agent). Tuiles 512² :
  écart moyen des bords ≤ pas moyen intérieur (sans couture). Mailles sombres (lum. moy. 63/255),
  laine claire (174) : le centrage à 0,5 de l'étape 4 est nécessaire. Planche
  `docs/research/ga1_probe_sheet.png` (945 Ko) à juger en session principale.
- 28/09 : GA1 étapes 3-5 en cours : 12 matières dans `materials.yaml` (`tile_m` = `GA1_TILE_SIZE`
  du shader, testé), génération en cours vers le scratch ; shader (`ga1_detail`, `fine_detail_albedo`),
  chargement (`--no-ga1`), test `ga1_maps_test.gd`. Suite : assembler les tableaux
  (`material_gen.build_fine_arrays`), import Godot, tests, A/B, ADR 0104.
- 28/09 : GA1 fait (branche `feat/ga1`, non fusionnée) : 12 matières (0,59 $ au total, dont un
  appel coupé facturé), `fine_detail_ga1.png` + `fine_detail_albedo.png` (ajout 2,33 Mo), shader
  `ga1_detail`, `--no-ga1`, `ga1_maps_test.gd` OK (et `-- --no-ga1`), fg3_maps OK, smoke OK ;
  A/B dans le bruit (≤ +5 %) ; ADR 0104. Planche des 12 : `docs/research/ga1_sheet.png` à juger
  (jugement en jeu : session principale). `main` a bougé (`tools/cent_ans_tools/cli.py`) : conflit
  possible, simple, à la fusion ; le shader n'a pas bougé sur `main`.
