extends SceneTree

## Lot TB5 (campagne façon Thrones of Britannia) : mer et côtes.
## Point 1 : type de côte par région et par pente (falaise de craie à Douvres et Étretat, de
## granite à la pointe du Raz, plage de sable dans les Landes), bande côtière du terrain.
## Point 2 : mers par bassin (mer du Nord et Manche sombres, Atlantique houleux, Méditerranée
## claire), valeurs de `data/map/sea_basins.json`.
## Point 3 : ressac animé au trait de côte (lame partagée par la mer et la plage), sobre.
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
## Points en mer (pixels carte) et bassin attendu ("" : mer par défaut).
const SEAS := {
	"Manche": [Vector2(1905.0, 2955.0), "north_sea_channel"],
	"Mer du Nord": [Vector2(2386.0, 2110.0), "north_sea_channel"],
	"Golfe de Gascogne": [Vector2(1479.0, 3620.0), "atlantic"],
	"Golfe du Lion": [Vector2(2362.0, 4203.0), "mediterranean"],
	"Baltique": [Vector2(3752.0, 1937.0), "baltic"],
	"Mer Noire": [Vector2(5670.0, 3715.0), ""],
}


func _init() -> void:
	var ok := true
	ok = _check_coast_types() and ok
	ok = _check_band() and ok
	ok = _check_basins() and ok
	ok = _check_swash() and ok
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


## Point 2 : chaque mer a son bassin ; la texture de poids et les réglages arrivent au matériau de
## la mer ; la Méditerranée est plus claire que la Manche, l'Atlantique a la plus forte houle.
func _check_basins() -> bool:
	var ok := true
	if SeaBasins.data().is_empty():
		push_error("TB5: data/map/sea_basins.json missing")
		return false
	var uniforms := _uniforms(WATER_SHADER)
	for uniform_name in ["sea_basins", "basin_on", "basin_tint", "basin_wave", "basin_water", "long_swell_length_px"]:
		if not uniforms.has(uniform_name):
			push_error("TB5: water shader lacks uniform %s" % uniform_name)
			ok = false
	var map_size := Vector2i(7168, 6144)
	var ids: Array = [""]
	for basin: Dictionary in SeaBasins.basins():
		ids.append(basin["id"])
	var image := SeaBasins.texture(map_size).get_image()
	var cell := int(SeaBasins.data()["cell_px"])
	for sea: String in SEAS:
		var at: Vector2 = SEAS[sea][0]
		var expected: String = SEAS[sea][1]
		var basin := SeaBasins.basin_at(at)
		var weights := image.get_pixelv(Vector2i(at / cell))
		var look := SeaBasins.look(basin)
		print("TB5 sea %s: basin '%s', weights %s, tint %s, clarity %s, swell %s, long swell %s" % [sea, basin, weights, look["tint"], look["clarity"], look["swell"], look["long_swell"]])
		if basin != expected:
			push_error("TB5: %s should be in basin '%s', got '%s'" % [sea, expected, basin])
			ok = false
		var slot := ids.find(expected)
		var own: float = 1.0 - (weights.r + weights.g + weights.b + weights.a) if slot == 0 else weights[slot - 1]
		if own < 0.8:
			push_error("TB5: basin texture at %s gives %.2f to '%s' (%s)" % [sea, own, expected, weights])
			ok = false
	var sea_node := Sea.new()
	sea_node.setup(map_size)
	var material := sea_node.material_override as ShaderMaterial
	var tints: PackedVector3Array = material.get_shader_parameter("basin_tint") if material.get_shader_parameter("basin_on") == true else PackedVector3Array()
	var waves: PackedVector4Array = material.get_shader_parameter("basin_wave") if tints.size() > 0 else PackedVector4Array()
	var waters: PackedVector2Array = material.get_shader_parameter("basin_water") if tints.size() > 0 else PackedVector2Array()
	if tints.size() != SeaBasins.MAX_BASINS + 1 or waves.size() != tints.size() or waters.size() != tints.size():
		push_error("TB5: sea material did not receive the basin settings")
		sea_node.free()
		return false
	var north := ids.find("north_sea_channel")
	var atlantic := ids.find("atlantic")
	var med := ids.find("mediterranean")
	if not (tints[north].length() < tints[atlantic].length() and tints[atlantic].length() < tints[med].length()):
		push_error("TB5: expected North Sea darker than the Atlantic, Mediterranean lighter")
		ok = false
	if not (waters[north].x < 1.0 and waters[med].x > 1.5 and tints[med].z > tints[med].x):
		push_error("TB5: expected a murky Channel and a clear blue Mediterranean")
		ok = false
	if not (waves[atlantic].x > waves[north].x and waves[atlantic].x > waves[med].x and waves[atlantic].y > 0.5 and waves[med].y < 0.1):
		push_error("TB5: expected the Atlantic to carry the heaviest swell")
		ok = false
	# TB1 conservé : la saison s'applique toujours par-dessus le bassin.
	sea_node.apply_season(SeasonVisuals.weights_of("winter"))
	if float(material.get_shader_parameter("season_grey_amount")) <= 0.3:
		push_error("TB5: the winter sea look is no longer applied")
		ok = false
	sea_node.free()
	return ok


## Point 3 : le ressac est la même lame dans la mer et sur la plage (`coast_swash`, include
## commun) ; ses réglages viennent des données et arrivent aux deux matériaux ; il reste sobre
## (lent, plus étroit que la plage, moindre au pied des falaises).
func _check_swash() -> bool:
	var ok := true
	for shader: Shader in [WATER_SHADER, TERRAIN_SHADER]:
		var uniforms := _uniforms(shader)
		for uniform_name in ["swash_amount", "swash_period_s", "swash_out_px", "swash_in_px", "swash_cliff", "coast_types"]:
			if not uniforms.has(uniform_name):
				push_error("TB5: %s lacks uniform %s" % [shader.resource_path, uniform_name])
				ok = false
	var common := FileAccess.get_file_as_string("res://shaders/coast_common.gdshaderinc")
	var band := FileAccess.get_file_as_string("res://shaders/coast_band.gdshaderinc")
	if not common.contains("vec2 coast_swash(") or not common.contains("TIME") or not band.contains("coast_swash(") or not WATER_SHADER.code.contains("coast_swash("):
		push_error("TB5: the animated swash is not shared by the sea and the coast band")
		ok = false
	var swash: Dictionary = CoastLook.data().get("swash", {})
	var sea_node := Sea.new()
	sea_node.setup(Vector2i(7168, 6144))
	var sea_material := sea_node.material_override as ShaderMaterial
	var terrain_material := ShaderMaterial.new()
	terrain_material.shader = TERRAIN_SHADER
	CoastLook.apply(terrain_material, Vector2i(7168, 6144))
	for material: ShaderMaterial in [sea_material, terrain_material]:
		if material.get_shader_parameter("coast_enabled") != true or not is_equal_approx(float(material.get_shader_parameter("swash_amount")), float(swash.get("amount", -1.0))) or not is_equal_approx(float(material.get_shader_parameter("swash_period_s")), float(swash.get("period_s", -1.0))):
			push_error("TB5: a material did not receive the swash settings")
			ok = false
	if float(swash.get("amount", 0.0)) <= 0.0 or float(swash.get("period_s", 0.0)) < 4.0 or float(swash.get("in_px", 9.0)) >= float(CoastLook.data()["band"]["beach_px"]) or float(swash.get("cliff", 1.0)) >= 1.0:
		push_error("TB5: swash settings are not sober (%s)" % swash)
		ok = false
	sea_node.free()
	return ok
