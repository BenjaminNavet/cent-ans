# SC mapload (DT4) — chargeur Rust des données vectorielles de la carte

Branche `sc/mapload`. Objectif : remplacer les parseurs GDScript de provinces / rivières / côte /
routes / rivières rendues par un chargeur Rust (GDExtension) avec cache des provinces.

## État
- Mesure de départ : `game/tests/mapload_bench.gd` (parse JSON seul : ~250 ms ; construction des
  dictionnaires en plus, à chronométrer dans le banc).
- Squelette Rust : `data-model/src/map_geo.rs` (parsing pur) + `godot-bridge/src/map_geo.rs` (classe `MapGeoLoader`).

## Prochaine étape
Implémenter le parsing, brancher `MapDataLoader` (GDScript, point d'accès unique), test
ancien/nouveau, supprimer les parseurs GDScript.
