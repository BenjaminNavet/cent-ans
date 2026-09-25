extends Node3D

## Rendu des incendies de siège (lot S2, `docs/design/s2-incendies.md`).
##
## Purement visuel : l'état du feu vient du cœur Rust (`BattleSim.get_siege()` : `houses[i].fire =
## {state, intensity}`, `gate_fire`, `wind`). Par maison en feu : flammes et fumée
## (`GPUParticles3D`, fumée poussée par le vent) ; les `max_lights` foyers les plus intenses portent
## une `OmniLight3D` qui vacille ; une maison brûlée s'effondre (instances abaissées dans les
## `MultiMesh` des maisons de `BattleSiege`) et laisse un tas noirci de poutres calcinées.
## Paramètres : `data/fx/siege_fire.json` (schéma `data/schemas/fx_siege_fire.schema.json`).
## Lot V3 (A1-13) : flammes et fumée en planches animées procédurales (`fire_flame.gdshader`,
## `fire_smoke.gdshader`, planches de `tools/cent_ans_tools/fire_flipbooks.py`), braises
## (`fire_ember.gdshader`) qui dérivent au vent, lumière qui vacille (bruit, couleur, position).

const FX_PATH := "fx/siege_fire.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const FLAME_SHADER := preload("res://shaders/fire_flame.gdshader")
const SMOKE_SHADER := preload("res://shaders/fire_smoke.gdshader")
const EMBER_SHADER := preload("res://shaders/fire_ember.gdshader")
const FLAME_FLIPBOOK := "res://assets/textures/fx/flame_flipbook.png"
const SMOKE_FLIPBOOK := "res://assets/textures/fx/smoke_flipbook.png"
const DEFAULTS := {
	"update_period_s": 0.2,
	"flame_height_m": 7.5,
	"max_lights": 8,
	"light": {"color": [1.0, 0.52, 0.18], "energy": 5.0, "range_m": 30.0, "height_m": 7.0, "flicker_speed": 9.0, "flicker_amount": 0.35},
	"flames": {"amount": 26, "lifetime_s": 1.5, "size_m": [3.5, 7.0], "velocity_m_s": [1.2, 2.8], "spread_m": 4.5, "color_start": [1.0, 0.72, 0.28, 1.0], "color_end": [0.35, 0.03, 0.0, 1.0], "anim_loops": 1.5, "emission": 3.2},
	"smoke": {"amount": 48, "lifetime_s": 10.0, "size_m": [5.0, 14.0], "velocity_m_s": [2.0, 4.0], "spread_m": 4.0, "color": [0.13, 0.12, 0.11, 0.7], "wind_drift_m_s": 3.5},
	"embers": {"amount": 36, "lifetime_s": 3.0, "size_m": [0.12, 0.3], "velocity_m_s": [3.0, 7.0], "energy": 6.0},
	"ruin": {"char_color": [0.07, 0.06, 0.05], "rubble_height_m": 1.3, "collapse_scale": 0.25, "beams": 5, "embers": 10},
}

var params: Dictionary = {}
var height_at: Callable
var houses_root: Node3D = null  # maisons regroupées de `BattleSiege` (MultiMesh)
var _fires: Dictionary = {}  # clé (index de maison, ou "gate") -> {node, flames, smoke, intensity, p}
var _ruins: Dictionary = {}  # index de maison -> Node3D
var _lights: Array[OmniLight3D] = []
var _light_keys: Array = []
var _last_update_ms: int = -1000000
var _time: float = 0.0
var _wind := Vector2.ZERO
var _flame_mat: Material
var _smoke_mat: Material
var _ember_mat: ShaderMaterial
var _base_light_color := Color(1.0, 0.52, 0.18)
var _char_mat: StandardMaterial3D
## Compteurs lus par le smoke test.
var burning_count: int = 0
var ruin_count: int = 0


## `siege_view` : le `BattleSiege` déjà construit (maisons en place).
func setup(siege_view: Node3D, p_height_at: Callable) -> void:
	name = "SiegeFire"
	height_at = p_height_at
	params = _load_params()
	houses_root = siege_view.get_node_or_null("Houses")
	_flame_mat = _flame_material(params["flames"])
	_smoke_mat = _smoke_material(_color(params["smoke"]["color"]))
	_ember_mat = ShaderMaterial.new()
	_ember_mat.shader = EMBER_SHADER
	_ember_mat.set_shader_parameter("energy", float(params["embers"]["energy"]))
	_char_mat = StandardMaterial3D.new()
	_char_mat.albedo_color = _color(params["ruin"]["char_color"])
	_char_mat.roughness = 1.0
	var light_params: Dictionary = params["light"]
	_base_light_color = _color(light_params["color"])
	for i in int(params["max_lights"]):
		var light := OmniLight3D.new()
		light.light_color = _color(light_params["color"])
		light.omni_range = float(light_params["range_m"])
		light.shadow_enabled = false
		light.visible = false
		add_child(light)
		_lights.append(light)
		_light_keys.append(null)


static func _load_params() -> Dictionary:
	var merged: Dictionary = DEFAULTS.duplicate(true)
	var path: String = _data_dir().path_join(FX_PATH)
	if not FileAccess.file_exists(path):
		return merged
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not parsed is Dictionary:
		return merged
	for key in parsed:
		if merged.has(key) and merged[key] is Dictionary and parsed[key] is Dictionary:
			(merged[key] as Dictionary).merge(parsed[key], true)
		else:
			merged[key] = parsed[key]
	return merged


## Dossier `data/` (autoload `MapPaths` s'il existe : le smoke test tourne sans autoloads nommés).
static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func _color(values: Array) -> Color:
	var alpha := float(values[3]) if values.size() > 3 else 1.0
	return Color(float(values[0]), float(values[1]), float(values[2]), alpha)


func _ground(x: float, z: float) -> float:
	return float(height_at.call(x, z)) if height_at.is_valid() else 0.0


## Lit l'état du feu (au plus toutes les `update_period_s` secondes).
func update(siege: Dictionary) -> void:
	var now := Time.get_ticks_msec()
	if now - _last_update_ms < int(float(params["update_period_s"]) * 1000.0):
		return
	_last_update_ms = now
	_wind = siege.get("wind", Vector2.ZERO)
	var seen := {}
	var houses: Array = siege.get("houses", [])
	for i in houses.size():
		var house: Dictionary = houses[i]
		var fire: Dictionary = house.get("fire", {})
		var state := str(fire.get("state", "intact"))
		var p := Vector2(float(house["x"]), float(house["z"]))
		if state == "burning":
			seen[i] = true
			_show_fire(i, p, float(house["radius"]), float(fire.get("intensity", 0.0)))
		elif state == "burnt" and not _ruins.has(i):
			_make_ruin(i, p, float(house["radius"]))
	var gate_fire: Dictionary = siege.get("gate_fire", {})
	if str(gate_fire.get("state", "intact")) == "burning":
		var pieces: Array = siege.get("pieces", [])
		var gate_index := int(siege.get("gate", -1))
		if gate_index >= 0 and gate_index < pieces.size():
			var gate: Dictionary = pieces[gate_index]
			var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
			seen["gate"] = true
			_show_fire("gate", mid, 6.0, float(gate_fire.get("intensity", 0.0)))
	for key in _fires.keys():
		if not seen.has(key):
			(_fires[key]["node"] as Node).queue_free()
			_fires.erase(key)
	burning_count = _fires.size()
	ruin_count = _ruins.size()
	_assign_lights()


func _show_fire(key: Variant, p: Vector2, radius: float, intensity: float) -> void:
	if not _fires.has(key):
		_fires[key] = _make_fire(p, radius)
	var fire: Dictionary = _fires[key]
	fire["intensity"] = intensity
	var ratio := clampf(0.25 + 0.75 * intensity, 0.0, 1.0)
	(fire["flames"] as GPUParticles3D).amount_ratio = ratio
	(fire["smoke"] as GPUParticles3D).amount_ratio = ratio
	var drift := float(params["smoke"]["wind_drift_m_s"])
	var smoke_process := (fire["smoke"] as GPUParticles3D).process_material as ParticleProcessMaterial
	smoke_process.gravity = Vector3(_wind.x * drift, 0.6, _wind.y * drift)
	var embers := fire.get("embers", null) as GPUParticles3D
	if embers != null:
		embers.amount_ratio = ratio
		(embers.process_material as ParticleProcessMaterial).gravity = Vector3(_wind.x * drift * 0.8, 0.4, _wind.y * drift * 0.8)


func _make_fire(p: Vector2, radius: float) -> Dictionary:
	var root := Node3D.new()
	root.name = "Fire"
	root.position = Vector3(p.x, _ground(p.x, p.y), p.y)
	add_child(root)
	var flame_params: Dictionary = params["flames"]
	var smoke_params: Dictionary = params["smoke"]
	# Flammes : planche animée (boucles pendant la vie), base du panneau au foyer.
	var flames := _emitter(flame_params, _flame_mat, radius, Color.WHITE, Color.WHITE, 0.45)
	var flame_process := flames.process_material as ParticleProcessMaterial
	var loops := float(flame_params.get("anim_loops", 1.5))
	flame_process.anim_speed_min = loops * 0.8
	flame_process.anim_speed_max = loops * 1.2
	flame_process.anim_offset_max = 1.0
	flames.position.y = float(params["flame_height_m"]) * 0.45
	root.add_child(flames)
	# Fumée : bouffées qui tournent, grossissent en montant et dérivent au vent.
	var smoke_color := _color(smoke_params["color"])
	var smoke := _emitter(smoke_params, _smoke_mat, radius, smoke_color, Color(smoke_color, smoke_color.a * 0.6), 0.0)
	var smoke_process := smoke.process_material as ParticleProcessMaterial
	smoke_process.angle_min = -180.0
	smoke_process.angle_max = 180.0
	smoke_process.angular_velocity_min = -12.0
	smoke_process.angular_velocity_max = 12.0
	var growth := Curve.new()
	growth.add_point(Vector2(0.0, 0.5))
	growth.add_point(Vector2(0.35, 0.8))
	growth.add_point(Vector2(1.0, 1.25))
	var growth_texture := CurveTexture.new()
	growth_texture.curve = growth
	smoke_process.scale_curve = growth_texture
	smoke_process.damping_min = 0.15
	smoke_process.damping_max = 0.3
	smoke.position.y = float(params["flame_height_m"]) + 1.0
	smoke.sorting_offset = -1.0  # derrière les flammes
	root.add_child(smoke)
	var embers := _make_embers(radius)
	embers.position.y = float(params["flame_height_m"]) * 0.6
	root.add_child(embers)
	return {"node": root, "flames": flames, "smoke": smoke, "embers": embers, "intensity": 0.0, "p": p}


## Braises : étincelles qui montent en tourbillonnant et que le vent emporte.
func _make_embers(radius: float) -> GPUParticles3D:
	var spec: Dictionary = params["embers"]
	var embers := _emitter({"amount": maxi(int(spec["amount"]), 1), "lifetime_s": spec["lifetime_s"], "spread_m": radius * 0.7,
		"velocity_m_s": spec["velocity_m_s"], "size_m": spec["size_m"]}, _ember_mat, radius, Color.WHITE, Color.WHITE, 0.0)
	var process := embers.process_material as ParticleProcessMaterial
	process.spread = 35.0
	process.turbulence_enabled = true
	process.turbulence_noise_strength = 2.5
	process.turbulence_noise_scale = 3.0
	process.turbulence_influence_min = 0.1
	process.turbulence_influence_max = 0.3
	process.damping_min = 0.5
	process.damping_max = 1.2
	embers.visibility_aabb = AABB(Vector3(-40, -5, -40), Vector3(80, 50, 80))
	return embers


func _emitter(spec: Dictionary, material: Material, radius: float, start: Color, end: Color, quad_lift: float = 0.0) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.amount = int(spec["amount"])
	particles.lifetime = float(spec["lifetime_s"])
	particles.preprocess = float(spec["lifetime_s"]) * 0.5
	particles.visibility_aabb = AABB(Vector3(-40, -5, -40), Vector3(80, 60, 80))
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	process.emission_sphere_radius = minf(float(spec["spread_m"]), radius)
	process.direction = Vector3.UP
	process.spread = 15.0
	var velocity: Array = spec["velocity_m_s"]
	process.initial_velocity_min = float(velocity[0])
	process.initial_velocity_max = float(velocity[1])
	process.gravity = Vector3(0, 0.5, 0)
	var size: Array = spec["size_m"]
	process.scale_min = float(size[0])
	process.scale_max = float(size[1])
	var ramp := Gradient.new()
	ramp.set_color(0, start)
	ramp.set_color(1, end)
	var ramp_texture := GradientTexture1D.new()
	ramp_texture.gradient = ramp
	process.color_ramp = ramp_texture
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	quad.center_offset = Vector3(0.0, quad_lift, 0.0)
	quad.material = material
	particles.draw_pass_1 = quad
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return particles


## Flammes : planche animée si elle est importée, sinon disque flou (matériau S2).
func _flame_material(spec: Dictionary) -> Material:
	if not ResourceLoader.exists(FLAME_FLIPBOOK):
		return _particle_material(false)
	var material := ShaderMaterial.new()
	material.shader = FLAME_SHADER
	material.set_shader_parameter("flipbook", load(FLAME_FLIPBOOK))
	material.set_shader_parameter("color_hot", _color(spec["color_start"]))
	material.set_shader_parameter("color_cold", _color(spec["color_end"]))
	material.set_shader_parameter("emission", float(spec.get("emission", 3.2)))
	return material


func _smoke_material(color: Color) -> Material:
	if not ResourceLoader.exists(SMOKE_FLIPBOOK):
		return _particle_material(false)
	var material := ShaderMaterial.new()
	material.shader = SMOKE_SHADER
	material.set_shader_parameter("flipbook", load(SMOKE_FLIPBOOK))
	material.set_shader_parameter("smoke_color", Color(color, 1.0))
	return material


static func _particle_material(additive: bool) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	material.billboard_keep_scale = true  # sinon l'échelle des particules (`size_m`) est perdue
	material.vertex_color_use_as_albedo = true
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	if additive:
		material.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	# Disque flou procédural (pas de texture à importer).
	var gradient := Gradient.new()
	gradient.set_color(0, Color(1, 1, 1, 1))
	gradient.set_color(1, Color(1, 1, 1, 0))
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.fill = GradientTexture2D.FILL_RADIAL
	texture.fill_from = Vector2(0.5, 0.5)
	texture.fill_to = Vector2(0.5, 0.0)
	texture.width = 64
	texture.height = 64
	material.albedo_texture = texture
	return material


## Maison brûlée : instances de la maison abaissées dans les MultiMesh, tas noirci et poutres.
func _make_ruin(index: int, p: Vector2, radius: float) -> void:
	var ruin_params: Dictionary = params["ruin"]
	var ground := _ground(p.x, p.y)
	_collapse_instances(p, radius, ground, float(ruin_params["collapse_scale"]))
	var root := Node3D.new()
	root.name = "Ruin%d" % index
	root.position = Vector3(p.x, ground, p.y)
	add_child(root)
	var rubble := MeshInstance3D.new()
	var mound := BoxMesh.new()
	var rubble_height := float(ruin_params["rubble_height_m"])
	mound.size = Vector3(radius * 1.5, rubble_height, radius * 1.2)
	rubble.mesh = mound
	rubble.material_override = _char_mat
	rubble.position.y = rubble_height * 0.5
	rubble.rotation.y = float(index) * 1.7
	root.add_child(rubble)
	var rng := RandomNumberGenerator.new()
	rng.seed = 2002 + index
	for b in int(ruin_params["beams"]):
		var beam := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(0.35, 0.35, rng.randf_range(3.0, radius * 1.4))
		beam.mesh = box
		beam.material_override = _char_mat
		beam.position = Vector3(rng.randf_range(-radius, radius) * 0.5, rubble_height + 0.2, rng.randf_range(-radius, radius) * 0.5)
		beam.rotation = Vector3(rng.randf_range(-0.6, 0.6), rng.randf() * TAU, rng.randf_range(-0.3, 0.3))
		root.add_child(beam)
	var embers_count := int(ruin_params["embers"])
	if embers_count > 0:
		var embers := _emitter({"amount": embers_count, "lifetime_s": 2.5, "spread_m": radius * 0.6, "velocity_m_s": [0.5, 1.5], "size_m": [0.15, 0.35]}, _ember_mat, radius, Color(1.0, 0.45, 0.1, 1.0), Color(0.6, 0.1, 0.0, 0.0))
		embers.position.y = rubble_height
		root.add_child(embers)
	_ruins[index] = root


## Abaisse (effondre) les instances des maisons regroupées dont l'origine tombe dans le disque.
func _collapse_instances(p: Vector2, radius: float, ground: float, keep: float) -> void:
	if houses_root == null or not is_instance_valid(houses_root):
		return
	for child in houses_root.get_children():
		if not child is MultiMeshInstance3D:
			continue
		var mm := (child as MultiMeshInstance3D).multimesh
		for i in mm.instance_count:
			var t := mm.get_instance_transform(i)
			if Vector2(t.origin.x, t.origin.z).distance_to(p) > radius * 1.3:
				continue
			t.basis = Basis.from_scale(Vector3(1.0, keep, 1.0)) * t.basis
			t.origin.y = ground + (t.origin.y - ground) * keep
			mm.set_instance_transform(i, t)


## Les `max_lights` lumières vont aux foyers les plus intenses.
func _assign_lights() -> void:
	var keys := _fires.keys()
	keys.sort_custom(func(a: Variant, b: Variant) -> bool: return float(_fires[a]["intensity"]) > float(_fires[b]["intensity"]))
	var height := float(params["light"]["height_m"])
	for i in _lights.size():
		var light := _lights[i]
		if i < keys.size():
			var fire: Dictionary = _fires[keys[i]]
			var p: Vector2 = fire["p"]
			light.position = Vector3(p.x, _ground(p.x, p.y) + height, p.y)
			light.set_meta("base_position", light.position)
			light.visible = true
			_light_keys[i] = keys[i]
		else:
			light.visible = false
			_light_keys[i] = null


func _process(delta: float) -> void:
	_time += delta
	var light_params: Dictionary = params.get("light", {})
	if light_params.is_empty():
		return
	var speed := float(light_params["flicker_speed"])
	var amount := float(light_params["flicker_amount"])
	var energy := float(light_params["energy"])
	for i in _lights.size():
		var light := _lights[i]
		var key: Variant = _light_keys[i]
		if not light.visible or key == null or not _fires.has(key):
			continue
		# Vacillement : trois fréquences non commensurables (pas de motif répété), la lumière
		# jaunit dans les pics et danse de quelques décimètres (ombres portées qui bougent).
		var phase := float(i) * 1.37
		var t := _time * speed
		var wave := 0.5 * sin(t + phase) + 0.3 * sin(t * 2.31 + phase * 2.0) + 0.2 * sin(t * 5.17 + phase * 3.3)
		var flicker := 1.0 + amount * wave
		light.light_energy = energy * float(_fires[key]["intensity"]) * flicker
		light.light_color = _base_light_color.lerp(Color(1.0, 0.78, 0.42), clampf(wave, 0.0, 1.0) * 0.35)
		var base: Vector3 = light.get_meta("base_position", light.position)
		light.position = base + Vector3(sin(t * 0.73 + phase), sin(t * 1.1 + phase) * 0.5, cos(t * 0.91 + phase)) * 0.35
