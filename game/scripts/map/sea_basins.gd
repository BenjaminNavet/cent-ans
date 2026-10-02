class_name SeaBasins
extends RefCounted

## Lot TB5 : mers de la carte de campagne par bassin, lues dans `data/map/sea_basins.json` (schéma
## `data/schemas/sea_basins.schema.json`) : mer du Nord et Manche sombres, Atlantique houleux,
## Méditerranée claire. Les bassins (quatre au plus) sont cuits dans une texture de poids (un canal
## chacun, limites fondues) ; leurs réglages sont des tableaux du shader (`sea_basins.gdshaderinc`,
## indice 0 = mer par défaut). La saison (TB1) s'applique par-dessus. Sans fichier (données de
## test) : rendu inchangé. Purement visuel : aucune règle de jeu.

const DATA_PATH := "map/sea_basins.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const MAX_BASINS := 4
const NEUTRAL := {"tint": [1.0, 1.0, 1.0], "clarity": 1.0, "swell": 1.0, "long_swell": 0.0, "foam": 1.0, "whitecaps": 0.0, "grey_scale": 1.0}

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


static func basins() -> Array:
	var list: Array = data().get("basins", [])
	return list.slice(0, MAX_BASINS)


## Identifiant du bassin au point `px` (pixels carte) : le dernier qui le contient, "" hors bassin.
static func basin_at(px: Vector2) -> String:
	var found := ""
	for basin: Dictionary in basins():
		if Geometry2D.is_point_in_polygon(px, CoastLook.polygon_of(basin)):
			found = str(basin.get("id", ""))
	return found


## Réglages d'un bassin ("" ou inconnu : mer par défaut), complétés par les valeurs neutres.
static func look(basin_id: String) -> Dictionary:
	var found: Dictionary = data().get("default", {})
	for basin: Dictionary in basins():
		if str(basin.get("id", "")) == basin_id:
			found = basin.get("look", {})
	var result := NEUTRAL.duplicate()
	result.merge(found, true)
	return result


## Texture des poids pour une carte de `map_size` px : un canal par bassin (R, V, B, A dans
## l'ordre du fichier), limites fondues sur `blur_cells` cellules.
static func texture(map_size: Vector2i) -> ImageTexture:
	if _texture != null and _texture_size == map_size:
		return _texture
	var cell := maxi(int(data().get("cell_px", 32)), 1)
	var blur := maxi(int(data().get("blur_cells", 4)), 1)
	var width := ceili(map_size.x / float(cell))
	var height := ceili(map_size.y / float(cell))
	var image := Image.create(width, height, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	var list := basins()
	for index in list.size():
		var color := Color(0, 0, 0, 0)
		color[index] = 1.0
		CoastLook.fill_polygon(image, CoastLook.polygon_of(list[index]), cell, color)
	if blur > 1:  # fondu : réduction filtrée puis agrandissement lisse
		image.resize(maxi(width / blur, 1), maxi(height / blur, 1), Image.INTERPOLATE_LANCZOS)
		image.resize(width, height, Image.INTERPOLATE_CUBIC)
	_texture = ImageTexture.create_from_image(image)
	_texture_size = map_size
	return _texture


## Pose les bassins sur le matériau de la mer. Sans données : `basin_on` reste faux.
static func apply(material: ShaderMaterial, map_size: Vector2i) -> bool:
	if material == null or data().is_empty():
		return false
	var tints := PackedVector3Array()
	var waves := PackedVector4Array()
	var waters := PackedVector2Array()
	var ids := [""]
	for basin: Dictionary in basins():
		ids.append(str(basin.get("id", "")))
	for slot in MAX_BASINS + 1:
		var entry := look(ids[slot]) if slot < ids.size() else look("")
		var tint: Array = entry["tint"]
		tints.append(Vector3(float(tint[0]), float(tint[1]), float(tint[2])))
		waves.append(Vector4(float(entry["swell"]), float(entry["long_swell"]), float(entry["foam"]), float(entry["whitecaps"])))
		waters.append(Vector2(float(entry["clarity"]), float(entry["grey_scale"])))
	material.set_shader_parameter("sea_basins", texture(map_size))
	material.set_shader_parameter("basin_tint", tints)
	material.set_shader_parameter("basin_wave", waves)
	material.set_shader_parameter("basin_water", waters)
	var swell: Dictionary = data().get("swell", {})
	for key: String in ["length_px", "speed", "contrast", "normal"]:
		if swell.has(key):
			material.set_shader_parameter("long_swell_" + key, float(swell[key]))
	var caps: Dictionary = data().get("whitecaps", {})
	for key: String in ["cells_per_px", "stretch", "veil"]:
		if caps.has(key):
			material.set_shader_parameter("whitecap_" + key, float(caps[key]))
	var fade: Variant = caps.get("fade_footprint")
	if fade is Array and (fade as Array).size() >= 2:
		material.set_shader_parameter("whitecap_fade", Vector2(float(fade[0]), float(fade[1])))
	material.set_shader_parameter("basin_on", true)
	return true
