class_name MapDataLoader
extends RefCounted

## Point d'accès unique aux données vectorielles de la carte (lot SC DT4, ADR 0206) : provinces,
## rivières, côte, routes et rivières rendues, lues par le chargeur natif `MapGeoLoader`
## (GDExtension, serde) et rendues en tableaux compacts. Les provinces ne sont parsées qu'une fois
## par chemin (cache côté Rust). Les chemins sont ceux du dossier `data/map/`.

static var _native: RefCounted = null


static func _loader() -> RefCounted:
	if _native == null:
		if not ClassDB.class_exists("MapGeoLoader"):
			push_error("MapDataLoader: GDExtension class MapGeoLoader missing (run core/build.sh)")
			return null
		_native = ClassDB.instantiate("MapGeoLoader")
	return _native


## Provinces de `provinces.geojson` : `{index, id, name, owner, terrain, capital_name, centroid,
## capital_px, neighbors, area_px, rings}` ; vide si le fichier manque.
static func provinces(map_dir: String) -> Array:
	var loader := _loader()
	return loader.call("provinces", map_dir.path_join("provinces.geojson")) if loader != null else []


## Provinces du sélecteur de faction : `{id, owner, polygons, seat?}` (`geojson_path` : fichier).
static func province_polygons(geojson_path: String) -> Array:
	var loader := _loader()
	return loader.call("province_polygons", geojson_path) if loader != null else []


## Rivières de `rivers.geojson` : `{name, importance, points}`.
static func rivers(map_dir: String) -> Array:
	var loader := _loader()
	return loader.call("rivers", map_dir.path_join("rivers.geojson")) if loader != null else []


## Lignes de côte de `coastline.geojson` (PackedVector2Array).
static func coastline(map_dir: String) -> Array:
	var loader := _loader()
	return loader.call("coastline", map_dir.path_join("coastline.geojson")) if loader != null else []


## Routes de `roads.geojson` : `{type, main, points}`.
static func roads(map_dir: String) -> Array:
	var loader := _loader()
	return loader.call("roads", map_dir.path_join("roads.geojson")) if loader != null else []


## `rivers_render.json` : `{bank_px, zones, rivers}` ; vide si le fichier manque.
static func rendered_rivers(map_dir: String, file_name: String = "rivers_render.json") -> Dictionary:
	var loader := _loader()
	return loader.call("rendered_rivers", map_dir.path_join(file_name)) if loader != null else {}


## Oublie les provinces en cache (tests qui réécrivent le fichier).
static func clear_cache() -> void:
	var loader := _loader()
	if loader != null:
		loader.call("clear_cache")
