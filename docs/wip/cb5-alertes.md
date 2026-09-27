# CB5 — Alertes de bataille typées (état)

Branche : `feat/cb5-alerts` (depuis `main` @ be631979, qui contient `cad1ca05` — CB0 + CB-M1
seulement ; CB-M2/CB-M3/CB-M4/CB1 ne sont **pas** encore dans `main`, malgré `docs/wip/cb.md`).
Cible cargo privée : `core/target-cb5`.

## État
- [x] Squelette : `core/crates/sim-battle/src/alerts.rs` (`AlertKind`, `BattleAlert`, `AlertRules`
      chargée depuis `data/rules/battle_alerts.json`), module déclaré dans `lib.rs`.
- [x] `data/rules/battle_alerts.json` + `data/schemas/battle_alerts_rules.schema.json` +
      `tools/tests/test_battle_alerts_schema.py`.
- [ ] Champ `alerts`/`alerts_read` sur `BattleSim`, méthode `alert()`, `take_new_alerts()`.
- [ ] Émission aux points du plan : déroute, général tué/capturé, flanc (front montant),
      renforts, munitions, mur/porte rompus (point unique : `record_siege_transitions`).
- [ ] Pont `godot-bridge` : `get_alerts()`.
- [ ] `core/crates/sim-battle/tests/cb5_alerts.rs`.
- [ ] `game/scripts/battle/battle_alerts_column.gd` + branchement scène/HUD.
- [ ] `battle_minimap.gd` : repère pulsé au clic.
- [ ] Cris `BattleAudio.play_event`.
- [ ] `game/tests/cb5_alerts_test.gd`, `game/tests/cb5_alerts_shot.gd` (probe seulement).

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
Câbler les points d'émission dans `sim.rs` / `sim/reinforcements.rs` / `sim/siege_assault.rs`,
puis les tests core, puis le pont, puis Godot.
