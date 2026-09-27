# CB4 — Capacités actives (état)

Branche : `feat/cb4-abilities` (depuis `main` 07eff33b, après « docs: CB wave 4 merged, CB4 next »).
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ CB4) ; relecture
historique : `docs/research/cb4-capacites.md`. Cible cargo privée :
`CARGO_TARGET_DIR=<worktree>/core/target-cb4`.

## État : squelette

- [x] Données : `data/battle_abilities/*.json` (5 capacités), schéma
      `battle_ability.schema.json`, `tools/tests/test_battle_abilities_schema.py` ;
      `order_pavise.json` retiré de `battle_orders` (fiche Codex `cdx_jeu_pavois` → `ability_pavise`).
- [x] data-model : `BattleAbility` (+ chargement `GameData.battle_abilities`).
- [x] Cœur (squelette) : `sim-battle/src/abilities.rs`, `Command::UseAbility`,
      `BattleSetup.abilities`, `Unit.ability_state`.
- [ ] Effets branchés dans `sim.rs` (portée, tir, mêlée, charge, vitesse).
- [ ] IA `ai_abilities.rs`.
- [ ] Tests `cb4_abilities.rs`.
- [ ] Pont (`get_units` : `abilities`, `use_ability`), RuleValues.
- [ ] Godot : boutons de carte, Alt+1…4, aide F1, test, capture.
- [ ] Marges EP7/EQ7.

## Marges (release, graines des tests)

| Bataille | Bande | Avant (main) | Après |
|---|---|---|---|
| Crécy | 14-19/20 | 16 | |
| Azincourt | 14-19/20 | 18 | |
| Poitiers | 11-18/20 | 14 | |
| EQ7 | ≥ 12/16 | 15 | |

## Prochaine étape

Brancher les effets dans `sim.rs`, puis l'IA, puis les tests.
