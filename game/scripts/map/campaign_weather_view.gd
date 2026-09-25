class_name CampaignWeatherView
extends Node3D

## Lot CM2 : météo de la carte de campagne, rendue à partir du cœur (`get_campaign_weather`,
## ADR 0027 ; aucune météo inventée ici). Une fois par tour : masque par province (R pluie,
## G neige, B brouillard, A orage) posé sur le terrain (sol mouillé, neige fraîche, brouillard
## matinal qui se lève, ombre des nuées, éclairs) et sur un plan de nuées qui défilent avec le
## vent ; de près, pluie ou neige en particules autour du point visé et éclairs d'orage.
## Options (après `--`) : `--map-weather=rain|snow|fog|storm|clear` (impose partout, captures),
## `--no-map-weather` (A/B).

const CLOUD_SHADER := preload("res://shaders/campaign_clouds.gdshader")
const KINDS := ["clear", "fog", "rain", "snow", "storm"]

## Altitude du plan de nuées (unités monde ; le relief culmine vers 29).
@export var cloud_height: float = 36.0
## Distances caméra : nuées visibles au-delà de `cloud_near` (fondu), pluie en particules en deçà
## de `particles_far`.
@export var cloud_near: Vector2 = Vector2(90.0, 200.0)
@export var cloud_max_alpha: float = 0.8
@export var particles_far: float = 320.0
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


func setup(map: Node) -> void:
	_map = map
	_map_data = map.get("map_data")
	_terrain = map.get("terrain")
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-map-weather":
			enabled = false
		elif arg.begins_with("--map-weather="):
			forced = arg.trim_prefix("--map-weather=")
	if not enabled or _map_data == null:
		return
	_build_clouds()
	_rain = _make_particles(false)
	_snow = _make_particles(true)
	_sun = map.get_node_or_null("Sun") as DirectionalLight3D


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
	var cloud_alpha := smoothstep(cloud_near.x, cloud_near.y, distance) * cloud_max_alpha * (1.0 - parchment)
	_cloud_material.set_shader_parameter("cloud_alpha", cloud_alpha)
	_clouds.visible = cloud_alpha > 0.01
	_focus_kind = weather_at(Vector2(focus.x, focus.z))
	_update_particles(focus, distance)
	_update_lightning(delta, distance)


# --- Masque par province ----------------------------------------------------------------


func _upload_mask() -> void:
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
	if _mask == null:
		_mask = ImageTexture.create_from_image(image)
	else:
		_mask.update(image)
	for material: ShaderMaterial in [_terrain.material if _terrain != null else null, _cloud_material]:
		if material == null:
			continue
		material.set_shader_parameter("weather_mask", _mask)
		material.set_shader_parameter("weather_enabled", any)


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
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = Vector3(60, 10, 60)
	process.direction = Vector3(0.12, -1.0, 0.05)
	process.spread = 4.0 if not snow else 18.0
	process.gravity = Vector3.ZERO
	process.initial_velocity_min = 60.0 if not snow else 8.0
	process.initial_velocity_max = 75.0 if not snow else 12.0
	if snow:
		process.turbulence_enabled = true
		process.turbulence_noise_strength = 0.6
		process.turbulence_noise_scale = 4.0
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2(0.18, 0.18) if snow else Vector2(0.035, 1.1)
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.billboard_mode = BaseMaterial3D.BILLBOARD_ENABLED if snow else BaseMaterial3D.BILLBOARD_FIXED_Y
	material.billboard_keep_scale = true
	material.albedo_color = Color(0.95, 0.96, 1.0, 0.85) if snow else Color(0.72, 0.76, 0.82, 0.38)
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	quad.material = material
	particles.draw_pass_1 = quad
	add_child(particles)
	return particles


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
	# Boîte d'émission et vitesse proportionnelles au zoom (même densité apparente).
	var k := distance / 100.0
	var height := distance * 0.35
	for particles: GPUParticles3D in [_rain, _snow]:
		particles.global_position = focus + Vector3(0.0, height, 0.0)
	if absf(k - _particle_scale) / maxf(_particle_scale, 0.01) > 0.08:
		_particle_scale = k
		for particles: GPUParticles3D in [_rain, _snow]:
			var process := particles.process_material as ParticleProcessMaterial
			process.emission_box_extents = Vector3(80.0 * k, 8.0 * k, 80.0 * k)
			var is_snow := particles == _snow
			process.initial_velocity_min = (8.0 if is_snow else 60.0) * k
			process.initial_velocity_max = (12.0 if is_snow else 75.0) * k
			var quad := particles.draw_pass_1 as QuadMesh
			quad.size = (Vector2(0.18, 0.18) if is_snow else Vector2(0.035, 1.1)) * maxf(k, 0.3)


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
