# UI3 — interface de campagne, suite (lots U5, U7 fin, U10, U11, U12, U8 de l'audit A3)

Branche : `worktree-agent-ae5685646cc8b59ac` (main + UI2 `worktree-agent-a817c3f698be6306c` fusionnés).
Source : `docs/audit/a3-ui.md` § 3 et § 8 ; suite de `docs/wip/ui2-interface.md`.
Captures : `docs/audit/captures/ui3/`. Propriétaire exclusif de `game/scripts/map/map_ui.gd`.
Ne pas toucher à l'interface de bataille (agent UB1).

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Captures : `godot --resolution 1280x720 --path game res://scenes/campaign_map.tscn -- --stage=<étape> --screenshot=<png>` ;
   fin de tour : `-- --flow-stage=report|alerts --flow-shot=<png>`.

## État
- [x] U5 fin de tour utile : `season_report.gd` (rubriques Vos terres / Trésor / Armées / Constructions
  et recherches / Le monde, pertes ▼ en rouge puis prises ▲ en tête, bouton d'action par ligne,
  ligne de synthèse du trésor), `news_interest.gd` (voisins, alliés, ennemis, vassaux, 5 grandes
  puissances ; réglage `interface/news_filter`), lettres filtrées et bornées au-dessus des pastilles,
  toast de bataille filtré, `end_turn_cluster.gd` (colonne de pastilles libellées + compteur),
  bandeau « Tour des autres factions » (`MapUI.request_end_turn`, `FlowController.end_turn_would_proceed`).
- [x] U7 fin : actions InputMap ajoutées (P N R O G L F1 F5 F9), contrôleurs passés aux actions,
  cartouches de touche sur les boutons, boutons Objectifs (O) et Agents (G) (`press_action`),
  Codex libellé, onglet « Commandes » (disposition auto / AZERTY / QWERTY, fiche générée,
  `shortcut_sheet.gd`), aide F1 générée.
- [ ] U10 personnages
- [ ] U11 Codex et encyclopédie réunis
- [ ] U12 accessibilité
- [ ] U8 finitions
- [x] police de repli pour ₶ : Noto Serif (sous-ensemble monnaies) + Noto Sans Symbols (sous-ensemble)
  dans les `fallbacks` des FontVariation du thème (`assets/third_party/fonts/noto/`)

## Prochaine étape
U10 personnages, puis U11, U12 (réglages `access/*` et `accessibility.gd` déjà en place), U8.
