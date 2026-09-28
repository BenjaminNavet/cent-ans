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
- [ ] Branche `feat/ga` + worktree.
- [ ] `tools/cent_ans_tools/material_gen.py` : API vide (`generate`, `make_tileable`,
      `derive_maps`, `contact_sheet`) + tests pytest désactivés (`skip`).
- [ ] `data/art/materials.yaml` (vide typé) + `data/schemas/materials.schema.json`.
- [ ] Section « GA » dans `docs/budget.md`. Commit `wip: GA0 skeleton`.

### GA1 — Matières des figurines (IA, ≤ 4 $, agent `cent-ans-dev`)
Fichiers : `material_gen.py`, `data/art/materials.yaml`,
`tools/blender_scripts/battle_fine_tiles.py` (assemblage), `game/assets/models/battle_fine/textures/`,
`game/shaders/battle_soldier_skinned.gdshader` (sous `#ifdef FG3_BAKED` seulement),
`game/scripts/battle/battle_skinned.gd` (chargement), ADR `0104-albedo-de-detail-des-figurines.md`.
1. [ ] Chaîne : `make_tileable` (décalage ½ + fondu des coutures), `derive_maps` (hauteur =
       luminance passe-haut → normale OpenGL → rugosité), `contact_sheet`. Tests : écart des
       bords ≤ 4/255, dimensions, canaux. Commit.
2. [ ] Sonde : 2 matières (laine, mailles) → planche ; jugement en session principale (1 capture).
3. [ ] Lot : 12 matières (laine, lin, futaine, gambison, mailles, cuir, plates, bois, peau,
       cheveux, robe claire, robe foncée), 512², budget consigné.
4. [ ] Nouveau `fine_detail_albedo` (Texture2DArray, centré en luminance moyenne 0,5) multiplié
       à la couleur de sommet ; tuiles RG/B/A remplacées ; même `FG3_TILE_SIZE`. Drapeau `--no-ga1`.
5. [ ] Tests : mémoire (modèle `fg3_maps_test.gd`, ajout ≤ 4 Mo), smoke ; A/B `--closeup`
       et standard (≤ +5 %). ADR 0104. Commit.

### GA2 — Sol de bataille (CC0, 0 $, agent `cent-ans-mech`)
Fichiers : `game/assets/textures/battle/build_textures.py`, `README.md`, tableaux du sol,
`game/scripts/battle/battle_terrain.gd`, shader du sol de bataille, données des couches
(`data/fx/` ou `battle_layers`), ADR `0105-textures-2k-et-macro-variation.md`.
1. [ ] Mesure mémoire : 13 couches 2k BC7 (si > 120 Mo : normales en 1k). Noter ici.
2. [ ] Poly Haven 2k pour les 9 couches + 3–4 nouvelles (prairie fleurie, herbe piétinée,
       chaume/éteules, labour) ; identifiants choisis consignés dans `README.md`.
3. [ ] Câbler les nouvelles couches dans l'occupation du sol de bataille (données).
4. [ ] Macro-variation procédurale (octaves 50–200 m, teinte + luminance). `--no-ga2`.
5. [ ] Tests mémoire + smoke ; A/B ≤ +5 %. ADR 0105. Commit.

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
