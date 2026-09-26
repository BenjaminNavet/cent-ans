# DA5 — Boutons-médaillons et icônes d'action à l'encre

Branche : `worktree-agent-aa84e124b506691c1` (a fusionné `feat/da-direction-artistique`).
Bible : `docs/design/2026-09-25-bible-da.md` § 5, § 8. ADR 0065. Plafond du lot : 5 $.

## État : terminé (en attente de fusion par l'orchestrateur)

- Catalogue `data/ui/icons_ink.json` + schéma `data/schemas/icons_ink.schema.json` : 78 icônes
  (110 identifiants d'interface), 15 médaillons (la cloche reprend `bouton_cloche.png`).
- Générateur `tools/cent_ans_tools/ink_icons.py`, CLI `cent-ans assets ink-icons`
  (`--dry-run`, `--only`, `--limit`, `--kind`, `--build-only`, `--envelope`) ; tests
  `tools/tests/test_ink_icons.py`. Sources brutes `tools/da5_raw/` (idempotent).
- Dépense DA5 : 4,46 $ réels (sonde 0,18 ; 76 icônes 3,43 ; 12 médaillons 0,54 ; reprises de
  6 icônes et d'un médaillon 0,31). 95 images payantes au total.
- Reprises : dette, pied à terre, pavois, revendications, cri de guerre (sujets ambigus) ;
  techniques (compas et équerre = emblème maçonnique) → roue dentée et marteau, icône et médaillon.
- Godot : `IconLibrary` (index à l'encre prioritaire, repli SVG ; `tint`, `apply_state_tints`,
  `get_medallion`, `medallion_states`, `decorate_medallion`), bandeau du haut (8 médaillons),
  cloche (`EndTurnCluster._draw_medallion`), Recruter / Former une armée (fiches de ville et de
  province), ordres de bataille (`battle_hud`), ordres du chef (`leader_orders_bar`), filtres
  (menu, bouton), mini-carte (Politique, Relief, Légende), mariage (fiche de personnage).
- Captures `docs/img/da5/avant_*.png` / `apres_*.png`, planches `planche_icones_24px.png`
  (24/48/128 px, encre puis or) et `planche_medaillons.png`.

## Limites / suites

- Icônes d'entité (unités, bâtiments, techniques, compétences, ressources, régimes) : toujours
  SVG game-icons.net ; à remplacer par des miniatures peintes (bible § 8), lot à créer.
- Médaillons `journal` et `research` générés mais pas branchés (panneau du journal et pastille
  « Aucune recherche » gardent leur forme ; icône à l'encre sur la pastille).
- Barre du haut un peu plus haute (médaillons 26 px) : plus de boutons passent en icône seule
  à 1600 px (Techniques, Codex, Objectifs, Agents).
- Marqueurs de bataille : les catégories `infantry`/`archer`/`ranged` en repli prennent l'encre,
  les types d'unité gardent leur SVG (mélange visible à petite taille).
