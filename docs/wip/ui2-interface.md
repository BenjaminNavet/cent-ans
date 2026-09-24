# UI2 — refonte de l'interface de campagne (lots U1, U3, U4 de l'audit A3 ; U5, U7 si le temps le permet)

Branche : `worktree-agent-a817c3f698be6306c` (main + branche U1 `worktree-agent-ac3b609abdc259c3f` fusionnées).
Source : `docs/audit/a3-ui.md` § 3 et § 8. Captures : `docs/audit/captures/ui2/`.
Propriétaire exclusif de `game/scripts/map/map_ui.gd` pendant le lot. Ne pas toucher au HUD de bataille.

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Captures : `godot --resolution 1280x720 --path game res://scenes/campaign_map.tscn -- --stage=<étape> --screenshot=<png>`.

## État
- [x] U1 empilement des fenêtres : `game/scripts/ui/panel_stack.gd` (genres DOCKED / CENTRAL /
  COMPANION / MODAL), branché dans `map_ui.gd` (`_setup_panel_stack`, `register_panel`,
  `_shortcut_input` pour Échap, `_keep_on_screen`, minicarte masquée si un panneau la couvre) ;
  `campaign_map.gd` : objectifs et aide enregistrés, la Cour / la fiche / les technologies ne se
  rouvrent plus au rafraîchissement après fermeture. Test : `tests/ui_panel_stack_test.gd`.
- [x] U3 économie lisible : `economy_balance.rs` (`BudgetRecord`, `budget_lines`, historique de
  12 saisons dans `FactionState.budget_history`, enregistré en fin de `end_turn`, `signed_livres`),
  pont (`budget_lines`, `net_change`, `budget_history`), `money.gd` (format unique ₶),
  `budget_table.gd`, `treasury_chart.gd`, panneau de faction élargi (500 px), infobulles du solde
  et du trésor dans `map_ui.gd`, ℔ remplacé partout. Étape de capture `--stage=budget`.
  Test Rust : `tests/u3_budget.rs`.
- [ ] U4 échelle et responsivité
- [ ] U5 fin de tour utile (optionnel)
- [ ] U7 barre du haut et raccourcis (optionnel)

## Prochaine étape
U4 : échelle automatique (hauteur / 900, bornée 0,9–1,6), réglages « Taille de l'interface » / « Taille du texte », test 4 résolutions, polices EB Garamond.
