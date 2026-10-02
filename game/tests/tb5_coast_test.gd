extends SceneTree

## Lot TB5 (campagne façon Thrones of Britannia) : mer et côtes.
## Point 1 : type de côte par région et par pente (falaise de craie à Douvres et Étretat, de
## granite à la pointe du Raz, plage de sable dans les Landes), bande côtière du terrain.
## Les points 2 (mers par bassin) et 3 (ressac) ajoutent leurs contrôles.
## Usage : godot --headless --path game --script res://tests/tb5_coast_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
const WATER_SHADER := preload("res://shaders/water.gdshader")
## Lieux de référence (pixels carte, projetés depuis lon/lat) et type de côte attendu.
const PLACES := {
	"Douvres (South Foreland)": [Vector2(2153.0, 2841.0), "chalk_cliff"],
	"Étretat": [Vector2(2012.6, 3047.0), "chalk_cliff"],
	"Pointe du Raz": [Vector2(1474.6, 3218.5), "granite_cliff"],
	"Land's End": [Vector2(1441.8, 2889.8), "granite_cliff"],
	"Mimizan (Landes)": [Vector2(1739.1, 3867.2), "sand_beach"],
	"Calais": [Vector2(2197.8, 2874.5), "sand_beach"],
}


func _init() -> void:
	var ok := true
	ok = _check_coast_types() and ok
	ok = _check_band() and ok
	if ok:
		print("TB5 coast test OK")
	quit(0 if ok else 1)


func _uniforms(shader: Shader) -> Array:
	return shader.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))


## Point 1 : la région donne la roche, la pente choisit falaise ou plage.
func _check_coast_types() -> bool:
	var ok := true
	if CoastLook.data().is_empty():
		push_error("TB5: data/map/coast_types.json missing")
		return false
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if map_data.load_error != "":
		push_error("TB5: map data failed to load (%s)" % map_data.load_error)
		return false
	for place: String in PLACES:
		var at: Vector2 = PLACES[place][0]
		var kind := CoastLook.kind_at(at, map_data)
		var probe := CoastLook.shore_probe(at, map_data)
		var height := map_data.height_m_at(probe["inland"].x, probe["inland"].y) if not probe.is_empty() else NAN
		print("TB5 coast %s: %s (probe %.0f m, region '%s')" % [place, kind, height, CoastLook.region_at(at)["id"]])
		if kind != PLACES[place][1]:
			push_error("TB5: %s should be %s, got '%s'" % [place, PLACES[place][1], kind])
			ok = false
	# La texture de géologie reprend les régions : craie à Douvres, granite au Raz, rien au large.
	var image := CoastLook.texture(map_data.size).get_image()
	var cell := int(CoastLook.data()["cell_px"])
	var dover := image.get_pixelv(Vector2i(PLACES["Douvres (South Foreland)"][0] / cell))
	var raz := image.get_pixelv(Vector2i(PLACES["Pointe du Raz"][0] / cell))
	var landes := image.get_pixelv(Vector2i(PLACES["Mimizan (Landes)"][0] / cell))
	if not (dover.r > 0.9 and dover.g < 0.1 and dover.b > 0.9):
		push_error("TB5: geology texture at Dover is %s, expected chalk and shingle" % dover)
		ok = false
	if not (raz.g > 0.9 and raz.r < 0.1):
		push_error("TB5: geology texture at the pointe du Raz is %s, expected granite" % raz)
		ok = false
	if not (landes.r < 0.1 and landes.g < 0.1 and landes.b < 0.1 and landes.a > dover.a):
		push_error("TB5: geology texture in the Landes is %s, expected a wide sand beach" % landes)
		ok = false
	return ok


## Point 1 : la bande côtière est branchée dans le shader de terrain par un seul crochet et reçoit
## ses réglages des données.
func _check_band() -> bool:
	var ok := true
	var uniforms := _uniforms(TERRAIN_SHADER)
	for uniform_name in ["coast_types", "coast_enabled", "coast_cliff_m", "coast_probe_px", "coast_cliff_px", "coast_beach_px", "coast_color", "coast_shade"]:
		if not uniforms.has(uniform_name):
			push_error("TB5: terrain shader lacks uniform %s" % uniform_name)
			ok = false
	if TERRAIN_SHADER.code.count("coast_band(") != 1:
		push_error("TB5: terrain shader must call coast_band exactly once")
		ok = false
	var material := ShaderMaterial.new()
	material.shader = TERRAIN_SHADER
	if not CoastLook.apply(material, Vector2i(7168, 6144)):
		push_error("TB5: coast settings not applied")
		return false
	var colors: PackedVector3Array = material.get_shader_parameter("coast_color")
	var chalk := colors[CoastLook.TYPES.find("chalk")]
	var granite := colors[CoastLook.TYPES.find("granite")]
	if material.get_shader_parameter("coast_enabled") != true or colors.size() != CoastLook.TYPES.size():
		push_error("TB5: terrain material did not receive the coast types")
		ok = false
	if not (chalk.x > 0.6 and chalk.x > granite.x * 2.0):
		push_error("TB5: chalk %s should be much lighter than granite %s" % [chalk, granite])
		ok = false
	return ok
