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
## - PB3g (ADR 0092) : sélection, résidence (LRU) et paramètres d'instance calculés par la classe
##   native `ReliefLod` (crate `relief-lod`) quand l'extension l'expose ; GDScript ne fait que
##   créer, déplacer et masquer les nœuds signalés. Repli GDScript complet sans l'extension ou
##   avec `--no-native-quadtree` (mêmes nœuds sélectionnés, `pb3g_quadtree_test.gd`).
## - Processeur : les octets des pages chargées sont gardés ; `surface_height_at` rend la surface
##   bilinéaire de la page chargée la plus fine (indépendante de la vue), `surface_changed(rect)`
##   signale l'arrivée ou l'éviction d'une page.

signal surface_changed(rect: Rect2)

const PATCH_QUADS := 64
const ROOT_UNITS := 4096.0
## Profondeur n d'un nœud de la taille d'une tuile E0 (étage L = n − DEPTH_E0).
const DEPTH_E0 := 4
const PAGE_PX := ReliefPyramid.TILE_PX
const ROOT_TILE_UNITS := ReliefPyramid.ROOT_TILE_UNITS
const PARAM_NAMES: Array[String] = ["qt_fine", "qt_coarse", "qt_fine_nbr", "qt_fine_diag", "qt_coarse_nbr", "qt_coarse_diag", "qt_morph"]
const SIDES: Array[Vector2i] = [Vector2i(-1, 0), Vector2i(1, 0), Vector2i(0, -1), Vector2i(0, 1)]
const DIAGONALS: Array[Vector2i] = [Vector2i(-1, -1), Vector2i(1, -1), Vector2i(-1, 1), Vector2i(1, 1)]

## PB3g : sélection native (`ReliefLod`) si l'extension l'expose.
@export var use_native_select: bool = true
## Espacement maximal des sommets à l'écran (pixels).
@export var max_vertex_px: float = 4.0
@export var max_items: int = 700
## Couches de pages (VRAM : 0,67 Mo chacune, mipmaps compris). Plafond ADR : 256 Mo.
@export var max_pages: int = 256
@export var max_jobs: int = 4
@export var max_uploads_per_frame: int = 2
## Décodage Rust sur le fil principal (≈ 3 ms par tuile) : au plus N tuiles et ce budget par image.
@export var use_rust_decoder: bool = true
@export var max_main_decodes_per_frame: int = 2
@export var main_decode_budget_ms: float = 4.0
@export var fade_seconds: float = 0.35
## Profondeur au-delà de l'étage de données le plus fin (3 = un sommet par pixel de page).
@export var extra_depth: int = 3
@export var max_depth: int = 14
## Début du morphing, en fraction de la portée du niveau.
@export var morph_ratio: float = 0.7
## Profondeur des jupes, en espacements de sommets (bornée à `skirt_max`).
@export var skirt_factor: float = 1.5
@export var skirt_max: float = 4.0
## PF1 : les patchs dont le point le plus proche est au-delà de cette distance de la caméra ne
## portent plus d'ombre (réglée par `TerrainBuilder` selon le préréglage : bord d'une cascade du
## soleil). Ils en reçoivent toujours. INF : tous portent une ombre.
var shadow_cast_distance: float = INF

var pyramid: ReliefPyramid
## Lot ZG5b : objet optionnel dont `carve_job(key) -> Object` (sur le fil principal) rend une
## tâche `apply(bytes) -> PackedByteArray` exécutée dans un fil avant le téléversement de la page
## (lit des fleuves fins, `FineBedCarver`), ou null.
var page_filter: Object = null
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
## Décodeur Rust (`GameDataStore`) du fil principal : godot-rust interdit tout appel depuis un
## autre fil (liaison mono-fil), le décodage Rust se fait donc ici, borné par image.
var _main_store: Object = null
var _main_queue: Array[int] = []
## Décodeur Rust asynchrone (`ReliefDecoder`, fils natifs) quand l'extension l'expose : préféré à
## tout le reste ; clés demandées et pas encore rendues.
var _decoder: Object = null
var _requested: Dictionary = {}
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
## Par morceau E0 (16 × 16, coordonnées carte) : étage le plus fin des pages chargées qui le
## touchent (-1 : aucune) ; `surface_height_at` y commence sa recherche.
var _chunk_top: PackedInt32Array = PackedInt32Array()
## Cache de la dernière page lue par `surface_height_at`.
var _hit_level: int = -1
var _hit_version: int = -1
var _hit_origin: Vector2 = Vector2.ZERO
var _hit_units: float = 0.0
var _hit_px: float = 1.0
var _hit_bytes: PackedByteArray = PackedByteArray()
var _param_cache: Dictionary = {}
## PB3g : sélection native (`ReliefLod`), pages voulues triées par priorité, nombre de nœuds,
## `px_scale` utilisé par la dernière sélection native (comparaison des sélections), facteur de
## projection de la dernière caméra.
var _native: Object = null
var _wanted_order: PackedInt64Array = PackedInt64Array()
var _item_count: int = 0
var _native_px_used: float = 1.0
var _last_k: float = 1.0
var _param_cache_version: int = -1
var _missing_wanted: int = 0
var _decode_ms: PackedFloat32Array = PackedFloat32Array()
var _upload_ms_max: float = 0.0
var _select_ms: float = 0.0
var _update_ms_total: float = 0.0
## ZG7a : pires durées (ms) des étapes de `update_view` (collecte et téléversement des pages,
## écouteurs de `surface_changed` compris ; sélection ; application des nœuds ; demandes).
var _step_ms_max: Dictionary = {"collect": 0.0, "poll": 0.0, "carve_job": 0.0, "layer": 0.0, "emit": 0.0, "select": 0.0, "apply": 0.0, "start": 0.0}


## `pyramid` doit être disponible ; `terrain_material` est le matériau partagé des morceaux E0
## (mêmes splat, forêts, frontières, brouillard, surbrillance) ; `chunk_bounds` : 16 × 16 bornes
## (min, max) des hauteurs en mètres par morceau E0, pour les boîtes englobantes.
func setup(relief: ReliefPyramid, terrain_material: ShaderMaterial, data: MapData, chunk_bounds: PackedVector2Array) -> void:
	pyramid = relief
	material = terrain_material
	map_data = data
	_bounds = chunk_bounds
	_decoder = null
	_main_store = null
	_requested.clear()
	if use_rust_decoder and ClassDB.class_exists("ReliefDecoder"):
		_decoder = ClassDB.instantiate("ReliefDecoder")
		_decoder.call("start", max_jobs)
	elif use_rust_decoder and ClassDB.class_exists("GameDataStore"):
		_main_store = ClassDB.instantiate("GameDataStore")
	_main_queue.clear()
	_chunk_top.resize(256)
	_chunk_top.fill(-1)
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
	_setup_native()
	material.set_shader_parameter("qt_pages", _page_array)
	material.set_shader_parameter("qt_page_h_min", pyramid.height_min_m)
	material.set_shader_parameter("qt_page_h_range", pyramid.height_range_m)


## PB3g : `ReliefLod` avec les tuiles de la pyramide et les bornes des morceaux.
func _setup_native() -> void:
	_native = null
	_wanted_order = PackedInt64Array()
	if not use_native_select or not ClassDB.class_exists("ReliefLod") or "--no-native-quadtree" in OS.get_cmdline_user_args():
		return
	_native = ClassDB.instantiate("ReliefLod")
	var tiles: Array = []
	for level in pyramid.max_level + 1:
		tiles.append(pyramid.tile_indices(level))
	_native.call("set_pyramid", pyramid.max_level, tiles)
	for key: int in pyramid.broken_keys():
		_native.call("mark_broken", key)
	_native.call("set_bounds", _bounds)


## Vrai si la sélection passe par `ReliefLod`.
func is_native() -> bool:
	return _native != null


func _exit_tree() -> void:
	wait_jobs(false)


## Sélection des nœuds, demandes de pages, téléversements et paramètres d'instance pour `camera`.
func update_view(camera: Camera3D) -> void:
	if pyramid == null:
		return
	if camera == null:
		# Vue parchemin (CM2) ou hors arbre : rien n'est dessiné ni voulu. Sans cette remise à
		# zéro, les pages voulues de la dernière vue en relief resteraient comptées comme
		# manquantes et `is_settled()` ne deviendrait jamais vrai (constaté par PB1 à d = 1500).
		_collect_jobs()
		_wanted.clear()
		_wanted_order = PackedInt64Array()
		_missing_wanted = 0
		return
	_frame += 1
	var t0 := Time.get_ticks_usec()
	_collect_jobs()
	var t1 := Time.get_ticks_usec()
	var t2 := t1
	# Portées et facteur de projection : aussi en natif (miroir pour les comparaisons, bon marché).
	_prepare_camera(camera)
	if _native != null:
		var result := _native_update()
		t2 = Time.get_ticks_usec()
		_apply_native(result)
	else:
		_items.clear()
		_wanted.clear()
		_page_cache.clear()
		_missing_wanted = 0
		_select(0, 0, 0)
		if _items.size() > max_items:
			_px_scale = minf(_px_scale * 1.2, 8.0)
		elif _items.size() < max_items * 0.6 and _px_scale > 1.0:
			_px_scale = maxf(_px_scale / 1.1, 1.0)
		_item_count = _items.size()
		_wanted_order = _sorted_wanted(_wanted)
		t2 = Time.get_ticks_usec()
		_apply_items()
	var t3 := Time.get_ticks_usec()
	_start_jobs()
	var t4 := Time.get_ticks_usec()
	_note_step("collect", t1 - t0)
	_note_step("select", t2 - t1)
	_note_step("apply", t3 - t2)
	_note_step("start", t4 - t3)
	_select_ms = (Time.get_ticks_usec() - t0) / 1000.0
	_update_ms_total += _select_ms
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
	_last_k = k
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


## Clés voulues triées par priorité croissante (étage le plus grossier, puis distance ; clé).
static func _sorted_wanted(wanted: Dictionary) -> PackedInt64Array:
	var order: Array = []
	for key: int in wanted:
		order.append([float(wanted[key]), key])
	order.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0] or (a[0] == b[0] and a[1] < b[1]))
	var out := PackedInt64Array()
	out.resize(order.size())
	for i in order.size():
		out[i] = order[i][1]
	return out


# --- Sélection native (PB3g) -----------------------------------------------------------


func _native_update() -> Dictionary:
	var vs := MapData.vertical_scale()
	var view := PackedFloat64Array([
		_last_k, vs, 1.0 + MapData.relief_gain(), 1.0 - MapData.relief_squash_max_for_scale(vs),
		Time.get_ticks_msec() / 1000.0, shadow_cast_distance, float(_frame),
	])
	var config := PackedFloat64Array([max_vertex_px, max_items, max_depth, extra_depth, morph_ratio, skirt_factor, skirt_max, fade_seconds])
	_native_px_used = _px_scale
	var result: Dictionary = _native.call("update", _cam, _planes, view, config)
	_px_scale = float(result["px_scale"])
	_item_count = int(result["items"])
	_missing_wanted = int(result["missing"])
	_wanted_order = result["wanted"]
	return result


## Applique les changements rendus par `ReliefLod.update` : nœuds retirés, ajoutés, ombres et
## paramètres d'instance changés (seuls ceux marqués dans le masque).
func _apply_native(result: Dictionary) -> void:
	for key: int in (result["removed"] as PackedInt64Array):
		var slot: MeshInstance3D = _slots.get(key)
		if slot == null:
			continue
		_slots.erase(key)
		slot.visible = false
		_pool.append(slot)
	var added_keys: PackedInt64Array = result["added_keys"]
	var added: PackedFloat32Array = result["added"]
	for i in added_keys.size():
		var o := i * 9
		var slot := _take_slot()
		_slots[added_keys[i]] = slot
		var s := added[o + 2]
		var quads := added[o + 5]
		slot.mesh = _patch_full if int(added[o + 4]) == 4 else _patch_half
		slot.transform = Transform3D(Basis.from_scale(Vector3(s, 1.0, s)), Vector3(added[o], 0.0, added[o + 1]))
		slot.custom_aabb = AABB(Vector3(0.0, added[o + 6], 0.0), Vector3(quads, added[o + 7], quads))
		slot.set_instance_shader_parameter("qt_node", Vector4(added[o], added[o + 1], s, added[o + 3]))
		slot.visible = true
		var cast := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if added[o + 8] > 0.5 else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		if slot.cast_shadow != cast:
			slot.cast_shadow = cast
	var cast_keys: PackedInt64Array = result["cast_keys"]
	var cast_on: PackedByteArray = result["cast_on"]
	for i in cast_keys.size():
		(_slots[cast_keys[i]] as MeshInstance3D).cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if cast_on[i] != 0 else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var keys: PackedInt64Array = result["param_keys"]
	var masks: PackedByteArray = result["param_masks"]
	var values: PackedFloat32Array = result["param_values"]
	for i in keys.size():
		var slot: MeshInstance3D = _slots[keys[i]]
		var mask := masks[i]
		var base := i * 28
		for p in 7:
			if mask & (1 << p):
				var o := base + p * 4
				slot.set_instance_shader_parameter(PARAM_NAMES[p], Vector4(values[o], values[o + 1], values[o + 2], values[o + 3]))
	stats["items"] = _item_count


## PB3g (tests) : sélections native et GDScript de la dernière image, sur le même état des pages
## et le même `px_scale` : {native: `ReliefLod.items()`, gdscript: {keys, fine, coarse, dist},
## native_missing, gd_missing, native_wanted, gd_wanted}. Vide sans sélection native.
func compare_selection(camera: Camera3D) -> Dictionary:
	if _native == null:
		return {}
	var saved_px := _px_scale
	var saved_missing := _missing_wanted
	_px_scale = _native_px_used
	_prepare_camera(camera)
	_items.clear()
	_wanted.clear()
	_page_cache.clear()
	_missing_wanted = 0
	_select(0, 0, 0)
	var keys := PackedInt64Array()
	var fine := PackedInt64Array()
	var coarse := PackedInt64Array()
	var dist := PackedFloat64Array()
	for item in _items:
		keys.append(item["key"])
		fine.append(item["fine"])
		coarse.append(item["coarse"])
		dist.append(item["dist"])
	var out := {
		"native": _native.call("items"), "gdscript": {"keys": keys, "fine": fine, "coarse": coarse, "dist": dist},
		"native_missing": saved_missing, "gd_missing": _missing_wanted,
		"native_wanted": _wanted_order, "gd_wanted": _sorted_wanted(_wanted),
	}
	_items.clear()
	_wanted.clear()
	_missing_wanted = saved_missing
	_px_scale = saved_px
	return out


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
	if _bounds.size() < 256:
		return _to_world_bounds(Vector2(-400.0, 5000.0))
	if n >= DEPTH_E0:
		var shift := n - DEPTH_E0
		return _to_world_bounds(_bounds[clampi(r >> shift, 0, 15) * 16 + clampi(c >> shift, 0, 15)])
	var span := 1 << (DEPTH_E0 - n)
	var result := Vector2(INF, -INF)
	for j in span:
		for i in span:
			var b := _bounds[(r * span + j) * 16 + c * span + i]
			result = Vector2(minf(result.x, b.x), maxf(result.y, b.y))
	return _to_world_bounds(result)


## Bornes en mètres → hauteurs affichées (ZG8, SZ1 : s·(1 − c·k)·h ≤ y ≤ s·(1 + g)·h pour h ≥ 0).
static func _to_world_bounds(bounds_m: Vector2) -> Vector2:
	var vs := MapData.vertical_scale()
	var up := 1.0 + MapData.relief_gain()
	var down := 1.0 - MapData.relief_squash_max_for_scale(vs)
	return Vector2(bounds_m.x * vs * (down if bounds_m.x > 0.0 else 1.0), bounds_m.y * vs * (up if bounds_m.y > 0.0 else 1.0))


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
		"fine": fine, "coarse": coarse, "ymin": bmin.y, "ymax": bmax.y, "center": center, "dist": dist,
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
			# Boîte en hauteurs monde de l'échelle courante (ZG4 : `on_vertical_scale_changed`).
			slot.custom_aabb = AABB(Vector3(0.0, float(item["ymin"]) - skirt, 0.0), Vector3(quads, float(item["ymax"]) - float(item["ymin"]) + skirt, quads))
			slot.set_instance_shader_parameter("qt_node", Vector4(origin.x, origin.y, s, n))
			slot.visible = true
		var cast := GeometryInstance3D.SHADOW_CASTING_SETTING_ON if float(item["dist"]) < shadow_cast_distance else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		if slot.cast_shadow != cast:
			slot.cast_shadow = cast
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
			_set_page_params(slot, item, fine, coarse, fade, fresh)
	for key: int in _slots.keys():
		if not alive.has(key):
			var slot: MeshInstance3D = _slots[key]
			_slots.erase(key)
			slot.visible = false
			_pool.append(slot)
	stats["items"] = _items.size()


## Paramètres d'instance du nœud ; seuls ceux qui ont changé depuis la dernière image sont
## renvoyés au serveur de rendu (valeurs mémorisées dans les métadonnées du nœud).
func _set_page_params(slot: MeshInstance3D, item: Dictionary, fine: int, coarse: int, fade: float, fresh: bool) -> void:
	var n: int = item["n"]
	var s := node_size(n) / PATCH_QUADS
	var morph := Vector4(1.0e9, 1.0, fade, minf(s * skirt_factor, skirt_max) + 0.02)
	if n > 0:
		var reach: float = _ranges[n]
		morph.x = morph_ratio * reach
		morph.y = 1.0 / maxf((1.0 - morph_ratio) * reach, 1e-4)
	var fine_params := _page_params(fine)
	var coarse_params := _page_params(coarse)
	var values := [fine_params[0], coarse_params[0], fine_params[1], fine_params[2], coarse_params[1], coarse_params[2], morph]
	var previous: Array = [] if fresh else slot.get_meta("params", [])
	for i in PARAM_NAMES.size():
		if previous.size() != values.size() or previous[i] != values[i]:
			slot.set_instance_shader_parameter(PARAM_NAMES[i], values[i])
	slot.set_meta("params", values)


## [emprise, voisines, diagonales] d'une page, mémorisés jusqu'au prochain changement de résidence.
func _page_params(key: int) -> Array:
	if _param_cache_version != _residency_version:
		_param_cache.clear()
		_param_cache_version = _residency_version
	var cached: Array = _param_cache.get(key, [])
	if cached.is_empty():
		cached = [_page_vec(key), _neighbors(key, false), _neighbors(key, true)]
		_param_cache[key] = cached
	return cached


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


## Lot ZG4 : l'échelle verticale a changé ; les boîtes englobantes des nœuds affichés (hauteurs
## monde) sont remises à l'échelle (les nouveaux nœuds lisent directement la nouvelle échelle).
func on_vertical_scale_changed(old_scale: float, new_scale: float) -> void:
	var ratio := new_scale / maxf(old_scale, 1e-9)
	# ZG8 : le gain local suit l'échelle ; le haut positif de la boîte suit s·(1 + g).
	var ratio_up := ratio * (1.0 + MapData.relief_gain_for_scale(new_scale)) / (1.0 + MapData.relief_gain_for_scale(old_scale))
	# SZ1 : le bas positif suit s·(1 − c·k).
	var ratio_down := ratio * (1.0 - MapData.relief_squash_max_for_scale(new_scale)) / (1.0 - MapData.relief_squash_max_for_scale(old_scale))
	for slot: MeshInstance3D in _slots.values():
		var box := slot.custom_aabb
		var lo := box.position.y * (ratio_down if box.position.y > 0.0 else ratio)
		var hi := box.end.y * (ratio_up if box.end.y > 0.0 else ratio)
		# Jupe (constante, non proportionnelle) : marge de sécurité en plus.
		var margin := absf(hi - lo) * 0.02 + 0.05
		slot.custom_aabb = AABB(Vector3(box.position.x, lo - margin, box.position.z), Vector3(box.size.x, hi - lo + 2.0 * margin, box.size.z))


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


## Pages voulues non chargées, par priorité (étage le plus grossier puis distance) : décodées au
## début de l'image suivante sur le fil principal par Rust (`_main_queue`, budget
## `main_decode_budget_ms`), sinon confiées à `WorkerThreadPool` (repli GDScript `Png16`).
func _start_jobs() -> void:
	_main_queue.clear()
	if _wanted_order.is_empty():
		return
	if _decoder != null:
		for key: int in _wanted_order:
			if _requested.size() >= max_jobs:
				break
			if _requested.has(key) or _jobs.has(key):
				continue
			var path := pyramid.tile_path(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
			if _decoder.call("request", key, path):
				_requested[key] = path
		return
	if _main_store != null:
		for key: int in _wanted_order:
			if _main_queue.size() >= max_main_decodes_per_frame:
				break
			if not _jobs.has(key):
				_main_queue.append(key)
		return
	if _jobs.size() >= max_jobs:
		return
	for key: int in _wanted_order:
		if _jobs.size() >= max_jobs:
			break
		if _jobs.has(key):
			continue
		var job := PageJob.new()
		job.key = key
		job.path = pyramid.tile_path(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
		var task := WorkerThreadPool.add_task(job.run, false, "relief page %d" % key)
		_jobs[key] = {"task": task, "job": job}


## Récupère les décodages terminés et en téléverse au plus `max_uploads_per_frame` (tous avec
## `block`).
func _collect_jobs(block: bool = false) -> void:
	var uploads := 0
	var t_poll := Time.get_ticks_usec()
	if _decoder != null:
		var guard := 0
		while not _requested.is_empty() and guard < 2000:
			for item: Dictionary in _decoder.call("poll", 1 << 20 if block else max_uploads_per_frame - uploads):
				var key := int(item["id"])
				var job := PageJob.new()
				job.key = key
				job.path = str(_requested.get(key, ""))
				_requested.erase(key)
				job.bytes = item["bytes"]
				if job.bytes.size() == PAGE_PX * PAGE_PX * 2:
					# ZG7a : image et mipmaps faites dans un fil (1-3 ms par page sous charge au fil
					# principal), avec le creusement du lit s'il y en a un ; téléversement quand
					# la tâche est finie (une image plus tard).
					job.decode_ms = float(item["ms"])
					_dispatch_image(key, job)
				elif _finish_job(key, job):
					uploads += 1
			if not block or _requested.is_empty():
				break
			OS.delay_msec(1)
			guard += 1
	_note_step("poll", Time.get_ticks_usec() - t_poll)
	var t0 := Time.get_ticks_usec()
	for key in _main_queue:
		if not block and (uploads >= max_uploads_per_frame or Time.get_ticks_usec() - t0 > main_decode_budget_ms * 1000.0):
			break
		var job := PageJob.new()
		job.key = key
		job.path = pyramid.tile_path(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
		job.decode_with(_main_store)
		if _finish_job(key, job):
			uploads += 1
	_main_queue.clear()
	for key: int in _jobs.keys():
		var entry: Dictionary = _jobs[key]
		if not block and (uploads >= max_uploads_per_frame or not WorkerThreadPool.is_task_completed(entry["task"])):
			continue
		WorkerThreadPool.wait_for_task_completion(entry["task"])
		_jobs.erase(key)
		if _finish_job(key, entry["job"]):
			uploads += 1
	if block and not _jobs.is_empty():
		_collect_jobs(true)  # ZG5b : pages parties au creusement pendant cette passe


## ZG7a : octets décodés (fils natifs) → image et mipmaps dans un fil, creusement compris.
func _dispatch_image(key: int, job: PageJob) -> void:
	job.filter_checked = true
	if page_filter != null:
		var t_carve := Time.get_ticks_usec()
		var task: Object = page_filter.call("carve_job", key)
		_note_step("carve_job", Time.get_ticks_usec() - t_carve)
		if task != null:
			job.filter = task
			_jobs[key] = {"task": WorkerThreadPool.add_task(job.run_filter, false, "relief carve %d" % key), "job": job}
			return
	_jobs[key] = {"task": WorkerThreadPool.add_task(job.run_image, false, "relief image %d" % key), "job": job}


func _finish_job(key: int, job: PageJob) -> bool:
	if not job.ok:
		pyramid.mark_broken(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
		if _native != null:
			_native.call("mark_broken", key)
		push_warning("ReliefQuadtree: unreadable tile %s" % job.path)
		return false
	if page_filter != null and job.filter == null and not job.filter_checked:
		var t_carve := Time.get_ticks_usec()
		var task: Object = page_filter.call("carve_job", key)
		_note_step("carve_job", Time.get_ticks_usec() - t_carve)
		if task != null:
			job.filter = task
			_jobs[key] = {"task": WorkerThreadPool.add_task(job.run_filter, false, "relief carve %d" % key), "job": job}
			return false
	_decode_ms.append(job.decode_ms)
	return _upload(key, job)


func _upload(key: int, job: PageJob) -> bool:
	if _pages.has(key):
		return false
	var layer := _alloc_layer()
	if layer < 0:
		return false
	var t0 := Time.get_ticks_usec()
	_page_array.update_layer(job.image, layer)
	_upload_ms_max = maxf(_upload_ms_max, (Time.get_ticks_usec() - t0) / 1000.0)
	_note_step("layer", Time.get_ticks_usec() - t0)
	var t_upload := Time.get_ticks_msec() / 1000.0
	_pages[key] = {"layer": layer, "last_used": _frame, "t_upload": t_upload}
	_page_bytes[key] = job.bytes
	if _native != null:
		_native.call("add_page", key, layer, t_upload, _frame, job.bytes)
	_layer_keys[layer] = key
	_residency_version += 1
	var rect := _tile_rect(key)
	var level := ReliefPyramid.level_of_key(key)
	for index in _chunks_of(rect):
		_chunk_top[index] = maxi(_chunk_top[index], level)
	var t_emit := Time.get_ticks_usec()
	surface_changed.emit(rect)
	_note_step("emit", Time.get_ticks_usec() - t_emit)
	return true


func _note_step(step: String, usec: int) -> void:
	_step_ms_max[step] = maxf(float(_step_ms_max[step]), usec / 1000.0)
	PerfProbe.add("qt/" + step, usec)  # SZ6


## Morceaux E0 (index ligne × 16 + colonne) touchés par un rectangle carte.
static func _chunks_of(rect: Rect2) -> PackedInt32Array:
	var result := PackedInt32Array()
	var c0 := clampi(int(floor(rect.position.x / 256.0)), 0, 15)
	var r0 := clampi(int(floor(rect.position.y / 256.0)), 0, 15)
	var c1 := clampi(int(ceil(rect.end.x / 256.0)) - 1, 0, 15)
	var r1 := clampi(int(ceil(rect.end.y / 256.0)) - 1, 0, 15)
	for r in range(r0, r1 + 1):
		for c in range(c0, c1 + 1):
			result.append(r * 16 + c)
	return result


func _alloc_layer() -> int:
	if not _free_layers.is_empty():
		return _free_layers.pop_back()
	var oldest := -1
	if _native != null:
		oldest = int(_native.call("oldest_page", _frame - 1))  # PB3g : `last_used` tenu en Rust
	else:
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
	if _native != null:
		_native.call("remove_page", oldest)
	_residency_version += 1
	var rect := _tile_rect(oldest)
	# ZG7a : seuls les morceaux dont la page évincée était l'étage le plus fin sont recalculés
	# (un parcours des 256 pages par éviction, deux évictions par image au pire, sinon).
	var old_level := ReliefPyramid.level_of_key(oldest)
	var touched := PackedInt32Array()
	for index in _chunks_of(rect):
		if _chunk_top[index] <= old_level:
			touched.append(index)
			_chunk_top[index] = -1
	for key: int in (_pages if not touched.is_empty() else {}):
		var level := ReliefPyramid.level_of_key(key)
		var page_rect := _tile_rect(key)
		for index in touched:
			if level > _chunk_top[index] and page_rect.intersects(Rect2((index % 16) * 256.0, (index / 16) * 256.0, 256.0, 256.0)):
				_chunk_top[index] = level
	surface_changed.emit(rect)
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
		if _main_store != null:
			_main_queue.clear()
			for key: int in _wanted_order:
				if not _pages.has(key):
					_main_queue.append(key)
		if _decoder != null:
			for key: int in _wanted_order:
				if not _pages.has(key) and not _requested.has(key):
					var path := pyramid.tile_path(ReliefPyramid.level_of_key(key), ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
					if _decoder.call("request", key, path):
						_requested[key] = path
		_collect_jobs(true)
		max_uploads_per_frame = saved
		return
	for entry: Dictionary in _jobs.values():
		WorkerThreadPool.wait_for_task_completion(entry["task"])
	_jobs.clear()
	_requested.clear()


## Vrai quand toutes les pages voulues à la dernière sélection sont chargées et aucun décodage
## n'est en cours (captures, mesures).
func is_settled() -> bool:
	return _jobs.is_empty() and _main_queue.is_empty() and _requested.is_empty() and _missing_wanted == 0


## Étage le plus fin des pages chargées qui touchent le morceau E0 `index` (-1 : aucune).
func chunk_top(index: int) -> int:
	return _chunk_top[index] if index >= 0 and index < _chunk_top.size() else -1


func page_count() -> int:
	return _pages.size()


func pending_jobs() -> int:
	return _jobs.size() + _requested.size() + _main_queue.size()


func item_count() -> int:
	return _item_count


## Mesures : nœuds, pages, VRAM des pages, décodage moyen / max, téléversement max, sélection.
func perf_stats() -> Dictionary:
	var total := 0.0
	var worst := 0.0
	for ms in _decode_ms:
		total += ms
		worst = maxf(worst, ms)
	return {
		"items": _item_count,
		"pages": _pages.size(),
		"page_layers": max_pages,
		"page_vram_mb": snappedf(max_pages * PAGE_PX * PAGE_PX * 2 * 4.0 / 3.0 / 1048576.0, 0.1),
		"decoded": _decode_ms.size(),
		"decode_ms_avg": snappedf(total / maxf(_decode_ms.size(), 1.0), 0.01),
		"decode_ms_max": snappedf(worst, 0.01),
		"upload_ms_max": snappedf(_upload_ms_max, 0.01),
		"select_ms": snappedf(_select_ms, 0.01),
		"update_ms_avg": snappedf(_update_ms_total / maxf(_frame, 1.0), 0.01),
		"px_scale": snappedf(_px_scale, 0.01),
		"qt_step_ms_max": _step_ms_max.duplicate(),
	}


# --- Surface côté processeur -----------------------------------------------------------


## Hauteur monde de la surface la plus fine chargée en (x, y) carte (bilinéaire dans la page),
## NAN si aucune page de la pyramide ne couvre le point.
func surface_height_at(x: float, y: float) -> float:
	if pyramid == null or x < 0.0 or y < 0.0 or x >= 4096.0 or y >= 4096.0:
		return NAN
	var top := _chunk_top[int(y / 256.0) * 16 + int(x / 256.0)]
	if top < 0:
		return NAN
	# Dernière page utilisée : valable si elle contient le point et qu'aucune page plus fine ne
	# touche ce morceau (requêtes groupées dans une zone : maquettes, routes, arbres).
	if _hit_level == top and _hit_version == _residency_version:
		var lx := x - _hit_origin.x
		var ly := y - _hit_origin.y
		if lx >= 0.0 and ly >= 0.0 and lx < _hit_units and ly < _hit_units:
			return MapData.display_height(_bilinear(_hit_bytes, lx / _hit_px - 0.5, ly / _hit_px - 0.5, pyramid.height_min_m, pyramid.height_range_m), x, y)
	var side := 1 << top
	for level in range(top, -1, -1):
		var units := ROOT_TILE_UNITS / side
		var col := int(floor((x - ReliefPyramid.GRID_OFFSET) / units))
		var row := int(floor((y - ReliefPyramid.GRID_OFFSET) / units))
		var key := (level << 24) | (row << 12) | col
		side >>= 1
		if not _page_bytes.has(key):
			continue
		var bytes: PackedByteArray = _page_bytes[key]
		_hit_level = level if level == top else -1
		_hit_version = _residency_version
		_hit_origin = Vector2(col * units + ReliefPyramid.GRID_OFFSET, row * units + ReliefPyramid.GRID_OFFSET)
		_hit_units = units
		_hit_px = units / PAGE_PX
		_hit_bytes = bytes
		return MapData.display_height(_bilinear(bytes, (x - _hit_origin.x) / _hit_px - 0.5, (y - _hit_origin.y) / _hit_px - 0.5, pyramid.height_min_m, pyramid.height_range_m), x, y)
	return NAN


## PB3g : `surface_height_at` pour une série de points (NAN hors pages) ; bilinéaire natif
## (`ReliefLod.heights_m`, même arithmétique) quand la sélection est native.
func surface_heights_at(points: PackedVector2Array) -> PackedFloat64Array:
	var out := PackedFloat64Array()
	if _native == null or pyramid == null:
		out.resize(points.size())
		for n in points.size():
			out[n] = surface_height_at(points[n].x, points[n].y)
		return out
	out = _native.call("heights_m", points, pyramid.height_min_m, pyramid.height_range_m)
	for n in points.size():
		if not is_nan(out[n]):
			out[n] = MapData.display_height(out[n], points[n].x, points[n].y)
	return out


## Bilinéaire aux coordonnées pixel (fx, fy) d'une page (bornées au bord), altitude en MÈTRES
## (ZG8 : la hauteur affichée passe par `MapData.display_height`, qui dépend du point).
static func _bilinear(bytes: PackedByteArray, fx: float, fy: float, h_min: float, h_range: float) -> float:
	fx = clampf(fx, 0.0, PAGE_PX - 1.0)
	fy = clampf(fy, 0.0, PAGE_PX - 1.0)
	var i := mini(int(fx), PAGE_PX - 2)
	var j := mini(int(fy), PAGE_PX - 2)
	var tx := fx - i
	var ty := fy - j
	var o := (j * PAGE_PX + i) * 2
	var a := bytes.decode_u16(o)
	var b := bytes.decode_u16(o + 2)
	var c := bytes.decode_u16(o + PAGE_PX * 2)
	var d := bytes.decode_u16(o + PAGE_PX * 2 + 2)
	var top := a + (b - a) * tx
	var v := (top + (c + (d - c) * tx - top) * ty) / 65535.0
	return h_min + v * h_range


## Instantané des pages chargées qui touchent `rect`, lisible depuis un fil de travail sans
## verrou (octets partagés en copie sur écriture) : grille pour `TerrainBuilder.grid_height`
## (coordonnées locales à `origin`), repli sur la heightmap 4096 hors pages.
## ZG7a : étage de page le plus fin chargé qui touche chacun des rectangles (-1 : aucun), en
## un seul parcours des pages (sans copier leurs octets comme `surface_snapshot`).
func finest_levels(rects: Array[Rect2]) -> PackedInt32Array:
	var out := PackedInt32Array()
	out.resize(rects.size())
	out.fill(-1)
	for key: int in _page_bytes:
		var level := ReliefPyramid.level_of_key(key)
		var page_rect := _tile_rect(key)
		for i in rects.size():
			if level > out[i] and page_rect.intersects(rects[i]):
				out[i] = level
	return out


func surface_snapshot(rect: Rect2, origin: Vector2) -> Dictionary:
	var pages := {}
	for key: int in _page_bytes:
		if _tile_rect(key).intersects(rect):
			pages[key] = _page_bytes[key]
	return {
		"qt_pages": pages, "max_level": pyramid.max_level, "h_min": pyramid.height_min_m,
		"h_range": pyramid.height_range_m, "origin": origin, "map": map_data,
		# PB3g : magasin natif des mêmes pages (octets partagés, sans copie côté Rust).
		"qt_store": _native,
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
	var h := sample_pages_m(pages, top_level, h_min, h_range, x, y)
	return MapData.display_height(h, x, y) if not is_nan(h) else NAN


## Altitude (m) bilinéaire dans la page la plus fine de `pages` couvrant (x, y) ; NAN si aucune.
static func sample_pages_m(pages: Dictionary, top_level: int, h_min: float, h_range: float, x: float, y: float) -> float:
	if pages.is_empty():
		return NAN
	for level in range(top_level, -1, -1):
		var t := ReliefPyramid.tile_at(level, x, y)
		if t.x < 0 or t.y < 0:
			return NAN
		var key := ReliefPyramid.key_of(level, t.x, t.y)
		if not pages.has(key):
			continue
		var origin := ReliefPyramid.tile_origin(level, t.x, t.y)
		var px_units := ReliefPyramid.pixel_units(level)
		return _bilinear(pages[key], (x - origin.x) / px_units - 0.5, (y - origin.y) / px_units - 0.5, h_min, h_range)
	return NAN


## Décodage d'une tuile : octets little-endian (Rust sur le fil principal avec `decode_with`, ou
## `Png16` + inversion des octets dans un fil de travail avec `run`) et image R16 avec mipmaps
## prête à téléverser.
class PageJob:
	extends RefCounted

	var key: int = 0
	var path: String = ""
	var bytes: PackedByteArray = PackedByteArray()
	var image: Image
	var decode_ms: float = 0.0
	var ok: bool = false
	## Lot ZG5b : retouche des octets (lit creusé) dans un fil, puis image refaite.
	var filter: Object = null
	## ZG7a : creusement déjà demandé (ou page sans lit) : pas de second passage.
	var filter_checked: bool = false

	## ZG7a : image et mipmaps d'octets déjà décodés (fils natifs), dans un fil.
	func run_image() -> void:
		var decoded := decode_ms
		_finish(Time.get_ticks_usec())
		decode_ms += decoded

	func run_filter() -> void:
		var t0 := Time.get_ticks_usec()
		bytes = filter.call("apply", bytes)
		var decoded := decode_ms
		_finish(t0)
		decode_ms += decoded

	func run() -> void:
		var t0 := Time.get_ticks_usec()
		var expected := PAGE_PX * PAGE_PX * 2
		if not FileAccess.file_exists(path):
			return
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
		_finish(t0)

	## Décodage Rust (fil principal seulement).
	func decode_with(store: Object) -> void:
		var t0 := Time.get_ticks_usec()
		bytes = store.call("load_heightmap_u16", path)
		if bytes.size() == PAGE_PX * PAGE_PX * 2:
			_finish(t0)

	func _finish(t0: int) -> void:
		image = Image.create_from_data(PAGE_PX, PAGE_PX, false, Image.FORMAT_R16, bytes)
		image.generate_mipmaps()
		decode_ms = (Time.get_ticks_usec() - t0) / 1000.0
		ok = true
