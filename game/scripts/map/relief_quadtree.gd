class_name ReliefQuadtree
extends Node3D

## Quadtree de relief streamé (chantier ZG, lot ZG2, ADR 0036). Quand la pyramide de relief est
## en cache, il dessine tout le terrain de campagne à la place des morceaux E0 de
## `TerrainBuilder` (qui restent le repli sans cache, avec le relief fin `FineTerrainJob`).
##
## - Arbre : racine = toute la carte (4096 unités), profondeur n, nœud (n, col, row) de côté
##   `4096 / 2^n` aligné sur la grille des tuiles de la pyramide (étage L = n − 4 : un nœud de
##   profondeur 4 est une tuile E0 de 256 unités). Profondeur maximale : étage de données le plus
##   fin sous le nœud + `extra_depth` (au-delà, les sommets suréchantillonneraient la page).
## - Sélection CDLOD par la distance, équivalente à l'erreur à l'écran : un nœud de profondeur n
##   (espacement des sommets s = côté / 64) est accepté quand sa projection s × K / d ≤
##   `max_vertex_px` (K = hauteur de la vue / (2 tan(fov / 2))). Les pixels de la page du nœud
##   (8 par sommet) font donc ≤ `max_vertex_px` / 8 pixel écran. Budget `max_items` : au-delà, le
##   seuil s'élargit (hystérésis ×1,2 / ÷1,1).
## - Géométrie : un patch fixe partagé (64 × 64 quads + jupe, demi-patch 32 × 32 pour un quadrant
##   dessiné au niveau du parent), déplacé dans le vertex shader (`relief_quadtree.gdshaderinc`)
##   depuis des pages de hauteurs ; morphing géomorphe sur la distance vers la grille du parent
##   (aucune fissure ni saut de niveau), jupes contre les écarts de pages voisines.
## - Pages : `Texture2DArray` R16 (entier 16 bits normalisé, pas de demi-flottant) + mipmaps,
##   `max_pages` couches de 512² (0,67 Mo chacune : 256 couches = 171 Mo de VRAM). Paramètres de
##   nœud et table des pages en `instance uniform` (couche, emprise, couches des 8 voisines).
##   Décodage PNG dans `WorkerThreadPool` (Rust `GameDataStore.load_heightmap_u16`, repli `Png16`),
##   téléversement ≤ `max_uploads_per_frame` par image, LRU, fondu d'arrivée `fade_seconds`.
## - Processeur : les octets des pages chargées sont gardés ; `surface_height_at` rend la surface
##   bilinéaire de la page chargée la plus fine (indépendante de la vue), `surface_changed(rect)`
##   signale l'arrivée ou l'éviction d'une page.

signal surface_changed(rect: Rect2)

const PATCH_QUADS := 64
const ROOT_UNITS := 4096.0
## Profondeur n d'un nœud de la taille d'une tuile E0 (étage L = n − DEPTH_E0).
const DEPTH_E0 := 4
const PAGE_PX := ReliefPyramid.TILE_PX
const SIDES: Array[Vector2i] = [Vector2i(-1, 0), Vector2i(1, 0), Vector2i(0, -1), Vector2i(0, 1)]
const DIAGONALS: Array[Vector2i] = [Vector2i(-1, -1), Vector2i(1, -1), Vector2i(-1, 1), Vector2i(1, 1)]

## Espacement maximal des sommets à l'écran (pixels).
@export var max_vertex_px: float = 4.0
@export var max_items: int = 700
## Couches de pages (VRAM : 0,67 Mo chacune, mipmaps compris). Plafond ADR : 256 Mo.
@export var max_pages: int = 256
@export var max_jobs: int = 4
@export var max_uploads_per_frame: int = 2
@export var fade_seconds: float = 0.35
## Profondeur au-delà de l'étage de données le plus fin (3 = un sommet par pixel de page).
@export var extra_depth: int = 3
@export var max_depth: int = 14
## Début du morphing, en fraction de la portée du niveau.
@export var morph_ratio: float = 0.7
## Profondeur des jupes, en espacements de sommets (bornée à `skirt_max`).
@export var skirt_factor: float = 1.5
@export var skirt_max: float = 4.0

var pyramid: ReliefPyramid
var material: ShaderMaterial
var map_data: MapData
var stats: Dictionary = {}

## Pages chargées : clé de tuile → {"layer", "last_used", "t_upload"} ; octets à part (lus par les
## fils de travail via les instantanés).
var _pages: Dictionary = {}
var _page_bytes: Dictionary = {}
var _layer_keys: PackedInt64Array = PackedInt64Array()
var _free_layers: Array[int] = []
var _page_array: Texture2DArray
var _jobs: Dictionary = {}
var _store_available := false
## Demandes de l'image courante : clé → priorité (étage × 1e6 + distance).
var _wanted: Dictionary = {}
var _items: Array[Dictionary] = []
var _slots: Dictionary = {}
var _pool: Array[MeshInstance3D] = []
var _patch_full: ArrayMesh
var _patch_half: ArrayMesh
var _frame: int = 0
var _residency_version: int = 0
var _range_version: int = 0
var _bounds: PackedVector2Array = PackedVector2Array()
var _cam: Vector3 = Vector3.ZERO
var _planes: Array[Plane] = []
var _k_proj: float = 1.0
var _px_scale: float = 1.0
var _ranges: PackedFloat32Array = PackedFloat32Array()
var _page_cache: Dictionary = {}
var _missing_wanted: int = 0
var _decode_ms: PackedFloat32Array = PackedFloat32Array()
var _upload_ms_max: float = 0.0
var _select_ms: float = 0.0


## `pyramid` doit être disponible ; `terrain_material` est le matériau partagé des morceaux E0
## (mêmes splat, forêts, frontières, brouillard, surbrillance) ; `chunk_bounds` : 16 × 16 bornes
## (min, max) des hauteurs en mètres par morceau E0, pour les boîtes englobantes.
func setup(relief: ReliefPyramid, terrain_material: ShaderMaterial, data: MapData, chunk_bounds: PackedVector2Array) -> void:
	pyramid = relief
	material = terrain_material
	map_data = data
	_bounds = chunk_bounds
	_store_available = ClassDB.class_exists("GameDataStore")
	wait_jobs(false)
	_pages.clear()
	_page_bytes.clear()
	for slot: MeshInstance3D in _slots.values():
		slot.queue_free()
	_slots.clear()
	_patch_full = _build_patch(PATCH_QUADS)
	_patch_half = _build_patch(PATCH_QUADS / 2)
	var blank := Image.create(PAGE_PX, PAGE_PX, true, Image.FORMAT_R16)
	var layers: Array[Image] = []
	for i in max_pages:
		layers.append(blank)
	_page_array = Texture2DArray.new()
	_page_array.create_from_images(layers)
	_layer_keys.resize(max_pages)
	_layer_keys.fill(-1)
	_free_layers.clear()
	for i in range(max_pages - 1, -1, -1):
		_free_layers.append(i)
	material.set_shader_parameter("qt_pages", _page_array)
	material.set_shader_parameter("qt_page_h_min", pyramid.height_min_m)
	material.set_shader_parameter("qt_page_h_range", pyramid.height_range_m)


func _exit_tree() -> void:
	wait_jobs(false)


## Sélection des nœuds, demandes de pages, téléversements et paramètres d'instance pour `camera`.
func update_view(camera: Camera3D) -> void:
	if pyramid == null or camera == null:
		return
	_frame += 1
	var t0 := Time.get_ticks_usec()
	_collect_jobs()
	_prepare_camera(camera)
	_items.clear()
	_wanted.clear()
	_page_cache.clear()
	_missing_wanted = 0
	_select(0, 0, 0)
	if _items.size() > max_items:
		_px_scale = minf(_px_scale * 1.2, 8.0)
	elif _items.size() < max_items * 0.6 and _px_scale > 1.0:
		_px_scale = maxf(_px_scale / 1.1, 1.0)
	_apply_items()
	_start_jobs()
	_select_ms = (Time.get_ticks_usec() - t0) / 1000.0
	material.set_shader_parameter("qt_camera", _cam)


# --- Sélection -------------------------------------------------------------------------


func _prepare_camera(camera: Camera3D) -> void:
	_cam = camera.global_position
	_planes = camera.get_frustum()
	var viewport_h := 720.0
	var viewport := camera.get_viewport()
	if viewport != null:
		viewport_h = maxf(viewport.get_visible_rect().size.y, 64.0)
	var k := viewport_h * 0.5 / tan(deg_to_rad(camera.fov) * 0.5)
	var threshold := max_vertex_px * _px_scale
	if absf(k / threshold - _k_proj) > 0.001 * _k_proj or _ranges.is_empty():
		_k_proj = k / threshold
		_ranges.resize(max_depth + 2)
		for n in max_depth + 2:
			var size := ROOT_UNITS / float(1 << n)
			# Portée du niveau n = distance d'acceptation du parent : 2 × s_n × K / seuil, au moins
			# 3 côtés (le morphing doit s'achever avant le voisin plus grossier).
			_ranges[n] = maxf(2.0 * size / PATCH_QUADS * _k_proj, 3.0 * size)
		_range_version += 1


func node_size(n: int) -> float:
	return ROOT_UNITS / float(1 << n)


func _select(n: int, c: int, r: int) -> bool:
	var size := node_size(n)
	var ox := c * size + ReliefPyramid.GRID_OFFSET
	var oz := r * size + ReliefPyramid.GRID_OFFSET
	var yb := _y_bounds(n, c, r)
	var bmin := Vector3(ox, yb.x, oz)
	var bmax := Vector3(ox + size, yb.y, oz + size)
	if n > 0 and not _box_in_sphere(bmin, bmax, _ranges[n]):
		return false
	if not _box_in_frustum(bmin, bmax):
		return true
	if n >= _depth_cap(n, c, r) or not _box_in_sphere(bmin, bmax, _ranges[n + 1]):
		_add_item(n, c, r, 4, bmin, bmax)
		return true
	for q in 4:
		if not _select(n + 1, c * 2 + (q & 1), r * 2 + (q >> 1)):
			var half := size * 0.5
			var qmin := Vector3(ox + (q & 1) * half, yb.x, oz + (q >> 1) * half)
			_add_item(n, c, r, q, qmin, qmin + Vector3(half, yb.y - yb.x, half))
	return true


func _depth_cap(n: int, c: int, r: int) -> int:
	if n < DEPTH_E0:
		return max_depth
	var level := n - DEPTH_E0
	var data := pyramid.max_level_under(level, c, r)
	if data < 0:
		data = pyramid.finest_ancestor(level, c, r)
	return mini(DEPTH_E0 + data + extra_depth, max_depth)


## Bornes (min, max) des hauteurs monde du nœud, depuis les morceaux E0 (mètres, marges
## comprises) et le facteur vertical courant.
func _y_bounds(n: int, c: int, r: int) -> Vector2:
	var vs := MapData.vertical_scale()
	if _bounds.size() < 256:
		return Vector2(-400.0, 5000.0) * vs
	if n >= DEPTH_E0:
		var shift := n - DEPTH_E0
		return _bounds[clampi(r >> shift, 0, 15) * 16 + clampi(c >> shift, 0, 15)] * vs
	var span := 1 << (DEPTH_E0 - n)
	var result := Vector2(INF, -INF)
	for j in span:
		for i in span:
			var b := _bounds[(r * span + j) * 16 + c * span + i]
			result = Vector2(minf(result.x, b.x), maxf(result.y, b.y))
	return result * vs


func _box_in_sphere(bmin: Vector3, bmax: Vector3, radius: float) -> bool:
	var closest := _cam.clamp(bmin, bmax)
	return closest.distance_squared_to(_cam) <= radius * radius


func _box_in_frustum(bmin: Vector3, bmax: Vector3) -> bool:
	for plane in _planes:
		var nrm := plane.normal
		var p := Vector3(bmin.x if nrm.x > 0.0 else bmax.x, bmin.y if nrm.y > 0.0 else bmax.y, bmin.z if nrm.z > 0.0 else bmax.z)
		if plane.distance_to(p) > 0.0:
			return false
	return true


func _add_item(n: int, c: int, r: int, quadrant: int, bmin: Vector3, bmax: Vector3) -> void:
	var center := (bmin + bmax) * 0.5
	var dist := _cam.distance_to(_cam.clamp(bmin, bmax))
	var fine := _page_of(n, c, r, dist)
	var coarse := fine if n == 0 else _page_of(n - 1, c >> 1, r >> 1, dist)
	_items.append({
		"key": (n << 40) | (r << 20) | (c << 3) | quadrant,
		"n": n, "origin": Vector2(bmin.x, bmin.z), "quadrant": quadrant,
		"fine": fine, "coarse": coarse, "ymin": bmin.y, "ymax": bmax.y, "center": center,
	})


## Page (clé de tuile, -1 = heightmap) utilisée pour le nœud (n, c, r) : la tuile de son étage
## (ou de l'ancêtre le plus fin qui existe), demandée si absente ; en attendant, l'ancêtre chargé
## le plus fin.
func _page_of(n: int, c: int, r: int, dist: float) -> int:
	if n < DEPTH_E0:
		return -1
	var level := n - DEPTH_E0
	var cache_key := (n << 40) | (r << 20) | c
	if _page_cache.has(cache_key):
		return _page_cache[cache_key]
	var result := -1
	var target := pyramid.finest_ancestor(level, c, r)
	if target >= 0:
		var shift := level - target
		var want := ReliefPyramid.key_of(target, c >> shift, r >> shift)
		if _pages.has(want):
			result = want
		else:
			_missing_wanted += 1
			var priority := target * 1.0e6 + dist
			if priority < float(_wanted.get(want, INF)):
				_wanted[want] = priority
			for up in range(target - 1, -1, -1):
				var s := level - up
				var key := ReliefPyramid.key_of(up, c >> s, r >> s)
				if _pages.has(key):
					result = key
					break
	_page_cache[cache_key] = result
	return result


# --- Instances -------------------------------------------------------------------------


func _apply_items() -> void:
	var now := Time.get_ticks_msec() / 1000.0
	var alive := {}
	for item in _items:
		var key: int = item["key"]
		alive[key] = true
		var slot: MeshInstance3D = _slots.get(key)
		var fresh := slot == null
		if fresh:
			slot = _take_slot()
			_slots[key] = slot
			var n: int = item["n"]
			var quadrant: int = item["quadrant"]
			var s := node_size(n) / PATCH_QUADS
			var origin: Vector2 = item["origin"]
			slot.mesh = _patch_full if quadrant == 4 else _patch_half
			slot.transform = Transform3D(Basis.from_scale(Vector3(s, 1.0, s)), Vector3(origin.x, 0.0, origin.y))
			var quads := PATCH_QUADS if quadrant == 4 else PATCH_QUADS / 2
			var skirt := minf(s * skirt_factor, skirt_max) + 0.02
			slot.custom_aabb = AABB(Vector3(0.0, float(item["ymin"]) - skirt, 0.0), Vector3(quads, float(item["ymax"]) - float(item["ymin"]) + skirt, quads))
			slot.set_instance_shader_parameter("qt_node", Vector4(origin.x, origin.y, s, n))
			slot.visible = true
		var fine: int = item["fine"]
		var coarse: int = item["coarse"]
		var fade := 1.0
		if fine >= 0:
			var page: Dictionary = _pages[fine]
			page["last_used"] = _frame
			fade = clampf((now - float(page["t_upload"])) / maxf(fade_seconds, 0.001), 0.0, 1.0)
		if coarse >= 0:
			_pages[coarse]["last_used"] = _frame
		var sig := Vector4i(fine, coarse, _residency_version, _range_version)
		if fresh or slot.get_meta("sig", Vector4i.ZERO) != sig or fade < 1.0 or slot.get_meta("fading", false):
			slot.set_meta("sig", sig)
			slot.set_meta("fading", fade < 1.0)
			_set_page_params(slot, item, fine, coarse, fade)
	for key: int in _slots.keys():
		if not alive.has(key):
			var slot: MeshInstance3D = _slots[key]
			_slots.erase(key)
			slot.visible = false
			_pool.append(slot)
	stats["items"] = _items.size()


func _set_page_params(slot: MeshInstance3D, item: Dictionary, fine: int, coarse: int, fade: float) -> void:
	var n: int = item["n"]
	var s := node_size(n) / PATCH_QUADS
	slot.set_instance_shader_parameter("qt_fine", _page_vec(fine))
	slot.set_instance_shader_parameter("qt_coarse", _page_vec(coarse))
	slot.set_instance_shader_parameter("qt_fine_nbr", _neighbors(fine, false))
	slot.set_instance_shader_parameter("qt_fine_diag", _neighbors(fine, true))
	slot.set_instance_shader_parameter("qt_coarse_nbr", _neighbors(coarse, false))
	slot.set_instance_shader_parameter("qt_coarse_diag", _neighbors(coarse, true))
	var morph := Vector4(1.0e9, 1.0, fade, minf(s * skirt_factor, skirt_max) + 0.02)
	if n > 0:
		var reach: float = _ranges[n]
		morph.x = morph_ratio * reach
		morph.y = 1.0 / maxf((1.0 - morph_ratio) * reach, 1e-4)
	slot.set_instance_shader_parameter("qt_morph", morph)


func _page_vec(key: int) -> Vector4:
	if key < 0 or not _pages.has(key):
		return Vector4(-1.0, 0.0, 0.0, 1.0)
	var level := ReliefPyramid.level_of_key(key)
	var origin := ReliefPyramid.tile_origin(level, ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
	return Vector4(float(_pages[key]["layer"]), origin.x, origin.y, ReliefPyramid.tile_units(level))


## Couches des voisines de même étage : (O, E, N, S) ou (NO, NE, SO, SE) ; -1 si non chargée.
func _neighbors(key: int, diagonal: bool) -> Vector4:
	if key < 0:
		return Vector4(-1.0, -1.0, -1.0, -1.0)
	var level := ReliefPyramid.level_of_key(key)
	var c := ReliefPyramid.col_of_key(key)
	var r := ReliefPyramid.row_of_key(key)
	var offsets: Array[Vector2i] = DIAGONALS if diagonal else SIDES
	var result := Vector4()
	var side := ReliefPyramid.tiles_per_side(level)
	for i in 4:
		var nc := c + offsets[i].x
		var nr := r + offsets[i].y
		var layer := -1.0
		if nc >= 0 and nr >= 0 and nc < side and nr < side:
			var page: Dictionary = _pages.get(ReliefPyramid.key_of(level, nc, nr), {})
			if not page.is_empty():
				layer = float(page["layer"])
				page["last_used"] = _frame
		result[i] = layer
	return result


func _take_slot() -> MeshInstance3D:
	if not _pool.is_empty():
		return _pool.pop_back()
	var slot := MeshInstance3D.new()
	slot.material_override = material
	add_child(slot)
	return slot


## Patch de `quads` × `quads` quads en coordonnées de grille entières (y = 0), plus une jupe
## (même disposition et mêmes index que `FineTerrainJob`, y = -1 marque les sommets de jupe).
static func _build_patch(quads: int) -> ArrayMesh:
	var side := quads + 1
	var vertices := PackedVector3Array()
	vertices.resize(side * side + 4 * side)
	var k := 0
	for j in side:
		for i in side:
			vertices[k] = Vector3(i, 0.0, j)
			k += 1
	var base := side * side
	for i in side:
		vertices[base + i] = Vector3(i, -1.0, 0.0)
		vertices[base + side + i] = Vector3(i, -1.0, quads)
		vertices[base + 2 * side + i] = Vector3(0.0, -1.0, i)
		vertices[base + 3 * side + i] = Vector3(quads, -1.0, i)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = FineTerrainJob.build_indices(side)
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


# --- Pages : décodage, téléversement, LRU ----------------------------------------------


func _start_jobs() -> void:
	if _jobs.size() >= max_jobs or _wanted.is_empty():
		return
	var order: Array = []
	for key: int in _wanted:
		if not _jobs.has(key):
			order.append([float(_wanted[key]), key])
	order.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for entry in order:
		if _jobs.size() >= max_jobs:
			break
		var key: int = entry[1]
		var job := PageJob.new()
		job.key = key
		job.path = pyramid.tile_path(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
		if _store_available:
			job.store = ClassDB.instantiate("GameDataStore")
		var task := WorkerThreadPool.add_task(job.run, false, "relief page %d" % key)
		_jobs[key] = {"task": task, "job": job}


## Récupère les décodages terminés et en téléverse au plus `max_uploads_per_frame` (tous avec
## `block`).
func _collect_jobs(block: bool = false) -> void:
	var uploads := 0
	for key: int in _jobs.keys():
		var entry: Dictionary = _jobs[key]
		if not block and (uploads >= max_uploads_per_frame or not WorkerThreadPool.is_task_completed(entry["task"])):
			continue
		WorkerThreadPool.wait_for_task_completion(entry["task"])
		_jobs.erase(key)
		var job: PageJob = entry["job"]
		if not job.ok:
			pyramid.mark_broken(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
			push_warning("ReliefQuadtree: unreadable tile %s" % job.path)
			continue
		_decode_ms.append(job.decode_ms)
		if _upload(key, job):
			uploads += 1


func _upload(key: int, job: PageJob) -> bool:
	if _pages.has(key):
		return false
	var layer := _alloc_layer()
	if layer < 0:
		return false
	var t0 := Time.get_ticks_usec()
	_page_array.update_layer(job.image, layer)
	_upload_ms_max = maxf(_upload_ms_max, (Time.get_ticks_usec() - t0) / 1000.0)
	_pages[key] = {"layer": layer, "last_used": _frame, "t_upload": Time.get_ticks_msec() / 1000.0}
	_page_bytes[key] = job.bytes
	_layer_keys[layer] = key
	_residency_version += 1
	surface_changed.emit(_tile_rect(key))
	return true


func _alloc_layer() -> int:
	if not _free_layers.is_empty():
		return _free_layers.pop_back()
	var oldest := -1
	var oldest_frame := _frame - 1
	for key: int in _pages:
		var used: int = _pages[key]["last_used"]
		if used < oldest_frame:
			oldest_frame = used
			oldest = key
	if oldest < 0:
		return -1
	var layer: int = _pages[oldest]["layer"]
	_pages.erase(oldest)
	_page_bytes.erase(oldest)
	_residency_version += 1
	surface_changed.emit(_tile_rect(oldest))
	return layer


static func _tile_rect(key: int) -> Rect2:
	var level := ReliefPyramid.level_of_key(key)
	var origin := ReliefPyramid.tile_origin(level, ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
	var t := ReliefPyramid.tile_units(level)
	return Rect2(origin, Vector2(t, t))


## Attend les décodages en cours ; avec `upload`, les téléverse tous (captures, tests).
func wait_jobs(upload: bool = true) -> void:
	if upload:
		var saved := max_uploads_per_frame
		max_uploads_per_frame = 1 << 20
		_collect_jobs(true)
		max_uploads_per_frame = saved
		return
	for entry: Dictionary in _jobs.values():
		WorkerThreadPool.wait_for_task_completion(entry["task"])
	_jobs.clear()


## Vrai quand toutes les pages voulues à la dernière sélection sont chargées et aucun décodage
## n'est en cours (captures, mesures).
func is_settled() -> bool:
	return _jobs.is_empty() and _missing_wanted == 0


func page_count() -> int:
	return _pages.size()


func pending_jobs() -> int:
	return _jobs.size()


func item_count() -> int:
	return _items.size()


## Mesures : nœuds, pages, VRAM des pages, décodage moyen / max, téléversement max, sélection.
func perf_stats() -> Dictionary:
	var total := 0.0
	var worst := 0.0
	for ms in _decode_ms:
		total += ms
		worst = maxf(worst, ms)
	return {
		"items": _items.size(),
		"pages": _pages.size(),
		"page_layers": max_pages,
		"page_vram_mb": snappedf(max_pages * PAGE_PX * PAGE_PX * 2 * 4.0 / 3.0 / 1048576.0, 0.1),
		"decoded": _decode_ms.size(),
		"decode_ms_avg": snappedf(total / maxf(_decode_ms.size(), 1.0), 0.01),
		"decode_ms_max": snappedf(worst, 0.01),
		"upload_ms_max": snappedf(_upload_ms_max, 0.01),
		"select_ms": snappedf(_select_ms, 0.01),
		"px_scale": snappedf(_px_scale, 0.01),
	}


# --- Surface côté processeur -----------------------------------------------------------


## Hauteur monde de la surface la plus fine chargée en (x, y) carte (bilinéaire dans la page),
## NAN si aucune page de la pyramide ne couvre le point.
func surface_height_at(x: float, y: float) -> float:
	if pyramid == null:
		return NAN
	return sample_pages(_page_bytes, pyramid.max_level, pyramid.height_min_m, pyramid.height_range_m, x, y)


## Instantané des pages chargées qui touchent `rect`, lisible depuis un fil de travail sans
## verrou (octets partagés en copie sur écriture) : grille pour `TerrainBuilder.grid_height`
## (coordonnées locales à `origin`), repli sur la heightmap 4096 hors pages.
func surface_snapshot(rect: Rect2, origin: Vector2) -> Dictionary:
	var pages := {}
	for key: int in _page_bytes:
		if _tile_rect(key).intersects(rect):
			pages[key] = _page_bytes[key]
	return {
		"qt_pages": pages, "max_level": pyramid.max_level, "h_min": pyramid.height_min_m,
		"h_range": pyramid.height_range_m, "origin": origin, "map": map_data,
	}


## Hauteur monde dans un instantané (`surface_snapshot`), coordonnées locales à son origine.
static func sample_snapshot(grid: Dictionary, lx: float, ly: float) -> float:
	var origin: Vector2 = grid["origin"]
	var x := origin.x + lx
	var y := origin.y + ly
	var h := sample_pages(grid["qt_pages"], grid["max_level"], grid["h_min"], grid["h_range"], x, y)
	if is_nan(h):
		var data: MapData = grid["map"]
		return data.height_world_at(x, y) if data != null else 0.0
	return h


## Échantillonnage bilinéaire (octets little-endian 16 bits) dans la page la plus fine de `pages`
## (clé de tuile → octets) couvrant (x, y) ; NAN si aucune.
static func sample_pages(pages: Dictionary, top_level: int, h_min: float, h_range: float, x: float, y: float) -> float:
	if pages.is_empty():
		return NAN
	for level in range(top_level, -1, -1):
		var t := ReliefPyramid.tile_at(level, x, y)
		if t.x < 0 or t.y < 0:
			return NAN
		var key := ReliefPyramid.key_of(level, t.x, t.y)
		if not pages.has(key):
			continue
		var bytes: PackedByteArray = pages[key]
		var origin := ReliefPyramid.tile_origin(level, t.x, t.y)
		var px_units := ReliefPyramid.pixel_units(level)
		var fx := clampf((x - origin.x) / px_units - 0.5, 0.0, PAGE_PX - 1.0)
		var fy := clampf((y - origin.y) / px_units - 0.5, 0.0, PAGE_PX - 1.0)
		var i := mini(int(fx), PAGE_PX - 2)
		var j := mini(int(fy), PAGE_PX - 2)
		var tx := fx - i
		var ty := fy - j
		var o := (j * PAGE_PX + i) * 2
		var a := bytes.decode_u16(o)
		var b := bytes.decode_u16(o + 2)
		var c := bytes.decode_u16(o + PAGE_PX * 2)
		var d := bytes.decode_u16(o + PAGE_PX * 2 + 2)
		var v := lerpf(lerpf(a, b, tx), lerpf(c, d, tx), ty) / 65535.0
		return (h_min + v * h_range) * MapData.vertical_scale()
	return NAN


## Décodage d'une tuile hors fil principal : octets little-endian (Rust, sinon `Png16` puis
## inversion des octets) et image R16 avec mipmaps prête à téléverser.
class PageJob:
	extends RefCounted

	var key: int = 0
	var path: String = ""
	var store: Object = null
	var bytes: PackedByteArray = PackedByteArray()
	var image: Image
	var decode_ms: float = 0.0
	var ok: bool = false

	func run() -> void:
		var t0 := Time.get_ticks_usec()
		var expected := PAGE_PX * PAGE_PX * 2
		if not FileAccess.file_exists(path):
			return
		if store != null and store.has_method("load_heightmap_u16"):
			bytes = store.call("load_heightmap_u16", path)
		if bytes.size() != expected:
			bytes = PackedByteArray()
			var decoded := Png16.load_gray16(path)
			if decoded.is_empty() or int(decoded["width"]) != PAGE_PX or int(decoded["height"]) != PAGE_PX:
				return
			var big: PackedByteArray = decoded["data"]
			bytes.resize(expected)
			for o in range(0, expected, 2):
				bytes[o] = big[o + 1]
				bytes[o + 1] = big[o]
		image = Image.create_from_data(PAGE_PX, PAGE_PX, false, Image.FORMAT_R16, bytes)
		image.generate_mipmaps()
		decode_ms = (Time.get_ticks_usec() - t0) / 1000.0
		ok = true
