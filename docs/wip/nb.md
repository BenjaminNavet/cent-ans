# NB — Nano Banana 2 : ancre de style et interface (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-nb-nano-banana-interface-design.md` (0d0498bde, approuvée).
Branche `feat/nb`, worktree `../game_project-nb`. Fusion `--ff-only` dans `main` après jugement.
Budget : plafond 10 $, section « Nano Banana NB » de `docs/budget.md` (cap passé explicitement
à `BudgetLedger(cap=10)`, session nommée).

## Consigne de reprise

> Lis ce fichier et `git log --oneline feat/nb -15`, puis continue à la première case non cochée.
> N'ouvre les images qu'aux étapes de jugement.

## Lots

### NB-S — Squelette
- [x] `tools/cent_ans_tools/ui_ornaments.py` (API vide), `data/art/ui_ornaments.yaml` +
      `data/schemas/ui_ornaments.schema.json`, `tools/tests/test_ui_ornaments.py` (skip),
      `.gitignore` : `tools/nb_raw/`. Section budget. Commit `wip: NB skeleton`.

### NB-C — Client OpenRouter
- [x] `request_image`/`generate_image` : `image_config`, `seed`, `images` transmis ;
      estimation selon `image_size` (747/1120/1680/2520 tokens × `image_output`).
      Tests `test_openrouter.py`. Commit.

### NB-DA — Planche maîtresse (≤ 1 $, session principale)
- [x] 4-6 folios domaine public → `tools/nb_raw/sources/` + `SOURCES.md`.
- [x] 4 variantes NB2 2K 3:2 → planche contact → **jugement joueur**.
- [x] `data/art/style/anchor.jpg` (v0, 1536 px JPEG) + `anchor.yaml` ; bible DA § 13. Commit.

### NB0 — Sonde des modèles (≤ 1,5 $)
- [x] 3 sujets × {NB2, Lite, Pro} × {sans, avec ancre}, 1K ; grille `docs/research/nb0_models.png`.
- [x] **Jugement joueur** → `docs/research/nb0-sonde-modeles.md`. Commit.

### NB1 — Kit d'interface (≤ 5 $)
- [x] Chaîne locale (`key_out`, `fit_to_piece`, `seam_fix`, `contact_sheet`, repli dans
      `ui_illumination.build`) + tests — agent `cent-ans-mech`. Commit.
- [x] Prompts des 4 cadres + 6 décors dans `ui_ornaments.yaml` ; `--dry-run`.
- [x] Génération 3 variantes/pièce → planche avant/après → **jugement joueur** (`selected`) : panel_illuminated v1, panel v2, top_bar v2, tooltip v2.
- [x] Rebuild du kit, `smoke.gd` OK (dylib recompilée : champ `map_scene`), captures `docs/img/nb1/`, ADR 0135, fusionné dans `main`.

## Journal
- 09-30 : spec approuvée, worktree créé.
- 09-30 : NB-DA fait (0,40 $), v0 retenue par le joueur (après v3). Prochaine étape : NB0.

## Points ouverts
- **Figurines : le joueur choisit la piste C (30/09)** — planches de référence par type
  d'unité (face, profil, dos) en NB2, ~1 $, comme guide de retouche des modèles Blender ou
  d'entrée TRELLIS. Chantier propre à spécifier (brainstorming) ; à trancher dans la spec :
  la spec GA excluait les soldats 3D entièrement générés (TRELLIS réservé aux objets de décor).
  Pistes A (12 matières GA1 en NB2 2K) et B (motifs de livrée) non retenues pour l'instant.
- NB1 restreint aux 4 cadres à marges larges (panel_illuminated 38 px, panel 20, top_bar,
  tooltip 12) : barres, curseur, onglets, boutons, encart (1-9 px) restent procéduraux ;
  gain réel limité par la taille 1× du kit (interface 2× = chantier suivant).
- Carte (question du joueur 09-30) : vue 3D = CC0 photo + DEM + modèles Blender, NB2 peu utile
  sauf imposteurs d'arbres lointains (à lier au chantier FPS carte, appels de dessin +21 %).
  Vue parchemin (> 1200) = cible forte : montagnes en pitons, forêts, vignettes de villes par
  rang, mer, rose des vents, cartouches, style ancre v0 (≈ 60-80 sprites, ~5 $). Chantier
  « parchemin peint » à spécifier après NB1, plafond propre ~8 $.
- 09-30 : NB1 installé (4 PNG), ADR 0135, 3,10 $ au total. Reste : smoke + capture en jeu dans main.
- 09-30 : NB1 fusionné et clos. Nouveau mandat du joueur : autonomie, semi-réaliste campagne/bataille, figurines = vrais soldats en armure, NB2 ≈ 20 $ au total, sans variantes. Suite : chantier SR (`docs/wip/sr.md`).
- 09-30 : erreur — worktree supprimé avec `--force` : `tools/nb_raw/` perdu (brutes NB-DA/NB0/NB1 et les 6 décors non installés, ~0,42 $). Ce qui est livré est commité. À l'avenir, brutes dans `~/dev/cent-ans-raw/` (hors worktree).
