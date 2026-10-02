extends SceneTree

## Lot TB6 : lumière et atmosphère de la carte de campagne. Vérifie les paramètres appliqués par
## saison et par météo (ombres de nuages bornées, lumière dorée, brume du matin) et que la brume
## épargne la province sélectionnée. Les chiffres de rendu viennent de `tb6_shot.gd`.
## Usage : godot --headless --path game --script res://tests/tb6_light_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := Vector2(2213.2, 3203.9)
const SCREEN := Vector2i(1600, 900)
const SEASONS := ["spring", "summer", "autumn", "winter"]
## Baisse de luminance tolérée au cœur d'une ombre de nuage (brief TB6).
const MAX_GROUND_DIM := 0.15

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	if _failures == 0:
		print("tb6_light OK")
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_DISABLED
	root.size = SCREEN
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	_test_cloud_shadows(map)
	await _test_golden_light(map)
	_test_quality(map)
	_test_morning_mist(map)
	map.queue_free()
	await process_frame


## 1. Ombres de nuages : aucune par temps clair, douces et bornées sous une vraie météo.
func _test_cloud_shadows(map: Node3D) -> void:
	var view: CampaignWeatherView = map.weather_view
	if not _check(view != null and view.enabled, "campaign weather view missing"):
		return
	var material: ShaderMaterial = map.terrain.material
	var clouds := MapReadability.section("clouds")
	for key: String in ["wet_dim", "mask_shadow", "mask_shadow_softness", "mask_shadow_scale"]:
		_check(clouds.has(key), "clouds.%s missing in data/ui/campaign_map.json" % key)
	# Les valeurs des données arrivent sur le matériau du terrain.
	var pairs := {
		"weather_wet_dim": "wet_dim", "weather_cloud_shade": "mask_shadow",
		"weather_cloud_shade_soft": "mask_shadow_softness", "weather_cloud_shade_scale": "mask_shadow_scale",
	}
	for uniform_name: String in pairs:
		var applied: Variant = material.get_shader_parameter(uniform_name)
		_check(applied != null and is_equal_approx(float(applied), float(clouds.get(pairs[uniform_name], -1.0))),
			"%s should carry clouds.%s (got %s)" % [uniform_name, pairs[uniform_name], applied])
	# Cumul au cœur d'un orage : sol mouillé × ombre des nuées × ombres de nuages du terrain.
	_check(view.max_ground_dim() <= MAX_GROUND_DIM, "weather may not dim the ground by more than 15 %% (%.3f)" % view.max_ground_dim())
	_check(view.max_ground_dim() > 0.03, "real weather should still shade the ground a little")
	# Ombre large et fondue : seuil plus large et bruit plus lent que le plan de nuées (0,22 ; 1).
	_check(view.mask_shadow_softness >= 0.4, "cloud shade edge should be soft (%.2f)" % view.mask_shadow_softness)
	_check(view.mask_shadow_scale <= 0.6, "cloud shade should be broader than the clouds (%.2f)" % view.mask_shadow_scale)
	# Temps clair : rien (règle TB2).
	_check(is_zero_approx(view.shadow_amount_for("clear")) and is_zero_approx(view.shadow_amount_for("fog")), "no cloud shadow in fair weather")
	var saved: Dictionary = view.weather.duplicate(true)
	view.weather = {}
	view._upload_mask()
	_check(not bool(material.get_shader_parameter("weather_enabled")), "clear weather everywhere: no weather mask on the ground")
	view.weather = saved
	view._upload_mask()
	print("tb6 cloud shadows: wet %.2f, mask shade %.2f (soft %.2f, scale %.2f), terrain %.2f -> max dim %.3f" % [
		view.wet_dim, view.mask_shadow, view.mask_shadow_softness, view.mask_shadow_scale, view.weather_shadow_amount, view.max_ground_dim()])


## 2. Lumière dorée par défaut : soleil plus bas qu'avant TB6 (ombres plus longues), teinte chaude
## au printemps, en été et en automne ; l'hiver garde son soleil pâle et son étalonnage bleuté (TB1).
func _test_golden_light(map: Node3D) -> void:
	# Hauteurs d'avant TB6 (PO3) : la lumière dorée ne remonte jamais le soleil.
	var former := {"spring": 26.0, "summer": 28.0, "autumn": 22.0, "winter": 18.0}
	var atmosphere: CampaignAtmosphere = null
	for node in map.find_children("*", "", true, false):
		if node is CampaignAtmosphere:
			atmosphere = node
	if not _check(atmosphere != null, "campaign atmosphere node missing"):
		return
	var sun := map.get_node_or_null("Sun") as DirectionalLight3D
	var initial := atmosphere.current_season()
	for season: String in SEASONS:
		var preset := CampaignAtmosphere.resolve_preset(season)
		if not _check(not preset.is_empty(), "%s: no campaign preset" % season):
			continue
		var elevation := float(preset["sun_elevation"])
		var color: Color = preset["sun_color"]
		var shadow_length := 1.0 / tan(deg_to_rad(elevation))
		_check(elevation <= float(former[season]) and elevation >= 18.0, "%s: sun should sit lower than before TB6 (%.1f)" % [season, elevation])
		_check(elevation <= 21.0 and shadow_length >= 2.6, "%s: long golden-hour shadows expected (%.2f x height)" % [season, shadow_length])
		if season == "winter":
			# L'hiver reste bleuté : soleil pâle, étalonnage froid de TB1 intact.
			_check(color.r - color.b <= 0.25, "winter: sun should stay pale (%s)" % color)
			_check(float(SeasonLook.grade("winter").get("temperature", 0.0)) < 0.0, "winter: TB1 cold grade lost")
		else:
			_check(color.r - color.b >= 0.3 and color.g - color.b >= 0.15, "%s: sun should be golden (%s)" % [season, color])
			_check(float(preset["fog_sun_scatter"]) >= 0.3, "%s: golden haze towards the sun expected" % season)
		var fog: Color = preset["fog_color"]
		_check(fog.b > fog.r, "%s: aerial perspective should stay bluish away from the sun" % season)
		# Les valeurs arrivent sur le soleil de la scène.
		atmosphere.apply_season(season)
		await process_frame
		if sun != null and not atmosphere._sun_dirty:
			var applied := rad_to_deg(asin(clampf(sun.global_basis.z.y, -1.0, 1.0)))
			_check(absf(applied - elevation) < 0.5, "%s: sun node elevation %.1f, expected %.1f" % [season, applied, elevation])
			_check(sun.light_color.is_equal_approx(color), "%s: sun node colour %s" % [season, sun.light_color])
			_check(is_equal_approx(sun.light_energy, float(preset["sun_energy"])), "%s: sun node energy" % season)
		print("tb6 light %s: elevation %.0f (was %.0f), shadows %.2f x height, sun %s x %.2f, scatter %.2f" % [
			season, elevation, float(former[season]), shadow_length, color.to_html(false), float(preset["sun_energy"]), float(preset["fog_sun_scatter"])])
	atmosphere.apply_season(initial)


## 3. SSIL derrière le réglage de qualité (Haute et Ultra) ; ni SDFGI ni brouillard volumétrique
## sur la carte, à aucun niveau (ADR 0156).
func _test_quality(map: Node3D) -> void:
	var world_env := map.get_node_or_null("WorldEnvironment") as WorldEnvironment
	if not _check(world_env != null and world_env.environment != null, "campaign environment missing"):
		return
	var expected := {"low": false, "medium": false, "high": true, "ultra": true}
	var saved := RenderQuality.override_level
	for level: String in expected:
		RenderQuality.override_level = level
		RenderQuality.apply_environment(world_env)
		var env := world_env.environment
		_check(env.ssil_enabled == bool(expected[level]), "%s: SSIL should be %s on the map" % [level, expected[level]])
		_check(not env.sdfgi_enabled, "%s: SDFGI must stay off on the campaign map" % level)
		_check(not env.volumetric_fog_enabled, "%s: no volumetric fog on the campaign map (height mist in the ground shader)" % level)
	RenderQuality.override_level = saved
	RenderQuality.apply_environment(world_env)
	print("tb6 quality: SSIL on for high and ultra, SDFGI and volumetric fog off on the map at every level")


## 4. Brume du matin : nappe de vallée réglée par les données, portée par la seule météo « brume »
## (canal B du masque), qui se lève au fil du tour et épargne la province sélectionnée.
func _test_morning_mist(map: Node3D) -> void:
	var view: CampaignWeatherView = map.weather_view
	var material: ShaderMaterial = map.terrain.material
	var mist := MapReadability.section("morning_mist")
	if not _check(not mist.is_empty(), "morning_mist block missing in data/ui/campaign_map.json"):
		return
	var opacity := float(mist.get("opacity", 0.0))
	_check(opacity > 0.2 and opacity <= 0.7, "valley mist should be visible but never opaque (%.2f)" % opacity)
	_check(is_equal_approx(float(material.get_shader_parameter("weather_valley_mist")), opacity), "weather_valley_mist should carry morning_mist.opacity")
	var depth: Vector2 = material.get_shader_parameter("weather_valley_depth_m")
	_check(depth.x >= 10.0 and depth.y > depth.x, "mist should pool in valleys only, not on open plains (%s)" % depth)
	_check(float(material.get_shader_parameter("weather_valley_lift")) < 1.0, "mist should thin out once lifted")
	# Par météo : seul le brouillard remplit le canal B.
	var paris: int = map.map_data.index_of_id("prov_ile_de_france")
	var id := str(map.map_data.get_province(paris).get("id", ""))
	var saved: Dictionary = view.weather.duplicate(true)
	for kind: String in CampaignWeatherView.KINDS:
		view.weather = {id: {"kind": kind, "intensity": 0.8, "label": kind}}
		view._upload_mask()
		var texel: Color = view.mask_image().get_pixel(paris, 0)
		_check((texel.b > 0.0) == (kind == "fog"), "%s: mist channel %.2f" % [kind, texel.b])
		_check(bool(material.get_shader_parameter("weather_enabled")) == (kind != "clear"), "%s: weather mask enabled flag" % kind)
		_check(is_equal_approx(float(material.get_shader_parameter("weather_valley_mist")), opacity), "%s: valley mist opacity kept" % kind)
	view.weather = saved
	view._upload_mask()
	# La brume se lève au fil du tour.
	var focus := Vector3(PARIS.x, 0.0, PARIS.y)
	view._fog_clock = 0.0
	view.update_view(focus, 400.0, 0.0)
	var early := float(material.get_shader_parameter("weather_fog_morning"))
	view._fog_clock = view.fog_lift_seconds + 1.0
	view.update_view(focus, 400.0, 0.0)
	var late := float(material.get_shader_parameter("weather_fog_morning"))
	_check(early > 0.9 and late < 0.05, "morning mist should lift during the turn (%.2f -> %.2f)" % [early, late])
	# Jamais sur la province sélectionnée : le shader multiplie la nappe par le dégagement.
	var include := FileAccess.get_file_as_string("res://shaders/campaign_weather.gdshaderinc")
	var line := ""
	for candidate in include.split("\n"):
		if candidate.strip_edges().begins_with("valley_mist = clamp("):
			line = candidate
	_check(line.contains("* clear"), "valley mist must be cut on the selected province (wx_selection_clear)")
	map.selected_index = paris
	view._update_selection_clear()
	_check(int(material.get_shader_parameter("weather_clear_id")) == paris, "selected province should be cleared of mist")
	map.selected_index = 0
	view._update_selection_clear()
	_check(int(material.get_shader_parameter("weather_clear_id")) == 0, "no cleared province once the selection is dropped")
	print("tb6 mist: opacity %.2f, valley depth %s m, lift %.2f -> %.2f, cleared province %d" % [opacity, depth, early, late, paris])


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb6_light: " + message)
	return condition
