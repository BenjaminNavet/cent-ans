# CB4 — Capacités actives (état)

Branche : `feat/cb4-abilities` (depuis `main` 07eff33b, après « docs: CB wave 4 merged, CB4 next »).
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ CB4) ; relecture
historique : `docs/research/cb4-capacites.md`. Cible cargo privée :
`CARGO_TARGET_DIR=<worktree>/core/target-cb4`.

## État : cœur branché, équilibrage de l'IA en cours

- [x] Données : `data/battle_abilities/*.json` (5 capacités), schéma
      `battle_ability.schema.json`, `tools/tests/test_battle_abilities_schema.py` ;
      `order_pavise.json` retiré de `battle_orders` (fiche Codex `cdx_jeu_pavois` → `ability_pavise`).
- [x] data-model : `BattleAbility` (+ chargement `GameData.battle_abilities`).
- [x] Cœur : `sim-battle/src/abilities.rs`, `Command::UseAbility`, `BattleSetup.abilities`,
      `Unit.ability_state` ; effets dans `sim.rs` (portée, tir, mêlée, charge, vitesse).
- [~] IA `ai_abilities.rs` (règles écrites ; tir tendu limité au tir de flanc pour tenir
      Azincourt ≤ 19).
- [ ] Tests `cb4_abilities.rs`.
- [ ] Pont (`get_units` : `abilities`, `use_ability`), RuleValues.
- [ ] Godot : boutons de carte, Alt+1…4, aide F1, test, capture.

## Marges (release, graines des tests)

| Bataille | Bande | Avant (main) | Après |
|---|---|---|---|
| Crécy | 14-19/20 | 16 | |
| Azincourt | 14-19/20 | 18 | |
| Poitiers | 11-18/20 | 14 | |
| EQ7 | ≥ 12/16 | 15 | |

## Notes d'équilibrage

- L'ordre `order_pavise` ne se déclenchait jamais dans les batailles de référence : le filtre de
  scénario EP7 (`scenario_filter`) écarte les ordres des régiments tenus ou lancés à l'assaut.
  `UseAbility` passe désormais par le même filtre : à Crécy, les Génois ne dressent pas plus
  leurs pavois qu'avant (résultat identique graine par graine).
- Sonde : `cargo test --release -p sim-battle --test cb4_abilities -- --ignored --nocapture probe_margins`.
  Première mesure (règles non restreintes) : tir tendu seul → Azincourt 20/20 (hors bande).

## Prochaine étape

Finir l'équilibrage de l'IA, puis les tests `cb4_abilities.rs`, le pont, Godot.
