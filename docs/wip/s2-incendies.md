# WIP — S2 incendies de siège

Spéc : `docs/design/s2-incendies.md`. Branche : `worktree-agent-a9315344c0b1dad9c`.

## État
- [x] Spéc écrite.
- [ ] Squelette Rust (API publique, tests `#[ignore]`).
- [ ] Données `data/rules/siege_fire.json` + schéma + pytest.
- [ ] Règles (allumage, propagation, météo, chaleur, fumée, ruine franchissable, porte, faubourgs, commande burn).
- [ ] Pont `get_siege()` + commande `burn`.
- [ ] Rendu `game/scripts/battle/siege_fire_fx.gd` + `data/fx/siege_fire.json`, ligne HUD.
- [ ] Smoke, sonde, ADR 0008.

## Prochaine étape
Squelette Rust.
