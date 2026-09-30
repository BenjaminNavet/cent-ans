class_name GroundClutter
extends Node3D

## Lot FC3 : touffes d'herbe et broussailles proches de la caméra sur la carte de campagne.
##
## - Cellules de `cell_size` unités (plus fines que les tuiles de terrain de 256 : une tuile
##   entière ferait des centaines de milliers de touffes) semées autour du point visé, un
##   `MultiMeshInstance3D` (un appel de dessin) par cellule, sans ombre ; cache borné des cellules
##   construites (`max_cached_cells`, les plus anciennes hors champ libérées), comme `Vegetation`.
## - Semis déterministe par cellule (graine = coordonnées) sur l'occupation du sol de
##   `VegetationMask` (splatmap : prairie, lande, un peu de cultures, lisières de forêt), hors eau
##   et hors clairières des colonies (`exclusions`, mêmes cercles que la végétation).
## - Densité selon le préréglage (`clutter_density`, 0 en Basse : rien n'est construit). Les
##   instances sont semées pour la densité maximale (`max_density`) dans un ordre aléatoire : le
##   préréglage ne fait que borner `visible_instance_count`, sans nouveau semis.
## - Seulement sous `max_camera_distance` ; fondu par rang (shader) sur `fade_band` et au bord du
##   disque visible ; taille liée à `campaign_prop_scale` (arbres, lot SZ4) et fondu quand elle
##   devient trop petite.
## - Recalage sur la surface affichée : cellules touchées par `chunk_surface_changed` /
##   `surface_rect_changed` re-posées (hauteurs seules) par tranches.
## Purement visuel : aucune règle de jeu.

const SHADER := preload("res://shaders/ground_clutter.gdshader")
## Lot FC5 : touffe d'herbe texturée (copie mipmappée de `grass_blades.png`, `build_leaf_cards.py`).
const GRASS_TEXTURE := "res://assets/textures/vegetation/campaign_grass_tuft.png"
const FLOATS_PER_INSTANCE := 16  # transformation 3 × 4 + données personnalisées

@export var camera_rig_path: NodePath = ^"../CameraRig"
## Distance caméra au-delà de laquelle plus aucune touffe (fondu sur `fade_band` en deçà).
@export var max_camera_distance: float = 40.0
@export var fade_band: float = 8.0
## Côté (unités monde) d'une cellule de semis.
@export var cell_size: float = 32.0
## Rayon du disque garni autour du point visé : distance caméra × `radius_factor`, borné.
@export var radius_factor: float = 1.3
@export var min_radius: float = 16.0
@export var max_radius: float = 52.0
## Candidats par cellule à la densité 1 (préréglage Haute) sur une couverture pleine.
@export var base_per_cell: int = 4000
## Densité maximale d'un préréglage (Ultra) : nombre de candidats semés.
@export var max_density: float = 1.5
## Hauteur (unités monde, taille de carte) d'une touffe d'herbe et d'une broussaille.
## Lot FC5 : 0,22 / 0,42 (≈ 6 px à d = 25 en 720p) ; le shader les réduit comme
## `campaign_prop_scale`^`prop_scale_power` (plus lentement que les arbres : encore visibles à d = 10).
@export var grass_height: float = 0.22
@export var bush_height: float = 0.42
@export var prop_scale_power: float = 0.5
## Échelle des arbres (`campaign_prop_scale`) sous laquelle les touffes disparaissent (trop
## petites pour valoir leur coût).
@export var min_prop_scale: float = 0.025
## Cellules posées par image (semis fait dans le `WorkerThreadPool` si `threaded`).
## Lot FC6 : une seule cellule posée par image (le semis et les hauteurs sont faits dans le
## `WorkerThreadPool` ; le fil principal ne crée que le MultiMesh).
@export var max_cells_per_frame: int = 1
@export var max_concurrent_jobs: int = 4
@export var threaded: bool = true
@export var max_reground_per_frame: int = 2
@export var max_cached_cells: int = 96
## Plafond des instances visibles (toutes cellules).
@export var max_visible_instances: int = 60000

## Préréglage de qualité : densité (0 : rien).
var quality_density: float = 1.0
var map_data: MapData
var mask: VegetationMask
var terrain: TerrainBuilder
var exclusions: PackedVector3Array = PackedVector3Array()
## Tests : `weight_sampler(x, y) -> Vector2(probabilité, part de broussaille)` et
## `height_sampler(points: PackedVector2Array) -> PackedFloat32Array` remplacent le masque et le
## terrain.
var weight_sampler: Callable
var height_sampler: Callable
var stats: Dictionary = {"cells": 0, "visible_cells": 0, "instances": 0, "visible": 0, "build_ms_max": 0.0}  # build_ms_max : pose sur le fil principal

var _vegetation: Node
var _rig: Node3D
var _material: ShaderMaterial
var _mesh: ArrayMesh
var _cells: Dictionary = {}  # Vector2i → {"mmi", "points": PackedVector2Array, "buffer", "last_seen"}
var _dirty: Dictionary = {}  # Vector2i → vrai (recalage à faire)
var _jobs: Dictionary = {}  # Vector2i → {"task": int, "result": Dictionary}
var _ground_jobs: Dictionary = {}  # FC6 : Vector2i → {"task", "mmi", "buffer"[, "stale"]}
var _frame: int = 0
var _focus := Vector2.ZERO
var _radius: float = 0.0


func _ready() -> void:
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	_rig = get_node_or_null(camera_rig_path) as Node3D


func apply_render_quality(p: Dictionary) -> void:
	quality_density = float(p.get("clutter_density", 1.0))
	_apply_counts()


## Branche le semis sur la carte. `vegetation` : nœud `Vegetation` dont on réutilise le masque
## (lu dès qu'il existe) ; `terrain_builder` : surface affichée.
func setup(data: MapData, terrain_builder: TerrainBuilder, vegetation: Node = null, exclusion_circles := PackedVector3Array()) -> void:
	clear()
	map_data = data
	_vegetation = vegetation
	exclusions = exclusion_circles
	if terrain != null:
		if terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
			terrain.chunk_surface_changed.disconnect(_on_chunk_surface_changed)
		if terrain.surface_rect_changed.is_connected(_on_surface_rect_changed):
			terrain.surface_rect_changed.disconnect(_on_surface_rect_changed)
	terrain = terrain_builder
	if terrain != null:
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
		terrain.surface_rect_changed.connect(_on_surface_rect_changed)


func _exit_tree() -> void:
	_wait_jobs()


func _wait_jobs() -> void:
	for holder: Dictionary in _jobs.values():
		WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
	_jobs.clear()
	for holder: Dictionary in _ground_jobs.values():
		WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
	_ground_jobs.clear()


## Tâches de semis en cours (tests, captures).
func pending_jobs() -> int:
	return _jobs.size() + _ground_jobs.size()


func clear() -> void:
	_wait_jobs()
	for entry: Dictionary in _cells.values():
		(entry["mmi"] as Node).queue_free()
	_cells.clear()
	_dirty.clear()
	stats["cells"] = 0
	stats["visible_cells"] = 0
	stats["instances"] = 0
	stats["visible"] = 0


func cell_count() -> int:
	return _cells.size()


## Instances affichées (cellules visibles, `visible_instance_count`).
func visible_instances() -> int:
	return int(stats["visible"])


## Cellules visibles : Array de {"rect": Rect2, "count": int, "points": PackedVector2Array}.
func visible_cells() -> Array:
	var result: Array = []
	for key: Vector2i in _cells:
		var entry: Dictionary = _cells[key]
		var mmi: MultiMeshInstance3D = entry["mmi"]
		if mmi.visible:
			result.append({"rect": _cell_rect(key), "count": mmi.multimesh.visible_instance_count, "points": entry["points"]})
	return result


func focus() -> Vector2:
	return _focus


func radius() -> float:
	return _radius


func _process(_delta: float) -> void:
	if map_data == null:
		return
	if mask == null and _vegetation != null:
		mask = _vegetation.get("mask") as VegetationMask
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	var focus_value: Variant = _rig.get("focus") if _rig != null else null
	var at := Vector2(focus_value.x, focus_value.z) if focus_value is Vector3 else Vector2(camera.global_position.x, camera.global_position.z)
	update_view(at, distance)


## Mise à jour : cellules du disque autour de `at` construites (`max_cells_per_frame` par appel),
## visibilité, fondu. Sans coût quand la caméra est au-delà de la portée.
func update_view(at: Vector2, camera_distance: float) -> void:
	_frame += 1
	var prop_scale := MapPropScale.shared().tree_scale(camera_distance)  # = `campaign_prop_scale`
	var fade := clampf((max_camera_distance - camera_distance) / maxf(fade_band, 0.001), 0.0, 1.0)
	fade *= smoothstep(min_prop_scale, min_prop_scale * 2.0, prop_scale)
	var active := fade > 0.001 and quality_density > 0.001 and (mask != null or weight_sampler.is_valid())
	if not active:
		if visible:
			visible = false
			stats["visible"] = 0
			stats["visible_cells"] = 0
		return
	visible = true
	_focus = at
	_radius = clampf(camera_distance * radius_factor, min_radius, max_radius)
	_ensure_resources()
	_material.set_shader_parameter("fade", fade)
	_material.set_shader_parameter("focus", at)
	_material.set_shader_parameter("radius", _radius)
	var lo := Vector2i(floori((at.x - _radius) / cell_size), floori((at.y - _radius) / cell_size))
	var hi := Vector2i(floori((at.x + _radius) / cell_size), floori((at.y + _radius) / cell_size))
	var wanted: Dictionary = {}
	var missing: Array[Vector2i] = []
	for cy in range(lo.y, hi.y + 1):
		for cx in range(lo.x, hi.x + 1):
			var key := Vector2i(cx, cy)
			if not _cell_in_disc(key, at, _radius) or not _cell_on_map(key):
				continue
			wanted[key] = true
			if not _cells.has(key) and not _jobs.has(key):
				missing.append(key)
	# Cellules les plus proches du point visé d'abord.
	missing.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return _cell_rect(a).get_center().distance_squared_to(at) < _cell_rect(b).get_center().distance_squared_to(at))
	var built := 0
	for key in missing:
		if threaded:
			if _jobs.size() >= max_concurrent_jobs:
				break
			var t_snap := Time.get_ticks_usec()
			var source := _height_source(_cell_rect(key))
			stats["snapshot_ms_max"] = maxf(float(stats.get("snapshot_ms_max", 0.0)), (Time.get_ticks_usec() - t_snap) / 1000.0)
			var holder := {"result": {}}
			holder["task"] = WorkerThreadPool.add_task(func() -> void: holder["result"] = _ground_seeded(_seed_cell(key), source), false, "GroundClutter")
			_jobs[key] = holder
		elif built < max_cells_per_frame:
			_install_cell(key, _ground_seeded(_seed_cell(key), _height_source(_cell_rect(key))))
			built += 1
	for key: Vector2i in _jobs.keys():
		if built >= max_cells_per_frame:
			break
		var holder: Dictionary = _jobs[key]
		if not WorkerThreadPool.is_task_completed(int(holder["task"])):
			continue
		WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
		_jobs.erase(key)
		_install_cell(key, holder["result"])
		if holder.get("stale", false):
			_dirty[key] = true  # surface changée pendant le semis
		built += 1
	_update_regrounds()
	for key: Vector2i in _cells:
		var entry: Dictionary = _cells[key]
		var shown := wanted.has(key)
		(entry["mmi"] as MultiMeshInstance3D).visible = shown
		if shown:
			entry["last_seen"] = _frame
	_evict()
	_apply_counts()


func _ensure_resources() -> void:
	if _material != null:
		return
	_material = ShaderMaterial.new()
	_material.shader = SHADER
	_material.set_shader_parameter("prop_scale_power", prop_scale_power)
	if ResourceLoader.exists(GRASS_TEXTURE):
		_material.set_shader_parameter("grass_texture", load(GRASS_TEXTURE))
		_material.set_shader_parameter("has_grass_texture", true)
	_mesh = crossed_cards_mesh()


## Trois cartes verticales croisées à 60°, 1 × 1, pied à l'origine (UV.y = 0 en haut).
static func crossed_cards_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	for k in 3:
		var angle := PI * float(k) / 3.0
		var side := Vector3(cos(angle), 0.0, sin(angle)) * 0.5
		var normal := Vector3(-sin(angle), 0.0, cos(angle))
		var base := vertices.size()
		vertices.append_array([-side, side, side + Vector3.UP, -side + Vector3.UP])
		for n in 4:
			normals.append(normal.lerp(Vector3.UP, 0.6).normalized())
		uvs.append_array([Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)])
		indices.append_array([base, base + 1, base + 2, base, base + 2, base + 3])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _cell_rect(key: Vector2i) -> Rect2:
	return Rect2(Vector2(key) * cell_size, Vector2(cell_size, cell_size))


func _cell_on_map(key: Vector2i) -> bool:
	if key.x < 0 or key.y < 0:
		return false
	return map_data == null or (key.x * cell_size < map_data.size.x and key.y * cell_size < map_data.size.y)


func _cell_in_disc(key: Vector2i, at: Vector2, r: float) -> bool:
	var rect := _cell_rect(key)
	var nearest := Vector2(clampf(at.x, rect.position.x, rect.end.x), clampf(at.y, rect.position.y, rect.end.y))
	return nearest.distance_squared_to(at) <= r * r


## Probabilité de touffe et part de broussaille au point (x, y) ; `forest` : couverture
## forestière grossière de la cellule.
func _weight(x: float, y: float, forest: float) -> Vector2:
	if weight_sampler.is_valid():
		return weight_sampler.call(x, y)
	if map_data != null and not map_data.is_land_px(int(x), int(y)):
		return Vector2.ZERO
	var grass := 0.5
	var heath := 0.0
	var crops := 0.0
	if mask.has_splat():
		var splat := mask.splat_at(x, y)
		grass = splat.r
		heath = splat.a
		crops = splat.g
	var edge := clampf(forest * (1.0 - forest) * 4.0, 0.0, 1.0)
	var p := clampf((grass + heath * 0.6) * (1.0 - forest) + crops * 0.12 + edge * 0.7, 0.0, 1.0)
	var bush := clampf(0.12 + edge * 0.6 + heath * 0.35, 0.0, 0.9)
	return Vector2(p, bush)


## Couverture forestière grossière (3 × 3 échantillons de `VegetationMask.sample`).
func _forest_grid(rect: Rect2) -> PackedFloat32Array:
	var grid := PackedFloat32Array()
	grid.resize(9)
	if mask == null or weight_sampler.is_valid():
		return grid
	var noise := VegetationMask.make_noise()
	for j in 3:
		for i in 3:
			var p := rect.position + Vector2(i + 0.5, j + 0.5) * rect.size / 3.0
			grid[j * 3 + i] = float(mask.sample(p.x, p.y, noise)["forest"])
	return grid


## Semis d'une cellule (sans nœud ni terrain : appelable depuis une tâche) : points, tampon
## MultiMesh à hauteur 0 (posé par `_install_cell`), sommet des touffes.
func _seed_cell(key: Vector2i) -> Dictionary:
	var rect := _cell_rect(key)
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(Vector3i(key.x, key.y, 0xFC3))
	var forest := _forest_grid(rect)
	var circles := PackedVector3Array()
	for c in exclusions:
		if Rect2(rect.position - Vector2(c.z, c.z), rect.size + Vector2(c.z, c.z) * 2.0).has_point(Vector2(c.x, c.y)):
			circles.append(c)
	var candidates := int(round(base_per_cell * max_density))
	var points := PackedVector2Array()
	var buffer := PackedFloat32Array()
	buffer.resize(candidates * FLOATS_PER_INSTANCE)
	var top := 0.0
	var o := 0
	for n in candidates:
		var p := rect.position + Vector2(rng.randf(), rng.randf()) * cell_size
		var roll := rng.randf()
		var kind_roll := rng.randf()
		var tint := rng.randf()
		var phase := rng.randf()
		var yaw := rng.randf() * TAU
		var size := rng.randf_range(0.7, 1.3)
		var local := (p - rect.position) / cell_size * 3.0
		var w := _weight(p.x, p.y, forest[clampi(int(local.y), 0, 2) * 3 + clampi(int(local.x), 0, 2)])
		if roll >= w.x or (not circles.is_empty() and _excluded(p, circles)):
			continue
		points.append(p)
		var bush := 1.0 if kind_roll < w.y else 0.0
		var s := size * (bush_height if bush > 0.5 else grass_height)
		var c := cos(yaw) * s
		var si := sin(yaw) * s
		buffer[o] = c
		buffer[o + 2] = si
		buffer[o + 3] = p.x - rect.position.x
		buffer[o + 5] = s
		buffer[o + 7] = -s * 0.08  # pied légèrement enfoncé ; + hauteur du sol à la pose
		buffer[o + 8] = -si
		buffer[o + 10] = c
		buffer[o + 11] = p.y - rect.position.y
		buffer[o + 12] = bush
		buffer[o + 13] = tint
		buffer[o + 14] = phase
		o += FLOATS_PER_INSTANCE
		top = maxf(top, s)
	buffer.resize(o)
	var count := points.size()
	for n in count:  # rang : ordre de semis (aléatoire), coupe du fondu
		buffer[n * FLOATS_PER_INSTANCE + 15] = float(n) / float(maxi(count, 1))
	return {"points": points, "buffer": buffer, "top": top}


## Pose (fil principal) : hauteurs de la surface affichée, `MultiMeshInstance3D`.
func _install_cell(key: Vector2i, seeded: Dictionary) -> void:
	var t0 := Time.get_ticks_usec()
	var rect := _cell_rect(key)
	var points: PackedVector2Array = seeded.get("points", PackedVector2Array())
	var buffer: PackedFloat32Array = seeded.get("buffer", PackedFloat32Array())
	var count := points.size()
	var bottom: float = seeded.get("bottom", 0.0)
	var high: float = seeded.get("high", 0.0)
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = _mesh
	multimesh.instance_count = count
	if count > 0:
		multimesh.buffer = buffer
	multimesh.custom_aabb = AABB(Vector3(-1.0, bottom - 1.0, -1.0), Vector3(cell_size + 2.0, high - bottom + float(seeded.get("top", 0.0)) + 3.0, cell_size + 2.0))
	var mmi := MultiMeshInstance3D.new()
	mmi.name = "Clutter_%d_%d" % [key.x, key.y]
	mmi.multimesh = multimesh
	mmi.material_override = _material
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mmi.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
	mmi.position = Vector3(rect.position.x, 0.0, rect.position.y)
	mmi.visible = false  # rendu visible par `update_view` si la cellule est dans le disque
	add_child(mmi)
	_cells[key] = {"mmi": mmi, "points": points, "buffer": buffer, "last_seen": _frame}
	stats["cells"] = _cells.size()
	stats["build_ms_max"] = maxf(float(stats["build_ms_max"]), (Time.get_ticks_usec() - t0) / 1000.0)


func _excluded(p: Vector2, circles: PackedVector3Array) -> bool:
	for c in circles:
		if p.distance_squared_to(Vector2(c.x, c.y)) < c.z * c.z:
			return true
	return false


## Lot FC6 : source de hauteurs lisible depuis un fil de travail, prise sur le fil principal :
## instantané des pages du quadtree qui touchent `rect` (octets partagés en copie sur écriture),
## sinon {} (échantillonneur de test ou heightmap de la carte, lus directement).
func _height_source(rect: Rect2) -> Dictionary:
	if height_sampler.is_valid() or terrain == null or terrain.quadtree == null or terrain.map_data == null:
		return {}
	return terrain.quadtree.surface_snapshot(rect.grow(1.0), Vector2.ZERO)


## Hauteurs des points depuis `source` (`_height_source`) ; appelable hors du fil principal.
func _heights_from(points: PackedVector2Array, source: Dictionary) -> PackedFloat32Array:
	if source.is_empty() and height_sampler.is_valid():
		return height_sampler.call(points)
	var result := PackedFloat32Array()
	if source.is_empty():  # sans quadtree : heightmap de la carte (lisible hors fil principal)
		result.resize(points.size())
		if map_data != null:
			for n in points.size():
				result[n] = map_data.surface_world_at(points[n].x, points[n].y)
		return result
	result.resize(points.size())
	for n in points.size():
		result[n] = maxf(ReliefQuadtree.sample_snapshot(source, points[n].x, points[n].y), 0.0)
	return result


## Lot FC6 (fil de travail) : pose les touffes semées sur la surface, bornes verticales.
func _ground_seeded(seeded: Dictionary, source: Dictionary) -> Dictionary:
	var points: PackedVector2Array = seeded.get("points", PackedVector2Array())
	var buffer: PackedFloat32Array = seeded.get("buffer", PackedFloat32Array())
	var heights := _heights_from(points, source)
	var bottom := INF
	var high := -INF
	for n in points.size():
		var h := heights[n]
		buffer[n * FLOATS_PER_INSTANCE + 7] += h
		bottom = minf(bottom, h)
		high = maxf(high, h)
	if points.is_empty():
		bottom = 0.0
		high = 0.0
	seeded["buffer"] = buffer
	seeded["bottom"] = bottom
	seeded["high"] = high
	return seeded


## Lot FC6 : recalages (surface affichée changée) dans le `WorkerThreadPool` ; au plus une
## cellule remplacée par image sur le fil principal.
func _update_regrounds() -> void:
	for key: Vector2i in _ground_jobs.keys():
		var holder: Dictionary = _ground_jobs[key]
		if not WorkerThreadPool.is_task_completed(int(holder["task"])):
			continue
		WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
		_ground_jobs.erase(key)
		var entry: Dictionary = _cells.get(key, {})
		if not entry.is_empty() and entry["mmi"] == holder["mmi"]:
			var t0 := Time.get_ticks_usec()
			var buffer: PackedFloat32Array = holder["buffer"]
			entry["buffer"] = buffer
			(entry["mmi"] as MultiMeshInstance3D).multimesh.buffer = buffer
			stats["reground_ms_max"] = maxf(float(stats.get("reground_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)
		if holder.get("stale", false):
			_dirty[key] = true
		break
	var started := 0
	for key: Vector2i in _dirty.keys():
		if started >= max_reground_per_frame:
			break
		if _ground_jobs.has(key):
			continue
		_dirty.erase(key)
		if not _cells.has(key):
			continue
		var entry: Dictionary = _cells[key]
		var points: PackedVector2Array = entry["points"]
		if points.is_empty():
			continue
		if not threaded:
			_reground_cell(key, _height_source(_cell_rect(key)))
			continue
		var holder := {"mmi": entry["mmi"], "buffer": PackedFloat32Array()}
		var source := _height_source(_cell_rect(key))
		var old: PackedFloat32Array = entry["buffer"]
		holder["task"] = WorkerThreadPool.add_task(func() -> void: holder["buffer"] = _regrounded(points, old, source), false, "GroundClutter reground")
		_ground_jobs[key] = holder
		started += 1


## Nouveau tampon (hauteurs seules) depuis la copie processeur ; appelable hors du fil principal.
func _regrounded(points: PackedVector2Array, old: PackedFloat32Array, source: Dictionary) -> PackedFloat32Array:
	var buffer := old.duplicate()
	var heights := _heights_from(points, source)
	for n in points.size():
		var o := n * FLOATS_PER_INSTANCE
		buffer[o + 7] = heights[n] - buffer[o + 5] * 0.08
	return buffer


## Recalage direct (sans fil de travail).
func _reground_cell(key: Vector2i, source: Dictionary = {}) -> void:
	var entry: Dictionary = _cells[key]
	var points: PackedVector2Array = entry["points"]
	if points.is_empty():
		return
	var buffer := _regrounded(points, entry["buffer"], source)
	entry["buffer"] = buffer
	(entry["mmi"] as MultiMeshInstance3D).multimesh.buffer = buffer


func _on_chunk_surface_changed(index: int) -> void:
	if terrain == null or terrain.chunk_px <= 0:
		return
	var origin := Vector2((index % terrain.chunks_x) * terrain.chunk_px, (index / terrain.chunks_x) * terrain.chunk_px)
	_on_surface_rect_changed(Rect2(origin, Vector2(terrain.chunk_px, terrain.chunk_px)))


func _on_surface_rect_changed(rect: Rect2) -> void:
	for key: Vector2i in _cells:
		if _cell_rect(key).intersects(rect, true):
			_dirty[key] = true
	# FC6 : semis ou recalage en cours sur l'ancienne surface → à refaire une fois installé.
	for jobs: Dictionary in [_jobs, _ground_jobs]:
		for key: Vector2i in jobs:
			if _cell_rect(key).intersects(rect, true):
				(jobs[key] as Dictionary)["stale"] = true


## Instances affichées par cellule : part `quality_density / max_density` des candidats, bornée
## par `max_visible_instances` sur l'ensemble des cellules visibles.
func _apply_counts() -> void:
	var share := clampf(quality_density / maxf(max_density, 0.001), 0.0, 1.0)
	var total := 0
	var shown_cells := 0
	for entry: Dictionary in _cells.values():
		var mmi: MultiMeshInstance3D = entry["mmi"]
		if mmi.visible and visible:
			total += int(round(mmi.multimesh.instance_count * share))
			shown_cells += 1
	if total > max_visible_instances:
		share *= float(max_visible_instances) / float(total)
	var visible_total := 0
	for entry: Dictionary in _cells.values():
		var mmi: MultiMeshInstance3D = entry["mmi"]
		var n := int(floor(mmi.multimesh.instance_count * share))
		mmi.multimesh.visible_instance_count = n
		if mmi.visible and visible:
			visible_total += n
	stats["visible"] = visible_total
	stats["visible_cells"] = shown_cells
	stats["instances"] = _count_instances()


func _count_instances() -> int:
	var total := 0
	for entry: Dictionary in _cells.values():
		total += (entry["mmi"] as MultiMeshInstance3D).multimesh.instance_count
	return total


func _evict() -> void:
	if _cells.size() <= max_cached_cells:
		return
	var hidden: Array = []
	for key: Vector2i in _cells:
		if not (_cells[key]["mmi"] as MultiMeshInstance3D).visible:
			hidden.append(key)
	hidden.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return int(_cells[a]["last_seen"]) < int(_cells[b]["last_seen"]))
	var excess := _cells.size() - max_cached_cells
	for n in mini(excess, hidden.size()):
		var key: Vector2i = hidden[n]
		(_cells[key]["mmi"] as Node).queue_free()
		_cells.erase(key)
		_dirty.erase(key)
	stats["cells"] = _cells.size()
