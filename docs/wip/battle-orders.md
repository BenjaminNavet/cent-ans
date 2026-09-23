# WIP — Ordres du chef en bataille (F10b)

Branche : `feat/battle-orders`. Spécification : `docs/design/battle-orders.md`.

## État : terminé (à fusionner)
- [x] Données `data/battle_orders/*.json` (5 ordres) + `data/schemas/battle_order.schema.json` + test pytest.
- [x] `data-model` : `entities/battle_order.rs`, `GameData::battle_orders` (dossier facultatif).
- [x] `sim-campaign` : `battle_setup` remplit `BattleSetup::orders` (2 lignes).
- [x] `sim-battle` : `orders.rs`, `Command::LeaderOrder`, erreurs FR, `SideResult::no_quarter`,
      `Unit::dismount` factorisé avec le siège, IA `plan_orders`, 18 tests (`tests/orders.rs`).
- [x] Pont `get_leader_orders(side)`, `get_no_quarter`, champs `pavise`/`dismounted` dans `get_units`.
- [x] UI `leader_orders_bar.gd` + accroche (3 lignes) dans `battle_scene.gd` ; smoke étendu (vert).
- [x] Capture `docs/img/battle-leader-orders.png`, doc.

## Points ouverts
- `sim-campaign` n'exploite pas encore `no_quarter` (rançons, prisonniers).
- Icônes PNG facultatives (`game/assets/ui/orders/<icon>.png`) absentes : glyphes de repli.
- La marge basse de la barre (216 px) suit le bandeau actuel de `battle_hud.gd` ; à réajuster si
  la refonte du HUD (autre agent) change sa hauteur.
