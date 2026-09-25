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
- [ ] U5 fin de tour utile
- [ ] U7 fin (barre du haut, Agents, Codex, onglet Commandes, fiche des raccourcis)
- [ ] U10 personnages
- [ ] U11 Codex et encyclopédie réunis
- [ ] U12 accessibilité
- [ ] U8 finitions
- [ ] police de repli pour ₶

## Prochaine étape
U5.
