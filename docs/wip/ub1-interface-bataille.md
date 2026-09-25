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
- [x] Lot 1 — écran d'avant-bataille : `pre_battle_dialog.gd` réécrit (plein écran, bannière
  `evt_crecy`/`evt_sluys`, barre d'équilibre et verdict, médaillons des généraux, cartes
  `RosterCard`, renforts, conditions, Combattre / Résolution automatique / Retraite ou Maintenir le
  siège). Cœur : `battle_forecast.rs` (`battle_forecast`, `withdraw_pending_battle`, test
  `tests/ub1_forecast.rs`) ; pont `get_battle_forecast`, `withdraw_pending_battle`. Mise en scène
  `--stage=assault`. Captures 10, 11, 12.
- [x] Lot 2 — HUD de bataille compact (U9) : bandeau de 128 px (au lieu de 182, 14 % de 900),
  sceau du chef (portrait, anneau de moral de sa garde, clic / double clic), cartes avec barres
  santé / moral / munitions et pastilles agrandies (déroute, charge, épuisée…), noms distincts des
  homonymes (« Chevaliers II ») en infobulle, ordres en icônes, « Retraite générale » isolée et
  confirmée (Échap annule), journal regroupé (×2) et repliable (bouton, touche J), barre des
  ordres du chef recalée. `tests/ub1_screenshots.gd` (captures 22, 23). Captures 20, 21.
  L'erreur « doivent être placés dans votre zone » de la capture A3 21 vient de la mise en scène
  `--deploy-shot` (refus volontaire) : aucune erreur à l'ouverture en jeu.
- [x] Lot 3 — écran de fin détaillé : bannière illustrée (Crécy / Poitiers / Azincourt) « Victoire »,
  « Victoire à la Pyrrhus » (vainqueur ayant perdu ≥ 30 % et plus que le vaincu) ou « Défaite »,
  bilan engagés / pertes / survivants, cartes `RosterCard` en mode bilan (−pertes, épées croisées
  + tués, héros doré), tableau Régiment / Engagés / Pertes / Tués / Sort (homonymes numérotés),
  encarts Héros, Captifs et rançons, Expérience, Butin, faits notables. Cœur : `Unit::kills`
  (sim-battle, statistique pure créditée aux tirs et à la mêlée, test `tests/ub1_kills.rs`),
  `kills` dans `get_units`, `experience` dans les unités de `get_army`. `battle_scene.gd` applique
  `resolve_battle` dès la fin (plus au retour) pour lire les suites avant / après
  (`battle_aftermath.gd`). Captures 30, 31, 24 (siège 1280×720).
- [ ] Lot 4 — sons d'interface (U13)

## Prochaine étape
Lot 4 : sons d'interface (banque AU1, bus Interface).

## Reprise
`core/build.sh`, `godot --headless --path game --import`. Captures :
`godot --resolution 1440x900 --path game res://scenes/campaign_map.tscn -- --stage=battle --screenshot=<png>` ;
`godot --path game res://scenes/battle/battle.tscn -- [--result-shot|--deploy-shot] --screenshot=<png>`.
