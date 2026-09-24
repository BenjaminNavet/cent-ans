# F5c — Déploiement et maisons de siège dans Godot

État : terminé (points 1 à 4).

- `game/scripts/battle/deployment_controller.gd`, `deployment_zone.gd` : phase de déploiement.
- `battle_scene.gd` : section « F5c » en fin de fichier, `--deploy-shot`, Entrée, clic droit routé.
- `battle_hud.gd` : `show_toast` (fin de fichier).
- `battle_siege.gd` : `_house_sites()` (source = `get_siege().houses`), `_build_houses` réécrit.
- Smoke : `_check_battle_deployment_f5c`, `_check_siege_f5c`.
- Captures : `docs/img/godot-battle-deploy.png`, `docs/img/godot-siege-f5.png`.

Prochaine étape : aucune (fusion).
