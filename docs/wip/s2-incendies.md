# WIP — S2 incendies de siège

Spéc : `docs/design/s2-incendies.md`. ADR : `docs/decisions/0008-incendies-regle-coeur.md`.
Branche : `worktree-agent-a9315344c0b1dad9c` (non fusionnée).

## État
- [x] Spéc écrite et mise à jour (faubourgs latéraux, durée de combustion fixe, mesures de la sonde).
- [x] Données `data/rules/siege_fire.json` + schéma + pytest `tools/tests/test_siege_fire_schema.py`.
- [x] Cœur : `sim-battle/src/fire.rs`, `src/sim/fire.rs`, branché dans `sim.rs` ; ruines franchissables
  (`pathing.rs`) ; commande `burn` (`command.rs`) ; sonde `WEATHER=` / `FIRE=off`.
- [x] Tests `sim-battle/tests/fire.rs` (10) ; `cargo test` complet vert, clippy propre.
- [x] Pont `get_siege()` (fire, suburb, gate_fire, wind, compteurs), `burn`, `debug_ignite`.
- [x] Rendu `game/scripts/battle/siege_fire_fx.gd` + `data/fx/siege_fire.json` (+ schéma), appel dans
  `battle_siege.gd`, ligne HUD dans `battle_scene.gd`.
- [x] Test Godot `game/tests/s2_fire_fx_test.gd` vert ; smoke vert.
- [ ] Capture fenêtrée (reportée : mode silencieux actif, aucune fenêtre Godot autorisée).

## Prochaine étape
Lot terminé, en attente de fusion. Points ouverts : capture `docs/img/s2/…` quand le mode silencieux
sera levé (`battle.tscn -- --siege`, puis `debug_ignite`), bouton « incendier » dans l'interface,
IA tactique qui évite les rues en feu.
