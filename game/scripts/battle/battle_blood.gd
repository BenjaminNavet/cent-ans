class_name BattleBlood
extends Node3D

## Sang au sol (lot BV1, idée du joueur), rendu seulement, piloté par l'état de la simulation :
## - gerbes à l'impact (particules GPU, pool fixe) : touches des volées (`BattleVolleys`, pertes
##   rapportées par le cœur) et pertes en mêlée (baisse des effectifs d'un régiment au contact) ;
## - flaques qui s'étalent, éclaboussures et traînées : décalques persistants posés à plat sur le
##   sol (MultiMesh `battle_blood_decal.gdshader`, `MAX_DECALS` puis remplacement au hasard) ;
##   traînées derrière les régiments qui fuient après de lourdes pertes.
## Réglage « Sang » (`Settings`, `battle/blood`) : 0 désactivé, 1 modéré (gerbes discrètes,
## flaques plus petites), 2 complet (gerbes, flaques, éclaboussures, traînées).
## Le sang SUR les figurines, les chutes et les démembrements relèvent du lot suivant (BV2).

const DECAL_SHADER := preload("res://shaders/battle_blood_decal.gdshader")

const OFF := 0
const MODERATE := 1
const FULL := 2

const MAX_DECALS := 6000
const SPRAY_EMITTERS := 16
## Au-delà, ni gerbe ni décalque (la tache ne se verrait pas).
const BLOOD_DISTANCE := 450.0
## Gerbes et décalques au plus par régiment et par mise à jour (les grosses pertes d'un coup ne
## doivent pas vider le pool).
const MAX_PER_UNIT := 6

var level: int = MODERATE
## Multiplicateur de taille des unités (ADR 0016) : plus de figurines tombent, plus de sang.
var figure_scale: float = 1.0
var time_now: float = 0.0
var decal_count: int = 0
## Fusion BV1/BV2 : une seule source par événement. Quand BV2 est actif, chaque mort arrive par
## `on_corpse` (signal `BattleSoldiers.corpse_fallen`) : BV2 dessine la gerbe (gouttes de
## `BattleGore`), BV1 seulement la flaque persistante sous le corps. Les pertes en mêlée et les
## touches des volées ne produisent alors plus rien ici (elles aboutissent à ces mêmes morts).
## Sans BV2 (`--no-bv2`) : ancien chemin BV1 (gerbes GPU et flaques sur pertes et touches).
var corpse_driven: bool = false
## Dernier décalque posé (captures : cadrer la caméra dessus).
var last_pos: Vector3 = Vector3.ZERO

var _height_at: Callable
var _water_at: Callable
var _rng := RandomNumberGenerator.new()
var _decals: MultiMesh
var _decal_mat: ShaderMaterial
var _sprays: Array[GPUParticles3D] = []
var _spray_next: int = 0
var _pending: Array = []  # [{time, pos, strength}] touches à venir (traits encore en vol)
var _track: Dictionary = {}  # unit id -> {soldiers, trail_at}


## `water_at(x, z)` : 1 dans l'eau (pas de flaque : le sang s'y dilue ; la gerbe reste).
func setup(height_at: Callable, blood_level: int, water_at: Callable = Callable()) -> void:
	_rng.seed = 6661
	_height_at = height_at
	_water_at = water_at
	level = clampi(blood_level, OFF, FULL)
	if level == OFF:
		return
	_decal_mat = ShaderMaterial.new()
	_decal_mat.shader = DECAL_SHADER
	_decals = MultiMesh.new()
	_decals.transform_format = MultiMesh.TRANSFORM_3D
	_decals.use_custom_data = true
	var plane := PlaneMesh.new()
	plane.size = Vector2(1, 1)
	_decals.mesh = plane
	_decals.instance_count = MAX_DECALS
	_decals.visible_instance_count = 0
	_decals.custom_aabb = AABB(Vector3(-600, -100, -600), Vector3(2800, 700, 2400))
	var instance := MultiMeshInstance3D.new()
	instance.name = "BloodDecals"
	instance.multimesh = _decals
	instance.material_override = _decal_mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.custom_aabb = _decals.custom_aabb
	add_child(instance)
	for i in SPRAY_EMITTERS:
		_sprays.append(_spray_emitter("BloodSpray%d" % i))


func tick_time(now: float) -> void:
	time_now = now
	if level == OFF:
		return
	_decal_mat.set_shader_parameter("time_now", now)
	while not _pending.is_empty() and float(_pending[0]["time"]) <= now:
		var hit: Dictionary = _pending.pop_front()
		_wound(hit["pos"], float(hit["strength"]), Vector3.ZERO)


## Mort d'une figurine (BV2, `corpse_fallen`) : flaque sous le corps, éclaboussures en complet ;
## pas de gerbe (celle de BV2 suffit).
func on_corpse(pos: Vector3, _side: String, kind: String, cause: String, camera_pos: Vector3) -> void:
	if level == OFF or camera_pos.distance_to(pos) > BLOOD_DISTANCE or cause == "fire":
		return
	var size := _rng.randf_range(0.7, 1.4) * (1.0 if level == FULL else 0.7) * (1.4 if kind == "cavalry" else 1.0)
	_add_decal(pos, 0, Vector2(size, size * _rng.randf_range(0.7, 1.0)), _rng.randf() * TAU, 1.0)
	if level == FULL and _rng.randf() < 0.5:
		var off := Vector3(_rng.randf_range(-0.8, 0.8), 0, _rng.randf_range(-0.8, 0.8))
		_add_decal(pos + off, 1, Vector2.ONE * _rng.randf_range(1.0, 1.8), _rng.randf() * TAU, 0.9)


## Touche d'un trait qui se fichera à l'instant `time` (le trait est encore en vol).
func add_hit(pos: Vector3, time: float, camera_pos: Vector3) -> void:
	if level == OFF or corpse_driven or camera_pos.distance_to(pos) > BLOOD_DISTANCE:
		return
	_pending.append({"time": time, "pos": pos, "strength": 0.7})
	_pending.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return float(a["time"]) < float(b["time"]))


## Pertes des régiments au contact (mêlée) et traînées des fuyards.
func update(units: Array, camera_pos: Vector3) -> void:
	if level == OFF:
		return
	for unit in units:
		var id := int(unit["id"])
		var soldiers := int(unit.get("soldiers", 0))
		var rec: Dictionary = _track.get(id, {})
		if rec.is_empty():
			_track[id] = {"soldiers": soldiers, "trail_at": 0.0, "losses": 0.0}
			continue
		var lost := int(rec["soldiers"]) - soldiers
		rec["soldiers"] = soldiers
		if not bool(unit.get("present", false)):
			continue
		var pos := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
		if camera_pos.distance_to(pos) > BLOOD_DISTANCE:
			continue
		var state := str(unit.get("state", ""))
		var facing := float(unit.get("facing", 0.0))
		var fwd := Vector3(sin(facing), 0, cos(facing))
		var right := Vector3(cos(facing), 0, -sin(facing))
		var width := float(unit.get("width", 10.0))
		var depth := float(unit.get("depth", 4.0))
		if lost > 0:
			rec["losses"] = float(rec["losses"]) + lost
		if lost > 0 and not corpse_driven and (state == "melee" or state == "routing"):
			# Au premier rang, là où l'on se bat (les pertes au tir arrivent avec les traits).
			for _i in mini(ceili(lost * figure_scale), MAX_PER_UNIT):
				var p := pos + fwd * depth * 0.5 * _rng.randf_range(0.4, 1.1) + right * width * 0.5 * _rng.randf_range(-1.0, 1.0)
				_wound(p, 1.0, fwd)
		# Traînées : un régiment en déroute qui a beaucoup saigné laisse des traces derrière lui.
		if level == FULL and state == "routing" and float(rec["losses"]) > 8.0 and time_now >= float(rec["trail_at"]):
			rec["trail_at"] = time_now + 1.2
			for _i in 2:
				var p := pos + right * width * 0.5 * _rng.randf_range(-0.8, 0.8) - fwd * depth * 0.5 * _rng.randf()
				_add_decal(p, 2, Vector2(_rng.randf_range(1.6, 3.2), _rng.randf_range(0.35, 0.6)), atan2(fwd.x, fwd.z) + PI * 0.5 + _rng.randf_range(-0.3, 0.3), 0.8)


## Un homme touché en `pos` : gerbe, flaque, éclaboussures (complet), parfois une traînée
## (blessé traîné, sens `dir`).
func _wound(pos: Vector3, strength: float, dir: Vector3) -> void:
	pos.y = _h(pos.x, pos.z)
	_spray(pos + Vector3(0, 1.0, 0), strength)
	var big := level == FULL
	var size := _rng.randf_range(0.7, 1.4) * (1.0 if big else 0.7) * strength
	_add_decal(pos, 0, Vector2(size, size * _rng.randf_range(0.7, 1.0)), _rng.randf() * TAU, 1.0)
	if big:
		var off := Vector3(_rng.randf_range(-0.8, 0.8), 0, _rng.randf_range(-0.8, 0.8))
		_add_decal(pos + off, 1, Vector2.ONE * _rng.randf_range(1.0, 1.8), _rng.randf() * TAU, 0.9)
		if dir != Vector3.ZERO and _rng.randf() < 0.25:
			var back := -dir.rotated(Vector3.UP, _rng.randf_range(-0.6, 0.6))
			_add_decal(pos + back * 1.2, 2, Vector2(_rng.randf_range(1.8, 2.8), 0.45), atan2(back.x, back.z) + PI * 0.5, 0.85)


func _spray(pos: Vector3, strength: float) -> void:
	if _sprays.is_empty():
		return
	var particles: GPUParticles3D = _sprays[_spray_next]
	_spray_next = (_spray_next + 1) % _sprays.size()
	particles.position = pos
	particles.amount_ratio = clampf(strength * (1.0 if level == FULL else 0.45), 0.1, 1.0)
	particles.scale = Vector3.ONE * (1.0 if level == FULL else 0.7)
	particles.restart()
	particles.emitting = true


## Décalque au sol : `kind` 0 flaque, 1 éclaboussures, 2 traînée ; `size` (x, z) en mètres ;
## incliné selon la pente.
func _add_decal(pos: Vector3, kind: int, size: Vector2, yaw: float, intensity: float) -> void:
	if _water_at.is_valid() and int(_water_at.call(pos.x, pos.z)) > 0:
		return
	var idx := decal_count
	if decal_count < MAX_DECALS:
		decal_count += 1
		_decals.visible_instance_count = decal_count
	else:
		idx = _rng.randi_range(0, MAX_DECALS - 1)
	var y := _h(pos.x, pos.z)
	last_pos = Vector3(pos.x, y, pos.z)
	var normal := Vector3(_h(pos.x - 0.6, pos.z) - _h(pos.x + 0.6, pos.z), 1.2, _h(pos.x, pos.z - 0.6) - _h(pos.x, pos.z + 0.6)).normalized()
	var basis := Basis(Vector3.UP, yaw)
	var tilt_axis := Vector3.UP.cross(normal)
	if tilt_axis.length() > 1e-4:
		basis = Basis(tilt_axis.normalized(), Vector3.UP.angle_to(normal)) * basis
	basis = basis.scaled_local(Vector3(size.x, 1.0, size.y))
	_decals.set_instance_transform(idx, Transform3D(basis, Vector3(pos.x, y, pos.z)))
	_decals.set_instance_custom_data(idx, Color(time_now, float(kind), _rng.randf() * 100.0, intensity))


func _h(x: float, z: float) -> float:
	return float(_height_at.call(x, z)) if _height_at.is_valid() else 0.0


func _spray_emitter(node_name: String) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = node_name
	particles.amount = 28
	particles.lifetime = 0.9
	particles.one_shot = true
	particles.explosiveness = 0.95
	particles.randomness = 0.4
	particles.local_coords = false
	particles.emitting = false
	particles.fixed_fps = 30
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	particles.visibility_aabb = AABB(Vector3(-6, -3, -6), Vector3(12, 8, 12))
	var mat := ParticleProcessMaterial.new()
	mat.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	mat.emission_sphere_radius = 0.25
	mat.direction = Vector3(0, 1, 0)
	mat.spread = 60.0
	mat.initial_velocity_min = 1.5
	mat.initial_velocity_max = 4.5
	mat.gravity = Vector3(0, -9.8, 0)
	mat.scale_min = 0.05
	mat.scale_max = 0.16
	var fade := Gradient.new()
	fade.offsets = PackedFloat32Array([0.0, 0.7, 1.0])
	fade.colors = PackedColorArray([Color(1, 1, 1, 1), Color(1, 1, 1, 0.9), Color(1, 1, 1, 0)])
	var ramp := GradientTexture1D.new()
	ramp.gradient = fade
	mat.color_ramp = ramp
	particles.process_material = mat
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	var draw := StandardMaterial3D.new()
	draw.albedo_color = Color(0.42, 0.02, 0.03)
	draw.vertex_color_use_as_albedo = true
	draw.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	draw.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	draw.billboard_keep_scale = true
	draw.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	var drop := GradientTexture2D.new()
	var g := Gradient.new()
	g.offsets = PackedFloat32Array([0.0, 0.6, 1.0])
	g.colors = PackedColorArray([Color(1, 1, 1, 1), Color(1, 1, 1, 0.8), Color(1, 1, 1, 0)])
	drop.gradient = g
	drop.fill = GradientTexture2D.FILL_RADIAL
	drop.fill_from = Vector2(0.5, 0.5)
	drop.fill_to = Vector2(1.0, 0.5)
	drop.width = 32
	drop.height = 32
	draw.albedo_texture = drop
	quad.material = draw
	particles.draw_pass_1 = quad
	add_child(particles)
	return particles
