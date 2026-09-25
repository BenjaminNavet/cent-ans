class_name NavalFire
extends RefCounted

## Feu à bord (lot NV1) : flammes et fumée en planches animées du lot V3 (`fire_flame.gdshader`,
## `fire_smoke.gdshader`), étincelles, lueur qui vacille. Particules en coordonnées du monde : la
## fumée traîne derrière un navire qui dérive et le vent la couche. Rendu seulement : l'intensité
## vient de `ship.fire` (0-1) de la simulation.

const FLAME_SHADER := preload("res://shaders/fire_flame.gdshader")
const SMOKE_SHADER := preload("res://shaders/fire_smoke.gdshader")
const FLAME_FLIPBOOK := "res://assets/textures/fx/flame_flipbook.png"
const SMOKE_FLIPBOOK := "res://assets/textures/fx/smoke_flipbook.png"

static var _flame_mat: Material = null
static var _smoke_mat: Material = null


static func make(ship_length: float, ship_beam: float) -> Node3D:
	var root := Node3D.new()
	root.name = "Fire"
	var spread := Vector3(ship_length * 0.38, 1.0, ship_beam * 0.35)
	var flames := _emitter(64, 1.4, Vector2(2.5, 6.5), Vector2(1.5, 3.5), spread, _flame_material(), Color.WHITE, Color.WHITE)
	flames.name = "Flames"
	var flame_process := flames.process_material as ParticleProcessMaterial
	flame_process.anim_speed_min = 1.2
	flame_process.anim_speed_max = 1.8
	flame_process.anim_offset_max = 1.0
	flames.position.y = 2.0
	root.add_child(flames)
	var smoke_color := Color(0.2, 0.18, 0.16, 0.8)
	var smoke := _emitter(110, 12.0, Vector2(9.0, 22.0), Vector2(1.2, 2.6), spread, _smoke_material(smoke_color), smoke_color, Color(smoke_color, 0.25))
	smoke.name = "Smoke"
	var smoke_process := smoke.process_material as ParticleProcessMaterial
	smoke_process.angle_min = -180.0
	smoke_process.angle_max = 180.0
	smoke_process.angular_velocity_min = -10.0
	smoke_process.angular_velocity_max = 10.0
	var growth := Curve.new()
	growth.add_point(Vector2(0.0, 0.45))
	growth.add_point(Vector2(0.4, 0.85))
	growth.add_point(Vector2(1.0, 1.4))
	var growth_texture := CurveTexture.new()
	growth_texture.curve = growth
	smoke_process.scale_curve = growth_texture
	smoke_process.damping_min = 0.3
	smoke_process.damping_max = 0.6
	smoke.position.y = 7.0
	smoke.sorting_offset = -1.0
	root.add_child(smoke)
	var light := OmniLight3D.new()
	light.name = "Glow"
	light.light_color = Color(1.0, 0.55, 0.2)
	light.omni_range = ship_length * 1.4
	light.position.y = 5.0
	light.shadow_enabled = false
	root.add_child(light)
	set_intensity(root, 0.0, Vector2.ZERO)
	return root


## `wind` : vent (m/s dans le plan x-z) qui couche la fumée.
static func set_intensity(root: Node3D, fire: float, wind: Vector2) -> void:
	var flames := root.get_node("Flames") as GPUParticles3D
	var smoke := root.get_node("Smoke") as GPUParticles3D
	var light := root.get_node("Glow") as OmniLight3D
	var on := fire > 0.02
	flames.emitting = on
	smoke.emitting = on or fire > 0.0
	var ratio := clampf(0.2 + fire, 0.0, 1.0)
	flames.amount_ratio = ratio
	smoke.amount_ratio = ratio
	(smoke.process_material as ParticleProcessMaterial).gravity = Vector3(wind.x * 0.6, 0.15, wind.y * 0.6)
	(flames.process_material as ParticleProcessMaterial).gravity = Vector3(wind.x * 0.3, 1.0, wind.y * 0.3)
	light.visible = on
	light.light_energy = (2.0 + 6.0 * fire) * (0.85 + 0.15 * sin(Time.get_ticks_msec() * 0.013))


static func _emitter(amount: int, lifetime: float, size: Vector2, velocity: Vector2, spread: Vector3, material: Material, start: Color, end: Color) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.amount = amount
	particles.lifetime = lifetime
	particles.local_coords = false
	# Un feu qui vient de prendre montre déjà sa colonne de fumée.
	particles.preprocess = minf(lifetime * 0.5, 5.0)
	particles.visibility_aabb = AABB(Vector3(-80, -10, -80), Vector3(160, 90, 160))
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = spread
	process.direction = Vector3.UP
	process.spread = 15.0
	process.initial_velocity_min = velocity.x
	process.initial_velocity_max = velocity.y
	process.scale_min = size.x
	process.scale_max = size.y
	var ramp := Gradient.new()
	ramp.set_color(0, start)
	ramp.set_color(1, end)
	var ramp_texture := GradientTexture1D.new()
	ramp_texture.gradient = ramp
	process.color_ramp = ramp_texture
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	quad.center_offset = Vector3(0.0, 0.45 if material == _flame_mat else 0.0, 0.0)
	quad.material = material
	particles.draw_pass_1 = quad
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return particles


static func _flame_material() -> Material:
	if _flame_mat != null:
		return _flame_mat
	if ResourceLoader.exists(FLAME_FLIPBOOK):
		var material := ShaderMaterial.new()
		material.shader = FLAME_SHADER
		material.set_shader_parameter("flipbook", load(FLAME_FLIPBOOK))
		material.set_shader_parameter("color_hot", Color(1.0, 0.72, 0.28))
		material.set_shader_parameter("color_cold", Color(0.35, 0.03, 0.0))
		material.set_shader_parameter("emission", 3.2)
		_flame_mat = material
	else:
		_flame_mat = _fallback(true)
	return _flame_mat


static func _smoke_material(color: Color) -> Material:
	if _smoke_mat != null:
		return _smoke_mat
	if ResourceLoader.exists(SMOKE_FLIPBOOK):
		var material := ShaderMaterial.new()
		material.shader = SMOKE_SHADER
		material.set_shader_parameter("flipbook", load(SMOKE_FLIPBOOK))
		material.set_shader_parameter("smoke_color", Color(color, 1.0))
		_smoke_mat = material
	else:
		_smoke_mat = _fallback(false)
	return _smoke_mat


static func _fallback(additive: bool) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	material.billboard_keep_scale = true
	material.vertex_color_use_as_albedo = true
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	if additive:
		material.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	return material
