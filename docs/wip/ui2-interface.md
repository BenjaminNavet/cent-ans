# UI2 — refonte de l'interface de campagne (lots U1, U3, U4 de l'audit A3 ; U5, U7 si le temps le permet)

Branche : `worktree-agent-a817c3f698be6306c` (main + branche U1 `worktree-agent-ac3b609abdc259c3f` fusionnées).
Source : `docs/audit/a3-ui.md` § 3 et § 8. Captures : `docs/audit/captures/ui2/`.
Propriétaire exclusif de `game/scripts/map/map_ui.gd` pendant le lot. Ne pas toucher au HUD de bataille.

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Captures : `godot --resolution 1280x720 --path game res://scenes/campaign_map.tscn -- --stage=<étape> --screenshot=<png>`.

## État
- [ ] U1 empilement des fenêtres
- [ ] U3 économie lisible
- [ ] U4 échelle et responsivité
- [ ] U5 fin de tour utile (optionnel)
- [ ] U7 barre du haut et raccourcis (optionnel)

## Prochaine étape
Lire `map_ui.gd` et `campaign_map.gd`, faire les captures « avant ».
