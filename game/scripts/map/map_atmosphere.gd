class_name MapAtmosphere
extends Node3D

## Lot ME5 (chantier DN, carte) : atmosphère de la carte de campagne, rendu seulement. Réglages
## dans `data/fx/map_atmosphere.json` (schéma `fx_map_atmosphere.schema.json`) :
## - cartes de cumulus éclairées par le soleil de la carte (même direction que les ombres RV-C) ;
## - cirrus de haute couche ; rideaux de pluie lointains sur les provinces en pluie ou orage du
##   cœur ; bancs de brouillard du matin sur les fleuves (levés comme la brume TB6) ;
## - aurore boréale au bord nord (hiver, soirée dorée).
## Chaque famille est un `MultiMesh` ou un seul plan : aucun nœud par instance. Rien sous le seuil
## parchemin (le fondu `parchment` les éteint) ; rien devant le point visé. Fumées de villes et de
## feux : déjà `LifeEffects` (cheminées, panaches RV-F, incendies). `--no-me5` éteint le tout.

const DATA_PATH := "fx/map_atmosphere.json"
const CUMULUS_SHADER := preload("res://shaders/map_atmo_cumulus.gdshader")
const CIRRUS_SHADER := preload("res://shaders/map_atmo_cirrus.gdshader")
const RAIN_SHADER := preload("res://shaders/map_atmo_rain.gdshader")
const FOG_SHADER := preload("res://shaders/map_atmo_fog.gdshader")
const AURORA_SHADER := preload("res://shaders/map_atmo_aurora.gdshader")

static var _lookup := JsonLookup.new(DATA_PATH)

var enabled := true
var tuning: Dictionary = {}
var _map: Node
var _map_data: MapData
var _weather_view: CampaignWeatherView
var _sun: DirectionalLight3D
var _season := Vector4(0.0, 1.0, 0.0, 0.0)  # printemps, été, automne, hiver

var _cumulus: MultiMeshInstance3D
var _cirrus: MeshInstance3D
var _rain: MultiMeshInstance3D
var _fog: MultiMeshInstance3D
var _aurora: MeshInstance3D
var _wet_sky := 0.0
var _focus_kind := "clear"


static func data() -> Dictionary:
	return _lookup.data()


static func reload() -> void:
	_lookup.reload()


## Fondu 0 -> 1 entre `fade_in` (x, y) puis 1 -> 0 entre `fade_out` (x, y) selon la distance caméra.
static func band_fade(distance: float, fade_in: Vector2, fade_out: Vector2) -> float:
	return smoothstep(fade_in.x, fade_in.y, distance) * (1.0 - smoothstep(fade_out.x, fade_out.y, distance))


static func pair(values: Variant, fallback: Vector2) -> Vector2:
	if values is Array and (values as Array).size() >= 2:
		return Vector2(float(values[0]), float(values[1]))
	return fallback


static func color3(values: Variant, fallback: Vector3) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return fallback


## Part de bancs de brouillard présents pour des poids de saison (x printemps, y été, z automne, w hiver).
static func season_share(weights: Vector4, shares: Variant) -> float:
	if not (shares is Array and (shares as Array).size() >= 4):
		return 1.0
	var total := 0.0
	for index in 4:
		total += float(shares[index]) * weights[index]
	return clampf(total, 0.0, 1.0)


## Intensité de l'aurore : saison d'hiver seulement, plancher de jour `night_floor`, pleine au soir doré.
static func aurora_strength(winter: float, dusk: float, night_floor: float) -> float:
	return clampf(winter, 0.0, 1.0) * lerpf(night_floor, 1.0, clampf(dusk, 0.0, 1.0))


## Points de pose des bancs de brouillard : un tous les `step_px` le long des fleuves d'importance
## >= `min_importance`, au plus `max_banks` (répartis à pas régulier de la liste, déterministe).
static func river_bank_points(rivers: Array, min_importance: int, step_px: float, max_banks: int) -> PackedVector2Array:
	var points := PackedVector2Array()
	for river: Dictionary in rivers:
		if int(river.get("importance", 0)) < min_importance:
			continue
		var line: PackedVector2Array = river.get("points", PackedVector2Array())
		var carried := 0.0
		for index in range(1, line.size()):
			var segment := line[index].distance_to(line[index - 1])
			carried += segment
			if carried >= step_px:
				carried = 0.0
				points.append(line[index])
	if max_banks <= 0:
		return PackedVector2Array()
	if points.size() <= max_banks:
		return points
	var kept := PackedVector2Array()
	var stride := float(points.size()) / float(max_banks)
	for index in max_banks:
		kept.append(points[int(index * stride)])
	return kept


func setup(map: Node, weather_view: CampaignWeatherView) -> void:
	_map = map
	_map_data = map.get("map_data")
	_weather_view = weather_view
	tuning = data()
	enabled = bool(tuning.get("enabled", false)) and not CmdArgs.has("--no-me5") and _map_data != null
	if not enabled:
		return
	_sun = map.get_node_or_null("Sun") as DirectionalLight3D
	_build_cumulus()
	_build_cirrus()
	_build_fog()
	_build_aurora()
	_apply_common()


## Poids de saison (x printemps, y été, z automne, w hiver), poussé par `CampaignLife`.
func set_season(weights: Vector4) -> void:
	_season = weights


## Météo du tour : (re)pose les rideaux de pluie sur les provinces en pluie ou orage.
func refresh(weather: Dictionary) -> void:
	if not enabled:
		return
	_build_rain(weather)
	var wet := 0
	for entry: Variant in weather.values():
		if (entry as Dictionary).get("kind", "clear") in ["rain", "storm", "snow"]:
			wet += 1
	_wet_sky = clampf(float(wet) / maxf(float(weather.size()), 1.0) * 4.0, 0.0, 1.0)


func update_view(focus: Vector3, distance: float, parchment: float) -> void:
	if not enabled:
		return
	var clear := 1.0 - parchment
	var focus_rule: Dictionary = tuning.get("focus_clear", {})
	var radius := distance * float(focus_rule.get("radius_per_distance", 0.0))
	var feather := maxf(distance * float(focus_rule.get("feather_per_distance", 0.2)), 1.0)
	var sun_dir := _sun.global_transform.basis.z.normalized() if _sun != null else Vector3(0.3, 0.8, 0.5)
	var sun_color := _sun.light_color if _sun != null else Color.WHITE
	_focus_kind = _weather_view.weather_at(Vector2(focus.x, focus.z)) if _weather_view != null else "clear"
	var wet_focus := _focus_kind in ["rain", "storm", "snow"]

	var cards: Dictionary = tuning.get("cumulus_cards", {})
	_set_family(_cumulus, band_fade(distance, pair(cards.get("distance_in"), Vector2(260, 520)), pair(cards.get("distance_out"), Vector2(1000, 1250))) * clear)
	if _cumulus != null:
		var material := _cumulus.material_override as ShaderMaterial
		material.set_shader_parameter("keep", lerpf(1.0, float(cards.get("cover_wet", 0.35)), 1.0 if wet_focus else _wet_sky * 0.5))

	var cirrus_rule: Dictionary = tuning.get("cirrus", {})
	_set_family(_cirrus, band_fade(distance, pair(cirrus_rule.get("distance_in"), Vector2(420, 800)), pair(cirrus_rule.get("distance_out"), Vector2(1000, 1250))) * clear)

	var rain_rule: Dictionary = tuning.get("rain_curtains", {})
	_set_family(_rain, band_fade(distance, pair(rain_rule.get("distance"), Vector2(190, 330)), pair(rain_rule.get("distance_out"), Vector2(1000, 1250))) * clear)

	var fog_rule: Dictionary = tuning.get("river_fog", {})
	var morning := _weather_view.morning_mist() if _weather_view != null else 0.0
	var lifted := float(fog_rule.get("lifted_share", 0.1))
	var fog_fade := band_fade(distance, pair(fog_rule.get("distance"), Vector2(120, 260)), pair(fog_rule.get("distance_out"), Vector2(700, 950))) * clear
	_set_family(_fog, fog_fade * (lifted + (1.0 - lifted) * morning))
	if _fog != null:
		(_fog.material_override as ShaderMaterial).set_shader_parameter("present", season_share(_season, fog_rule.get("season_share")))

	var aurora_rule: Dictionary = tuning.get("aurora", {})
	var turn_light := _map.get_node_or_null("TurnLight") if _map != null else null
	var dusk: float = float(turn_light.get("dusk")) if turn_light != null else 0.0
	var aurora_fade := band_fade(distance, pair(aurora_rule.get("distance_in"), Vector2(450, 900)), pair(aurora_rule.get("distance_out"), Vector2(2200, 3200))) * clear
	_set_family(_aurora, aurora_fade)
	if _aurora != null:
		(_aurora.material_override as ShaderMaterial).set_shader_parameter("strength",
				aurora_strength(_season.w, dusk, float(aurora_rule.get("night_floor", 0.35))) * float(aurora_rule.get("opacity", 0.9)))

	for node: GeometryInstance3D in [_cumulus, _cirrus, _rain, _fog, _aurora]:
		if node == null or not node.visible:
			continue
		var material := node.material_override as ShaderMaterial
		material.set_shader_parameter("atm_focus", Vector2(focus.x, focus.z))
		material.set_shader_parameter("atm_clear_radius", radius if node != _aurora else 0.0)
		material.set_shader_parameter("atm_clear_feather", feather)
		material.set_shader_parameter("atm_sun_dir", sun_dir)
		material.set_shader_parameter("atm_sun_color", Vector3(sun_color.r, sun_color.g, sun_color.b))


# --- Construction ---------------------------------------------------------------------------


func _set_family(node: GeometryInstance3D, fade: float) -> void:
	if node == null:
		return
	node.visible = fade > 0.01
	(node.material_override as ShaderMaterial).set_shader_parameter("atm_fade", fade)


func _apply_common() -> void:
	var wind := pair(tuning.get("wind"), Vector2(3.0, 0.6))
	for node: GeometryInstance3D in [_cumulus, _cirrus, _rain, _fog, _aurora]:
		if node == null:
			continue
		var material := node.material_override as ShaderMaterial
		material.set_shader_parameter("atm_wind", wind)
		material.set_shader_parameter("atm_map_size", Vector2(_map_data.size))
		node.visible = false


func _instance_node(name_text: String, mesh: Mesh, shader: Shader, count: int) -> MultiMeshInstance3D:
	var multi := MultiMesh.new()
	multi.transform_format = MultiMesh.TRANSFORM_3D
	multi.use_custom_data = true
	multi.mesh = mesh
	multi.instance_count = count
	var node := MultiMeshInstance3D.new()
	node.name = name_text
	node.multimesh = multi
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.extra_cull_margin = 16384.0  # cartes dérivantes : jamais écartées par la boîte du maillage
	var material := ShaderMaterial.new()
	material.shader = shader
	node.material_override = material
	add_child(node)
	return node


func _build_cumulus() -> void:
	var rule: Dictionary = tuning.get("cumulus_cards", {})
	var count := int(rule.get("count", 0))
	if not bool(rule.get("enabled", true)) or count <= 0:
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = int(rule.get("seed", 1))
	var heights := pair(rule.get("height"), Vector2(33, 41))
	var sizes := pair(rule.get("size"), Vector2(70, 130))
	var squash := float(rule.get("squash", 0.55))
	_cumulus = _instance_node("CumulusCards", QuadMesh.new(), CUMULUS_SHADER, count)
	var map_size := Vector2(_map_data.size)
	for index in count:
		var width := rng.randf_range(sizes.x, sizes.y)
		var transform := Transform3D(Basis.from_scale(Vector3(width, width * squash, 1.0)),
				Vector3(rng.randf() * map_size.x, rng.randf_range(heights.x, heights.y), rng.randf() * map_size.y))
		_cumulus.multimesh.set_instance_transform(index, transform)
		_cumulus.multimesh.set_instance_custom_data(index, Color(rng.randf(), 0.0, 0.0, 0.0))
	var material := _cumulus.material_override as ShaderMaterial
	material.set_shader_parameter("opacity", float(rule.get("opacity", 0.55)))
	material.set_shader_parameter("drift_share", float(rule.get("drift_share", 1.0)))
	material.set_shader_parameter("lit_color", color3(rule.get("lit_color"), Vector3(1.0, 0.97, 0.92)))
	material.set_shader_parameter("shade_color", color3(rule.get("shade_color"), Vector3(0.56, 0.62, 0.72)))


func _build_cirrus() -> void:
	var rule: Dictionary = tuning.get("cirrus", {})
	if not bool(rule.get("enabled", true)):
		return
	var plane := PlaneMesh.new()
	plane.size = Vector2(_map_data.size)
	_cirrus = MeshInstance3D.new()
	_cirrus.name = "Cirrus"
	_cirrus.mesh = plane
	_cirrus.position = Vector3(_map_data.size.x * 0.5, float(rule.get("height", 96.0)), _map_data.size.y * 0.5)
	_cirrus.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := ShaderMaterial.new()
	material.shader = CIRRUS_SHADER
	for key: String in ["opacity", "scale", "stretch", "drift_share"]:
		if rule.has(key):
			material.set_shader_parameter(key, float(rule[key]))
	material.set_shader_parameter("cirrus_color", color3(rule.get("color"), Vector3(0.96, 0.97, 1.0)))
	_cirrus.material_override = material
	add_child(_cirrus)


func _build_rain(weather: Dictionary) -> void:
	if _rain != null:
		_rain.queue_free()
		_rain = null
	var rule: Dictionary = tuning.get("rain_curtains", {})
	if not bool(rule.get("enabled", true)):
		return
	var placements := rain_placements(weather, rule)
	if placements.is_empty():
		return
	_rain = _instance_node("RainCurtains", _curtain_mesh(), RAIN_SHADER, placements.size())
	for index in placements.size():
		var item: Dictionary = placements[index]
		var ground := _map_data.surface_world_at(item["position"].x, item["position"].y)
		var height := maxf(float(rule.get("height_top", 34.0)) - ground, 8.0)
		var transform := Transform3D(Basis.from_scale(Vector3(item["width"], height, 1.0)),
				Vector3(item["position"].x, ground, item["position"].y))
		_rain.multimesh.set_instance_transform(index, transform)
		_rain.multimesh.set_instance_custom_data(index, Color(item["rand"], item["storm"], 0.0, 0.0))
	var material := _rain.material_override as ShaderMaterial
	material.set_shader_parameter("opacity", float(rule.get("opacity", 0.3)))
	material.set_shader_parameter("slant", float(rule.get("slant", 0.18)))
	material.set_shader_parameter("speed", float(rule.get("speed", 1.6)))
	material.set_shader_parameter("rain_tint", color3(rule.get("color"), Vector3(0.62, 0.67, 0.76)))
	material.set_shader_parameter("storm_tint", color3(rule.get("storm_color"), Vector3(0.34, 0.36, 0.43)))
	_apply_common_to(_rain)
	_rain.visible = false


## Rideaux à poser pour une météo : `per_province` par province en pluie ou orage (barycentre
## décalé par un tirage déterministe), au plus `max_curtains`, les orages d'abord.
## Retour : [{position: Vector2 (px carte), width, rand, storm}].
func rain_placements(weather: Dictionary, rule: Dictionary) -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	if _map_data == null:
		return out
	var rng := RandomNumberGenerator.new()
	rng.seed = int(rule.get("seed", 1))
	var widths := pair(rule.get("width"), Vector2(26, 50))
	var per_province := int(rule.get("per_province", 2))
	var limit := int(rule.get("max_curtains", 160))
	var ids: Array = weather.keys()
	ids.sort()  # déterministe
	for storm_pass in [true, false]:
		for id: String in ids:
			var kind := str((weather[id] as Dictionary).get("kind", "clear"))
			if kind != ("storm" if storm_pass else "rain"):
				continue
			var centroid := _map_data.centroid_of_id(id)
			for _i in per_province:
				if out.size() >= limit:
					return out
				var offset := Vector2(rng.randf_range(-1.0, 1.0), rng.randf_range(-1.0, 1.0)) * 18.0
				out.append({"position": centroid + offset, "width": rng.randf_range(widths.x, widths.y),
						"rand": rng.randf(), "storm": 1.0 if storm_pass else 0.0})
	return out


func _curtain_mesh() -> QuadMesh:
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	quad.center_offset = Vector3(0.0, 0.5, 0.0)  # pied du rideau à l'origine
	return quad


func _build_fog() -> void:
	var rule: Dictionary = tuning.get("river_fog", {})
	if not bool(rule.get("enabled", true)):
		return
	var points := river_bank_points(_map_data.rivers, int(rule.get("min_importance", 3)),
			float(rule.get("step_px", 38.0)), int(rule.get("max_banks", 220)))
	if points.is_empty():
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = int(rule.get("seed", 1))
	var sizes := pair(rule.get("size"), Vector2(20, 34))
	var lift := float(rule.get("lift", 1.4))
	_fog = _instance_node("RiverFog", _curtain_mesh(), FOG_SHADER, points.size())
	for index in points.size():
		var width := rng.randf_range(sizes.x, sizes.y)
		var ground := _map_data.surface_world_at(points[index].x, points[index].y)
		var transform := Transform3D(Basis.from_scale(Vector3(width, width * 0.28, 1.0)),
				Vector3(points[index].x, ground + lift, points[index].y))
		_fog.multimesh.set_instance_transform(index, transform)
		_fog.multimesh.set_instance_custom_data(index, Color(rng.randf(), 0.0, 0.0, 0.0))
	var material := _fog.material_override as ShaderMaterial
	material.set_shader_parameter("opacity", float(rule.get("opacity", 0.34)))
	material.set_shader_parameter("fog_color", color3(rule.get("color"), Vector3(0.86, 0.87, 0.86)))


func _build_aurora() -> void:
	var rule: Dictionary = tuning.get("aurora", {})
	if not bool(rule.get("enabled", true)):
		return
	var heights := pair(rule.get("height"), Vector2(30, 150))
	var quad := QuadMesh.new()
	quad.size = Vector2(_map_data.size.x * float(rule.get("width_share", 0.8)), heights.y - heights.x)
	_aurora = MeshInstance3D.new()
	_aurora.name = "Aurora"
	_aurora.mesh = quad
	# Le quad regarde +Z : au bord nord (z = marge), face au sud, donc à la caméra.
	_aurora.position = Vector3(_map_data.size.x * 0.5, (heights.x + heights.y) * 0.5, float(rule.get("north_margin_px", 0.0)))
	_aurora.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := ShaderMaterial.new()
	material.shader = AURORA_SHADER
	material.set_shader_parameter("speed", float(rule.get("speed", 0.05)))
	var colors: Array = rule.get("colors", [])
	for pair_key: Array in [["color_low", 0], ["color_mid", 1], ["color_high", 2]]:
		if colors.size() > pair_key[1]:
			material.set_shader_parameter(pair_key[0], color3(colors[pair_key[1]], Vector3.ONE))
	_aurora.material_override = material
	add_child(_aurora)


func _apply_common_to(node: GeometryInstance3D) -> void:
	var material := node.material_override as ShaderMaterial
	material.set_shader_parameter("atm_wind", pair(tuning.get("wind"), Vector2(3.0, 0.6)))
	material.set_shader_parameter("atm_map_size", Vector2(_map_data.size))
