# WIP — Ordres du chef en bataille (F10b)

Branche : `feat/battle-orders`.

## État
- [x] Données `data/battle_orders/*.json` (5 ordres) + `data/schemas/battle_order.schema.json` + test pytest.
- [x] `data-model` : `entities/battle_order.rs`, `GameData::battle_orders` (dossier facultatif).
- [x] `sim-campaign` : `battle_setup` remplit `BattleSetup::orders` (2 lignes).
- [x] `sim-battle` : `orders.rs` (catalogue, disponibilité, effets), `Command::LeaderOrder`,
      erreurs FR, `SideResult::no_quarter`, `Unit::dismount` factorisé avec le siège.
- [x] Tests Rust de chaque ordre (`tests/orders.rs`, 18 tests).
- [x] IA tactique (`ai.rs`, `plan_orders`).
- [x] Pont `get_leader_orders(side)`, `get_no_quarter`, champs `pavise`/`dismounted` dans `get_units`.
- [x] UI `leader_orders_bar.gd` + accroche (3 lignes) dans `battle_scene.gd` ; smoke étendu (vert).
- [ ] Smoke, capture `docs/img/battle-leader-orders.png`, doc `docs/design/battle-orders.md`.

## Prochaine étape
Capture `docs/img/battle-leader-orders.png`, relecture visuelle, doc.
