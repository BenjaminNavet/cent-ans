class_name CoastLook
extends RefCounted

## Lot TB5 : côtes de la carte de campagne, lues dans `data/map/coast_types.json` (schéma
## `data/schemas/coast_types.schema.json`). La région donne la géologie (roche : craie, granite ou
## roche ; plage : sable ou galets), la pente choisit entre falaise et plage : altitude lue à
## `cliff.probe_px` de la côte vers l'intérieur (même règle que `coast_common.gdshaderinc`).
## Sans fichier (données de test) : rendu inchangé. Purement visuel : aucune règle de jeu.

const DATA_PATH := "map/coast_types.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
## Ordre des types dans les tableaux du shader (`coast_color`, `coast_shade`, `coast_streak`).
const TYPES: Array[String] = ["rock", "chalk", "granite", "sand", "shingle"]
## Rayon (px carte) de recherche du rivage autour d'un point donné à `kind_at`.
const SHORE_SEARCH_PX := 6

static var _data: Dictionary = {}
static var _loaded: bool = false
static var _texture: ImageTexture = null
static var _texture_size: Vector2i = Vector2i.ZERO


static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := _data_dir().path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_data = parsed
	return _data


## Oublie le fichier lu (tests : autre dossier de données).
static func reload() -> void:
	_loaded = false
	_data = {}
	_texture = null


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Géologie de la côte au point `px` (pixels carte) : la dernière région qui le contient, sinon
## `default`. {id, rock, beach, beach_width}.
static func region_at(px: Vector2) -> Dictionary:
	var found: Dictionary = data().get("default", {})
	var id := ""
	for region: Dictionary in data().get("regions", []):
		if Geometry2D.is_point_in_polygon(px, polygon_of(region)):
			found = region
			id = str(region.get("id", ""))
	return {
		"id": id,
		"rock": str(found.get("rock", "rock")),
		"beach": str(found.get("beach", "sand")),
		"beach_width": float(found.get("beach_width", 1.0)),
	}


## Polygone `polygon_px` (pixels carte) d'une région de côte ou d'un bassin.
static func polygon_of(region: Dictionary) -> PackedVector2Array:
	var polygon := PackedVector2Array()
	for point: Array in region.get("polygon_px", []):
		polygon.append(Vector2(float(point[0]), float(point[1])))
	return polygon


## 0 plage, 1 falaise, pour l'altitude (m) lue au point de sonde (`coast_cliff_factor`).
static func cliff_factor(height_m: float) -> float:
	var cliff: Dictionary = data().get("cliff", {})
	return smoothstep(float(cliff.get("min_m", 36.0)), float(cliff.get("max_m", 52.0)), height_m)


## Type de côte près de `px` : `<roche>_cliff` (chalk, granite, rock) ou `<plage>_beach` (sand,
## shingle) ; "" si aucun rivage à moins de `SHORE_SEARCH_PX`. Le rivage le plus proche est
## cherché dans la distance à la côte, puis l'altitude est lue à `cliff.probe_px` vers l'intérieur.
static func kind_at(px: Vector2, map_data: MapData) -> String:
	var probe := shore_probe(px, map_data)
	if probe.is_empty():
		return ""
	var region := region_at(probe["shore"])
	if cliff_factor(map_data.height_m_at(probe["inland"].x, probe["inland"].y)) >= 0.5:
		return "%s_cliff" % region["rock"]
	return "%s_beach" % region["beach"]


## Rivage le plus proche de `px` et point de sonde de la règle de pente : {shore, inland}
## (pixels carte), {} sans rivage proche.
static func shore_probe(px: Vector2, map_data: MapData) -> Dictionary:
	var image := map_data.coast_dist_image if map_data != null else null
	if image == null:
		return {}
	var best := Vector2i(-1, -1)
	var best_d := INF
	var origin := Vector2i(px.floor())
	for dy in range(-SHORE_SEARCH_PX, SHORE_SEARCH_PX + 1):
		for dx in range(-SHORE_SEARCH_PX, SHORE_SEARCH_PX + 1):
			var cell := origin + Vector2i(dx, dy)
			var coast := _coast_px(image, cell)
			var distance := Vector2(dx, dy).length_squared()
			if coast > 0.0 and coast <= 1.01 and distance < best_d:  # premier pixel de terre (1 px, à l'arrondi près)
				best = cell
				best_d = distance
	if best.x < 0:
		return {}
	var gradient := Vector2(
		_coast_px(image, best + Vector2i(1, 0)) - _coast_px(image, best - Vector2i(1, 0)),
		_coast_px(image, best + Vector2i(0, 1)) - _coast_px(image, best - Vector2i(0, 1)))
	var shore := Vector2(best) + Vector2(0.5, 0.5)
	if gradient.length() < 1e-5:
		return {"shore": shore, "inland": shore}
	var probe_px := float((data().get("cliff", {}) as Dictionary).get("probe_px", 1.6))
	return {"shore": shore, "inland": shore + gradient.normalized() * (probe_px - _coast_px(image, best))}


## Distance signée à la côte (px carte, > 0 sur terre) du pixel `cell` de `coast_dist.png`.
static func _coast_px(image: Image, cell: Vector2i) -> float:
	var x := clampi(cell.x, 0, image.get_width() - 1)
	var y := clampi(cell.y, 0, image.get_height() - 1)
	return (image.get_pixel(x, y).r * 255.0 - 128.0) / 2.0


## Texture de la géologie pour une carte de `map_size` px : R craie, V granite, B galets,
## A largeur de plage ((largeur - 0,5) / 1,5). Une cellule = `cell_px` pixels carte.
static func texture(map_size: Vector2i) -> ImageTexture:
	if _texture != null and _texture_size == map_size:
		return _texture
	var cell := maxi(int(data().get("cell_px", 16)), 1)
	var width := ceili(map_size.x / float(cell))
	var height := ceili(map_size.y / float(cell))
	var image := Image.create(width, height, false, Image.FORMAT_RGBA8)
	image.fill(_geology_color(data().get("default", {})))
	for region: Dictionary in data().get("regions", []):
		fill_polygon(image, polygon_of(region), cell, _geology_color(region))
	_texture = ImageTexture.create_from_image(image)
	_texture_size = map_size
	return _texture


static func _geology_color(entry: Dictionary) -> Color:
	var rock := str(entry.get("rock", "rock"))
	return Color(
		1.0 if rock == "chalk" else 0.0,
		1.0 if rock == "granite" else 0.0,
		1.0 if str(entry.get("beach", "sand")) == "shingle" else 0.0,
		clampf((float(entry.get("beach_width", 1.0)) - 0.5) / 1.5, 0.0, 1.0))


## Remplit dans `image` (cellules de `cell` px carte) le polygone `polygon` (px carte) : balayage
## par lignes, une cellule est prise si son centre est dans le polygone.
static func fill_polygon(image: Image, polygon: PackedVector2Array, cell: int, color: Color) -> void:
	if polygon.size() < 3:
		return
	var top := INF
	var bottom := -INF
	for point in polygon:
		top = minf(top, point.y)
		bottom = maxf(bottom, point.y)
	var row_from := maxi(ceili(top / cell - 0.5), 0)
	var row_to := mini(floori(bottom / cell - 0.5), image.get_height() - 1)
	for row in range(row_from, row_to + 1):
		var y := (row + 0.5) * cell
		var crossings := PackedFloat32Array()
		for index in polygon.size():
			var a := polygon[index]
			var b := polygon[(index + 1) % polygon.size()]
			if (a.y > y) != (b.y > y):
				crossings.append(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y))
		crossings.sort()
		for pair in range(0, crossings.size() - 1, 2):
			var col_from := maxi(ceili(crossings[pair] / cell - 0.5), 0)
			var col_to := mini(ceili(crossings[pair + 1] / cell - 0.5), image.get_width())
			if col_to > col_from:
				image.fill_rect(Rect2i(col_from, row, col_to - col_from, 1), color)


## Pose la géologie, la règle de pente et le ressac sur un matériau (terrain ou mer) ; avec `band`,
## aussi la bande côtière du terrain (largeurs, couleurs). Sans données : `coast_enabled` reste faux.
static func apply(material: ShaderMaterial, map_size: Vector2i, band: bool = true) -> bool:
	if material == null or data().is_empty():
		return false
	var cliff: Dictionary = data().get("cliff", {})
	material.set_shader_parameter("coast_types", texture(map_size))
	material.set_shader_parameter("coast_enabled", true)
	material.set_shader_parameter("coast_probe_px", float(cliff.get("probe_px", 1.6)))
	material.set_shader_parameter("coast_cliff_m", Vector2(float(cliff.get("min_m", 36.0)), float(cliff.get("max_m", 52.0))))
	var swash: Dictionary = data().get("swash", {})
	for key: String in ["amount", "period_s", "out_px", "in_px", "edge_px", "film_px", "cliff"]:
		if swash.has(key):
			material.set_shader_parameter("swash_" + key, float(swash[key]))
	if not band:
		return true
	var settings: Dictionary = data().get("band", {})
	for key: String in ["cliff_px", "cliff_widen", "sea_reach_px", "beach_px", "wet_px", "strength", "relief", "far_keep"]:
		if settings.has(key):
			material.set_shader_parameter("coast_" + key, float(settings[key]))
	var colors := PackedVector3Array()
	var shades := PackedVector3Array()
	var streaks := PackedFloat32Array()
	for type_name in TYPES:
		var entry: Dictionary = (data().get("types", {}) as Dictionary).get(type_name, {})
		colors.append(_vec3(entry.get("color"), Vector3(0.4, 0.37, 0.32)))
		shades.append(_vec3(entry.get("shade"), Vector3(0.2, 0.18, 0.16)))
		streaks.append(float(entry.get("streak", 0.3)))
	material.set_shader_parameter("coast_color", colors)
	material.set_shader_parameter("coast_shade", shades)
	material.set_shader_parameter("coast_streak", streaks)
	return true


static func _vec3(values: Variant, fallback: Vector3) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return fallback
