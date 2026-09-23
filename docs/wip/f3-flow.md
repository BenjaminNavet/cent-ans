# Lot F3 « Écrans et flux » — suivi

Branche : `worktree-agent-a6a48de5c2de758a2`.

## Plan
1. [x] Illustration du menu (`menu_art.py`, `cent-ans assets menu-art`) + menu de départ illustré.
2. [x] Écran de chargement (`loading_screen.gd`).
3. [x] Menu pause (Échap) — `pause_menu.gd`, branché par `flow_controller.gd`.
4. [x] Réglages (autoload `Settings`, `settings_menu.gd`).
5. [x] Sauvegardes (`save_slots.gd`, dialogue avec vignettes, auto tournante, « Continuer »).
6. [x] Rapport de saison (`season_report.gd`).
7. [x] Alertes (`alerts.gd`).
8. [x] Crédits (`credits_screen.gd`, lit `CREDITS.md` s'il existe).
9. [x] Smoke § 12 « flow », captures `docs/img/`, `docs/status.md`, `docs/godot-map.md`.

## État
Terminé. Smoke complet OK (13 étapes), 60 tests Python OK, ruff propre, `core/build.sh` OK.

## Points ouverts
- Échelle d'interface (`content_scale_factor`) non vérifiée visuellement à 125/150 %.
- `CREDITS.md` n'est pas copié par `tools/export_macos.sh` (repli sur le texte intégré dans l'app).
- Le menu « Menu principal » de la barre (MapUI) quitte sans confirmation (MapUI hors périmètre) ;
  la confirmation passe par le menu pause.
