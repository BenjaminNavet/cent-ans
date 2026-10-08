class_name WallCollapseFx
extends Node3D

## Effondrement physique des murailles d'un siège (lot S1) : rendu seulement (ADR 0007).
##
## `BattleSiege.update` lit les PV des pans dans la simulation (`core/`) et nous passe, pour
## chaque pan dont le rapport change, `sync_piece(index, vue, rapport, intact)`. Rien de ce qui
## se passe ici ne remonte au cœur : la physique Godot (Jolt) ne sert qu'à l'image.
##  - pan qui tombe : la maçonnerie visible (BoxMesh, merlons en MultiMesh) est fracturée en
##    N blocs `RigidBody3D` (grille aux coupes décalées, graine tirée de l'index du pan) projetés
##    vers l'extérieur et vers le bas, avec poussière et légère secousse de caméra ; les éboulis
##    statiques de `BattleSiege` n'apparaissent qu'une fois les blocs retombés ;
##  - porte enfoncée : chaque vantail tombe en planches vers l'intérieur de la ville ;
##  - paliers de dégâts (rapport sous 0,75 / 0,5 / 0,25…) : quelques pierres du parapet chutent.
## Débris sur une couche de collision dédiée (ils ne touchent qu'eux-mêmes et un sol local
## échantillonné sur le relief) ; plafond de corps actifs (les plus vieux sont gelés) et de corps
## conservés (les plus vieux sont libérés). Tous les réglages : `data/fx/siege_fx.json`.

const SETTINGS_FILE := "fx/siege_fx.json"

var settings: Dictionary = {}
var enabled := false
var height_at: Callable
var _layer_bits := 0
var _clock := 0.0
var _primed := false
var _states: Dictionary = {}  # index du pan -> {intact, ratio}
var _bodies: Array = []  # [{body, settle_at, mode, sink, depth, frozen, settled}], du plus vieux au plus jeune
var _timers: Array = []  # [{at, kind: "reveal"|"free", node, wall}]
var _grounds: Dictionary = {}  # index du pan -> StaticBody3D
var _hidden_doors: Dictionary = {}  # index du pan -> [MeshInstance3D]
var _shake_left := 0.0
var _shake_total := 0.0
var _shake_amplitude := 0.0
var _shake_camera: Camera3D


## Lecture de `data/fx/siege_fx.json` : dossier de données du jeu (`MapPaths.data_dir`), puis
## `data/` du dépôt (le smoke pointe `MapPaths` sur des fixtures). `{}` si introuvable.
static func load_settings() -> Dictionary:
	if DataFile.exists(SETTINGS_FILE):
		var parsed: Variant = DataFile.read_json(SETTINGS_FILE)
		if parsed is Dictionary:
			return parsed
	push_warning("WallCollapseFx: %s introuvable, effets de siège désactivés" % SETTINGS_FILE)
	return {}


## `p_height_at(x, z)` : hauteur du sol (celle qui pose les murailles). `p_settings` vide =
## lecture du fichier de données.
func setup(p_height_at: Callable, p_settings: Dictionary = {}) -> void:
	height_at = p_height_at
	settings = p_settings if not p_settings.is_empty() else load_settings()
	enabled = not settings.is_empty()
	if enabled:
		_layer_bits = 1 << (int(settings["collision_layer"]) - 1)


## Premier passage de `BattleSiege.update` terminé : l'état initial (brèches de campagne, pans
## déjà battus) est enregistré sans effet ; seuls les changements suivants s'animent.
func prime() -> void:
	_primed = true


## Un pan a changé : `view` = entrée de `BattleSiege._pieces` ({node, wall, rubble, material,
## gate}), `ratio` = PV / PV max, `intact` = verdict de la simulation.
func sync_piece(index: int, view: Dictionary, ratio: float, intact: bool) -> void:
	if not enabled:
		return
	if not _primed:
		_states[index] = {"intact": intact, "ratio": ratio}
		return
	var state: Dictionary = _states.get(index, {"intact": true, "ratio": 1.0})
	var was_intact := bool(state["intact"])
	var old_ratio := float(state["ratio"])
	_states[index] = {"intact": intact, "ratio": ratio}
	if was_intact and not intact:
		if bool(view["gate"]):
			collapse_gate(index, view)
		else:
			collapse_wall(index, view)
	elif intact and not was_intact:
		for door in _hidden_doors.get(index, []):
			if is_instance_valid(door):
				(door as Node3D).visible = true
		_hidden_doors.erase(index)
	elif intact and not bool(view["gate"]):
		for threshold in settings["parapet"]["thresholds"]:
			if old_ratio >= float(threshold) and ratio < float(threshold):
				drop_parapet(index, view)


func active_body_count() -> int:
	var count := 0
	for entry in _bodies:
		if not entry["frozen"]:
			count += 1
	return count


func body_count() -> int:
	return _bodies.size()


func _process(delta: float) -> void:
	advance(delta)


## Horloge de l'effet (appelée par `_process`, ou à la main par les tests headless).
func advance(delta: float) -> void:
	_clock += delta
	var kept: Array = []
	for entry in _bodies:
		var body: RigidBody3D = entry["body"]
		if not is_instance_valid(body):
			continue
		if not entry["settled"] and _clock >= float(entry["settle_at"]):
			entry["settled"] = true
			if entry["mode"] == "free":
				body.queue_free()
				continue
			_freeze(entry)
		elif entry["settled"] and entry["mode"] == "sink":
			# Enfoncement sous le sol en `sink` s : les éboulis statiques prennent le relais.
			var sink := maxf(float(entry["sink"]), 0.01)
			body.global_position.y -= float(entry["depth"]) * delta / sink
			if _clock >= float(entry["settle_at"]) + sink:
				body.queue_free()
				continue
		kept.append(entry)
	_bodies = kept
	var pending: Array = []
	for timer in _timers:
		if _clock < float(timer["at"]):
			pending.append(timer)
			continue
		var node: Node3D = timer["node"]
		if not is_instance_valid(node):
			continue
		if timer["kind"] == "free":
			node.queue_free()
		elif not (timer["wall"] as Node3D).visible:
			node.visible = true  # éboulis, sauf si le pan a été relevé entre-temps
	_timers = pending
	_update_shake(delta)


# --- Effondrement d'un pan ----------------------------------------------------------------------


## Le pan `index` tombe : fracture de sa maçonnerie visible en blocs, poussière, secousse.
## Renvoie le nombre de corps créés.
func collapse_wall(index: int, view: Dictionary) -> int:
	var cfg: Dictionary = settings["wall"]
	var wall: Node3D = view["wall"]
	var node: Node3D = view["node"]
	var rubble: Node3D = view["rubble"]
	var rng := RandomNumberGenerator.new()
	rng.seed = index * 7919 + int(cfg["seed_salt"])
	var mat := _debris_material(view.get("material"))
	var outward := node.global_basis.z.normalized()
	_ensure_ground(index, view)
	var spawned := 0
	var extent := AABB()
	for child in wall.get_children():
		if child is MeshInstance3D and (child as MeshInstance3D).mesh is BoxMesh:
			spawned += _fracture_box(child, rng, cfg, mat, outward)
			extent = _box_extent(child)
		elif child is MultiMeshInstance3D and bool(cfg["merlons_as_bodies"]):
			spawned += _spawn_merlons(child, rng, cfg, mat, outward)
	wall.visible = false
	rubble.visible = false
	_timers.append({"at": _clock + float(cfg["rubble_reveal_seconds"]), "kind": "reveal", "node": rubble, "wall": wall})
	if extent.size != Vector3.ZERO:
		_spawn_dust(node.global_basis, extent, int(settings["dust"]["collapse_amount"]))
	_start_shake(node.global_position)
	return spawned


## Nombre de blocs d'une maçonnerie de `length` m : `blocks_per_meter`, borné à
## [blocks_min, blocks_max].
static func block_count(cfg: Dictionary, length: float) -> int:
	return clampi(roundi(length * float(cfg["blocks_per_meter"])), int(cfg["blocks_min"]), int(cfg["blocks_max"]))


## Découpe une BoxMesh (repère et échelle de son MeshInstance3D) en `block_count` blocs à peu
## près cubiques : assises (rangées) aux hauteurs décalées, une ou plusieurs couches dans
## l'épaisseur, joints verticaux décalés d'une assise à l'autre (appareil de pierre).
func _fracture_box(mesh_instance: MeshInstance3D, rng: RandomNumberGenerator, cfg: Dictionary, mat: Material, outward: Vector3) -> int:
	var box: BoxMesh = mesh_instance.mesh
	var xf := mesh_instance.global_transform
	var size := box.size * xf.basis.get_scale()
	var basis := xf.basis.orthonormalized()
	var target := block_count(cfg, size.x)
	var edge := pow(size.x * size.y * size.z / float(target), 1.0 / 3.0)
	var rows := clampi(roundi(size.y / edge), 1, target)
	var layers := clampi(roundi(size.z / edge), 1, maxi(target / rows, 1))
	var groups := rows * layers
	var jitter := float(cfg["cut_jitter"])
	var row_cuts := _cuts(rows, size.y, jitter, rng)
	var along := basis.x.normalized()
	var spawned := 0
	for g in groups:
		var r := g / layers
		var l := g % layers
		var cols := target / groups + (1 if g < target % groups else 0)
		if cols <= 0:
			continue
		var col_cuts := _cuts(cols, size.x, jitter, rng)
		var y0: float = row_cuts[r]
		var y1: float = row_cuts[r + 1]
		var depth := size.z / float(layers)
		var z := -size.z * 0.5 + (float(l) + 0.5) * depth
		var height01 := (0.5 * (y0 + y1) + size.y * 0.5) / maxf(size.y, 0.01)
		for c in cols:
			var x0: float = col_cuts[c]
			var x1: float = col_cuts[c + 1]
			var local := Vector3(0.5 * (x0 + x1), 0.5 * (y0 + y1), z)
			var block := Vector3(x1 - x0, y1 - y0, depth)
			var body := _spawn_body(Transform3D(basis, xf.origin + basis * local), block, mat, float(cfg["density"]), float(cfg["shape_margin"]), float(cfg["settle_seconds"]), str(cfg["settle_mode"]), float(cfg["sink_seconds"]))
			var out_speed := _rand_range(rng, cfg["outward_impulse"]) * (1.0 + float(cfg["height_boost"]) * height01)
			var lateral := rng.randf_range(-1.0, 1.0) * float(cfg["lateral_impulse"])
			body.linear_velocity = outward * out_speed + along * lateral + Vector3.DOWN * _rand_range(rng, cfg["downward_impulse"])
			body.angular_velocity = _random_unit(rng) * float(cfg["angular_impulse"])
			spawned += 1
	return spawned


## Les merlons (MultiMesh) deviennent chacun un petit corps.
func _spawn_merlons(multi: MultiMeshInstance3D, rng: RandomNumberGenerator, cfg: Dictionary, mat: Material, outward: Vector3) -> int:
	var mm := multi.multimesh
	if mm == null or not (mm.mesh is BoxMesh):
		return 0
	var size: Vector3 = (mm.mesh as BoxMesh).size
	var spawned := 0
	for i in mm.instance_count:
		var xf := multi.global_transform * mm.get_instance_transform(i)
		var scaled := size * xf.basis.get_scale()
		var body := _spawn_body(Transform3D(xf.basis.orthonormalized(), xf.origin), scaled, mat, float(cfg["density"]), float(cfg["shape_margin"]), float(cfg["settle_seconds"]), str(cfg["settle_mode"]), float(cfg["sink_seconds"]))
		body.linear_velocity = outward * _rand_range(rng, cfg["outward_impulse"]) * (1.0 + float(cfg["height_boost"])) + Vector3.DOWN * _rand_range(rng, cfg["downward_impulse"])
		body.angular_velocity = _random_unit(rng) * float(cfg["angular_impulse"])
		spawned += 1
	return spawned


# --- Porte enfoncée -----------------------------------------------------------------------------


## Les vantaux tombent en planches vers l'intérieur de la ville. Renvoie le nombre de corps.
func collapse_gate(index: int, view: Dictionary) -> int:
	var cfg: Dictionary = settings["gate"]
	var wall: Node3D = view["wall"]
	var node: Node3D = view["node"]
	var rng := RandomNumberGenerator.new()
	rng.seed = index * 7919 + int(settings["wall"]["seed_salt"]) + 1
	var inward := -node.global_basis.z.normalized()
	_ensure_ground(index, view)
	var hidden: Array = []
	var spawned := 0
	var extent := AABB()
	for child in wall.get_children():
		if not (child is MeshInstance3D and String(child.name).begins_with("Door")):
			continue
		var door := child as MeshInstance3D
		if not (door.mesh is BoxMesh):
			continue
		var xf := door.global_transform
		var size := (door.mesh as BoxMesh).size * xf.basis.get_scale()
		var basis := xf.basis.orthonormalized()
		var planks := int(cfg["planks_per_leaf"])
		var width := size.x / float(planks)
		var mat := _debris_material(door.material_override)
		for k in planks:
			var local := Vector3(-size.x * 0.5 + (float(k) + 0.5) * width, 0, 0)
			var body := _spawn_body(Transform3D(basis, xf.origin + basis * local), Vector3(width, size.y, size.z), mat, float(cfg["density"]), 0.9, float(cfg["settle_seconds"]), str(cfg["settle_mode"]), float(cfg["sink_seconds"]))
			body.linear_velocity = inward * _rand_range(rng, cfg["inward_impulse"])
			# Bascule autour de l'axe du vantail (le haut part le premier), un peu de vrille.
			body.angular_velocity = basis.x.normalized() * -float(cfg["angular_impulse"]) * rng.randf_range(0.6, 1.2) + _random_unit(rng) * 0.3
			spawned += 1
		door.visible = false
		hidden.append(door)
		extent = _box_extent(door) if extent.size == Vector3.ZERO else extent.merge(_box_extent(door))
	_hidden_doors[index] = hidden
	if extent.size != Vector3.ZERO:
		_spawn_dust(node.global_basis, extent, int(settings["dust"]["collapse_amount"]) / 2)
	_start_shake(node.global_position)
	return spawned


# --- Paliers de dégâts --------------------------------------------------------------------------


## Quelques pierres du parapet se détachent (bord extérieur du sommet). Renvoie le nombre de corps.
func drop_parapet(index: int, view: Dictionary) -> int:
	var cfg: Dictionary = settings["parapet"]
	var wall: Node3D = view["wall"]
	var node: Node3D = view["node"]
	var body_mesh: MeshInstance3D = null
	for child in wall.get_children():
		if child is MeshInstance3D and (child as MeshInstance3D).mesh is BoxMesh:
			body_mesh = child
			break
	if body_mesh == null:
		return 0
	var rng := RandomNumberGenerator.new()
	rng.seed = index * 7919 + int(settings["wall"]["seed_salt"]) + int(_clock * 1000.0) + 2
	_ensure_ground(index, view)
	var xf := body_mesh.global_transform
	var size := (body_mesh.mesh as BoxMesh).size * xf.basis.get_scale()
	var basis := xf.basis.orthonormalized()
	var outward := node.global_basis.z.normalized()
	var mat := _debris_material(view.get("material"))
	var count := rng.randi_range(int(cfg["stones_min"]), int(cfg["stones_max"]))
	for i in count:
		var s := _rand_range(rng, cfg["stone_size"])
		var local := Vector3(rng.randf_range(-0.45, 0.45) * size.x, size.y * 0.5 + s * 0.5, size.z * 0.5 - s * 0.5)
		var body := _spawn_body(Transform3D(basis, xf.origin + basis * local), Vector3(s, s * 0.7, s * 0.8), mat, float(cfg["density"]), 0.9, float(cfg["lifetime_seconds"]), "free")
		body.linear_velocity = outward * _rand_range(rng, cfg["outward_impulse"])
		body.angular_velocity = _random_unit(rng) * 2.0
	var top := AABB(Vector3(-size.x * 0.5, size.y * 0.5 - 1.0, -size.z * 0.5), Vector3(size.x, 2.0, size.z))
	_spawn_dust(basis, AABB(xf.origin + top.position, top.size), int(settings["dust"]["parapet_amount"]))
	return count


# --- Corps, sol, plafonds -----------------------------------------------------------------------


## `mode` à `lifetime` s : "free" (libéré), "freeze" (figé en décor) ou "sink" (figé puis enfoncé
## sous le sol en `sink_seconds` s, puis libéré).
func _spawn_body(xf: Transform3D, size: Vector3, mat: Material, density: float, margin: float, lifetime: float, mode: String, sink_seconds: float = 0.0) -> RigidBody3D:
	var body := RigidBody3D.new()
	body.collision_layer = _layer_bits
	body.collision_mask = _layer_bits
	body.mass = maxf(size.x * size.y * size.z * density, 0.05)
	body.can_sleep = true
	var mesh := BoxMesh.new()
	mesh.size = size
	var visual := MeshInstance3D.new()
	visual.mesh = mesh
	visual.material_override = mat
	body.add_child(visual)
	var shape := BoxShape3D.new()
	shape.size = size * margin
	var collider := CollisionShape3D.new()
	collider.shape = shape
	body.add_child(collider)
	add_child(body)
	body.global_transform = xf
	_bodies.append({"body": body, "settle_at": _clock + lifetime, "mode": mode, "sink": sink_seconds, "depth": size.length(), "frozen": false, "settled": false})
	_enforce_caps()
	return body


## Plafonds : au-delà de `max_active_bodies` corps simulés, les plus vieux sont gelés ; au-delà
## de `max_kept_bodies` corps gelés, les plus vieux sont libérés.
func _enforce_caps() -> void:
	var excess := active_body_count() - int(settings["max_active_bodies"])
	for entry in _bodies:
		if excess <= 0:
			break
		if not entry["frozen"]:
			_freeze(entry)
			excess -= 1
	var frozen := _bodies.size() - active_body_count()
	var too_many := frozen - int(settings["max_kept_bodies"])
	if too_many <= 0:
		return
	var kept: Array = []
	for entry in _bodies:
		if too_many > 0 and entry["frozen"]:
			(entry["body"] as Node).queue_free()
			too_many -= 1
			continue
		kept.append(entry)
	_bodies = kept


func _freeze(entry: Dictionary) -> void:
	entry["frozen"] = true
	var body: RigidBody3D = entry["body"]
	body.freeze_mode = RigidBody3D.FREEZE_MODE_STATIC
	body.freeze = true


## Sol local du pan : le terrain de bataille n'a pas de collision, on échantillonne son relief
## (`height_at`) dans une HeightMapShape3D carrée (longueur du pan + 2 × `patch_margin`) sur la
## couche des débris. Repli : pavé horizontal à la base du pan.
func _ensure_ground(index: int, view: Dictionary) -> void:
	if _grounds.has(index):
		return
	var cfg: Dictionary = settings["ground"]
	var node: Node3D = view["node"]
	var span := 0.0
	for child in (view["wall"] as Node3D).get_children():
		if child is MeshInstance3D and (child as MeshInstance3D).mesh is BoxMesh:
			span = maxf(span, ((child as MeshInstance3D).mesh as BoxMesh).size.x * (child as Node3D).global_basis.get_scale().x)
	var patch := span + 2.0 * float(cfg["patch_margin"])
	var ground := StaticBody3D.new()
	ground.name = "Ground%d" % index
	ground.collision_layer = _layer_bits
	ground.collision_mask = _layer_bits
	var material := PhysicsMaterial.new()
	material.friction = float(cfg["friction"])
	ground.physics_material_override = material
	var collider := CollisionShape3D.new()
	var center := node.global_position
	if height_at.is_valid():
		var spacing := float(cfg["patch_spacing"])
		var samples := maxi(int(patch / spacing), 2) + 1
		var heights := PackedFloat32Array()
		heights.resize(samples * samples)
		var half := float(samples - 1) * 0.5
		for j in samples:
			for i in samples:
				var x := center.x + (float(i) - half) * spacing
				var z := center.z + (float(j) - half) * spacing
				heights[i + j * samples] = float(height_at.call(x, z)) / spacing
		var shape := HeightMapShape3D.new()
		shape.map_width = samples
		shape.map_depth = samples
		shape.map_data = heights
		collider.shape = shape
		collider.scale = Vector3.ONE * spacing
		ground.position = Vector3(center.x, 0.0, center.z)
	else:
		var plane := BoxShape3D.new()
		plane.size = Vector3(patch, 1.0, patch)
		collider.shape = plane
		ground.position = center  # pavé de 1 m centré sur la base du pan (0,5 m sous le sol)
	ground.add_child(collider)
	add_child(ground)
	_grounds[index] = ground


# --- Poussière et secousse ----------------------------------------------------------------------


## Nuage de poussière (un tir) couvrant `extent` (AABB en coordonnées monde, axes de `basis`).
func _spawn_dust(basis: Basis, extent: AABB, amount: int) -> void:
	if amount <= 0:
		return
	var cfg: Dictionary = settings["dust"]
	var particles := GPUParticles3D.new()
	particles.one_shot = true
	particles.explosiveness = 0.75
	particles.amount = amount
	particles.lifetime = float(cfg["lifetime_seconds"])
	particles.local_coords = false
	particles.visibility_aabb = AABB(-extent.size - Vector3.ONE * 20.0, extent.size * 2.0 + Vector3.ONE * 40.0)
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = extent.size * 0.5
	process.direction = Vector3.UP
	process.spread = 70.0
	var velocity: Array = cfg["velocity"]
	process.initial_velocity_min = float(velocity[0])
	process.initial_velocity_max = float(velocity[1])
	process.gravity = Vector3(0, -0.5, 0)
	process.damping_min = 1.0
	process.damping_max = 2.0
	var size: Array = cfg["size"]
	process.scale_min = float(size[0])
	process.scale_max = float(size[1])
	var rgba: Array = cfg["color"]
	var color := Color(float(rgba[0]), float(rgba[1]), float(rgba[2]), float(rgba[3]))
	var ramp := Gradient.new()
	ramp.set_color(0, color)
	ramp.set_color(1, Color(color, 0.0))
	var ramp_texture := GradientTexture1D.new()
	ramp_texture.gradient = ramp
	process.color_ramp = ramp_texture
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.vertex_color_use_as_albedo = true
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	mat.albedo_texture = _puff_texture()
	mat.billboard_keep_scale = true
	quad.material = mat
	particles.draw_pass_1 = quad
	add_child(particles)
	particles.global_transform = Transform3D(basis.orthonormalized(), extent.get_center())
	particles.emitting = true
	_timers.append({"at": _clock + particles.lifetime + 0.5, "kind": "free", "node": particles, "wall": null})


func _start_shake(origin: Vector3) -> void:
	var cfg: Dictionary = settings["camera_shake"]
	if not bool(cfg["enabled"]) or not is_inside_tree():
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance := camera.global_position.distance_to(origin)
	var falloff := 1.0 - distance / maxf(float(cfg["max_distance"]), 0.01)
	if falloff <= 0.0:
		return
	_shake_camera = camera
	_shake_amplitude = maxf(_shake_amplitude if _shake_left > 0.0 else 0.0, float(cfg["amplitude"]) * falloff)
	_shake_total = float(cfg["duration_seconds"])
	_shake_left = _shake_total


## Secousse par les décalages h/v de la caméra : sa position (pilotée par `BattleCamera`) n'est
## jamais touchée.
func _update_shake(delta: float) -> void:
	if _shake_left <= 0.0 or not is_instance_valid(_shake_camera):
		return
	_shake_left = maxf(_shake_left - delta, 0.0)
	var decay := _shake_left / maxf(_shake_total, 0.001)
	var phase := _clock * float(settings["camera_shake"]["frequency"])
	_shake_camera.h_offset = sin(phase) * _shake_amplitude * decay
	_shake_camera.v_offset = cos(phase * 1.31) * _shake_amplitude * decay
	if _shake_left <= 0.0:
		_shake_camera.h_offset = 0.0
		_shake_camera.v_offset = 0.0


func _exit_tree() -> void:
	if is_instance_valid(_shake_camera):
		_shake_camera.h_offset = 0.0
		_shake_camera.v_offset = 0.0


# --- Outils -------------------------------------------------------------------------------------


static var _puff: GradientTexture2D


## Bouffée de poussière : disque blanc au bord fondu (la couleur vient de la rampe des particules).
static func _puff_texture() -> GradientTexture2D:
	if _puff == null:
		var gradient := Gradient.new()
		gradient.set_color(0, Color(1, 1, 1, 1))
		gradient.set_color(1, Color(1, 1, 1, 0))
		gradient.add_point(0.45, Color(1, 1, 1, 0.55))
		_puff = GradientTexture2D.new()
		_puff.gradient = gradient
		_puff.fill = GradientTexture2D.FILL_RADIAL
		_puff.fill_from = Vector2(0.5, 0.5)
		_puff.fill_to = Vector2(1.0, 0.5)
		_puff.width = 64
		_puff.height = 64
	return _puff


## Copie de la matière du pan en triplanaire local : une projection monde ferait glisser la
## texture sur les blocs en mouvement.
static func _debris_material(source: Variant) -> Material:
	if source is StandardMaterial3D:
		var mat := (source as StandardMaterial3D).duplicate() as StandardMaterial3D
		mat.uv1_world_triplanar = false
		return mat
	var fallback := StandardMaterial3D.new()
	fallback.albedo_color = Color(0.6, 0.57, 0.52)
	return fallback


## `n + 1` coupes de -length/2 à +length/2, les coupes intérieures décalées de ± jitter bloc.
static func _cuts(n: int, length: float, jitter: float, rng: RandomNumberGenerator) -> PackedFloat32Array:
	var cuts := PackedFloat32Array()
	var step := length / float(n)
	for k in n + 1:
		var offset := 0.0 if k == 0 or k == n else rng.randf_range(-jitter, jitter) * step
		cuts.append(-length * 0.5 + float(k) * step + offset)
	return cuts


static func _box_extent(mesh_instance: MeshInstance3D) -> AABB:
	var xf := mesh_instance.global_transform
	var size := (mesh_instance.mesh as BoxMesh).size * xf.basis.get_scale()
	return AABB(xf.origin - size * 0.5, size)


static func _rand_range(rng: RandomNumberGenerator, pair: Variant) -> float:
	var values: Array = pair
	return rng.randf_range(float(values[0]), float(values[1]))


static func _random_unit(rng: RandomNumberGenerator) -> Vector3:
	return Vector3(rng.randf_range(-1, 1), rng.randf_range(-1, 1), rng.randf_range(-1, 1)).normalized()
