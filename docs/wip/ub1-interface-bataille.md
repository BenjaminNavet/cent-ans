# UB1 — interface de bataille à la Total War (avant, pendant, après)

Agent UB1 (session 7). Branche `worktree-agent-a17058e0e6f7d70f2`, partie de `integration/night`
(fusion rapide pour disposer d'AU1 : bus Interface, `SoundBank`).
Sources : `docs/audit/a3-ui.md` (U9, U13, défauts B1-B7), `docs/audit/backlog-tw.md`,
`docs/wip/au1-audio.md`, `docs/wip/u1-bogues-ui.md`. Captures : `docs/audit/captures/ub1/`.

## Périmètre (fichiers touchés)
- `core/crates/sim-campaign/src/battle_forecast.rs` (prévision d'équilibre, retraite avant bataille),
  pont `core/crates/godot-bridge/src/battle_sim.rs` (`get_battle_forecast`, `withdraw_pending_battle`).
- `game/scripts/battle/pre_battle_dialog.gd` (écran d'avant-bataille), `battle_hud.gd`, `unit_card.gd`,
  `battle_result_screen.gd`, nouveaux scripts `game/scripts/battle/ub1_*.gd`.
- `game/scripts/map/campaign_map.gd` : seulement le branchement du signal de retraite.
- Sons d'interface : `data/audio/sound_bank.json` (événements `ui_*`), `game/scripts/audio/ui_sounds.gd`.
- Pas touché : `map_ui.gd`, thème global (UI2).

## État
- [ ] Lot 1 — écran d'avant-bataille
- [ ] Lot 2 — HUD de bataille compact (U9)
- [ ] Lot 3 — écran de fin détaillé
- [ ] Lot 4 — sons d'interface (U13)

## Prochaine étape
Lot 1 : prévision dans le cœur, pont, puis écran.

## Reprise
`core/build.sh`, `godot --headless --path game --import`. Captures :
`godot --resolution 1440x900 --path game res://scenes/campaign_map.tscn -- --stage=battle --screenshot=<png>` ;
`godot --path game res://scenes/battle/battle.tscn -- [--result-shot|--deploy-shot] --screenshot=<png>`.
