# WIP — Intégration du HUD de campagne (F10b)

Branche : `worktree-agent-ac6575fb236a85869` (depuis `lot-a`).

## Plan
1. [x] Squelette `HudController` (`game/scripts/map/hud_controller.gd`).
2. [x] `MapUI` : nœuds `ArmyStrip`, `GeneralSeal`, `EndTurnCluster`, `NewsLetters` ; `layout_hud()` ; accesseurs
       `selected_army_widget()`, `end_turn_control()` ; retrait d'`ArmyPanel` et du bouton « Fin du tour ».
3. [x] Alertes F3 → cloche (vue `AlertsPanel` retirée), décision de chronique bloquante.
4. [x] Lettres depuis `add_events` (après `journal_keeps`).
5. [x] Siège (assaut) déplacé au-dessus du bandeau.
6. [ ] Mise en page 1440×900 / 1920×1080, captures army/province/chronicle.
7. [ ] Smoke, hud_components_test, docs (`godot-map.md`, `hud-campagne.md`).

## Prochaine étape
Étape 6 : build, import, smoke, captures (code écrit, pas encore exécuté).
