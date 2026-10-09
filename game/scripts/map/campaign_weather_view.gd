class_name CampaignWeatherView
extends Node3D

## Lot CM2 : météo de la carte de campagne, rendue à partir du cœur (`get_campaign_weather`,
## ADR 0027 ; aucune météo inventée ici). Une fois par tour : masque par province (R pluie,
## G neige, B brouillard, A orage) posé sur le terrain (sol mouillé, neige fraîche, brouillard
## matinal qui se lève, ombre des nuées, éclairs) et sur un plan de nuées qui défilent avec le
## vent ; de près, pluie ou neige en particules autour du point visé et éclairs d'orage.
## Options (après `--`) : `--map-weather=rain|snow|fog|storm|clear` (impose partout, captures).

const CLOUD_SHADER := preload("res://shaders/campaign_clouds.gdshader")
const FIELD_BAKE_SHADER := preload("res://shaders/weather_field_bake.gdshader")
## FL3 (ADR 0192) : pixels carte par texel du champ météo cuit (frange de 30 px fondue au filtrage).
const FIELD_DOWNSAMPLE := 4
## PF1 : shader de particules maison (voir l'en-tête du shader : fuite à la fermeture avec
## `ParticleProcessMaterial`).
const PRECIPITATION_SHADER := preload("res://shaders/campaign_precipitation.gdshader")
const KINDS := ["clear", "fog", "rain", "snow", "storm"]

## Altitude du plan de nuées (unités monde ; le relief culmine vers 29).
@export var cloud_height: float = 36.0
## Distances caméra : nuées visibles au-delà de `cloud_near` (fondu), pluie en particules en deçà
## de `particles_far`.
## Q5 : à (90, 200) et 0,8, les nuées couvraient l'Île-de-France dès la vue de départ ; elles
## n'apparaissent plus qu'en vue stratégique lointaine, et laissent voir la carte dessous.
@export var cloud_near: Vector2 = Vector2(260.0, 520.0)
## CV3-0 (#3) : 0.6 -> 0.45, les nuées de près restaient déjà rares (`cloud_near`), c'est la
## densité en vue large (au-delà de `cloud_wide.y`, cf. `weather_wide_intensity_cut`) qui rendait
## presque chaque province couverte.
@export var cloud_max_alpha: float = 0.45
## TB2 : nuées plus fines en vue moyenne. Opacité plafonnée à `cloud_medium_alpha` jusqu'à
## `cloud_medium.x` (distance caméra), puis montée vers `cloud_max_alpha` à `cloud_medium.y`.
## Valeurs du bloc `clouds` de `data/ui/campaign_map.json` (repli : ces exports).
@export var cloud_medium_alpha: float = 0.2
@export var cloud_medium: Vector2 = Vector2(600.0, 1000.0)
## TB2 : ombres de nuages du terrain (`cloud_shadow_amount`, lot CV1) : seulement quand il y a de
## vraies nuées (pluie, neige, orage) sous le point visé ; nulles par temps clair.
@export var weather_shadow_amount: float = 0.12
@export var fair_shadow_amount: float = 0.0
## TB6 (ADR 0156) : assombrissement du sol par le masque météo (uniformes `weather_wet_dim`,
## `weather_cloud_shade*` de `campaign_weather.gdshaderinc`) : sol mouillé, ombre des nuées du
## masque (force, largeur du seuil, fréquence du bruit en part de celle des nuées). Avec
## `weather_shadow_amount`, la baisse de luminance au cœur d'une ombre reste sous 15 %
## (`max_ground_dim`).
@export var wet_dim: float = 0.04
@export var mask_shadow: float = 0.06
@export var mask_shadow_softness: float = 0.5
@export var mask_shadow_scale: float = 0.5
## TB6 (ADR 0156) : brume du matin en nappe basse dans les vallées (météo « brume », canal B du
## masque), bloc `morning_mist` de `data/ui/campaign_map.json` ; uniformes `weather_valley_*`.
## Opacité nulle sans données : pas de nappe.
var valley_mist: Dictionary = {}
## RV-C : bloc `clouds` des données (clés `cumulus_*` : ombres de cumulus, aussi par beau temps).
var cumulus: Dictionary = {}
var _shadow_sun := Vector3.ZERO
var _shadow_material_count := -1
## CV3-0 (#3) : bande (distance caméra) sur laquelle la coupure d'intensité passe de 0 à
## `cloud_wide_cut_max` — au-delà, seules les zones de forte pluie/neige (ou l'orage) gardent
## des nuées.
@export var cloud_wide: Vector2 = Vector2(650.0, 1400.0)
@export var cloud_wide_cut_max: float = 0.5
## CV3-0 (#4) : 320 -> 220, la pluie de près (aiguilles trop marquées) ne s'affichait pas
## assez rarement.
@export var particles_far: float = 220.0
## Durée (s) du lever du brouillard matinal après le début d'un tour.
@export var fog_lift_seconds: float = 40.0

var enabled: bool = true
var forced: String = ""
## {province_id: {kind, intensity, label}} du tour courant.
var weather: Dictionary = {}

var _map: Node
var _map_data: MapData
var _terrain: TerrainBuilder
var _mask: ImageTexture
var _clouds: MeshInstance3D
var _cloud_material: ShaderMaterial
var _rain: GPUParticles3D
var _snow: GPUParticles3D
var _fog_clock: float = 0.0
var _flash_timer: float = 0.0
var _sun: DirectionalLight3D
var _sun_energy: float = 1.0
var _next_flash: float = 2.0
var _particle_scale: float = -1.0
var _focus_kind: String = "clear"
## ME5 : atmosphère (cartes de cumulus, cirrus, rideaux de pluie, brouillard de fleuve, aurore).
var atmosphere: MapAtmosphere = null


func setup(map: Node) -> void:
	_map = map
	_map_data = map.get("map_data")
	_terrain = map.get("terrain")
	if CmdArgs.has("--map-weather"):
		forced = CmdArgs.value("--map-weather")
	if not enabled or _map_data == null:
		return
	_load_tuning()  # TB2
	_build_clouds()
	_rain = _make_particles(false)
	_snow = _make_particles(true)
	_sun = map.get_node_or_null("Sun") as DirectionalLight3D
	atmosphere = MapAtmosphere.new()  # ME5
	atmosphere.name = "Atmosphere"
	add_child(atmosphere)
	atmosphere.setup(map, self)


## Relit la météo du tour (une fois par tour, au chargement).
func refresh(sim: Object) -> void:
	if not enabled or _map_data == null:
		return
	var previous := weather.hash()
	weather = {}
	if forced != "":
		for index in range(1, _map_data.province_count + 1):
			var id := str(_map_data.get_province(index).get("id", ""))
			weather[id] = {"kind": forced, "intensity": 0.8, "label": forced}
	elif sim != null and sim.has_method("get_campaign_weather"):
		weather = sim.call("get_campaign_weather")
	if weather.hash() != previous:
		_fog_clock = 0.0  # nouveau tour : le brouillard du matin se reforme
	_upload_mask()
	if atmosphere != null:
		atmosphere.refresh(weather)  # ME5


## ME5 : brume du matin (1 au début du tour, 0 une fois levée), même courbe que le sol.
func morning_mist() -> float:
	return 1.0 - smoothstep(fog_lift_seconds * 0.15, fog_lift_seconds, _fog_clock)


## Météo (`kind`) au point de carte `px` ; `clear` hors des provinces ou sans campagne.
func weather_at(px: Vector2) -> String:
	if _map_data == null or weather.is_empty():
		return "clear"
	var index := _map_data.province_index_at(px.x, px.y)
	if index <= 0:
		return "clear"
	var id := str(_map_data.get_province(index).get("id", ""))
	return str(weather.get(id, {}).get("kind", "clear"))


func update_view(focus: Vector3, distance: float, parchment: float) -> void:
	if not enabled or _map_data == null:
		return
	var delta := get_process_delta_time()
	_fog_clock += delta
	if _terrain != null and _terrain.material != null:
		var morning := 1.0 - smoothstep(fog_lift_seconds * 0.15, fog_lift_seconds, _fog_clock)
		_terrain.material.set_shader_parameter("weather_fog_morning", morning)
	var cloud_alpha := cloud_alpha_at(distance) * (1.0 - parchment)
	_cloud_material.set_shader_parameter("cloud_alpha", cloud_alpha)
	_update_selection_clear()
	_clouds.visible = cloud_alpha > 0.01
	var wide_cut := smoothstep(cloud_wide.x, cloud_wide.y, distance) * cloud_wide_cut_max
	_cloud_material.set_shader_parameter("weather_wide_intensity_cut", wide_cut)
	if _terrain != null and _terrain.material != null:
		_terrain.material.set_shader_parameter("weather_wide_intensity_cut", wide_cut)
	_focus_kind = weather_at(Vector2(focus.x, focus.z))
	_update_cloud_shadows(delta)
	_update_cloud_shadow_sun()  # RV-C
	_update_cumulus_medium(distance)  # A6-C9
	_update_particles(focus, distance)
	_update_lightning(delta, distance)
	if atmosphere != null:
		atmosphere.update_view(focus, distance, parchment)  # ME5


# --- TB2 : nuées sobres -----------------------------------------------------------------


## Réglages du bloc `clouds` de `data/ui/campaign_map.json`.
func _load_tuning() -> void:
	var clouds := MapReadability.section("clouds")
	cloud_medium_alpha = float(clouds.get("medium_alpha", cloud_medium_alpha))
	var band: Variant = clouds.get("medium_distance", null)
	if band is Array and (band as Array).size() == 2:
		cloud_medium = Vector2(float(band[0]), float(band[1]))
	cloud_max_alpha = float(clouds.get("max_alpha", cloud_max_alpha))
	weather_shadow_amount = float(clouds.get("weather_shadow", weather_shadow_amount))
	fair_shadow_amount = float(clouds.get("fair_shadow", fair_shadow_amount))
	wet_dim = float(clouds.get("wet_dim", wet_dim))
	mask_shadow = float(clouds.get("mask_shadow", mask_shadow))
	mask_shadow_softness = float(clouds.get("mask_shadow_softness", mask_shadow_softness))
	mask_shadow_scale = float(clouds.get("mask_shadow_scale", mask_shadow_scale))
	valley_mist = MapReadability.section("morning_mist")
	cumulus = clouds


## RV-C : réglages des ombres de cumulus (clés `cumulus_*` du bloc `clouds`) vers les uniformes
## `cloud_shadow_*` de campaign_cloud_shadow.gdshaderinc ; clé absente : valeur du shader.
func _apply_cumulus_tuning(material: ShaderMaterial) -> void:
	for pair: Array in [["cumulus_shadow", "cloud_shadow_cumulus"], ["cumulus_scale", "cloud_shadow_scale"],
			["cumulus_cover_wet", "cloud_shadow_cover_wet"], ["cumulus_edge", "cloud_shadow_edge"],
			["cumulus_drift", "cloud_shadow_drift"]]:
		if cumulus.has(pair[0]):
			material.set_shader_parameter(pair[1], float(cumulus[pair[0]]))
	for pair: Array in [["cumulus_cover", "cloud_shadow_cover"], ["cumulus_fade_px", "cloud_shadow_fade_px"]]:
		var values: Variant = cumulus.get(pair[0], null)
		if values is Array and (values as Array).size() == 2:
			material.set_shader_parameter(pair[1], Vector2(float(values[0]), float(values[1])))
	var tint: Variant = cumulus.get("cumulus_tint", null)
	if tint is Array and (tint as Array).size() == 3:
		material.set_shader_parameter("cloud_shadow_tint", Vector3(float(tint[0]), float(tint[1]), float(tint[2])))
	material.set_shader_parameter("cloud_shadow_height", cloud_height - 2.0)  # sol moyen ≈ 2 (cf. `cloud_clear_ground_y`)


## RV-C : matériaux qui portent les ombres de cumulus (campaign_cloud_shadow.gdshaderinc) : sol,
## mer, imposteurs d'arbres (créés après la météo : relus à chaque image).
func _cloud_shadow_materials() -> Array[ShaderMaterial]:
	var out: Array[ShaderMaterial] = []
	var sea := _map.get("sea") as MeshInstance3D if _map != null else null
	var vegetation := _map.get_node_or_null("Vegetation") if _map != null else null
	for material: Variant in [_terrain.material if _terrain != null else null,
			atmosphere.cumulus_material() if atmosphere != null else null,  # ME5
			sea.material_override if sea != null else null,
			vegetation.get("_impostor_material") if vegetation != null else null]:
		if material is ShaderMaterial:
			out.append(material)
	return out


var _cumulus_scale_set := -1.0


## A6-C9 : en vue moyenne et lointaine, les ombres de cumulus (taches de ~30 px carte) donnaient un
## sol « camouflage » : leur force est ramenée à `cumulus_medium_scale` entre les deux distances de
## `cumulus_medium_distance` (bloc `clouds` des données).
func _update_cumulus_medium(distance: float) -> void:
	var band: Variant = cumulus.get("cumulus_medium_distance", null)
	if not (band is Array and (band as Array).size() >= 2):
		return
	var factor := lerpf(1.0, float(cumulus.get("cumulus_medium_scale", 1.0)), smoothstep(float(band[0]), float(band[1]), distance))
	if absf(factor - _cumulus_scale_set) < 0.01:
		return
	_cumulus_scale_set = factor
	var strength := float(cumulus.get("cumulus_shadow", 0.4)) * factor
	for material in _cloud_shadow_materials():
		material.set_shader_parameter("cloud_shadow_cumulus", strength)


## RV-C : direction vers le soleil pour décaler les ombres de nuages (le soleil change de saison
## en saison et au fil du tour, `turn_light.gd`) ; réglages reposés sur tout nouveau matériau.
func _update_cloud_shadow_sun() -> void:
	var materials := _cloud_shadow_materials()
	var to_sun := _sun.global_transform.basis.z.normalized() if _sun != null else _shadow_sun
	var fresh := materials.size() != _shadow_material_count
	if fresh:
		_shadow_material_count = materials.size()
		for material in materials:
			if _terrain == null or material != _terrain.material:  # le sol reçoit les siens avec le masque météo
				_apply_cumulus_tuning(material)
	if (fresh or not to_sun.is_equal_approx(_shadow_sun)) and to_sun != Vector3.ZERO:
		_shadow_sun = to_sun
		for material in materials:
			material.set_shader_parameter("cloud_shadow_sun", to_sun)


## TB6 : plus forte baisse de luminance (0..1) que la météo peut poser sur le sol : sol mouillé,
## ombre des nuées du masque et ombres de nuages du terrain cumulés, au cœur d'un orage.
func max_ground_dim() -> float:
	return 1.0 - (1.0 - wet_dim) * (1.0 - mask_shadow) * (1.0 - maxf(weather_shadow_amount, fair_shadow_amount))


## TB6 : pose les réglages du sol sur le matériau du terrain.
func _apply_ground_tuning() -> void:
	if _terrain == null or _terrain.material == null:
		return
	var material: ShaderMaterial = _terrain.material
	_apply_cumulus_tuning(material)  # RV-C
	material.set_shader_parameter("weather_wet_dim", wet_dim)
	material.set_shader_parameter("weather_cloud_shade", mask_shadow)
	material.set_shader_parameter("weather_cloud_shade_soft", mask_shadow_softness)
	material.set_shader_parameter("weather_cloud_shade_scale", mask_shadow_scale)
	material.set_shader_parameter("weather_valley_mist", valley_mist_opacity())
	if valley_mist.is_empty():
		return
	for pair: Array in [["valley_depth_m", "weather_valley_depth_m"], ["plain_altitude_m", "weather_valley_plain_m"],
			["crest_height_m", "weather_valley_crest_m"], ["height_lods", "weather_valley_lods"]]:
		var values: Variant = valley_mist.get(pair[0], null)
		if values is Array and (values as Array).size() == 2:
			material.set_shader_parameter(pair[1], Vector2(float(values[0]), float(values[1])))
	for pair: Array in [["plain_share", "weather_valley_plain"], ["bank_scale", "weather_valley_bank_scale"],
			["drift", "weather_valley_drift"], ["grazing_gain", "weather_valley_grazing"],
			["lifted_share", "weather_valley_lift"], ["near_share", "weather_valley_near"]]:
		if valley_mist.has(pair[0]):
			material.set_shader_parameter(pair[1], float(valley_mist[pair[0]]))
	var tint := Color(str(valley_mist.get("color", "#d9d6cc")))
	material.set_shader_parameter("weather_valley_color", Vector3(tint.r, tint.g, tint.b))


## TB6 : opacité maximale de la nappe de vallée (0 : pas de nappe).
func valley_mist_opacity() -> float:
	return clampf(float(valley_mist.get("opacity", 0.0)), 0.0, 1.0)


## Opacité des nuées à la distance caméra `distance` (hors fondu du parchemin) : nulles de près,
## fines en vue moyenne, pleines en vue large.
func cloud_alpha_at(distance: float) -> float:
	var ceiling := lerpf(cloud_medium_alpha, cloud_max_alpha, smoothstep(cloud_medium.x, cloud_medium.y, distance))
	return smoothstep(cloud_near.x, cloud_near.y, distance) * ceiling


## Vrai si la météo `kind` porte des nuées (pluie, neige, orage) ; le brouillard est une nappe au sol.
static func has_clouds(kind: String) -> bool:
	return kind in ["rain", "snow", "storm"]


## Ombres de nuages attendues sur le terrain pour la météo `kind` sous le point visé.
func shadow_amount_for(kind: String) -> float:
	return weather_shadow_amount if has_clouds(kind) else fair_shadow_amount


var _shadow_amount := -1.0
var _clear_id := -1


func _update_cloud_shadows(delta: float) -> void:
	if _terrain == null or _terrain.material == null:
		return
	var target := shadow_amount_for(_focus_kind)
	var amount := target if _shadow_amount < 0.0 else move_toward(_shadow_amount, target, delta * 0.15)
	if not is_equal_approx(amount, _shadow_amount):
		_shadow_amount = amount
		_terrain.material.set_shader_parameter("cloud_shadow_amount", amount)


## Jamais de nuée ni de brume sur la province sélectionnée (`weather_clear_id` des deux matériaux).
func _update_selection_clear() -> void:
	var selected: Variant = _map.get("selected_index") if _map != null else null
	var id := int(selected) if selected != null else 0
	if id == _clear_id:
		return
	_clear_id = id
	var feather := MapReadability.number("clouds", "clear_feather_px", 8.0)
	for material: ShaderMaterial in [_terrain.material if _terrain != null else null, _cloud_material]:
		if material != null:
			material.set_shader_parameter("weather_clear_id", id)
			material.set_shader_parameter("weather_clear_feather_px", feather)


# --- Masque par province ----------------------------------------------------------------


func _upload_mask() -> void:
	var image := mask_image()
	var any := _mask_any
	if _mask == null:
		_mask = ImageTexture.create_from_image(image)
	else:
		_mask.update(image)
	_apply_ground_tuning()  # TB6
	for material: ShaderMaterial in [_terrain.material if _terrain != null else null, _cloud_material]:
		if material == null:
			continue
		material.set_shader_parameter("weather_mask", _mask)
		material.set_shader_parameter("weather_enabled", any)
	_bake_field(any)


var _mask_any := false
## FL3 (ADR 0192) : banc `--bench-set=prop:weather_view.use_field=false` : frange recalculée à chaque pixel.
var use_field := true:
	set(value):
		use_field = value
		_bake_field(_mask_any)
var _field_viewport: SubViewport = null
var _field_material: ShaderMaterial = null


## Cuit la météo par point de carte (frange comprise) dans une texture basse résolution, une
## fois par tour : le sol et les nuées la lisent au lieu de recalculer la frange à chaque pixel.
func _bake_field(any: bool) -> void:
	var province_ids: Variant = _terrain.material.get_shader_parameter("province_ids") if _terrain != null and _terrain.material != null else null
	var on := use_field and any and province_ids != null
	if on and _field_viewport == null:
		_field_material = ShaderMaterial.new()
		_field_material.shader = FIELD_BAKE_SHADER
		_field_material.set_shader_parameter("map_size", Vector2(_map_data.size))
		_field_material.set_shader_parameter("weather_enabled", true)
		_field_viewport = SubViewport.new()
		_field_viewport.name = "WeatherField"
		_field_viewport.size = Vector2i(_map_data.size) / FIELD_DOWNSAMPLE
		_field_viewport.transparent_bg = true
		_field_viewport.disable_3d = true
		_field_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
		var quad := ColorRect.new()
		quad.size = Vector2(_field_viewport.size)
		quad.material = _field_material
		_field_viewport.add_child(quad)
		add_child(_field_viewport)
	if on:
		_field_material.set_shader_parameter("province_ids", province_ids)
		_field_material.set_shader_parameter("weather_mask", _mask)
		_field_viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	for material: ShaderMaterial in [_terrain.material if _terrain != null else null, _cloud_material]:
		if material != null:
			material.set_shader_parameter("weather_field", _field_viewport.get_texture() if on else null)
			material.set_shader_parameter("weather_field_on", on)


## Masque météo du tour (un texel par province : R pluie, G neige, B brouillard, A orage).
func mask_image() -> Image:
	var width := maxi(_map_data.province_count + 1, 1)
	var image := Image.create(width, 1, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	var any := false
	for index in range(1, _map_data.province_count + 1):
		var id := str(_map_data.get_province(index).get("id", ""))
		var entry: Dictionary = weather.get(id, {})
		var kind := str(entry.get("kind", "clear"))
		var k := 0.45 + 0.55 * clampf(float(entry.get("intensity", 0.5)), 0.0, 1.0)
		var c := Color(0, 0, 0, 0)
		match kind:
			"rain":
				c.r = k
			"storm":
				c.r = 1.0
				c.a = k
			"snow":
				c.g = k
			"fog":
				c.b = k
		if kind != "clear":
			any = true
		image.set_pixel(index, 0, c)
	_mask_any = any
	return image


# --- Nuées ---------------------------------------------------------------------------------


func _build_clouds() -> void:
	var plane := PlaneMesh.new()
	plane.size = Vector2(_map_data.size)
	_clouds = MeshInstance3D.new()
	_clouds.name = "Clouds"
	_clouds.mesh = plane
	_clouds.position = Vector3(_map_data.size.x * 0.5, cloud_height, _map_data.size.y * 0.5)
	_clouds.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_cloud_material = ShaderMaterial.new()
	_cloud_material.shader = CLOUD_SHADER
	_cloud_material.set_shader_parameter("map_size", Vector2(_map_data.size))
	if _terrain != null and _terrain.material != null:
		_cloud_material.set_shader_parameter("province_ids", _terrain.material.get_shader_parameter("province_ids"))
	_clouds.material_override = _cloud_material
	_clouds.visible = false
	add_child(_clouds)


# --- Pluie et neige de près -------------------------------------------------------------------


func _make_particles(snow: bool) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = "Snow" if snow else "Rain"
	particles.amount = 2600 if snow else 5000
	particles.lifetime = 2.2 if snow else 0.7
	particles.emitting = false
	particles.visible = false
	particles.local_coords = false
	particles.visibility_aabb = AABB(Vector3(-400, -200, -400), Vector3(800, 400, 800))
	var process := ShaderMaterial.new()
	process.shader = PRECIPITATION_SHADER
	process.set_shader_parameter("emission_box_extents", Vector3(60, 10, 60))
	process.set_shader_parameter("direction", Vector3(0.12, -1.0, 0.05))
	process.set_shader_parameter("spread_deg", 4.0 if not snow else 18.0)
	process.set_shader_parameter("velocity_min", 60.0 if not snow else 8.0)
	process.set_shader_parameter("velocity_max", 75.0 if not snow else 12.0)
	process.set_shader_parameter("sway", 3.0 if snow else 0.0)
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2(0.18, 0.18) if snow else Vector2(0.035, 1.1)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED if snow else BaseMaterial3D.BILLBOARD_FIXED_Y
	material.billboard_keep_scale = true
	# CV3-0 (#4) : pluie moins blanche et plus discrète (était 0.72, 0.76, 0.82, 0.38 -> lisait
	# comme de longues aiguilles blanches).
	material.albedo_color = Color(0.95, 0.96, 1.0, 0.85) if snow else Color(0.62, 0.67, 0.76, 0.24)
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	quad.material = material
	particles.draw_pass_1 = quad
	add_child(particles)
	return particles


## SZ5 (défaut ZG7c S7) : taille des gouttes, boîte d'émission, vitesse et densité en fonction
## continue de la distance caméra (`PrecipitationProfile`, `res://resources/precipitation.tres`),
## ancrées sur une échelle réaliste (mètres) au palier site et raccordées sans saut à l'ancien
## comportement (déjà validé) en vue vallée / lointaine. Voir l'en-tête de la ressource.
func _update_particles(focus: Vector3, distance: float) -> void:
	var near := distance < particles_far
	var rain := near and (_focus_kind == "rain" or _focus_kind == "storm")
	var snow := near and _focus_kind == "snow"
	for pair in [[_rain, rain], [_snow, snow]]:
		var particles: GPUParticles3D = pair[0]
		var on: bool = pair[1]
		if particles.emitting != on:
			particles.emitting = on
		particles.visible = on or particles.emitting
	if not (rain or snow):
		return
	var profile := PrecipitationProfile.load_default()
	if _map_data != null:
		profile.meters_per_unit = _map_data.meters_per_px
	var height := profile.height(distance)
	for particles: GPUParticles3D in [_rain, _snow]:
		particles.global_position = focus + Vector3(0.0, height, 0.0)
	if absf(distance - _particle_scale) / maxf(_particle_scale, 0.01) > 0.08:
		_particle_scale = distance
		for particles: GPUParticles3D in [_rain, _snow]:
			var is_snow := particles == _snow
			var process := particles.process_material as ShaderMaterial
			var extents := profile.box_extents(distance)
			process.set_shader_parameter("emission_box_extents", Vector3(extents.x, extents.y, extents.x))
			var v := profile.speed(distance, is_snow)
			process.set_shader_parameter("velocity_min", v.x)
			process.set_shader_parameter("velocity_max", v.y)
			process.set_shader_parameter("sway", profile.sway(distance) if is_snow else 0.0)
			particles.amount_ratio = profile.amount_ratio(distance) * (1.0 if is_snow else clampf(profile.rain_amount_scale, 0.0, 1.0))
			var quad := particles.draw_pass_1 as QuadMesh
			if is_snow:
				var side := profile.snow_size(distance)
				quad.size = Vector2(side, side)
			else:
				quad.size = profile.rain_size(distance)


func _update_lightning(delta: float, distance: float) -> void:
	if _sun == null:
		return
	if _flash_timer > 0.0:
		_flash_timer -= delta
		if _flash_timer <= 0.0:
			_sun.light_energy = _sun_energy
		else:
			_sun.light_energy = _sun_energy * (2.6 if fmod(_flash_timer, 0.08) > 0.04 else 1.4)
		return
	if _focus_kind != "storm" or distance > 700.0:
		return
	_next_flash -= delta
	if _next_flash <= 0.0:
		_next_flash = randf_range(3.0, 9.0)
		_sun_energy = _sun.light_energy
		_flash_timer = 0.18
