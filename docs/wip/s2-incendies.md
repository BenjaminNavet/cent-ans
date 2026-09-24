# WIP — S2 incendies de siège

Spéc : `docs/design/s2-incendies.md`. Branche : `worktree-agent-a9315344c0b1dad9c`.

## État
- [x] Spéc écrite.
- [x] Données `data/rules/siege_fire.json` + schéma `siege_fire_rules.schema.json` + pytest
  `tools/tests/test_siege_fire_schema.py` (le cas `fx` attend `data/fx/siege_fire.json`).
- [x] Cœur : `sim-battle/src/fire.rs` (types, règles embarquées), `src/sim/fire.rs` (tick, allumage,
  propagation, chaleur, fumée, porte, faubourgs, commande `burn`), branché dans `sim.rs`
  (`resolve_fire`, `incendiary_volley`, `smoke_factor`), maisons brûlées franchissables (`pathing.rs`).
- [x] Tests `sim-battle/tests/fire.rs` (10) verts ; toute la suite sim-battle verte.
- [ ] Pont `get_siege()` + commande `burn` (doc d'API).
- [ ] Rendu `game/scripts/battle/siege_fire_fx.gd` + `data/fx/siege_fire.json`, ligne HUD.
- [ ] Smoke, sonde (pluie/sans pluie), ADR 0008, spéc à jour (faubourgs latéraux, chiffres).

## Prochaine étape
Pont godot-bridge (`battle_sim.rs` : `get_siege`), puis rendu Godot.
