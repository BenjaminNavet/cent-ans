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
- [ ] 3 sujets × {NB2, Lite, Pro} × {sans, avec ancre}, 1K ; grille `docs/research/nb0_models.png`.
- [ ] **Jugement joueur** → `docs/research/nb0-sonde-modeles.md`. Commit.

### NB1 — Kit d'interface (≤ 5 $)
- [x] Chaîne locale (`key_out`, `fit_to_piece`, `seam_fix`, `contact_sheet`, repli dans
      `ui_illumination.build`) + tests — agent `cent-ans-mech`. Commit.
- [ ] Prompts des 16 pièces + ~10 décors dans `ui_ornaments.yaml` ; `--dry-run`.
- [ ] Génération 3 variantes/pièce → planche avant/après → **jugement joueur** (`selected`).
- [ ] Rebuild du kit, `smoke.gd`, une capture en jeu. ADR 0135. Commit, fusion dans `main`.

## Journal
- 09-30 : spec approuvée, worktree créé.
- 09-30 : NB-DA fait (0,40 $), v0 retenue par le joueur (après v3). Prochaine étape : NB0.

## Points ouverts
- Piste A (après NB1) : refaire les 12 matières GA1 des figurines en NB2 2K avec une
  ancre « réalisme peint » propre à la 3D (jamais l'ancre enluminure), ~1,50 $.
- NB1 restreint aux 4 cadres à marges larges (panel_illuminated 38 px, panel 20, top_bar,
  tooltip 12) : barres, curseur, onglets, boutons, encart (1-9 px) restent procéduraux ;
  gain réel limité par la taille 1× du kit (interface 2× = chantier suivant).
- Carte (question du joueur 09-30) : vue 3D = CC0 photo + DEM + modèles Blender, NB2 peu utile
  sauf imposteurs d'arbres lointains (à lier au chantier FPS carte, appels de dessin +21 %).
  Vue parchemin (> 1200) = cible forte : montagnes en pitons, forêts, vignettes de villes par
  rang, mer, rose des vents, cartouches, style ancre v0 (≈ 60-80 sprites, ~5 $). Chantier
  « parchemin peint » à spécifier après NB1, plafond propre ~8 $.
