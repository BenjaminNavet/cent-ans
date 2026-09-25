# DA5 — Boutons-médaillons et icônes d'action à l'encre

Branche : `worktree-agent-aa84e124b506691c1` (a fusionné `feat/da-direction-artistique`).
Bible : `docs/design/2026-09-25-bible-da.md` § 5, § 8. Plafond du lot : 5 $ (section DA de `docs/budget.md`).

## Décisions

- Périmètre : icônes **d'action et d'interface** (HUD, alertes, jauges, classes, catégories,
  ordres de bataille, filtres, actions de ville) = 78 images, 110 identifiants (`targets`).
  Les icônes **d'entité** (unités, bâtiments, techniques, compétences, ressources) restent des
  SVG game-icons.net : la bible (§ 8) les destine à des miniatures peintes, hors DA5.
- Médaillons : 15 (la cloche reprend `bouton_cloche.png` validé, 0 $ ; 14 générés avec la
  cloche en image de référence pour garder le cadre identique).
- Catalogue `data/ui/icons_ink.json` + schéma ; générateur `tools/cent_ans_tools/ink_icons.py`
  (`cent-ans assets ink-icons`) ; sources brutes `tools/da5_raw/` ; sorties
  `game/assets/icons/ink/` (PNG 128, RVB = INK, alpha = encre) et `game/assets/ui/medallions/`.
- Godot : `IconLibrary` lit `ink/index.json` avant `icons.json` (repli SVG si PNG absent).

## État

- [x] Inventaire, catalogue, schéma, captures avant (`docs/img/da5/avant_*.png`)
- [ ] Générateur + tests (dry-run)
- [ ] Sonde 4 images, génération
- [ ] Branchement Godot, captures après, planche 24 px, ADR

## Prochaine étape

Écrire `ink_icons.py` (plan, prompts, extraction d'alpha, médaillons) et ses tests.
