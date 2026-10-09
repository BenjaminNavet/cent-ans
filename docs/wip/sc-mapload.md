# SC mapload (DT4) - chargeur Rust des donnees vectorielles de la carte

Branche `sc/mapload`. ADR 0206.

## Etat : TERMINE (a merger)
- `data-model/src/map_geo.rs` : parsing serde pur + 4 tests unitaires.
- `godot-bridge/src/map_geo.rs` : classe `MapGeoLoader` (cache provinces par chemin).
- `game/scripts/map/map_data_loader.gd` : point d'acces unique `MapDataLoader`.
- Sites migres : `MapData` (provinces, rivieres, cote), `FactionMapPicker`, `SettlementData` (routes),
  `RiversRenderer` (rivieres rendues). Parseurs GDScript supprimes (-155 / +16 lignes).
- Mesure (`tests/mapload_bench.gd`, machine chargee) : GDScript 490-530 ms -> Rust 160-230 ms
  (provinces 6 ms, selecteur depuis le cache 0-1 ms).
- Ecart voulu : nom de riviere JSON null -> "" (avant "<null>").
- Tests : smoke, settlements_render_test, hb7_river_width_test, mapload_bench OK.
