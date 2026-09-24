class_name BattleEffects
extends Node3D

## Effets des batailles (lot B4), purement visuels et dérivés de l'état de la simulation :
## - poussière soulevée par les régiments en mouvement (surtout la cavalerie qui charge), sur sol
##   sec seulement (ni pluie ni neige) ; éclaboussures au passage des gués et de la rivière ;
## - traits en vol : chaque volée (munitions d'un régiment qui baissent) lance des flèches en
##   cloche ou des carreaux plus tendus, avec une traînée légère, qui restent fichés au sol ;
## - bombardes : éclair, fumée, boulet et gerbe de terre à l'impact ; pierres des engins ;
## - choc des charges : gerbe de poussière au contact.
## Sang : aucun (public large, cf. `docs/wip/b4-effets-animations.md`).
##
## Budget : émetteurs de particules GPU en nombre fixe (réaffectés chaque image aux régiments
## les plus proches de la caméra), traits dans deux MultiMesh à tampon circulaire dont la
## trajectoire est calculée par le shader (`battle_projectile.gdshader`) ; rien au-delà de
## `EFFECT_DISTANCE`.

const PROJECTILE_SHADER := preload("res://shaders/battle_projectile.gdshader")
const TRAIL_SHADER := preload("res://shaders/battle_projectile_trail.gdshader")

## Sortes de traits.
const ARROW := 0
const BOLT := 1
const BALL := 2
const STONE := 3

const MAX_PROJECTILES := 3072
const MAX_BALLS := 64
const DUST_EMITTERS := 10
const SPLASH_EMITTERS := 6
## B8 : sillage d'écume derrière les chevaux au gué (distinct des gerbes `SPLASH_EMITTERS`).
const WAKE_EMITTERS := 6
const BURST_EMITTERS := 8
## Distance caméra au-delà de laquelle ni poussière ni traits ne sont produits (m).
const EFFECT_DISTANCE := 700.0
const DUST_DISTANCE := 420.0
## Vitesse (m/s), flèche relative de l'arc et durée au sol (s) par sorte de trait.
const SPEED := [48.0, 62.0, 110.0, 34.0]
const ARC := [0.16, 0.06, 0.02, 0.3]
const STICK := [30.0, 30.0, 0.0, 0.0]
const DUST_COLOR := Color(0.74, 0.66, 0.52)
const SPLASH_COLOR := Color(0.93, 0.96, 0.98)

var enabled_dust: bool = true
var time_now: float = 0.0
## Traits lancés depuis le début (banc d'essai, captures).
var launched: int = 0

var _height_at: Callable
var _water_at: Callable
var _rng := RandomNumberGenerator.new()
var _arrows: MultiMesh
var _arrow_trails: MultiMesh
var _arrow_data := PackedFloat32Array()
var _arrow_next: int = 0
var _balls: MultiMesh
var _ball_data := PackedFloat32Array()
var _ball_next: int = 0
var _materials: Array[ShaderMaterial] = []
var _dust: Array[GPUParticles3D] = []
var _splash: Array[GPUParticles3D] = []
var _wake: Array[GPUParticles3D] = []  # B8 : sillage d'écume (chevaux au gué)
var _bursts: Dictionary = {}  # sorte -> Array[GPUParticles3D]
var _burst_next: Dictionary = {}
var _flash: OmniLight3D
var _flash_energy: float = 0.0
var _pending: Array = []  # [{time, pos, kind}] impacts à venir
var _track: Dictionary = {}  # unit id -> {ammo, state}
var _dust_spots: Array = []  # [{pos, size, strength}] poussière imposée (captures)
var _dirty: bool = false  # tampon des flèches à renvoyer à la carte graphique


## `weather` : clé météo du rendu ; `height_at(x, z)` : hauteur du sol ; `water_at(x, z)` :
## 0 terre ferme, 1 eau (gué ou rivière).
func setup(weather: String, height_at: Callable, water_at: Callable) -> void:
	_rng.seed = 7351
	_height_at = height_at
	_water_at = water_at
	enabled_dust = weather != "rain" and weather != "snow"
	_arrows = _projectile_layer("Arrows", _arrow_mesh(), MAX_PROJECTILES, false)
	_arrow_trails = _projectile_layer("ArrowTrails", _trail_mesh(), MAX_PROJECTILES, true)
	_arrow_data.resize(MAX_PROJECTILES * 12)
	_arrow_data.fill(0.0)
	_balls = _projectile_layer("Balls", _ball_mesh(), MAX_BALLS, false)
	_ball_data.resize(MAX_BALLS * 12)
	_ball_data.fill(0.0)
	for i in DUST_EMITTERS:
		_dust.append(_emitter("Dust%d" % i, _dust_material(false), 72, 2.8, false))
	for i in SPLASH_EMITTERS:
		_splash.append(_emitter("Splash%d" % i, _splash_material(), 160, 0.8, false))
	for i in WAKE_EMITTERS:
		_wake.append(_emitter("Wake%d" % i, _splash_material(), 90, 1.6, false))
	_bursts = {
		"impact": _burst_pool("Impact", _dust_material(true), 48, 2.2),
		"smoke": _burst_pool("Smoke", _smoke_material(), 40, 5.5),
		"flash": _burst_pool("Flash", _flash_material(), 24, 0.25),
		# B8 : gerbe renforcée à l'entrée d'une charge dans l'eau (écume, pas la poussière brune).
		"ford": _burst_pool("Ford", _splash_material(), 64, 1.0),
	}
	for key in _bursts:
		_burst_next[key] = 0
	_flash = OmniLight3D.new()
	_flash.light_color = Color(1.0, 0.75, 0.4)
	_flash.omni_range = 18.0
	_flash.light_energy = 0.0
	_flash.shadow_enabled = false
	_flash.visible = false
	add_child(_flash)


## Avance le temps des effets (`anim_time` de la bataille, figé en pause).
func tick_time(now: float, dt: float) -> void:
	time_now = now
	for mat in _materials:
		mat.set_shader_parameter("time_now", now)
	while not _pending.is_empty() and float(_pending[0]["time"]) <= now:
		var hit: Dictionary = _pending.pop_front()
		burst(hit["pos"], "impact", float(hit.get("scale", 1.0)))
	if _flash_energy > 0.0:
		_flash_energy = maxf(_flash_energy - dt * 40.0, 0.0)
		_flash.light_energy = _flash_energy
		_flash.visible = _flash_energy > 0.0
	for i in _dust_spots.size():
		if i < _dust.size():
			var spot: Dictionary = _dust_spots[i]
			_place(_dust[i], spot["pos"], spot["size"], 0.0, spot["strength"])


## Suit les régiments (`BattleSim.get_units()`) : volées, charges, poussière, gués.
func update(units: Array, soldiers: BattleSoldiers, now: float, dt: float, camera_pos: Vector3) -> void:
	tick_time(now, dt)
	var by_id := {}
	for unit in units:
		by_id[int(unit["id"])] = unit
	var dusty: Array = []
	var wet: Array = []
	var wakes: Array = []  # B8 : sillage d'écume (sous-ensemble de `wet` : cavalerie seulement)
	for unit in units:
		var id := int(unit["id"])
		var present := bool(unit["present"])
		var state := str(unit["state"])
		var ammo := int(unit.get("ammo", 0))
		var pos := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
		var near := camera_pos.distance_to(pos) < EFFECT_DISTANCE
		var prev: Dictionary = _track.get(id, {})
		if not prev.is_empty() and present and near:
			if ammo < int(prev["ammo"]):
				_on_volley(unit, by_id, soldiers, camera_pos)
			if str(prev["state"]) == "charging" and state == "melee":
				var fwd := _forward(unit)
				burst(pos + fwd * float(unit.get("depth", 6.0)) * 0.5, "impact", 1.6 if str(unit["render"]) == "cavalry" else 1.0)
		var was_wet := bool(prev.get("wet", false))
		_track[id] = {"ammo": ammo, "state": state, "wet": false}
		if not present or not near:
			continue
		var moving := state == "marching" or state == "charging" or state == "routing"
		if not moving:
			continue
		var mounted := str(unit["render"]) == "cavalry"
		var fast := state == "charging" or bool(unit.get("running", false)) or state == "routing"
		var strength := (1.0 if fast else 0.45) * (1.0 if mounted else 0.55)
		var d := camera_pos.distance_to(pos)
		var entry := {"unit": unit, "pos": pos, "strength": strength, "score": strength / (1.0 + d / 120.0)}
		# B7 : la troupe est dans l'eau dès qu'une partie de son emprise y est (pas seulement son
		# centre : la rivière fait ~18 m, un régiment 8 à 15 m de profondeur) ; les éclaboussures
		# ne couvrent que cette partie, à la surface de l'eau.
		var wet_span := _wet_span(unit, pos)
		if wet_span.y > wet_span.x:
			entry["span"] = wet_span
			entry["strength"] = maxf(strength, 0.6 if mounted else 0.4)
			entry["score"] = float(entry["strength"]) / (1.0 + d / 120.0)
			wet.append(entry)
			_track[id]["wet"] = true
			if mounted:
				# B8 : sillage d'écume derrière les chevaux au gué (émetteur dédié, trace le long
				# de l'axe de marche plutôt qu'une gerbe verticale).
				wakes.append(entry)
				# B8 : gerbe renforcée à l'instant où une charge entre dans l'eau (une fois, pas à
				# chaque image tant qu'elle y reste : `was_wet` mémorisé image par image).
				if fast and not was_wet:
					var fwd := _forward(unit)
					var mid := (wet_span.x + wet_span.y) * 0.5
					burst(pos + fwd * mid + Vector3(0, 0.45, 0), "ford", 2.2 if state == "charging" else 1.4)
		elif enabled_dust and d < DUST_DISTANCE:
			dusty.append(entry)
	if _dust_spots.is_empty():
		_assign(_dust, dusty)
	_assign(_splash, wet)
	_assign_wake(_wake, wakes)


## Poussière imposée à un endroit (captures hors simulation).
func dust_at(center: Vector3, size: Vector2, strength: float, _duration: float = 0.0) -> void:
	_dust_spots.append({"pos": center, "size": size, "strength": strength})


## Volée de traits depuis `starts` vers la zone `target` ± `spread` (m).
func volley(starts: PackedVector3Array, target: Vector3, spread: float, kind: int, launch_time: float) -> void:
	for start in starts:
		var end := target + Vector3(_rng.randf_range(-spread, spread), 0.0, _rng.randf_range(-spread, spread) * 1.4)
		if _height_at.is_valid():
			end.y = float(_height_at.call(end.x, end.z))
		launch(start, end, kind, launch_time + _rng.randf_range(0.0, 0.6))


## Un trait de `start` à `end`, tiré à l'instant `launch_time`.
func launch(start: Vector3, end: Vector3, kind: int, launch_time: float) -> void:
	var distance := start.distance_to(end)
	var arc := distance * float(ARC[kind]) * _rng.randf_range(0.85, 1.15)
	var flight := distance / float(SPEED[kind]) * (1.0 + float(ARC[kind]))
	var ball := kind == BALL or kind == STONE
	var data := _ball_data if ball else _arrow_data
	var slot := _ball_next if ball else _arrow_next
	var o := slot * 12
	# Transform3D -> tampon MultiMesh (lignes de la base puis origine) : colonne 0 = départ,
	# colonne 1 = arrivée, colonne 2 = (tir, vol, flèche), origine = (au sol, graine, 0).
	var cols := [start, end, Vector3(launch_time, flight, arc), Vector3(float(STICK[kind]), _rng.randf(), 0.0)]
	for row in 3:
		data[o + row * 4 + 0] = cols[0][row]
		data[o + row * 4 + 1] = cols[1][row]
		data[o + row * 4 + 2] = cols[2][row]
		data[o + row * 4 + 3] = cols[3][row]
	launched += 1
	if ball:
		_ball_next = (slot + 1) % MAX_BALLS
		_ball_data = data
		_balls.buffer = _ball_data
		_pending.append({"time": launch_time + flight, "pos": end, "scale": 1.4 if kind == BALL else 1.0})
		_pending.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return float(a["time"]) < float(b["time"]))
	else:
		_arrow_next = (slot + 1) % MAX_PROJECTILES
		_arrow_data = data
		_dirty = true



func _process(_delta: float) -> void:
	if _dirty:
		_dirty = false
		_arrows.buffer = _arrow_data
		_arrow_trails.buffer = _arrow_data


## Coup de bombarde : éclair, fumée à la bouche (`muzzle`, direction `dir`).
func cannon_fire(muzzle: Vector3, dir: Vector3, _launch_time: float) -> void:
	burst(muzzle + dir * 0.4, "flash", 1.0)
	burst(muzzle + dir * 1.2, "smoke", 1.0)
	_flash.position = muzzle + dir * 1.0 + Vector3(0, 0.5, 0)
	_flash_energy = 9.0
	_flash.light_energy = _flash_energy
	_flash.visible = true


## Gerbe ponctuelle (`impact` : poussière, `smoke` : fumée, `flash` : éclair).
func burst(pos: Vector3, kind: String, scale: float = 1.0) -> void:
	var pool: Array = _bursts.get(kind, [])
	if pool.is_empty():
		return
	var i: int = _burst_next[kind]
	_burst_next[kind] = (i + 1) % pool.size()
	var particles: GPUParticles3D = pool[i]
	particles.position = pos
	particles.scale = Vector3.ONE * scale
	particles.restart()
	particles.emitting = true


# --- Volées -----------------------------------------------------------------------------


func _on_volley(unit: Dictionary, by_id: Dictionary, soldiers: BattleSoldiers, camera_pos: Vector3) -> void:
	var target := _volley_target(unit, by_id)
	if target.is_empty():
		return
	var pos := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
	var aim := Vector3(float(target["x"]), float(target.get("y", 0.0)), float(target["z"]))
	var kind := _missile_kind(unit)
	var spread := maxf(float(target.get("width", 10.0)), 6.0) * 0.5 + 3.0
	var mid := (pos + aim) * 0.5
	var lod := 1.0 if camera_pos.distance_to(mid) < 350.0 else 0.4
	if kind == BALL or kind == STONE:
		var engines := soldiers.soldier_positions(int(unit["id"]), 4) if soldiers != null else PackedVector3Array()
		if engines.is_empty():
			engines.append(pos)
		var dir := (aim - pos).normalized()
		for engine in engines:
			var muzzle := engine + Vector3(0, 0.85, 0) + dir * 1.4
			if kind == BALL:
				cannon_fire(muzzle, dir, time_now)
			var end := aim + Vector3(_rng.randf_range(-spread, spread), 0.0, _rng.randf_range(-spread, spread))
			if _height_at.is_valid():
				end.y = float(_height_at.call(end.x, end.z))
			launch(muzzle if kind == BALL else engine + Vector3(0, 3.5, 0), end, kind, time_now)
		return
	var count := int(mini(int(unit["soldiers"]), 64) * 0.85 * lod)
	var starts := soldiers.soldier_positions(int(unit["id"]), count) if soldiers != null else PackedVector3Array()
	if starts.is_empty():
		for _i in count:
			starts.append(pos + Vector3(_rng.randf_range(-6, 6), 0, _rng.randf_range(-3, 3)))
	for i in starts.size():
		starts[i] += Vector3(0, 1.55 if kind == ARROW else 1.45, 0)
	volley(starts, aim, spread, kind, time_now)


## Régiment visé : la cible de la simulation, sinon l'ennemi le plus proche devant.
func _volley_target(unit: Dictionary, by_id: Dictionary) -> Dictionary:
	var target_id := int(unit.get("target", -1))
	if by_id.has(target_id) and bool(by_id[target_id]["present"]):
		return by_id[target_id]
	var pos := Vector2(float(unit["x"]), float(unit["z"]))
	var fwd := _forward(unit)
	var best := {}
	var best_score := INF
	for other in by_id.values():
		if str(other["side"]) == str(unit["side"]) or not bool(other["present"]):
			continue
		var delta := Vector2(float(other["x"]), float(other["z"])) - pos
		var d := delta.length()
		if d > 450.0:
			continue
		var ahead := delta.dot(Vector2(fwd.x, fwd.z)) / maxf(d, 0.01)
		var score := d * (2.0 - ahead)
		if score < best_score:
			best_score = score
			best = other
	return best


func _missile_kind(unit: Dictionary) -> int:
	var type := str(unit.get("type", ""))
	if str(unit["render"]) == "siege":
		return BALL if type == "unit_bombard" else STONE
	if type.contains("crossbow"):
		return BOLT
	return ARROW


## B7 : partie mouillée de l'emprise d'un régiment, en mètres le long de son axe avant
## (`x` = début, `y` = fin, relatifs au centre ; `x >= y` : au sec). Cinq points échantillonnés
## de l'arrière à l'avant.
func _wet_span(unit: Dictionary, pos: Vector3) -> Vector2:
	if not _water_at.is_valid():
		return Vector2(1, 0)
	var fwd := _forward(unit)
	var half := float(unit.get("depth", 6.0)) * 0.5 + 1.0
	var lo := INF
	var hi := -INF
	for i in 5:
		var t := lerpf(-half, half, i / 4.0)
		var p := pos + fwd * t
		if int(_water_at.call(p.x, p.z)) > 0:
			lo = minf(lo, t)
			hi = maxf(hi, t)
	if hi < lo:
		return Vector2(1, 0)
	var step := half * 0.5
	return Vector2(maxf(lo - step * 0.5, -half), minf(hi + step * 0.5, half))


static func _forward(unit: Dictionary) -> Vector3:
	var facing := float(unit.get("facing", 0.0))
	return Vector3(sin(facing), 0.0, cos(facing))


# --- Émetteurs ----------------------------------------------------------------------------


## Réaffecte le pool d'émetteurs aux entrées les mieux notées.
func _assign(pool: Array[GPUParticles3D], entries: Array) -> void:
	entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return float(a["score"]) > float(b["score"]))
	for i in pool.size():
		var emitter := pool[i]
		if i < entries.size():
			var unit: Dictionary = entries[i]["unit"]
			var size := Vector2(float(unit.get("width", 10.0)), float(unit.get("depth", 6.0)))
			var pos: Vector3 = entries[i]["pos"]
			if entries[i].has("span"):
				# Éclaboussures (B7) : seulement la partie de l'emprise dans l'eau, à la surface
				# (le lit est ~0,5 m sous l'eau ; les gerbes naissaient noyées).
				var span: Vector2 = entries[i]["span"]
				pos += _forward(unit) * (span.x + span.y) * 0.5 + Vector3(0, 0.45, 0)
				size.y = span.y - span.x
			_place(emitter, pos, size, float(unit.get("facing", 0.0)), float(entries[i]["strength"]))
		elif emitter.emitting:
			emitter.emitting = false


## B8 : sillage d'écume, réaffecté comme les gerbes mais posé au bord arrière (dans le sens de la
## marche) de la partie mouillée du régiment, avec une emprise plus étroite qu'une gerbe.
func _assign_wake(pool: Array[GPUParticles3D], entries: Array) -> void:
	entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return float(a["score"]) > float(b["score"]))
	for i in pool.size():
		var emitter := pool[i]
		if i < entries.size():
			var unit: Dictionary = entries[i]["unit"]
			var span: Vector2 = entries[i]["span"]
			var fwd := _forward(unit)
			var pos: Vector3 = entries[i]["pos"] + fwd * span.x + Vector3(0, 0.35, 0)
			var size := Vector2(maxf(float(unit.get("width", 10.0)) * 0.5, 3.0), 3.5)
			_place(emitter, pos, size, float(unit.get("facing", 0.0)), float(entries[i]["strength"]))
		elif emitter.emitting:
			emitter.emitting = false


func _place(emitter: GPUParticles3D, pos: Vector3, size: Vector2, facing: float, strength: float) -> void:
	emitter.position = pos
	emitter.rotation = Vector3(0, facing, 0)
	var mat := emitter.process_material as ParticleProcessMaterial
	var extents := Vector3(size.x * 0.5 + 1.0, 0.2, size.y * 0.5 + 1.0)
	if mat.emission_box_extents.distance_to(extents) > 0.5:
		mat.emission_box_extents = extents
	emitter.amount_ratio = clampf(strength, 0.05, 1.0)
	if not emitter.emitting:
		emitter.emitting = true


func _emitter(node_name: String, draw_mat: Material, amount: int, lifetime: float, one_shot: bool) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = node_name
	particles.amount = amount
	particles.lifetime = lifetime
	particles.one_shot = one_shot
	particles.explosiveness = 0.9 if one_shot else 0.0
	particles.randomness = 0.5
	particles.local_coords = false
	particles.emitting = false
	particles.fixed_fps = 30
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	particles.visibility_aabb = AABB(Vector3(-45, -5, -45), Vector3(90, 35, 90))
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	quad.material = draw_mat
	particles.draw_pass_1 = quad
	particles.process_material = _process_for(node_name)
	add_child(particles)
	return particles


func _burst_pool(node_name: String, draw_mat: Material, amount: int, lifetime: float) -> Array:
	var pool := []
	for i in BURST_EMITTERS if node_name != "Flash" else 3:
		pool.append(_emitter("%s%d" % [node_name, i], draw_mat, amount, lifetime, true))
	return pool


## Mouvement des particules selon le nom de l'émetteur (poussière, gerbe, fumée, éclair).
func _process_for(node_name: String) -> ParticleProcessMaterial:
	var mat := ParticleProcessMaterial.new()
	mat.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	mat.emission_box_extents = Vector3(5, 0.2, 3)
	mat.direction = Vector3(0, 1, 0)
	mat.angle_min = -180.0
	mat.angle_max = 180.0
	var grow := Curve.new()
	if node_name.begins_with("Dust"):
		mat.spread = 70.0
		mat.initial_velocity_min = 0.4
		mat.initial_velocity_max = 1.6
		mat.gravity = Vector3(0.3, 0.15, 0)
		mat.damping_min = 0.3
		mat.damping_max = 0.8
		mat.scale_min = 3.0
		mat.scale_max = 6.0
		grow.add_point(Vector2(0, 0.35))
		grow.add_point(Vector2(1, 1.0))
		mat.color_ramp = _ramp([0.0, 0.15, 1.0], [0.0, 0.6, 0.0])
	elif node_name.begins_with("Splash"):
		# B7 : gerbes plus nombreuses, plus grosses et plus opaques (à peine visibles avant).
		mat.spread = 28.0
		mat.initial_velocity_min = 2.0
		mat.initial_velocity_max = 4.8
		mat.gravity = Vector3(0, -9.8, 0)
		mat.damping_min = 0.5
		mat.damping_max = 1.5
		mat.scale_min = 0.9
		mat.scale_max = 2.2
		grow.add_point(Vector2(0, 0.5))
		grow.add_point(Vector2(1, 1.4))
		mat.color_ramp = _ramp([0.0, 0.08, 0.6, 1.0], [0.0, 0.9, 0.5, 0.0])
	elif node_name.begins_with("Wake"):
		# B8 : sillage d'écume, entraîné vers l'arrière (pas projeté vers le haut comme une gerbe)
		# et étalé sur les côtés, plus longue durée de vie pour laisser une traîne visible.
		mat.direction = Vector3(0, 0.3, -1)
		mat.spread = 45.0
		mat.initial_velocity_min = 0.6
		mat.initial_velocity_max = 1.8
		mat.gravity = Vector3(0, -0.4, 0)
		mat.damping_min = 0.6
		mat.damping_max = 1.4
		mat.scale_min = 0.7
		mat.scale_max = 1.6
		grow.add_point(Vector2(0, 0.4))
		grow.add_point(Vector2(1, 1.1))
		mat.color_ramp = _ramp([0.0, 0.15, 0.7, 1.0], [0.0, 0.65, 0.35, 0.0])
	elif node_name.begins_with("Ford"):
		# B8 : gerbe renforcée à l'entrée d'une charge dans l'eau (plus large et plus vive
		# qu'une gerbe de gué ordinaire, ponctuelle comme les autres tampons de rafale).
		mat.emission_box_extents = Vector3(3.5, 0.3, 2.5)
		mat.spread = 40.0
		mat.initial_velocity_min = 3.5
		mat.initial_velocity_max = 7.5
		mat.gravity = Vector3(0, -9.8, 0)
		mat.damping_min = 0.6
		mat.damping_max = 1.6
		mat.scale_min = 1.2
		mat.scale_max = 2.8
		grow.add_point(Vector2(0, 0.55))
		grow.add_point(Vector2(1, 1.5))
		mat.color_ramp = _ramp([0.0, 0.08, 0.6, 1.0], [0.0, 1.0, 0.55, 0.0])
	elif node_name.begins_with("Impact"):
		mat.emission_box_extents = Vector3(3, 0.3, 2)
		mat.spread = 55.0
		mat.initial_velocity_min = 2.0
		mat.initial_velocity_max = 5.0
		mat.gravity = Vector3(0, -2.0, 0)
		mat.damping_min = 2.0
		mat.damping_max = 3.0
		mat.scale_min = 1.6
		mat.scale_max = 3.5
		grow.add_point(Vector2(0, 0.5))
		grow.add_point(Vector2(1, 1.3))
		mat.color_ramp = _ramp([0.0, 0.1, 1.0], [0.0, 0.55, 0.0])
	elif node_name.begins_with("Smoke"):
		mat.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
		mat.emission_sphere_radius = 0.6
		mat.direction = Vector3(0, 0.25, 1)
		mat.spread = 35.0
		mat.initial_velocity_min = 0.5
		mat.initial_velocity_max = 4.0
		mat.gravity = Vector3(0.25, 0.45, 0)
		mat.damping_min = 1.2
		mat.damping_max = 2.0
		mat.scale_min = 2.0
		mat.scale_max = 3.5
		grow.add_point(Vector2(0, 0.45))
		grow.add_point(Vector2(1, 2.6))
		mat.color_ramp = _ramp([0.0, 0.04, 0.4, 1.0], [0.0, 0.9, 0.6, 0.0])
	else:  # Flash
		mat.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
		mat.emission_sphere_radius = 0.3
		mat.direction = Vector3(0, 0, 1)
		mat.spread = 20.0
		mat.initial_velocity_min = 4.0
		mat.initial_velocity_max = 12.0
		mat.scale_min = 0.8
		mat.scale_max = 1.8
		grow.add_point(Vector2(0, 1.0))
		grow.add_point(Vector2(1, 0.2))
		mat.color_ramp = _ramp([0.0, 0.3, 1.0], [1.0, 0.8, 0.0])
	var curve := CurveTexture.new()
	curve.curve = grow
	mat.scale_curve = curve
	return mat


static func _ramp(offsets: Array, alphas: Array) -> GradientTexture1D:
	var gradient := Gradient.new()
	gradient.offsets = PackedFloat32Array(offsets)
	var colors := PackedColorArray()
	for a in alphas:
		colors.append(Color(1, 1, 1, float(a)))
	gradient.colors = colors
	var texture := GradientTexture1D.new()
	texture.gradient = gradient
	return texture


## Tache ronde et floue (texture des particules).
static func _puff_texture() -> GradientTexture2D:
	var gradient := Gradient.new()
	gradient.offsets = PackedFloat32Array([0.0, 0.5, 1.0])
	gradient.colors = PackedColorArray([Color(1, 1, 1, 1), Color(1, 1, 1, 0.45), Color(1, 1, 1, 0)])
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.fill = GradientTexture2D.FILL_RADIAL
	texture.fill_from = Vector2(0.5, 0.5)
	texture.fill_to = Vector2(1.0, 0.5)
	texture.width = 64
	texture.height = 64
	return texture


func _billboard(color: Color, unshaded: bool, additive: bool) -> StandardMaterial3D:
	# Sans éclairage (un panneau face à la caméra s'assombrit à contre-jour) ; la couleur de
	# base tient lieu de lumière diffuse.
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color if unshaded else color * Color(0.9, 0.9, 0.9)
	mat.albedo_texture = _puff_texture()
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	mat.billboard_keep_scale = true
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	if additive:
		mat.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	return mat


func _dust_material(heavy: bool) -> StandardMaterial3D:
	return _billboard(DUST_COLOR.darkened(0.1) if heavy else DUST_COLOR, false, false)


func _splash_material() -> StandardMaterial3D:
	return _billboard(SPLASH_COLOR, true, false)


func _smoke_material() -> StandardMaterial3D:
	return _billboard(Color(0.86, 0.85, 0.82), false, false)


func _flash_material() -> StandardMaterial3D:
	return _billboard(Color(1.0, 0.72, 0.3), true, true)


# --- Traits -----------------------------------------------------------------------------


func _projectile_layer(node_name: String, mesh: Mesh, count: int, is_trail: bool) -> MultiMesh:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = mesh
	mm.instance_count = count
	var mat := ShaderMaterial.new()
	mat.shader = TRAIL_SHADER if is_trail else PROJECTILE_SHADER
	_materials.append(mat)
	var instance := MultiMeshInstance3D.new()
	instance.name = node_name
	instance.multimesh = mm
	instance.material_override = mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Les transformées portent des trajectoires, pas des positions : boîte fixe sur le champ.
	instance.custom_aabb = AABB(Vector3(-600, -100, -600), Vector3(2800, 700, 2400))
	add_child(instance)
	return mm


## Flèche (fût, pointe, empennage) le long de +Z, pointe en z = 0 ; carreaux : même maillage
## (la sorte ne change que la trajectoire).
static func _arrow_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var wood := Color(0.42, 0.3, 0.18)
	var iron := Color(0.25, 0.25, 0.27)
	var fletch := Color(0.85, 0.84, 0.78)
	BattleMeshes.add_box(st, Vector3(0, 0, -0.42), Vector3(0.025, 0.025, 0.8), wood)
	BattleMeshes.add_box(st, Vector3(0, 0, -0.03), Vector3(0.04, 0.04, 0.07), iron)
	BattleMeshes.add_box(st, Vector3(0, 0, -0.76), Vector3(0.005, 0.09, 0.14), fletch)
	BattleMeshes.add_box(st, Vector3(0, 0, -0.76), Vector3(0.09, 0.005, 0.14), fletch)
	return st.commit()


## Ruban de la traînée : largeur 5 cm, de z = 0 à z = -1 (allongé par le shader).
static func _trail_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var w := 0.03
	for p in [Vector3(-w, 0, 0), Vector3(w, 0, 0), Vector3(w, 0, -1), Vector3(-w, 0, 0), Vector3(w, 0, -1), Vector3(-w, 0, -1)]:
		st.set_normal(Vector3.UP)
		st.add_vertex(p)
	return st.commit()


static func _ball_mesh() -> ArrayMesh:
	var sphere := SphereMesh.new()
	sphere.radius = 0.2
	sphere.height = 0.4
	sphere.radial_segments = 8
	sphere.rings = 4
	var st := SurfaceTool.new()
	st.create_from(sphere, 0)
	var arrays := st.commit_to_arrays()
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var colors := PackedColorArray()
	colors.resize(verts.size())
	colors.fill(Color(0.3, 0.3, 0.31))
	arrays[Mesh.ARRAY_COLOR] = colors
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh
