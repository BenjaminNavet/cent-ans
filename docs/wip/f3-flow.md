# Lot F3 « Écrans et flux » — suivi

Branche : `worktree-agent-a6a48de5c2de758a2`.

## Plan (un commit `wip:` par point)
1. [x] Illustration du menu (`menu_art.py`, `cent-ans assets menu-art`) + menu de départ illustré.
2. [~] Écran de chargement (`loading_screen.gd`) — écrit, à vérifier en capture.
3. [~] Menu pause (Échap) — `pause_menu.gd`, branché par `flow_controller.gd`, à tester.
4. [~] Réglages (autoload `Settings`, `settings_menu.gd`) — écrits, à tester.
5. [~] Sauvegardes (`save_slots.gd`, dialogue avec vignettes, auto tournante, « Continuer »).
6. [~] Rapport de saison (`season_report.gd`).
7. [~] Alertes (`alerts.gd`).
8. [~] Crédits (`credits_screen.gd`).
9. [ ] Smoke « flow », captures, `docs/status.md`, `docs/godot-map.md`.

## État
Tous les fichiers écrits ; smoke existant OK. Hooks dans `campaign_map.gd` : `flow.setup`,
`flow.refresh` (fin de `refresh_all`), `flow.before_end_turn` / `flow.after_end_turn`.

## Prochaine étape
Étape smoke « flow » + captures de chaque écran (`--flow-stage=` à ajouter dans flow_controller).
