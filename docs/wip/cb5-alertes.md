# CB5 — Alertes de bataille typées (état)

Branche : `feat/cb5-alerts` (depuis `main` @ be631979, qui contient `cad1ca05` — CB0 + CB-M1
seulement ; CB-M2/CB-M3/CB-M4/CB1 ne sont **pas** encore dans `main`, malgré `docs/wip/cb.md`).
Cible cargo privée : `core/target-cb5`.

## État
- [x] Squelette : `core/crates/sim-battle/src/alerts.rs` (`AlertKind`, `BattleAlert`, `AlertRules`
      chargée depuis `data/rules/battle_alerts.json`), module déclaré dans `lib.rs`.
- [x] `data/rules/battle_alerts.json` + `data/schemas/battle_alerts_rules.schema.json` +
      `tools/tests/test_battle_alerts_schema.py`.
- [x] Champ `alerts`/`alerts_read` sur `BattleSim`, méthode `alert()`, `take_new_alerts()`.
- [x] Émission aux points du plan : déroute, général tué/capturé, flanc (front montant, via
      `flanked_alerted: Vec<bool>` car `Unit.flanked` est remis à 0 à chaque tick), renforts,
      munitions, mur/porte rompus (point unique : `record_siege_transitions`).
- [x] Pont `godot-bridge` : `get_alerts()` + constantes `cb5_alert_*` (`RuleValues`).
- [x] `core/crates/sim-battle/tests/cb5_alerts.rs` (8 tests, tous verts : un par type + digest
      + rejeu inchangés).
- [x] `game/scripts/battle/battle_alerts_column.gd` (colonne, fusion, borne 5, glyphes dessinés
      en code) + branchement `battle_hud.gd` (`_build_alerts`) / `battle_scene.gd`
      (`get_alerts` dans la boucle HUD, `_on_alert_pinged`, audio branché après `battle_audio`).
- [x] `battle_minimap.gd` : `ping(world)` + repère pulsé (anneau, `_process` seulement pendant
      le pulsé).
- [x] Cris `BattleAudio.play_event` : seulement rout/general_down (les autres types n'ont pas de
      cri dans la spec) ; `_detect_events` de `battle_audio.gd` joue déjà ces deux mêmes sons sur
      transition d'état, donc un doublon éventuel est amorti par le cooldown/max_instances de
      l'événement (pas de garde explicite anti-doublon — limite connue, notée ici).
- [x] `game/tests/cb5_alerts_test.gd` : OK, exit 0, aucune SCRIPT ERROR (fusion, zone différente,
      hors fenêtre, borne 5 + priorité, expiration, clic → signal `pinged`, intégration
      `battle.tscn` : colonne dans `hud.root`, `get_alerts()` répond, clic → caméra + minicarte).
- [ ] `game/tests/cb5_alerts_shot.gd` (probe overlap, pas encore écrit).

## Décisions
- Mur/porte rompus : un seul point d'émission, `record_siege_transitions` (sim/siege_assault.rs),
  qui détecte déjà le front montant intact→rompu de chaque pièce pour `SiegeFxKind::GateBroken` /
  `WallBreached`. Évite de dupliquer l'alerte entre bélier, engin de siège et feu (trois causes,
  un seul événement).
- Général : `GeneralDown` émis dans `kill_general` (couvre mort au combat et « pas de quartier »)
  et dans la branche « fait prisonnier » (capture). Un seul type d'alerte pour les deux, comme la
  spec (pas de modèle de blessure).
- Rejeu EP13 : `alerts`/`alerts_read` ne sont pas dans `state_digest` (comme `events`) — flux de
  sortie pur, aucun impact sur le déterminisme. À vérifier par un test.

## Prochaine étape
Cœur terminé et vert (`cargo test -p sim-battle`, `b6`, `ep13_replay` release inclus). Reste :
pont `godot-bridge` (`get_alerts`, constantes `cb5_alert_duration_s`/`cb5_alert_merge_window_s`
dans `get_rule_constants`), puis Godot (`battle_alerts_column.gd`, ping minicarte, cris,
branchement scène/HUD, tests + probe).
