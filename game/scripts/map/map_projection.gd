class_name MapProjection
extends RefCounted

## Projection unique de la carte pour le jeu (ADR 0355, lot C5) : enveloppe de `MapProjectionNative`
## (GDExtension, cœur Rust `data_model::MapMeta`). Aucune formule de projection en GDScript : les
## deux CRS (`EPSG:3035` et `composed`) sont gérés par le cœur. Préférer les px précuits des
## données ; n'utiliser ceci que pour des points donnés en longitude/latitude réelles.

## Valeur de `map.json.crs` de la carte composée (cartogramme, ADR 0355).
const CRS_COMPOSED := "composed"

static var _by_dir: Dictionary = {}

var map_dir: String = ""
var _native: Object = null


## Projection de la carte de `map_data` (`MapData.map_dir`), mise en cache par dossier.
static func for_map(map_data: MapData) -> MapProjection:
	return for_dir(map_data.map_dir if map_data != null else "")


## Projection du dossier `map_dir` (contient `map.json`). Invalide (`valid() == false`) si la
## GDExtension ou la carte manquent.
static func for_dir(dir: String) -> MapProjection:
	if _by_dir.has(dir):
		return _by_dir[dir]
	var projection := MapProjection.new()
	projection.map_dir = dir
	if not dir.is_empty() and ClassDB.class_exists("MapProjectionNative"):
		var native: Object = ClassDB.instantiate("MapProjectionNative")
		if native != null and bool(native.call("open", dir)):
			projection._native = native
	_by_dir[dir] = projection
	return projection


static func clear_cache() -> void:
	_by_dir.clear()


func valid() -> bool:
	return _native != null


func crs() -> String:
	return str(_native.call("crs")) if _native != null else ""


func is_composed() -> bool:
	return _native != null and bool(_native.call("is_composed"))


## (longitude, latitude) réels WGS84 → pixel de la carte ; `Vector2.INF` hors carte.
func lonlat_to_px(lon: float, lat: float) -> Vector2:
	if _native == null:
		return Vector2.INF
	return _finite(_native.call("lonlat_to_px", Vector2(lon, lat)))


## Pixel de la carte → (longitude, latitude) réels ; `Vector2.INF` hors carte.
func px_to_lonlat(px: Vector2) -> Vector2:
	if _native == null:
		return Vector2.INF
	return _finite(_native.call("px_to_lonlat", px))


## Mètres EPSG:3035 absolus (E, N) → pixel de la carte ; `Vector2.INF` hors carte.
func projected_to_px(easting: float, northing: float) -> Vector2:
	if _native == null:
		return Vector2.INF
	return _finite(_native.call("projected_to_px", easting, northing))


static func _finite(v: Vector2) -> Vector2:
	return v if is_finite(v.x) and is_finite(v.y) else Vector2.INF
