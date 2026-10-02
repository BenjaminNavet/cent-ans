extends SceneTree

## Lot TB1 (campagne façon Thrones of Britannia) : saisons visibles à tous les zooms.
## Squelette : les contrôles arrivent avec chaque point du lot (delta saisonnier par-dessus la carte
## de couleur, mer selon la saison, étalonnage par saison lu dans `data/ui/`, neige sur les toits
## des villes 1:1).
## Usage : godot --headless --path game --script res://tests/tb1_seasons_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
const WATER_SHADER := preload("res://shaders/water.gdshader")


func _init() -> void:
	var ok := true
	ok = _check_ground_delta() and ok
	ok = _check_sea() and ok
	ok = _check_grade() and ok
	ok = _check_roofs() and ok
	if ok:
		print("TB1 seasons test OK")
	quit(0 if ok else 1)


## Point 1 : delta saisonnier appliqué par-dessus la carte de couleur (SS2) et les matières (HB3).
## Les réglages existent, et l'écart de saison est bien passé aux deux crochets : il n'est plus
## effacé par la carte (les chiffres de rendu viennent de `ss_shot.gd --stats --season=`).
func _check_ground_delta() -> bool:
	var ok := true
	var uniforms := _uniforms(TERRAIN_SHADER)
	for uniform_name in ["sg_season_strength", "hb_season_strength", "hb_season_gain", "summer_gold", "winter_south", "winter_cover"]:
		if not uniforms.has(uniform_name):
			push_error("TB1: terrain shader lacks uniform %s" % uniform_name)
			ok = false
	var code := TERRAIN_SHADER.code
	for hook in ["sg_apply(uv, footprint, snow, land_f, season_k, col)", "k_grass, k_forest,"]:
		if not code.contains(hook):
			push_error("TB1: terrain shader no longer passes the seasonal delta (%s)" % hook)
			ok = false
	for path in ["res://shaders/satellite_ground.gdshaderinc", "res://shaders/hb_ground.gdshaderinc"]:
		var include := FileAccess.get_file_as_string(path)
		if not include.contains("season_k") and not include.contains("k_forest"):
			push_error("TB1: %s ignores the seasonal delta" % path)
			ok = false
	return ok


func _uniforms(shader: Shader) -> Array:
	return shader.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))


## Point 2 : mer selon la saison (valeurs de `data/ui/campaign_seasons.json`) : plus sombre et
## plus grise l'hiver que l'été, fondu entre deux saisons, écume de tempête réglée par les données
## et masque météo repris du terrain.
func _check_sea() -> bool:
	var ok := true
	var uniforms := _uniforms(WATER_SHADER)
	for uniform_name in ["season_tint", "season_desaturate", "season_foam", "weather_mask", "province_ids", "storm_foam_gain", "storm_whitecaps"]:
		if not uniforms.has(uniform_name):
			push_error("TB1: water shader lacks uniform %s" % uniform_name)
			ok = false
	var summer := SeasonLook.sea(SeasonVisuals.weights_of("summer"))
	var winter := SeasonLook.sea(SeasonVisuals.weights_of("winter"))
	var summer_tint: Vector3 = summer["tint"]
	var winter_tint: Vector3 = winter["tint"]
	if winter_tint.x + winter_tint.y + winter_tint.z > summer_tint.x + summer_tint.y + summer_tint.z - 0.5:
		push_error("TB1: winter sea %s not darker than summer %s" % [winter_tint, summer_tint])
		ok = false
	if float(winter["desaturate"]) < float(summer["desaturate"]) + 0.3:
		push_error("TB1: winter sea not greyer than summer")
		ok = false
	var half := SeasonLook.sea(Vector4(0, 0.5, 0, 0.5))
	if not is_equal_approx(float(half["desaturate"]), (float(summer["desaturate"]) + float(winter["desaturate"])) * 0.5):
		push_error("TB1: sea look not blended between seasons")
		ok = false
	var storm := SeasonLook.storm()
	if float(storm["foam_gain"]) <= 0.0 or float(storm["whitecaps"]) <= 0.0:
		push_error("TB1: storm foam missing from data/ui/campaign_seasons.json")
		ok = false
	# La mer reçoit la saison et le masque météo du terrain.
	var sea := Sea.new()
	sea.setup(Vector2i(64, 64))
	sea.apply_season(SeasonVisuals.weights_of("winter"))
	var material := sea.material_override as ShaderMaterial
	if material.get_shader_parameter("season_tint") != winter_tint or float(material.get_shader_parameter("storm_foam_gain")) != float(storm["foam_gain"]):
		push_error("TB1: sea material did not receive the season or storm settings")
		ok = false
	var terrain_material := ShaderMaterial.new()
	terrain_material.shader = TERRAIN_SHADER
	terrain_material.set_shader_parameter("weather_enabled", true)
	sea.sync_weather(terrain_material)
	if material.get_shader_parameter("weather_enabled") != true:
		push_error("TB1: sea material did not receive the weather mask state")
		ok = false
	sea.free()
	return ok


## Point 3 : étalonnage par saison lu dans `data/ui/campaign_seasons.json` et composé dans le
## préréglage de la carte. Un gris moyen sort plus chaud en automne qu'en été, froid au printemps,
## bleuté en hiver ; une couleur vive sort désaturée en hiver.
func _check_grade() -> bool:
	var ok := true
	var warmth := {}
	var chroma := {}
	for season in AtmosphereLibrary.SEASONS:
		var grade := SeasonLook.grade(season)
		var preset := CampaignAtmosphere.resolve_preset(season)
		if grade.is_empty() or preset.is_empty() or not (preset["grades"] as Array).has(grade):
			push_error("TB1: %s season grade missing from the campaign preset" % season)
			ok = false
			continue
		var prepared := AtmosphereLibrary._prepare(grade)
		var grey := AtmosphereLibrary._grade(Vector3(0.5, 0.5, 0.5), prepared)
		warmth[season] = grey.x - grey.z
		var vivid := AtmosphereLibrary._grade(Vector3(0.3, 0.6, 0.2), prepared)
		chroma[season] = maxf(vivid.x, maxf(vivid.y, vivid.z)) - minf(vivid.x, minf(vivid.y, vivid.z))
	if not ok:
		return false
	print("TB1 grade: warmth %s, chroma %s" % [warmth, chroma])
	if not (float(warmth["autumn"]) > float(warmth["summer"]) and float(warmth["summer"]) > 0.01):
		push_error("TB1: summer should be golden and autumn warmer (%s)" % warmth)
		ok = false
	if not (float(warmth["spring"]) < 0.0 and float(warmth["winter"]) < float(warmth["spring"])):
		push_error("TB1: spring should be cool and winter bluer (%s)" % warmth)
		ok = false
	if float(chroma["winter"]) > float(chroma["summer"]) * 0.85:
		push_error("TB1: winter should be desaturated (%s)" % chroma)
		ok = false
	return ok


## Point 4 : neige sur les toits des villes 1:1.
func _check_roofs() -> bool:
	return true
