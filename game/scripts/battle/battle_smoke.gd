class_name BattleSmoke
extends Node3D

## EP8 : fumées du champ de bataille, rendu seulement.
## - **Sources durables** : `add_smoke_source(position, intensity, kind)` (API publique, appelée
##   par les feux de camp derrière les lignes et par EP6 pour ses camps) ; `kind` = `campfire`
##   (fumée claire qui monte et dérive au vent, lueur la nuit) ou `column` (colonne sombre d'un
##   incendie : maisons en feu du système S2). Budget : `max_sources` par niveau de qualité
##   (PF1) ; au-delà, la source la plus faible est éteinte (jamais plus d'émetteurs que le budget).
## - **Fumée de bombarde qui s'attarde** : `bombard_smoke(position)` (nuage bas qui dérive
##   quelques secondes après le coup, en plus de la bouffée de B4) : petit tampon circulaire.
## Paramètres : `data/fx/battle_staging.json` (`smoke`). Planche de fumée V3
## (`fire_smoke.gdshader`) si présente, sinon disque flou.

signal sources_changed

const SMOKE_SHADER := preload("res://shaders/fire_smoke.gdshader")
const SMOKE_FLIPBOOK := "res://assets/textures/fx/smoke_flipbook.png"
const LINGER_POOL := 4
const MAX_GLOWS := 4
## CR1 : volutes fondues à moins de ce nombre de mètres de la caméra (gros plans).
const NEAR_FADE_M := 25.0

var cfg: Dictionary = {}
var wind_dir: Vector2 = Vector2(1, 0)
var wind_strength: float = 0.5
var light_level: float = 1.0
var max_sources: int = 8
## id -> {pos, intensity, kind, node}
var sources: Dictionary = {}

var _next_id: int = 1
var _linger: Array[GPUParticles3D] = []
var _linger_next: int = 0
var _glows: Array[OmniLight3D] = []


func setup(p_cfg: Dictionary, wind: Dictionary) -> void:
	cfg = p_cfg
	name = "Smoke"
	wind_dir = (wind.get("dir", Vector2(1, 0)) as Vector2).normalized()
	wind_strength = float(wind.get("strength", 0.5))
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	for i in LINGER_POOL:
		var linger := _emitter("Linger%d" % i, cfg.get("bombard_linger", {}), 1.0, true)
		_linger.append(linger)


## Ajoute une source de fumée durable ; rend son identifiant (-1 si le budget est plein et que
## la nouvelle source est la plus faible).
func add_smoke_source(position: Vector3, intensity: float = 1.0, kind: String = "campfire") -> int:
	intensity = clampf(intensity, 0.05, 2.0)
	if sources.size() >= max_sources:
		var weakest := _weakest()
		if weakest < 0 or float(sources[weakest]["intensity"]) >= intensity:
			return -1
		remove_smoke_source(weakest)
	var params: Dictionary = cfg.get(kind, cfg.get("campfire", {}))
	var node := _emitter("Source%d" % _next_id, params, intensity, false)
	node.position = position
	node.emitting = true
	var id := _next_id
	_next_id += 1
	sources[id] = {"pos": position, "intensity": intensity, "kind": kind, "node": node, "glow": bool(params.get("glow", false))}
	_update_glows()
	sources_changed.emit()
	return id


func remove_smoke_source(id: int) -> void:
	if not sources.has(id):
		return
	var node: Node = sources[id]["node"]
	if is_instance_valid(node):
		node.queue_free()
	sources.erase(id)
	_update_glows()
	sources_changed.emit()


## Change l'intensité d'une source (incendie qui faiblit) ; 0 l'éteint.
func set_smoke_intensity(id: int, intensity: float) -> void:
	if not sources.has(id):
		return
	if intensity <= 0.02:
		remove_smoke_source(id)
		return
	sources[id]["intensity"] = intensity
	(sources[id]["node"] as GPUParticles3D).amount_ratio = clampf(intensity, 0.05, 1.0)


## Nuage bas après un coup de bombarde (bouche du canon).
func bombard_smoke(position: Vector3) -> void:
	if _linger.is_empty():
		return
	var particles := _linger[_linger_next]
	_linger_next = (_linger_next + 1) % _linger.size()
	particles.position = position
	particles.restart()
	particles.emitting = true


## Lumière de l'heure : les feux de camp rougeoient au crépuscule et la nuit.
func set_light_level(level: float) -> void:
	light_level = level
	_update_glows()


func apply_render_quality(_preset: Dictionary) -> void:
	var budget: Dictionary = cfg.get("max_sources", {})
	max_sources = int(budget.get(RenderQuality.current(), budget.get("high", 8)))
	while sources.size() > max_sources:
		remove_smoke_source(_weakest())


func _process(delta: float) -> void:
	for glow in _glows:
		if glow.visible:
			glow.light_energy = float(glow.get_meta("base", 2.0)) * (0.85 + 0.15 * sin(Time.get_ticks_msec() * 0.011 + glow.position.x))
	if delta <= 0.0:
		return


func _weakest() -> int:
	var best := -1
	var low := INF
	for id in sources:
		var value := float(sources[id]["intensity"])
		if value < low:
			low = value
			best = int(id)
	return best


func _update_glows() -> void:
	var dark := clampf((0.75 - light_level) / 0.5, 0.0, 1.0)
	var fires: Array = []
	for id in sources:
		if bool(sources[id]["glow"]):
			fires.append(sources[id])
	fires.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return float(a["intensity"]) > float(b["intensity"]))
	while _glows.size() < mini(fires.size(), MAX_GLOWS):
		var light := OmniLight3D.new()
		light.light_color = Color(1.0, 0.55, 0.22)
		light.omni_range = 16.0
		light.shadow_enabled = false
		add_child(light)
		_glows.append(light)
	for i in _glows.size():
		var glow := _glows[i]
		glow.visible = dark > 0.0 and i < fires.size()
		if glow.visible:
			glow.position = (fires[i]["pos"] as Vector3) + Vector3(0, 1.2, 0)
			glow.set_meta("base", 2.5 * dark * float(fires[i]["intensity"]))


func _emitter(node_name: String, params: Dictionary, intensity: float, one_shot: bool) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = node_name
	particles.amount = int(params.get("amount", 14))
	particles.lifetime = float(params.get("lifetime_s", 8.0))
	particles.one_shot = one_shot
	particles.explosiveness = 0.7 if one_shot else 0.0
	particles.randomness = 0.4
	particles.local_coords = false
	particles.emitting = false
	particles.fixed_fps = 20
	particles.amount_ratio = clampf(intensity, 0.05, 1.0)
	particles.preprocess = 0.0 if one_shot else particles.lifetime * 0.8
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var size: Array = params.get("size_m", [3.0, 8.0])
	var reach := float(params.get("lifetime_s", 8.0)) * (float(params.get("rise_m_s", 1.5)) + float(params.get("drift_m_s", 1.5)))
	particles.visibility_aabb = AABB(Vector3(-reach, -2, -reach), Vector3(reach * 2.0, reach + float(size[1]) * 2.0, reach * 2.0))
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	process.emission_sphere_radius = 1.2 if not one_shot else 2.0
	process.direction = Vector3(0, 1, 0)
	process.spread = 12.0 if not one_shot else 60.0
	var rise := float(params.get("rise_m_s", 1.5))
	process.initial_velocity_min = rise * 0.6
	process.initial_velocity_max = rise * 1.2
	var drift := float(params.get("drift_m_s", 1.5)) * maxf(wind_strength, 0.2)
	process.gravity = Vector3(wind_dir.x * drift * 0.35, rise * 0.05, wind_dir.y * drift * 0.35)
	process.damping_min = 0.1
	process.damping_max = 0.4 if not one_shot else 1.2
	process.scale_min = float(size[0])
	process.scale_max = float(size[0]) * 1.4
	process.angle_min = -180.0
	process.angle_max = 180.0
	var grow := Curve.new()
	grow.add_point(Vector2(0, 0.35))
	grow.add_point(Vector2(1, float(size[1]) / maxf(float(size[0]), 0.1) / 1.2))
	var curve := CurveTexture.new()
	curve.curve = grow
	process.scale_curve = curve
	var gradient := Gradient.new()
	gradient.offsets = PackedFloat32Array([0.0, 0.1, 0.55, 1.0])
	var color: Array = params.get("color", [0.6, 0.6, 0.6, 0.4])
	var c := Color(float(color[0]), float(color[1]), float(color[2]), 1.0)
	var alpha := float(color[3])
	gradient.colors = PackedColorArray([Color(c, 0.0), Color(c, alpha), Color(c, alpha * 0.55), Color(c, 0.0)])
	var ramp := GradientTexture1D.new()
	ramp.gradient = gradient
	process.color_ramp = ramp
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	quad.material = _draw_material(c)
	particles.draw_pass_1 = quad
	add_child(particles)
	return particles


func _draw_material(color: Color) -> Material:
	if ResourceLoader.exists(SMOKE_FLIPBOOK):
		var mat := ShaderMaterial.new()
		mat.shader = SMOKE_SHADER
		mat.set_shader_parameter("flipbook", load(SMOKE_FLIPBOOK))
		mat.set_shader_parameter("smoke_color", color)
		mat.set_shader_parameter("ember_glow_energy", 0.0)
		mat.set_shader_parameter("density", 1.1)
		mat.set_shader_parameter("soft_distance", 3.0)
		mat.set_shader_parameter("near_fade", NEAR_FADE_M)
		return mat
	var fallback := StandardMaterial3D.new()
	fallback.albedo_color = color
	fallback.albedo_texture = BattleEffects._puff_texture()
	fallback.vertex_color_use_as_albedo = true
	fallback.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	fallback.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	fallback.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	return fallback
