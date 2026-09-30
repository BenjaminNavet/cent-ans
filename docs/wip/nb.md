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
- [ ] 4-6 folios domaine public → `tools/nb_raw/sources/` + `SOURCES.md`.
- [ ] 4 variantes NB2 2K 3:2 → planche contact → **jugement joueur**.
- [ ] `data/art/style/anchor.png` + `anchor.yaml` ; bible DA § 13. Commit.

### NB0 — Sonde des modèles (≤ 1,5 $)
- [ ] 3 sujets × {NB2, Lite, Pro} × {sans, avec ancre}, 1K ; grille `docs/research/nb0_models.png`.
- [ ] **Jugement joueur** → `docs/research/nb0-sonde-modeles.md`. Commit.

### NB1 — Kit d'interface (≤ 5 $)
- [ ] Chaîne locale (`key_out`, `fit_to_piece`, `seam_fix`, `contact_sheet`, repli dans
      `ui_illumination.build`) + tests — agent `cent-ans-mech`. Commit.
- [ ] Prompts des 16 pièces + ~10 décors dans `ui_ornaments.yaml` ; `--dry-run`.
- [ ] Génération 3 variantes/pièce → planche avant/après → **jugement joueur** (`selected`).
- [ ] Rebuild du kit, `smoke.gd`, une capture en jeu. ADR 0135. Commit, fusion dans `main`.

## Journal
- 09-30 : spec approuvée, worktree créé.
