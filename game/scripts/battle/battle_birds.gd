class_name BattleBirds
extends Node3D

## EP8 : oiseaux du champ de bataille, rendu seulement. Des volées nichent dans les bois et les
## haies du champ (`get_terrain().forests`, obstacles `hedge`) ; au premier choc, ou quand une
## charge passe à moins de `trigger_radius_m`, la volée s'envole (fuite montante loin du
## tumulte), puis tourne au-dessus du champ quelque temps avant de s'éloigner. À la fin de la
## bataille (ou après `crows_after_s` de mêlée), des corbeaux tournent bas au-dessus du lieu du
## choc. Tous les oiseaux sont dans un seul `MultiMesh` (silhouette plate, battement d'ailes par
## le shader `battle_bird.gdshader`) ; positions calculées ici, quelques centaines d'oiseaux au
## plus. Nombre de volées selon le niveau de qualité (PF1). Paramètres :
## `data/fx/battle_staging.json` (`birds`).

const BIRD_SHADER := preload("res://shaders/battle_bird.gdshader")
const PERCH_HEIGHT := 9.0
const FLEE_TIME := 5.0

var cfg: Dictionary = {}
var flocks: Array = []  # [{roost, state, t, center, birds:[{angle, radius, height, speed, phase, freq}], duration, away}]
var crows_out: bool = false
var launched: int = 0  # volées envolées (tests, captures)

var _height_at: Callable
var _rng := RandomNumberGenerator.new()
var _mm: MultiMesh = null
var _buffer := PackedFloat32Array()
var _roosts: Array = []
var _first_shock_seen: bool = false
var _melee_time: float = 0.0
var _last_shock: Vector3 = Vector3.ZERO
var _field_center: Vector3 = Vector3.ZERO
var _max_flocks: int = 2


## `terrain_data` : `BattleSim.get_terrain()` ; `height_at(x, z)` : hauteur du sol.
func setup(p_cfg: Dictionary, terrain_data: Dictionary, height_at: Callable, seed_value: int) -> void:
	cfg = p_cfg
	name = "Birds"
	_height_at = height_at
	_rng.seed = seed_value
	var width := float(terrain_data.get("width", 1200.0))
	var depth := float(terrain_data.get("depth", 800.0))
	_field_center = Vector3(width * 0.5, 0.0, depth * 0.5)
	_field_center.y = _h(_field_center.x, _field_center.z)
	_roosts = _find_roosts(terrain_data, width, depth)
	var budget: Dictionary = cfg.get("flocks", {})
	_max_flocks = int(budget.get(RenderQuality.current(), budget.get("high", 3)))
	var per: Array = cfg.get("birds_per_flock", [14, 28])
	var total := 0
	for i in mini(_max_flocks, _roosts.size()):
		var count := _rng.randi_range(int(per[0]), int(per[1]))
		flocks.append(_new_flock(_roosts[i], count, false))
		total += count
	var crow_count := int(cfg.get("crows", 18))
	flocks.append(_new_flock(_field_center, crow_count, true))
	total += crow_count
	_build(total)


## Suit la bataille : `units` (`get_units`), `dt` temps de bataille écoulé, `finished` fin.
func update(units: Array, dt: float, finished: bool) -> void:
	if _mm == null:
		return
	var disturb: Array = []
	var in_melee := false
	var melee_sum := Vector3.ZERO
	var melee_n := 0
	for unit in units:
		if not bool(unit["present"]):
			continue
		var state := str(unit["state"])
		if state == "melee":
			in_melee = true
			melee_sum += Vector3(float(unit["x"]), 0.0, float(unit["z"]))
			melee_n += 1
		if state == "charging" or state == "melee":
			disturb.append(Vector3(float(unit["x"]), 0.0, float(unit["z"])))
	if melee_n > 0:
		_last_shock = melee_sum / melee_n
		_last_shock.y = _h(_last_shock.x, _last_shock.z)
	if in_melee:
		_melee_time += dt
	if in_melee and not _first_shock_seen:
		_first_shock_seen = true
		# Premier choc : la volée la plus proche s'envole, où qu'elle soit.
		var nearest: Variant = _nearest_perched(_last_shock)
		if nearest != null:
			_launch(nearest, _last_shock)
	var radius := float(cfg.get("trigger_radius_m", 240.0))
	for flock in flocks:
		if bool(flock["crows"]) or str(flock["state"]) != "perched":
			continue
		for point in disturb:
			if Vector2(point.x, point.z).distance_to(Vector2(flock["roost"].x, flock["roost"].z)) < radius:
				_launch(flock, point)
				break
	if not crows_out and (finished or _melee_time > float(cfg.get("crows_after_s", 30.0))) and _last_shock != Vector3.ZERO:
		crows_out = true
		var crows: Dictionary = flocks[flocks.size() - 1]
		crows["center"] = _last_shock
		crows["state"] = "circle"
		crows["t"] = 0.0
		crows["duration"] = INF
	_advance(dt)


## Force l'envol de toutes les volées (captures).
func launch_all(from: Vector3) -> void:
	for flock in flocks:
		if not bool(flock["crows"]) and str(flock["state"]) == "perched":
			_launch(flock, from)


func _launch(flock: Dictionary, from: Vector3) -> void:
	flock["state"] = "flee"
	flock["t"] = 0.0
	var away := Vector3(flock["roost"].x - from.x, 0.0, flock["roost"].z - from.z)
	if away.length() < 1.0:
		away = Vector3(_rng.randf_range(-1, 1), 0, _rng.randf_range(-1, 1))
	flock["away"] = away.normalized()
	var span: Array = cfg.get("flight_duration_s", [40.0, 70.0])
	flock["duration"] = _rng.randf_range(float(span[0]), float(span[1]))
	# Le cercle se tient entre le perchoir et le tumulte, au-dessus du champ.
	flock["center"] = (flock["roost"] as Vector3).lerp(from, 0.35) + (flock["away"] as Vector3) * 40.0
	flock["pos"] = flock["roost"]
	launched += 1


func _nearest_perched(point: Vector3) -> Variant:
	var best: Variant = null
	var best_d := INF
	for flock in flocks:
		if bool(flock["crows"]) or str(flock["state"]) != "perched":
			continue
		var d := (flock["roost"] as Vector3).distance_to(point)
		if d < best_d:
			best_d = d
			best = flock
	return best


func _advance(dt: float) -> void:
	var flee_speed := float(cfg.get("flee_speed_m_s", 14.0))
	var circle_speed := float(cfg.get("circle_speed_m_s", 9.0))
	var i := 0
	for flock in flocks:
		var state := str(flock["state"])
		flock["t"] = float(flock["t"]) + dt
		var t := float(flock["t"])
		if state == "flee" and t > FLEE_TIME:
			flock["state"] = "circle"
			state = "circle"
		elif state == "circle" and t > float(flock["duration"]):
			flock["state"] = "leave"
			flock["t"] = 0.0
			state = "leave"
		elif state == "leave" and t > 30.0:
			flock["state"] = "gone"
			state = "gone"
		var crows := bool(flock["crows"])
		for bird in flock["birds"]:
			var visible := state != "perched" and state != "gone"
			var pos := Vector3.ZERO
			var heading := Vector3.FORWARD
			if visible:
				var speed := circle_speed * float(bird["speed"]) * (0.6 if crows else 1.0)
				bird["angle"] = float(bird["angle"]) + dt * speed / maxf(float(bird["radius"]), 1.0)
				var a := float(bird["angle"])
				var circle := Vector3(cos(a), 0.0, sin(a)) * float(bird["radius"])
				var center: Vector3 = flock["center"]
				var height := float(bird["height"]) * (0.45 if crows else 1.0)
				var target := center + circle + Vector3(0, height + sin(a * 2.0 + float(bird["phase"])) * 4.0, 0)
				if state == "flee":
					# Envol : du perchoir vers le haut et loin du tumulte, puis vers le cercle.
					var k := clampf(t / FLEE_TIME, 0.0, 1.0)
					var roost: Vector3 = flock["roost"]
					var burst := roost + Vector3(0, PERCH_HEIGHT, 0) + (flock["away"] as Vector3) * flee_speed * t + Vector3(0, 6.0 * t, 0) + Vector3(bird["scatter"].x, 0, bird["scatter"].y) * k
					pos = burst.lerp(target, k * k)
				elif state == "leave":
					pos = target + (flock["away"] as Vector3) * flee_speed * t + Vector3(0, 3.0 * t, 0)
				else:
					pos = target
				heading = Vector3(-sin(a), 0.0, cos(a))
			_write(i, pos, heading, visible, bird)
			i += 1
	_mm.buffer = _buffer


func _write(i: int, pos: Vector3, heading: Vector3, visible: bool, bird: Dictionary) -> void:
	var o := i * 16
	var s := float(cfg.get("wing_span_m", 0.8)) * float(bird["size"]) if visible else 0.0
	var fwd := heading.normalized() * s
	var up := Vector3.UP * s
	var right := up.cross(fwd).normalized() * s if s > 0.0 else Vector3.ZERO
	# Transform3D -> tampon (lignes de la base puis origine) ; x = aile, z = avant.
	_buffer[o + 0] = right.x
	_buffer[o + 1] = up.x
	_buffer[o + 2] = fwd.x
	_buffer[o + 3] = pos.x
	_buffer[o + 4] = right.y
	_buffer[o + 5] = up.y
	_buffer[o + 6] = fwd.y
	_buffer[o + 7] = pos.y
	_buffer[o + 8] = right.z
	_buffer[o + 9] = up.z
	_buffer[o + 10] = fwd.z
	_buffer[o + 11] = pos.z
	_buffer[o + 12] = float(bird["phase"])
	_buffer[o + 13] = float(bird["freq"])
	_buffer[o + 14] = 0.0
	_buffer[o + 15] = 0.0


func _new_flock(roost: Vector3, count: int, crows: bool) -> Dictionary:
	var birds: Array = []
	var radius: Array = cfg.get("circle_radius_m", [60, 140])
	var height: Array = cfg.get("circle_height_m", [35, 80])
	for _b in count:
		birds.append({
			"angle": _rng.randf() * TAU,
			"radius": _rng.randf_range(float(radius[0]), float(radius[1])) * (0.6 if crows else 1.0),
			"height": _rng.randf_range(float(height[0]), float(height[1])),
			"speed": _rng.randf_range(0.8, 1.2),
			"phase": _rng.randf() * TAU,
			"freq": _rng.randf_range(7.0, 11.0) if not crows else _rng.randf_range(3.5, 5.0),
			"size": _rng.randf_range(0.8, 1.2) * (1.25 if crows else 1.0),
			"scatter": Vector2(_rng.randf_range(-25, 25), _rng.randf_range(-25, 25)),
		})
	return {"roost": roost, "state": "perched", "t": 0.0, "center": roost, "birds": birds, "duration": 60.0, "away": Vector3.FORWARD, "crows": crows, "pos": roost}


## Perchoirs : bois du champ (bord le plus proche du centre) et haies, sinon lisières du champ.
func _find_roosts(terrain_data: Dictionary, width: float, depth: float) -> Array:
	var candidates: Array = []
	for zone in terrain_data.get("forests", []):
		var c := Vector2(float(zone["x"]), float(zone["z"]))
		var to_center := Vector2(_field_center.x, _field_center.z) - c
		var edge := c + to_center.normalized() * minf(float(zone.get("radius", 30.0)) * 0.7, to_center.length())
		candidates.append(Vector3(edge.x, 0.0, edge.y))
	for obstacle in terrain_data.get("obstacles", []):
		if str(obstacle.get("kind", "")) == "hedge":
			var mid: Vector2 = ((obstacle["a"] as Vector2) + (obstacle["b"] as Vector2)) * 0.5
			candidates.append(Vector3(mid.x, 0.0, mid.y))
	if candidates.is_empty():
		for k in 4:
			candidates.append(Vector3(width * (0.15 + 0.7 * _rng.randf()), 0.0, depth * (0.08 if k % 2 == 0 else 0.92)))
	# Mélange stable, puis les plus proches du centre d'abord (visibles au premier choc).
	candidates.sort_custom(func(a: Vector3, b: Vector3) -> bool: return a.distance_to(_field_center) < b.distance_to(_field_center))
	var picked: Array = []
	for c in candidates:
		var far_enough := true
		for p in picked:
			if (p as Vector3).distance_to(c) < 120.0:
				far_enough = false
				break
		if far_enough:
			var roost: Vector3 = c
			roost.y = _h(roost.x, roost.z)
			picked.append(roost)
	return picked


func _build(total: int) -> void:
	_mm = MultiMesh.new()
	_mm.transform_format = MultiMesh.TRANSFORM_3D
	_mm.use_custom_data = true
	_mm.mesh = _bird_mesh()
	_mm.instance_count = total
	_buffer.resize(total * 16)
	_buffer.fill(0.0)
	_mm.buffer = _buffer
	var instance := MultiMeshInstance3D.new()
	instance.name = "BirdFlocks"
	instance.multimesh = _mm
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.custom_aabb = AABB(Vector3(-3000, -200, -3000), Vector3(9000, 1200, 8000))
	var mat := ShaderMaterial.new()
	mat.shader = BIRD_SHADER
	var color: Array = cfg.get("color", [0.08, 0.075, 0.07])
	mat.set_shader_parameter("color", Color(float(color[0]), float(color[1]), float(color[2])))
	instance.material_override = mat
	add_child(instance)


## Silhouette plate : corps losange et deux ailes triangulaires (envergure 1 m, le long de x).
static func _bird_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var tris := [
		# Corps.
		[Vector3(0, 0, 0.35), Vector3(0.06, 0, 0), Vector3(-0.06, 0, 0)],
		[Vector3(0.06, 0, 0), Vector3(0, 0, -0.3), Vector3(-0.06, 0, 0)],
		# Ailes (pointe en retrait vers l'arrière).
		[Vector3(0.05, 0, 0.12), Vector3(0.5, 0, -0.08), Vector3(0.05, 0, -0.08)],
		[Vector3(-0.05, 0, 0.12), Vector3(-0.05, 0, -0.08), Vector3(-0.5, 0, -0.08)],
	]
	for tri in tris:
		for v in tri:
			st.set_normal(Vector3.UP)
			st.add_vertex(v)
	return st.commit()


func _h(x: float, z: float) -> float:
	return float(_height_at.call(x, z)) if _height_at.is_valid() else 0.0
