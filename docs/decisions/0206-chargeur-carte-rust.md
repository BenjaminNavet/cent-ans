# 0206 — Chargeur Rust des données vectorielles de la carte

Statut : accepté (lot SC DT4, 2026-10-09).

## Contexte
Le chargement de la carte de campagne parsait en GDScript (`JSON.parse_string`, puis boucles de
conversion) environ 17 Mo de JSON : `provinces.geojson` (deux fois : `MapData` et le sélecteur de
faction), `rivers.geojson`, `coastline.geojson`, `roads.geojson` et `rivers_render.json`. Chaque
coordonnée passait par un `Array` de `Variant` avant d'être copiée dans un `PackedVector2Array`.

## Décision
- Parsing pur dans `data-model/src/map_geo.rs` (serde, sans type Godot, testé en unitaire) ; les
  points sont lus en `f64` puis ramenés à `f32` comme le fait `Vector2` (valeurs identiques à
  l'ancien code).
- Classe GDExtension `MapGeoLoader` (`godot-bridge/src/map_geo.rs`) : lit le fichier par
  `FileAccess` (chemins `res://` et absolus), appelle le parseur et renvoie des dictionnaires de la
  même forme que les anciens parseurs, avec des `PackedVector2Array` / `PackedFloat32Array` prêts à
  l'emploi. Cache des provinces par chemin (un seul parsing pour `MapData` et le sélecteur).
- Point d'accès GDScript unique : `MapDataLoader` (`game/scripts/map/map_data_loader.gd`). Les
  parseurs GDScript de `MapData`, `FactionMapPicker`, `SettlementData` et `RiversRenderer` sont
  supprimés ; pas de repli GDScript (l'extension est de toute façon requise par le jeu).

## Conséquences
- Écart voulu : un nom de rivière JSON `null` donne `""` (l'ancien `str(null)` donnait `"<null>"`) ;
  aucun consommateur ne dépendait de cette valeur.
- Les zones de `rivers_render.json` sont rendues brutes ; `RiversRenderer` applique lui-même le
  `landmark_scale` (règle d'affichage).
- Test de non-régression : `game/tests/mapload_bench.gd` (comparaison ancien/nouveau + temps).
