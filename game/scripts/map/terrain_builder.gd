class_name TerrainBuilder
extends Node3D

## Terrain de campagne : grille de CHUNKS × CHUNKS tuiles (MeshInstance3D) construites
## depuis la heightmap. Deux niveaux de détail : le LOD lointain (`far_step`) est
## construit au chargement ; le LOD proche (`near_step`) est construit à la demande
## quand la caméra s'approche, puis mis en cache.
##
## Les normales ne sont pas stockées dans le maillage : le shader les dérive de la
## heightmap (aucune couture entre tuiles ni entre LOD).
##
## Lot C6 : troisième niveau « relief fin » en vue comté. Les tuiles proches du point visé sont
## remplacées par des maillages construits (dans `WorkerThreadPool`, `FineTerrainJob`) sur les
## tuiles de relief 8192² (`map.json.height_tiles`), chargées à la demande et gardées dans un
## cache LRU (`max_cached_fine`). `surface_height_at` donne la hauteur exacte de la surface
## affichée (quel que soit le niveau) pour poser les objets ; `chunk_surface_changed` signale
## qu'une tuile a changé de niveau (les objets posés dessus doivent être recalés).
##
## Lot ZG2 (ADR 0036) : quand la pyramide de relief est en cache (`ReliefPyramid`), un
## `ReliefQuadtree` dessine tout le terrain (patchs déplacés au GPU, pages streamées) et les
## morceaux E0 sont masqués ; `surface_height_at` rend alors la surface de la page chargée la plus
## fine, `surface_grid` un instantané de ces pages, et `chunk_surface_changed` signale aussi
## l'arrivée d'une page (au plus toutes les `surface_flush_interval_ms`). Sans cache (ou
## `--no-pyramid`), comportement inchangé.

signal chunk_surface_changed(index: int)
## Lot ZG2 : rectangle carte dont la surface a changé (page de la pyramide arrivée ou évincée),
## émis à chaque page, sans regroupement : pour les recalages fins (lot ZG5).
signal surface_rect_changed(rect: Rect2)
## Lot ZG4 : l'échelle verticale (`MapData.vertical_scale()`) vient de changer. Les calques
## d'objets ponctuels (armées, étiquettes) s'y recalent ; les calques par morceau reçoivent en
## plus `chunk_surface_changed` (étalé, `rescaling_vertical` vrai).
signal vertical_scale_changed(old_scale: float, new_scale: float)

const CHUNKS := 16
const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
## Couches de matériaux, dans l'ordre des Texture2DArray (voir game/assets/textures/terrain/README.md).
const MATERIAL_LAYERS: Array[String] = ["grass", "farmland", "forest", "rock", "heath", "snow", "sand"]
const TEXTURE_DIR := "res://assets/textures/terrain/"

## Pas (en pixels) entre deux sommets pour le LOD proche / lointain.
@export var near_step: int = 4
@export var far_step: int = 8
## Distance caméra → centre de tuile en dessous de laquelle le LOD proche est utilisé.
@export var near_distance: float = 600.0
@export var max_near_builds_per_frame: int = 4
## Relief fin : pas en pixels 8192 (1 = un sommet toutes les 0,5 unité), nombre maximal de
## tuiles fines affichées, rayon (autour du point visé), tâches simultanées, cache LRU.
## `fine_step` sert de valeur fixe quand `fine_step_auto` est faux (ex. `--fine-step=N`,
## bancs de perf) ; sinon le pas est choisi automatiquement selon la distance caméra
## (T2 : `fine_step_near` en dessous de `fine_step_switch_distance`, `fine_step_far` au-delà,
## avec hystérésis pour éviter les allers-retours au bord du seuil).
@export var fine_step: int = 1
@export var fine_step_auto: bool = true
@export var fine_step_near: int = 1
@export var fine_step_far: int = 2
@export var fine_step_switch_distance: float = 80.0
@export var fine_step_hysteresis: float = 20.0
@export var max_fine_chunks: int = 4
@export var fine_radius: float = 150.0
@export var max_fine_jobs: int = 2
@export var max_cached_fine: int = 10
@export var fine_enabled: bool = true
## Lot ZG2 : quadtree de relief si la pyramide est en cache ; `pyramid_manifest_path` remplace
## `data/map/relief_pyramid.json` (essais, `--pyramid-dir=<dossier>` contenant le manifeste).
@export var pyramid_enabled: bool = true
@export var pyramid_manifest_path: String = ""
@export var surface_flush_interval_ms: int = 250
@export var max_surface_emits_per_flush: int = 2
## Un morceau n'est recalé qu'après ce délai sans nouvelle page (E1 → E2 → E3 → E4 : un seul recalage).
@export var surface_settle_ms: int = 700
## Lot ZG4 : budget par image des recalages après un changement d'échelle verticale.
@export var rescale_budget_ms: float = 3.0
## Délai sans changement d'échelle avant de recaler les calques (zoom continu : un seul recalage).
@export var rescale_settle_ms: int = 180
@export var max_far_rescales_per_frame: int = 1
## SZ6 : budget par image des changements de niveau signalés (quadtree ; au moins un par image).
@export var level_emit_budget_ms: float = 4.0
## Vrai pendant les `chunk_surface_changed` émis pour un changement d'échelle verticale seul
## (la surface en mètres n'a pas changé).
var rescaling_vertical: bool = false
var _rescale_queue: Dictionary = {}
var _rescale_changed_ms: int = 0

var map_data: MapData
## PF1 : préréglage de qualité (`RenderQuality`, groupe `CLIENT_GROUP`) : densité du quadtree de
## relief (ZG2) et, sans pyramide en cache, relief fin `FineTerrainJob` et LOD proche.
var quality_fine: bool = true
var _quality: Dictionary = {}
var _base_near_distance: float = -1.0
## Bancs (`--map-ab`, `relief_cast:<n>`) : impose le nombre de cascades (0 = préréglage).
var relief_shadow_override: int = 0
var material: ShaderMaterial
var chunk_px: int = 0
var build_stats: Dictionary = {}

var _chunks: Array[MeshInstance3D] = []
var _far_meshes: Array[ArrayMesh] = []
var _near_meshes: Dictionary = {}
## Niveau affiché par tuile : 0 lointain, 1 proche, 2 fin.
var _is_near: PackedByteArray = PackedByteArray()
## Grilles de hauteurs des maillages (pour `surface_height_at`) : {"heights", "side", "unit"}.
var _far_grids: Array[Dictionary] = []
var _near_grids: Dictionary = {}
var _grid_indices_cache: Dictionary = {}  # quads → PackedInt32Array
## Relief fin : index → {"mesh", "grid", "last_used"} ; tâches en cours : index → {"task", "job"}.
var _fine_cache: Dictionary = {}
## Pas utilisé pour construire chaque entrée de `_fine_cache` (T2, détecte les tuiles à
## reconstruire quand le pas adaptatif change).
var _fine_cache_step: Dictionary = {}
var _fine_jobs: Dictionary = {}
var _fine_indices: PackedInt32Array = PackedInt32Array()
## Pas courant (adaptatif ou fixe, voir `fine_step_auto`) ; initialisé à `fine_step`.
var _current_fine_step: int = 0
var _fine_tiles_dir: String = ""
var _fine_pattern: String = ""
var _fine_tile_px: int = 0
var _fine_store: Object = null
var _lod_frame: int = 0
var _last_wanted_fine: Array = []
var _height_texture: ImageTexture
var _splat_texture: ImageTexture
var _border_texture: ImageTexture
var _coast_texture: ImageTexture
var _river_bed_texture: ImageTexture
var _landuse_texture: ImageTexture
var _albedo_array: Texture2DArray
var _normal_array: Texture2DArray
var _layer_means: PackedVector3Array = PackedVector3Array()
## Mipmap de niveau 2 de la heightmap R16 (blocs 4×4 moyennés) : hauteurs lissées du LOD
## lointain (pas de pics en dents de scie échantillonnés tous les `far_step` pixels).
var _smooth_bytes: PackedByteArray = PackedByteArray()
var _smooth_size: Vector2i = Vector2i.ZERO
var _ids_texture: ImageTexture
var _faction_texture: ImageTexture
var _mask_texture: ImageTexture
var _owner_colors: Dictionary = {}
var _province_colors: PackedColorArray = PackedColorArray()
var pyramid: ReliefPyramid
var quadtree: ReliefQuadtree
## ZG8 : gain de relief local des maillages cuits (E0, repli), durée du calcul du fond.
var _baked_gain: float = 0.0
var _relief_floor_ms: float = 0.0
## Morceaux dont la surface a changé (page arrivée ou évincée), signalés par paquets.
var _surface_dirty: Dictionary = {}
## Étage de page le plus fin au dernier signal de chaque morceau (recalage seulement s'il change).
var _emitted_top: PackedInt32Array = PackedInt32Array()
var _surface_flush_ms: int = 0


## Palette de repli (couleurs héraldiques), utilisée seulement sans `GameDataStore`
## (ex. fixtures de test) ; sinon `set_province_colors` fournit les vraies couleurs.
const FALLBACK_PALETTE: Array[Color] = [
	Color(0.20, 0.32, 0.75), Color(0.78, 0.18, 0.18), Color(0.85, 0.65, 0.15),
	Color(0.25, 0.55, 0.30), Color(0.55, 0.25, 0.60), Color(0.85, 0.45, 0.15),
	Color(0.20, 0.60, 0.65), Color(0.60, 0.50, 0.30), Color(0.75, 0.30, 0.50),
	Color(0.35, 0.35, 0.35), Color(0.45, 0.70, 0.25), Color(0.90, 0.80, 0.55),
]


func apply_render_quality(p: Dictionary) -> void:
	_quality = p
	if _base_near_distance < 0.0:
		_base_near_distance = near_distance
	quality_fine = bool(p.get("fine_relief", true))
	near_distance = _base_near_distance * float(p.get("terrain_near", 1.0))
	if quadtree != null:
		_apply_quadtree_quality()


## Quadtree : espacement des sommets à l'écran, budget de nœuds et profondeur au-delà des données
## (immédiats) ; couches de pages (au prochain chargement de la carte : tableau alloué au `setup`).
func _apply_quadtree_quality() -> void:
	quadtree.max_vertex_px = float(_quality.get("relief_vertex_px", quadtree.max_vertex_px))
	quadtree.max_items = int(_quality.get("relief_items", quadtree.max_items))
	quadtree.extra_depth = int(_quality.get("relief_extra_depth", quadtree.extra_depth))


## PF1 : portée des ombres portées par le relief du quadtree : bord de la cascade
## `relief_shadow_cascades` du soleil (1 = première cascade ; 4 ou plus = toutes).
func _relief_shadow_distance() -> float:
	var cascades := relief_shadow_override if relief_shadow_override > 0 else int(_quality.get("relief_shadow_cascades", 4))
	var sun := get_parent().get_node_or_null("Sun") as DirectionalLight3D if get_parent() != null else null
	if sun == null or not sun.shadow_enabled:
		return INF
	var splits := 4 if sun.directional_shadow_mode == DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS else 2
	if cascades >= splits:
		return INF
	var edges := [sun.directional_shadow_split_1, sun.directional_shadow_split_2, sun.directional_shadow_split_3]
	return sun.directional_shadow_max_distance * float(edges[cascades - 1])


func build(data: MapData) -> void:
	var t0 := Time.get_ticks_msec()
	add_to_group(RenderQuality.CLIENT_GROUP)
	apply_render_quality(RenderQuality.preset())
	clear_terrain()
	# ZG4 : nouvelle carte à l'échelle stratégique (les maillages E0 sont cuits à HEIGHT_SCALE).
	_rescale_queue.clear()
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	RenderingServer.global_shader_parameter_set("campaign_vertical_scale", MapData.HEIGHT_SCALE)
	map_data = data
	chunk_px = ceili(float(maxi(data.size.x, data.size.y)) / CHUNKS)
	_build_relief_floor()
	_build_textures()
	_build_material()
	_apply_sun_elevation()
	_is_near.resize(CHUNKS * CHUNKS)
	_is_near.fill(0)
	var vertex_count := 0
	# PB1 : sommets des 256 tuiles lointaines calculés en parallèle (fonction pure des hauteurs) ;
	# maillages et nœuds créés ensuite sur le fil principal, dans le même ordre.
	var far_vertices: Array = []
	var far_heights: Array = []
	far_vertices.resize(CHUNKS * CHUNKS)
	far_heights.resize(CHUNKS * CHUNKS)
	var task := WorkerThreadPool.add_group_task(func(i: int) -> void:
		var heights := PackedFloat32Array()
		far_vertices[i] = _chunk_vertices(i % CHUNKS, i / CHUNKS, far_step, heights)
		far_heights[i] = heights, CHUNKS * CHUNKS, -1, true, "terrain far chunks")
	WorkerThreadPool.wait_for_group_task_completion(task)
	for cy in CHUNKS:
		for cx in CHUNKS:
			var i := cy * CHUNKS + cx
			var built := _chunk_from(far_vertices[i], far_heights[i], far_step)
			var mesh: ArrayMesh = built["mesh"]
			_far_grids.append(built["grid"])
			vertex_count += mesh.surface_get_array_len(0)
			var instance := MeshInstance3D.new()
			instance.name = "Chunk_%d_%d" % [cx, cy]
			instance.position = Vector3(cx * chunk_px, 0.0, cy * chunk_px)
			instance.mesh = mesh
			instance.material_override = material
			add_child(instance)
			_chunks.append(instance)
			_far_meshes.append(mesh)
	_setup_fine_tiles()
	_setup_quadtree()
	build_stats = {
		"fine_tiles": _fine_tiles_dir != "",
		"quadtree": quadtree != null,
		"chunks": _chunks.size(),
		"far_vertices": vertex_count,
		"far_step": far_step,
		"near_step": near_step,
		"build_ms": Time.get_ticks_msec() - t0,
		"relief_floor_ms": _relief_floor_ms,
	}


func clear_terrain() -> void:
	_wait_fine_jobs()
	if quadtree != null:
		quadtree.wait_jobs(false)
		quadtree.queue_free()
		quadtree = null
	for chunk in _chunks:
		chunk.queue_free()
	_chunks.clear()
	_far_meshes.clear()
	_near_meshes.clear()
	_far_grids.clear()
	_near_grids.clear()
	_fine_cache.clear()


func _exit_tree() -> void:
	_wait_fine_jobs()


func chunk_count() -> int:
	return _chunks.size()


func near_chunk_count() -> int:
	return _is_near.count(1)


func fine_chunk_count() -> int:
	return _is_near.count(2)


func fine_pending_jobs() -> int:
	return _fine_jobs.size()


## Niveau affiché d'une tuile (0 lointain, 1 proche, 2 fin), -1 hors carte.
func chunk_level(index: int) -> int:
	return _is_near[index] if index >= 0 and index < _is_near.size() else -1


## Index de tuile contenant un point carte (-1 hors carte).
func chunk_index_at(x: float, y: float) -> int:
	if chunk_px <= 0:
		return -1
	var cx := int(floor(x / chunk_px))
	var cy := int(floor(y / chunk_px))
	if cx < 0 or cy < 0 or cx >= CHUNKS or cy >= CHUNKS:
		return -1
	return cy * CHUNKS + cx


## Hauteur monde exacte de la surface affichée en (x, y) carte (interpolation dans le triangle
## du maillage courant), jamais sous le niveau de la mer. Repli : heightmap bilinéaire.
func surface_height_at(x: float, y: float) -> float:
	if map_data == null:
		return 0.0
	if quadtree != null:
		var h := quadtree.surface_height_at(x, y)
		return maxf(h if not is_nan(h) else map_data.height_world_at(x, y), 0.0)
	var index := chunk_index_at(x, y)
	if index < 0:
		return map_data.surface_world_at(x, y)
	var grid: Dictionary = _grid_for(index)
	if grid.is_empty():
		return map_data.surface_world_at(x, y)
	return maxf(grid_height(grid, x - (index % CHUNKS) * chunk_px, y - (index / CHUNKS) * chunk_px), 0.0)


## `surface_height_at` pour une série de points (PB1 : rubans de route, recalages) : même
## résultat, la grille de la tuile courante reste en cache tant que les points y restent.
func surface_heights_at(points: PackedVector2Array) -> PackedFloat32Array:
	var result := PackedFloat32Array()
	result.resize(points.size())
	if quadtree != null or map_data == null:
		for n in points.size():
			result[n] = surface_height_at(points[n].x, points[n].y)
		return result
	var current := -2
	var heights := PackedFloat32Array()
	var side := 0
	var unit := 1.0
	var ox := 0.0
	var oy := 0.0
	var has_grid := false
	for n in points.size():
		var p := points[n]
		var index := chunk_index_at(p.x, p.y)
		if index != current:
			current = index
			has_grid = false
			if index >= 0 and map_data != null:
				var grid: Dictionary = _grid_for(index)
				if not grid.is_empty():
					has_grid = true
					heights = grid["heights"]
					side = grid["side"]
					unit = grid["unit"]
					ox = (index % CHUNKS) * chunk_px
					oy = (index / CHUNKS) * chunk_px
		if not has_grid:
			result[n] = map_data.surface_world_at(p.x, p.y) if map_data != null else 0.0
			continue
		var gx := clampf((p.x - ox) / unit, 0.0, side - 1.001)
		var gy := clampf((p.y - oy) / unit, 0.0, side - 1.001)
		var i := int(gx)
		var j := int(gy)
		var tx := gx - i
		var ty := gy - j
		var a := j * side + i
		var ha := heights[a]
		var hd := heights[a + side + 1]
		var h: float
		if tx >= ty:
			var hb := heights[a + 1]
			h = ha + (hb - ha) * tx + (hd - hb) * ty
		else:
			var hc := heights[a + side]
			h = ha + (hd - hc) * tx + (hc - ha) * ty
		result[n] = maxf(h, 0.0)
	return result


## Grille de hauteurs du maillage affiché pour la tuile `index` ({"heights", "side", "unit"},
## origine au coin de la tuile ; vide si inconnue). Jamais modifiée après construction : lisible
## depuis un fil de travail (recalage de la végétation, lot C7b) avec `grid_height`.
func surface_grid(index: int) -> Dictionary:
	if index < 0 or index >= _is_near.size():
		return {}
	if quadtree != null:
		var origin := Vector2((index % CHUNKS) * chunk_px, (index / CHUNKS) * chunk_px)
		return quadtree.surface_snapshot(Rect2(origin, Vector2(chunk_px, chunk_px)), origin)
	return _grid_for(index)


func _grid_for(index: int) -> Dictionary:
	match int(_is_near[index]):
		2:
			var entry: Dictionary = _fine_cache.get(index, {})
			return entry.get("grid", {})
		1:
			return _near_grids.get(index, {})
		_:
			return _far_grids[index] if index < _far_grids.size() else {}


## Hauteur dans une grille régulière triangulée comme les maillages (diagonale a → d).
## Lot ZG2 : accepte aussi un instantané du quadtree (`ReliefQuadtree.surface_snapshot`).
static func grid_height(grid: Dictionary, lx: float, ly: float) -> float:
	if grid.has("qt_pages"):
		return ReliefQuadtree.sample_snapshot(grid, lx, ly)
	var heights: PackedFloat32Array = grid["heights"]
	var side: int = grid["side"]
	var unit: float = grid["unit"]
	var gx := clampf(lx / unit, 0.0, side - 1.001)
	var gy := clampf(ly / unit, 0.0, side - 1.001)
	var i := int(gx)
	var j := int(gy)
	var tx := gx - i
	var ty := gy - j
	var a := j * side + i
	var ha := heights[a]
	var hb := heights[a + 1]
	var hc := heights[a + side]
	var hd := heights[a + side + 1]
	if tx >= ty:
		return ha + (hb - ha) * tx + (hd - hb) * ty
	return ha + (hd - hc) * tx + (hc - ha) * ty


## Couleurs par propriétaire (id de faction → Color), repli quand `set_province_colors`
## n'est pas appelé.
func set_owner_colors(colors: Dictionary) -> void:
	_owner_colors = colors
	if map_data != null:
		_build_faction_texture()
		material.set_shader_parameter("faction_colors", _faction_texture)


## Couleur explicite par province (`colors[index - 1]`, alpha 0 = neutre) : source de vérité
## quand `GameDataStore`/`CampaignSim` sont disponibles ; la palette de repli est ignorée.
func set_province_colors(colors: PackedColorArray) -> void:
	_province_colors = colors
	if map_data != null:
		_build_faction_texture()
		material.set_shader_parameter("faction_colors", _faction_texture)


## Masque 1D indexé par province : 1 = atteignable ce tour, 2 = sur le chemin prévisualisé.
## `reachable` et `path` sont des index raster. Un appel avec deux tableaux vides efface tout.
func set_reachable(reachable: PackedInt32Array, path: PackedInt32Array = PackedInt32Array()) -> void:
	if map_data == null or material == null:
		return
	var width := maxi(map_data.province_count + 1, 1)
	var image := Image.create(width, 1, false, Image.FORMAT_R8)
	image.fill(Color(0, 0, 0, 0))
	for index in reachable:
		if index > 0 and index < width:
			image.set_pixel(index, 0, Color(0.5, 0, 0))
	for index in path:
		if index > 0 and index < width:
			image.set_pixel(index, 0, Color(1.0, 0, 0))
	_mask_texture = ImageTexture.create_from_image(image)
	material.set_shader_parameter("province_mask", _mask_texture)
	material.set_shader_parameter("mask_enabled", not reachable.is_empty() or not path.is_empty())


## Brouillard de guerre (lot C1) : `visible` = index raster des provinces vues ; les autres
## sont voilées par le shader. `enabled` faux efface le voile.
func set_fog(enabled: bool, visible: PackedInt32Array) -> void:
	if map_data == null or material == null:
		return
	var width := maxi(map_data.province_count + 1, 1)
	var image := Image.create(width, 1, false, Image.FORMAT_R8)
	image.fill(Color(0, 0, 0, 0))
	for index in visible:
		if index > 0 and index < width:
			image.set_pixel(index, 0, Color(1.0, 0, 0))
	material.set_shader_parameter("fog_mask", ImageTexture.create_from_image(image))
	material.set_shader_parameter("fog_enabled", enabled)
	material.set_shader_parameter("fog_by_cell", false)


## Brouillard par case (lot M5a) : `cells` = texture de vue R8 de la simulation (255 = vu,
## bords doux), couvrant `size_px` pixels carte depuis l'origine. Remplace le masque par province.
func set_fog_cells(enabled: bool, cells: Texture2D, size_px: Vector2) -> void:
	if material == null:
		return
	material.set_shader_parameter("fog_cells", cells)
	material.set_shader_parameter("fog_cells_size", size_px)
	material.set_shader_parameter("fog_by_cell", enabled and cells != null)
	material.set_shader_parameter("fog_enabled", enabled)


func set_highlight(hovered_index: int, selected_index: int) -> void:
	if material == null:
		return
	material.set_shader_parameter("hovered_id", hovered_index)
	material.set_shader_parameter("selected_id", selected_index)


## Bascule LOD proche/lointain selon la distance caméra ; construit au plus
## `max_near_builds_per_frame` tuiles proches par appel. Avec `view_center` et une distance de
## rig sous `fine_distance`, les tuiles les plus proches du point visé passent en relief fin.
func update_lod(camera_position: Vector3, camera_distance: float = INF, view_center: Vector3 = Vector3.INF, fine_distance: float = 0.0) -> void:
	_lod_frame += 1
	if quadtree != null:
		_update_lod_quadtree(camera_position, camera_distance, view_center, fine_distance)
		return
	if _current_fine_step == 0:
		_current_fine_step = fine_step
	_current_fine_step = _select_fine_step(camera_distance)
	_collect_fine_jobs()
	var wanted_fine := _wanted_fine(camera_distance, view_center, fine_distance)
	_last_wanted_fine = wanted_fine
	var builds := 0
	var half := chunk_px * 0.5
	for i in _chunks.size():
		var chunk := _chunks[i]
		if wanted_fine.has(i) and _fine_cache.has(i):
			var entry: Dictionary = _fine_cache[i]
			entry["last_used"] = _lod_frame
			# Comparaison d'identité (pas `_is_near`) : rejoue l'affectation quand le maillage
			# en cache a changé (ex. reconstruction après changement de `fine_step` adaptatif),
			# même si la tuile était déjà au niveau fin.
			if chunk.mesh != entry["mesh"]:
				chunk.mesh = entry["mesh"]
				_is_near[i] = 2
				_emit_level_change(i)
			continue
		var center := chunk.position + Vector3(half, 0.0, half)
		var is_near := camera_position.distance_to(center) < near_distance or wanted_fine.has(i)
		if is_near and _is_near[i] != 1:
			if not _near_meshes.has(i):
				if builds >= max_near_builds_per_frame or (builds > 0 and not FrameBudget.has_time()):
					continue
				var built := _build_chunk(i % CHUNKS, i / CHUNKS, near_step)
				_near_meshes[i] = built["mesh"]
				_near_grids[i] = built["grid"]
				builds += 1
			chunk.mesh = _near_meshes[i]
			_is_near[i] = 1
			_emit_level_change(i)
		elif not is_near and _is_near[i] != 0:
			chunk.mesh = _far_meshes[i]
			_is_near[i] = 0
			_emit_level_change(i)
	for index in wanted_fine:
		if _fine_jobs.has(index) or _fine_jobs.size() >= max_fine_jobs:
			continue
		var stale: bool = _fine_cache.has(index) and int(_fine_cache_step.get(index, -1)) != _current_fine_step
		if not _fine_cache.has(index) or stale:
			_start_fine_job(index)
	_evict_fine()


## Pas de relief fin pour la distance caméra donnée (T2). Bande d'hystérésis
## `fine_step_switch_distance ± fine_step_hysteresis / 2` : le pas ne change pas tant que la
## distance reste dans la bande, pour éviter les allers-retours de reconstruction au bord du
## seuil ; en dehors de la bande, le pas correspond simplement à la distance (proche = fin).
func _select_fine_step(camera_distance: float) -> int:
	if not fine_step_auto:
		return fine_step
	var half := fine_step_hysteresis * 0.5
	if camera_distance <= fine_step_switch_distance - half:
		return fine_step_near
	if camera_distance >= fine_step_switch_distance + half:
		return fine_step_far
	return _current_fine_step


## Tuiles voulues en relief fin (les plus proches du point visé), triées par distance.
func _wanted_fine(camera_distance: float, view_center: Vector3, fine_distance: float) -> Array:
	var result: Array = []
	if not fine_enabled or not quality_fine or (_fine_tiles_dir == "" and quadtree == null) or view_center == Vector3.INF or camera_distance >= fine_distance:
		return result
	var center := Vector2(view_center.x, view_center.z)
	var radius := maxf(fine_radius, camera_distance * 1.2)
	var candidates: Array = []
	for i in _chunks.size():
		var rect := Rect2((i % CHUNKS) * chunk_px, (i / CHUNKS) * chunk_px, chunk_px, chunk_px)
		var nearest := Vector2(clampf(center.x, rect.position.x, rect.end.x), clampf(center.y, rect.position.y, rect.end.y))
		var d := nearest.distance_to(center)
		if d < radius:
			candidates.append([d, i])
	candidates.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for item in candidates.slice(0, max_fine_chunks):
		result.append(item[1])
	return result


# --- Relief streamé (lot ZG2) ----------------------------------------------------------


func _setup_quadtree() -> void:
	pyramid = null
	var manifest := pyramid_manifest_path
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-pyramid":
			return
		if arg.begins_with("--pyramid-dir="):
			manifest = arg.trim_prefix("--pyramid-dir=").path_join("relief_pyramid.json")
		if arg.begins_with("--qt-debug="):
			material.set_shader_parameter("qt_debug", int(arg.trim_prefix("--qt-debug=")))
	if not pyramid_enabled:
		return
	var relief := ReliefPyramid.new()
	# ZG7b : pyramide livrée à part (`MapPaths.relief_root_for`), sauf manifeste d'essai.
	var relief_root: String = preload("res://scripts/map/map_paths.gd").relief_root_for(map_data.map_dir)
	var tiles_override := relief_root.path_join("pyramid") if manifest == "" and relief_root != map_data.map_dir else ""
	if not relief.load_manifest(map_data.map_dir, manifest, tiles_override):
		return
	pyramid = relief
	quadtree = ReliefQuadtree.new()
	quadtree.name = "ReliefQuadtree"
	quadtree.max_pages = int(_quality.get("relief_pages", quadtree.max_pages))
	_apply_quadtree_quality()
	add_child(quadtree)
	quadtree.setup(pyramid, material, map_data, _chunk_bounds_m())
	quadtree.surface_changed.connect(_on_quadtree_surface_changed)
	for chunk in _chunks:
		chunk.visible = false
	print("TerrainBuilder: relief pyramid E1-E%d (%d tiles), streamed quadtree on" % [pyramid.max_level, pyramid.tile_count()])


## Bornes (min, max) en mètres des morceaux E0, depuis les maillages lointains (lissés : marges).
func _chunk_bounds_m() -> PackedVector2Array:
	var bounds := PackedVector2Array()
	bounds.resize(CHUNKS * CHUNKS)
	for i in bounds.size():
		var lo := 0.0
		var hi := 0.0
		if i < _far_grids.size():
			var heights: PackedFloat32Array = _far_grids[i]["heights"]
			lo = INF
			hi = -INF
			for h in heights:
				lo = minf(lo, h)
				hi = maxf(hi, h)
		# ZG8 : hauteurs cuites exagérées (y = s·(h + g·local), local ≤ h) : h ≥ y / (s·(1 + g)).
		lo = lo / MapData.HEIGHT_SCALE if lo < 0.0 else lo / (MapData.HEIGHT_SCALE * (1.0 + _baked_gain))
		hi /= MapData.HEIGHT_SCALE
		bounds[i] = Vector2(minf(lo, 0.0) - 150.0, hi + maxf(0.5 * (hi - lo), 200.0))
	return bounds


func _update_lod_quadtree(camera_position: Vector3, camera_distance: float, view_center: Vector3, fine_distance: float) -> void:
	# Vue stratégique parchemin (CM2) : le shader « parchemin seul » ne déplace pas les patchs,
	# on réaffiche alors les morceaux E0.
	var parchment := material.shader != TERRAIN_SHADER
	if quadtree.visible == parchment:
		quadtree.visible = not parchment
		for chunk in _chunks:
			chunk.visible = parchment
	var camera := get_viewport().get_camera_3d() if is_inside_tree() and not parchment else null
	quadtree.shadow_cast_distance = _relief_shadow_distance()
	var t0 := Time.get_ticks_usec()
	quadtree.update_view(camera)
	build_stats["qt_update_ms_max"] = maxf(float(build_stats.get("qt_update_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)
	var tp := PerfProbe.lap("lod/quadtree", t0)  # SZ6
	var wanted_fine := _wanted_fine(camera_distance, view_center, fine_distance)
	_last_wanted_fine = wanted_fine
	var half := chunk_px * 0.5
	# SZ6 : un zoom fait changer de niveau jusqu'à 20 morceaux dans la même image, et chaque
	# `chunk_surface_changed` recale colonies, ponts, routes… (jusqu'à 50 ms). Les plus proches de
	# la caméra d'abord, dans `level_emit_budget_ms` (au moins un par image) ; les autres gardent
	# leur niveau et sont repris aux images suivantes.
	var changes: Array = []
	for i in _chunks.size():
		var center := _chunks[i].position + Vector3(half, 0.0, half)
		var d := camera_position.distance_to(center)
		var level := 2 if wanted_fine.has(i) else (1 if d < near_distance else 0)
		if _is_near[i] != level:
			changes.append([d, i, level])
	if changes.size() > 1:
		changes.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var t_level := Time.get_ticks_usec()
	for k in changes.size():
		if k > 0 and FrameBudget.in_frame() and Time.get_ticks_usec() - t_level >= level_emit_budget_ms * 1000.0:
			break
		var i: int = changes[k][1]
		_is_near[i] = changes[k][2]
		_surface_dirty.erase(i)
		if _emitted_top.size() == CHUNKS * CHUNKS:
			_emitted_top[i] = quadtree.chunk_top(i)
		var t_emit := Time.get_ticks_usec()
		chunk_surface_changed.emit(i)
		_note_emit(t_emit, 1)
	tp = PerfProbe.lap("lod/level_emits", tp)
	_flush_surface_dirty(false)
	tp = PerfProbe.lap("lod/surface_flush", tp)
	_flush_rescale(view_center)
	PerfProbe.lap("lod/rescale_flush", tp)


func _on_quadtree_surface_changed(rect: Rect2) -> void:
	if chunk_px <= 0:
		return
	var c0 := clampi(int(floor(rect.position.x / chunk_px)), 0, CHUNKS - 1)
	var r0 := clampi(int(floor(rect.position.y / chunk_px)), 0, CHUNKS - 1)
	var c1 := clampi(int(floor(rect.end.x / chunk_px)), 0, CHUNKS - 1)
	var r1 := clampi(int(floor(rect.end.y / chunk_px)), 0, CHUNKS - 1)
	# Morceaux lointains ignorés : leurs objets sont recalés quand ils passent au niveau proche
	# (signal de changement de niveau). Ailleurs, un recalage seulement quand l'étage le plus fin
	# chargé du morceau change (E1 → E2 → …), pas à chaque tuile du même étage.
	surface_rect_changed.emit(rect)
	if _emitted_top.size() != CHUNKS * CHUNKS:
		_emitted_top.resize(CHUNKS * CHUNKS)
		_emitted_top.fill(-1)
	for r in range(r0, r1 + 1):
		for c in range(c0, c1 + 1):
			var index := r * CHUNKS + c
			if _is_near[index] >= 1 and (quadtree.chunk_top(index) != _emitted_top[index] or _surface_dirty.has(index)):
				_surface_dirty[index] = Time.get_ticks_msec()


## Signale les morceaux dont la surface a changé (pages), au plus `max_surface_emits_per_flush`
## toutes les `surface_flush_interval_ms` (tous avec `force`) : les recalages (routes, colonies,
## arbres) sont étalés au lieu de s'empiler dans une image.
func _flush_surface_dirty(force: bool) -> void:
	if _surface_dirty.is_empty():
		return
	var now := Time.get_ticks_msec()
	if not force and now - _surface_flush_ms < surface_flush_interval_ms:
		return
	_surface_flush_ms = now
	var dirty: Array = []
	for index: int in _surface_dirty:
		if force or now - int(_surface_dirty[index]) >= surface_settle_ms:
			dirty.append(index)
	if not force and dirty.size() > max_surface_emits_per_flush:
		dirty = dirty.slice(0, max_surface_emits_per_flush)
	var settled := dirty
	dirty = []
	for index: int in settled:
		_surface_dirty.erase(index)
		# Étage revenu à celui déjà signalé (page évincée puis rechargée) : rien à recaler.
		if force or quadtree.chunk_top(index) != _emitted_top[index]:
			_emitted_top[index] = quadtree.chunk_top(index)
			dirty.append(index)
	if dirty.is_empty():
		return
	var t0 := Time.get_ticks_usec()
	for index: int in dirty:
		chunk_surface_changed.emit(index)
	_note_emit(t0, dirty.size())
	build_stats["surface_page_emits"] = int(build_stats.get("surface_page_emits", 0)) + dirty.size()


## Changement de niveau d'un morceau (repli sans cache), mesuré comme les autres signaux.
func _emit_level_change(index: int) -> void:
	var t0 := Time.get_ticks_usec()
	chunk_surface_changed.emit(index)
	_note_emit(t0, 1)


## Mesure du coût des recalages déclenchés par `chunk_surface_changed` (écouteurs synchrones).
func _note_emit(t0: int, count: int) -> void:
	var ms := (Time.get_ticks_usec() - t0) / 1000.0
	build_stats["surface_emits"] = int(build_stats.get("surface_emits", 0)) + count
	build_stats["surface_emit_ms_max"] = maxf(float(build_stats.get("surface_emit_ms_max", 0.0)), ms)
	build_stats["surface_emit_ms_total"] = float(build_stats.get("surface_emit_ms_total", 0.0)) + ms


# --- Exagération verticale dynamique (lot ZG4) ------------------------------------------


## Change l'échelle verticale (`MapData.set_vertical_scale`, paramètre global des shaders) et
## recale les calques : `vertical_scale_changed` tout de suite (objets ponctuels, recalage bon
## marché), puis, l'échelle stable depuis `rescale_settle_ms`, `chunk_surface_changed` morceau par
## morceau, étalé sur les images suivantes (`rescale_budget_ms` par image, les morceaux les plus
## proches du point visé d'abord) avec
## `rescaling_vertical` vrai pendant l'émission (les calques dont la hauteur cuite est en mètres,
## comme `LandmarkModel`, l'ignorent). Sans quadtree (repli E0, maillages cuits à
## `HEIGHT_SCALE`), l'échelle ne change pas : rend faux.
func set_vertical_scale(value: float) -> bool:
	if quadtree == null or map_data == null:
		return false
	var old := MapData.vertical_scale()
	if not MapData.set_vertical_scale(value):
		return false
	quadtree.on_vertical_scale_changed(old, value)
	_rescale_queue.clear()
	for index in CHUNKS * CHUNKS:
		_rescale_queue[index] = true
	_rescale_changed_ms = Time.get_ticks_msec()
	build_stats["vertical_rescales"] = int(build_stats.get("vertical_rescales", 0)) + 1
	var t0 := Time.get_ticks_usec()
	vertical_scale_changed.emit(old, value)
	build_stats["vertical_signal_ms_max"] = maxf(float(build_stats.get("vertical_signal_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)
	return true


## Remet l'échelle stratégique (`HEIGHT_SCALE`) : nouvelle carte, tests.
func reset_vertical_scale() -> void:
	_rescale_queue.clear()
	if MapData.set_vertical_scale(MapData.HEIGHT_SCALE):
		for index in _chunks.size():
			chunk_surface_changed.emit(index)


## Morceaux restant à recaler après un changement d'échelle (tests, mesures).
func pending_rescales() -> int:
	return _rescale_queue.size()


## Émet les recalages en attente, une fois l'échelle stable depuis `rescale_settle_ms` (un zoom
## continu franchit une quinzaine de paliers : un seul recalage à l'arrêt au lieu de quinze) :
## morceaux proches (niveau ≥ 1) triés par distance au point visé, dans le budget de l'image (au
## moins un), puis les lointains (niveau 0, objets à peine visibles) au plus
## `max_far_rescales_per_frame` par image ; tout avec `force`.
func _flush_rescale(view_center: Vector3, force: bool = false) -> void:
	if _rescale_queue.is_empty():
		return
	if not force and Time.get_ticks_msec() - _rescale_changed_ms < rescale_settle_ms:
		return
	var order: Array = []
	var center := Vector2(view_center.x, view_center.z) if view_center != Vector3.INF else Vector2(2048.0, 2048.0)
	var half := chunk_px * 0.5
	for index: int in _rescale_queue:
		var c := Vector2((index % CHUNKS) * chunk_px + half, (index / CHUNKS) * chunk_px + half)
		var far_penalty := 0.0 if _is_near[index] >= 1 else 1.0e6
		order.append([far_penalty + c.distance_to(center), index])
	order.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var t0 := Time.get_ticks_usec()
	var emitted := 0
	var far_emitted := 0
	rescaling_vertical = true
	for entry in order:
		if not force and emitted > 0 and (Time.get_ticks_usec() - t0) / 1000.0 >= rescale_budget_ms:
			break
		var index: int = entry[1]
		if not force and _is_near[index] == 0:
			if far_emitted >= max_far_rescales_per_frame:
				break
			far_emitted += 1
		_rescale_queue.erase(index)
		chunk_surface_changed.emit(index)
		emitted += 1
	rescaling_vertical = false
	var ms := (Time.get_ticks_usec() - t0) / 1000.0
	build_stats["rescale_emits"] = int(build_stats.get("rescale_emits", 0)) + emitted
	build_stats["rescale_ms_total"] = float(build_stats.get("rescale_ms_total", 0.0)) + ms
	build_stats["rescale_ms_max"] = maxf(float(build_stats.get("rescale_ms_max", 0.0)), ms)


# --- Relief fin (tuiles 8192²) ---------------------------------------------------------


func _setup_fine_tiles() -> void:
	_fine_tiles_dir = ""
	var meta_path := map_data.map_dir.path_join("map.json")
	if not FileAccess.file_exists(meta_path):
		return
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(meta_path))
	if not (meta is Dictionary) or not (meta as Dictionary).has("height_tiles"):
		return
	var tiles: Dictionary = meta["height_tiles"]
	var size_px := int(tiles.get("size_px", 0))
	var tile_px := int(tiles.get("tile_px", 0))
	# Contrat : tuiles alignées sur les CHUNKS × CHUNKS tuiles de terrain.
	if tile_px <= 0 or size_px != tile_px * CHUNKS or size_px != 2 * maxi(map_data.size.x, map_data.size.y):
		push_warning("TerrainBuilder: height_tiles %s not aligned with the terrain chunks, fine relief disabled" % tiles)
		return
	var dir := map_data.map_dir.path_join(str(tiles.get("dir", "height")))
	if not DirAccess.dir_exists_absolute(dir):
		return
	_fine_tiles_dir = dir
	_fine_pattern = str(tiles.get("pattern", "h_{col}_{row}.png"))
	_fine_tile_px = tile_px
	if ClassDB.class_exists("GameDataStore"):
		_fine_store = ClassDB.instantiate("GameDataStore")
	_fine_indices = FineTerrainJob.build_indices(tile_px / fine_step + 1)


## Décodage de la tuile (fil principal : décodeur Rust, sinon `Png16`) puis tâche de maillage.
func _start_fine_job(index: int) -> void:
	var col := index % CHUNKS
	var row := index / CHUNKS
	var path := _fine_tiles_dir.path_join(_fine_pattern.replace("{col}", str(col)).replace("{row}", str(row)))
	var job := FineTerrainJob.new()
	job.tile_index = index
	if FileAccess.file_exists(path):
		var bytes := PackedByteArray()
		var little := true
		if _fine_store != null and _fine_store.has_method("load_heightmap_u16"):
			bytes = _fine_store.call("load_heightmap_u16", path)
			if bytes.size() != _fine_tile_px * _fine_tile_px * 2:
				bytes = PackedByteArray()
		if bytes.is_empty():
			var decoded := Png16.load_gray16(path)
			if not decoded.is_empty() and int(decoded["width"]) == _fine_tile_px:
				bytes = decoded["data"]
				little = false
		job.tile_bytes = bytes
		job.little_endian = little
	if job.tile_bytes.is_empty():
		# Tuile absente ou illisible : la tuile reste au LOD proche (pas de nouvel essai).
		if _near_meshes.has(index):
			_fine_cache[index] = {"mesh": _near_meshes[index], "grid": _near_grids.get(index, {}), "last_used": _lod_frame}
			_fine_cache_step[index] = _current_fine_step
		return
	job.origin_px = Vector2i(col * chunk_px, row * chunk_px)
	job.chunk_px = chunk_px
	job.step = _current_fine_step
	job.tile_side = _fine_tile_px
	job.h_min = map_data.height_min_m
	job.h_range = map_data.height_max_m - map_data.height_min_m
	job.height_scale = MapData.HEIGHT_SCALE
	job.relief_gain = _baked_gain
	job.map_bytes = map_data.height_bytes
	job.map_bpp = map_data.height_bpp
	job.map_little_endian = map_data.height_little_endian
	job.map_size = map_data.size
	job.edge_step = near_step
	var task := WorkerThreadPool.add_task(job.run, false, "fine terrain %d" % index)
	_fine_jobs[index] = {"task": task, "job": job}


## Récupère les tâches de relief fin terminées (toutes, en attendant, avec `block`).
func _collect_fine_jobs(block: bool = false) -> void:
	for index in _fine_jobs.keys():
		var item: Dictionary = _fine_jobs[index]
		if not block and not WorkerThreadPool.is_task_completed(item["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(item["task"])
		_fine_jobs.erase(index)
		var job: FineTerrainJob = item["job"]
		if not job.ok:
			continue
		if _fine_indices.size() != (job.side - 1) * (job.side - 1) * 6 + 4 * (job.side - 1) * 12:
			_fine_indices = FineTerrainJob.build_indices(job.side)
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = job.vertices
		arrays[Mesh.ARRAY_INDEX] = _fine_indices
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		var grid := {"heights": job.heights, "side": job.side, "unit": float(chunk_px) / float(job.side - 1)}
		_fine_cache[index] = {"mesh": mesh, "grid": grid, "last_used": _lod_frame}
		_fine_cache_step[index] = job.step
		build_stats["fine_build_ms_max"] = maxf(float(build_stats.get("fine_build_ms_max", 0.0)), job.build_ms)


## Vrai quand toutes les tuiles voulues en relief fin sont affichées (captures, mesures).
func fine_ready() -> bool:
	if quadtree != null:
		return quadtree.is_settled()
	for index in _last_wanted_fine:
		if _is_near[index] != 2 and not (_fine_cache.has(index) and _fine_cache[index]["mesh"] == _near_meshes.get(index)):
			return false
	return true


## Attend les tâches de relief fin en cours (captures, sortie).
## Les maillages sont installés au prochain `update_lod`.
func wait_fine_jobs() -> void:
	if quadtree != null:
		quadtree.wait_jobs(true)
		_flush_surface_dirty(true)
		_flush_rescale(Vector3.INF, true)
		return
	_collect_fine_jobs(true)


func _wait_fine_jobs() -> void:
	for item in _fine_jobs.values():
		WorkerThreadPool.wait_for_task_completion(item["task"])
	_fine_jobs.clear()


## Cache LRU : libère les maillages fins non affichés les plus anciens.
func _evict_fine() -> void:
	if _fine_cache.size() <= max_cached_fine:
		return
	var idle: Array = []
	for index in _fine_cache:
		if _is_near[index] != 2:
			idle.append([int(_fine_cache[index]["last_used"]), index])
	idle.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	for i in mini(_fine_cache.size() - max_cached_fine, idle.size()):
		_fine_cache.erase(idle[i][1])
		_fine_cache_step.erase(idle[i][1])


# --- Relief exagéré (lot ZG8) -------------------------------------------------------------


## Fond de vallée lissé (`ReliefFloor`), publié aux shaders et à `MapData.display_height` ; profil
## désactivé : aucun fond, gain nul (comportement ZG4). Gain des maillages cuits (E0, repli) : celui
## de l'échelle stratégique.
func _build_relief_floor() -> void:
	var relief := ReliefExaggerationProfile.load_default()
	_relief_floor_ms = 0.0
	if relief.enabled:
		var grid := ReliefFloor.compute(map_data, relief)
		_relief_floor_ms = float(grid["ms"])
		MapData.set_relief_floor(grid)
	else:
		MapData.set_relief_floor({})
	_baked_gain = MapData.relief_gain_for_scale(MapData.HEIGHT_SCALE)


## Soleil plus rasant (ZG8, `sun_elevation_deg`) : même azimut, hauteur imposée.
func _apply_sun_elevation() -> void:
	var relief := ReliefExaggerationProfile.load_default()
	var sun := get_parent().get_node_or_null("Sun") as DirectionalLight3D if get_parent() != null else null
	if sun == null or not relief.enabled or relief.sun_elevation_deg <= 0.0:
		return
	var toward_sun := sun.global_basis.z.normalized() if sun.is_inside_tree() else sun.basis.z.normalized()
	var flat := Vector2(toward_sun.x, toward_sun.z)
	if flat.length() < 1e-4:
		return
	flat = flat.normalized()
	var elevation := deg_to_rad(relief.sun_elevation_deg)
	var target := Vector3(flat.x * cos(elevation), sin(elevation), flat.y * cos(elevation))
	sun.basis = Basis.looking_at(-target, Vector3.UP)


func _build_textures() -> void:
	_height_texture = ImageTexture.create_from_image(_height_image_for_gpu())
	_ids_texture = ImageTexture.create_from_image(map_data.province_ids_image)
	_splat_texture = _optional_texture(map_data.splat_image)
	_border_texture = _optional_texture(map_data.border_dist_image)
	_coast_texture = _optional_texture(map_data.coast_dist_image)
	_river_bed_texture = _optional_texture(map_data.river_bed_image)
	_build_material_arrays()
	_build_faction_texture()


## Hauteur pour le GPU : `FORMAT_R16` (entier 16 bits normalisé) quand les octets sont
## little-endian (décodeur Rust) : le filtrage bilinéaire matériel est alors exact, contrairement
## à LA8 où octets fort et faible sont interpolés séparément (stries au dézoom). Mipmaps pour
## que l'ombrage lointain moyenne le relief au lieu de l'échantillonner (aliasing).
func _height_image_for_gpu() -> Image:
	var image: Image
	if map_data.height_bpp == 2 and map_data.height_little_endian:
		image = Image.create_from_data(map_data.size.x, map_data.size.y, false, Image.FORMAT_R16, map_data.height_bytes)
	else:
		image = map_data.height_image.duplicate()
	if map_data.height_bpp == 1 or image.get_format() == Image.FORMAT_R16:
		image.generate_mipmaps()
	_smooth_bytes = PackedByteArray()
	_smooth_size = Vector2i.ZERO
	if image.get_format() == Image.FORMAT_R16 and image.get_mipmap_count() >= 2:
		var level_size := image.get_size() / 4
		var start := image.get_mipmap_offset(2)
		_smooth_bytes = image.get_data().slice(start, start + level_size.x * level_size.y * 2)
		_smooth_size = level_size
	return image


## Mode de décodage de la heightmap dans le shader : 1 = canal R normalisé (R16 ou L8),
## 2 = LA8 big-endian (repli Png16, sans mipmaps).
func height_texture_mode() -> int:
	if _height_texture != null and _height_texture.get_format() == Image.FORMAT_LA8:
		return 2
	return 1


func height_texture() -> Texture2D:
	return _height_texture


func coast_texture() -> Texture2D:
	return _coast_texture


## Lot V4 : texture du lit des fleuves (null si absente) ; `RiversRenderer` la partage avec l'eau
## et y efface le lit sous les villes fortifiées (`update_river_bed`).
func river_bed_texture() -> ImageTexture:
	return _river_bed_texture


## Rectangles (x0, z0, x1, z1) des tuiles affichées en relief fin (niveau 2).
func fine_chunk_rects() -> PackedVector4Array:
	var rects := PackedVector4Array()
	for index in _is_near.size():
		if _is_near[index] == 2:
			var cx := index % CHUNKS
			var cy := index / CHUNKS
			rects.append(Vector4(cx * chunk_px, cy * chunk_px, (cx + 1) * chunk_px, (cy + 1) * chunk_px))
	return rects


static func _optional_texture(image: Image) -> ImageTexture:
	if image == null:
		return null
	return ImageTexture.create_from_image(image)


## Deux Texture2DArray (albédo ; normale XY + rugosité) depuis les JPEG Poly Haven importés.
## Moyenne linéaire de chaque albédo (dernier niveau de mipmap) : le shader s'en sert pour
## teinter les textures vers des couleurs réalistes réglables sans perdre leur détail.
func _build_material_arrays() -> void:
	var albedo_images: Array[Image] = []
	var normal_images: Array[Image] = []
	_layer_means = PackedVector3Array()
	for layer in MATERIAL_LAYERS:
		var albedo := _load_layer_image(TEXTURE_DIR + layer + "_albedo.jpg")
		var normal := _load_layer_image(TEXTURE_DIR + layer + "_normal_rough.jpg")
		if albedo == null or normal == null:
			push_warning("TerrainBuilder: missing texture layer %s, terrain textures disabled" % layer)
			_albedo_array = null
			_normal_array = null
			return
		albedo_images.append(albedo)
		normal_images.append(normal)
		var last := albedo.get_mipmap_count()
		var offset := albedo.get_mipmap_offset(last)
		var data := albedo.get_data()
		var mean := Color8(data[offset], data[offset + 1], data[offset + 2]).srgb_to_linear()
		_layer_means.append(Vector3(mean.r, mean.g, mean.b))
	_albedo_array = Texture2DArray.new()
	_albedo_array.create_from_images(albedo_images)
	_normal_array = Texture2DArray.new()
	_normal_array.create_from_images(normal_images)


static func _load_layer_image(path: String) -> Image:
	if not ResourceLoader.exists(path):
		return null
	var texture := load(path) as Texture2D
	if texture == null:
		return null
	var image := texture.get_image()
	if image == null:
		return null
	if image.is_compressed():
		image.decompress()
	if image.get_format() != Image.FORMAT_RGBA8:
		image.convert(Image.FORMAT_RGBA8)
	if image.get_size() != Vector2i(1024, 1024):
		image.resize(1024, 1024, Image.INTERPOLATE_LANCZOS)
	image.generate_mipmaps()
	return image


func _build_faction_texture() -> void:
	var width := maxi(map_data.province_count + 1, 1)
	var image := Image.create(width, 1, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	var palette_by_owner: Dictionary = {}
	for index in range(1, map_data.province_count + 1):
		var province: Dictionary = map_data.get_province(index)
		if province.is_empty():
			continue
		var owner: String = province.get("owner", "")
		var color: Color
		if index - 1 < _province_colors.size():
			color = _province_colors[index - 1]
		elif _owner_colors.has(owner):
			color = _owner_colors[owner]
			color.a = 1.0
		else:
			if not palette_by_owner.has(owner):
				palette_by_owner[owner] = FALLBACK_PALETTE[palette_by_owner.size() % FALLBACK_PALETTE.size()]
			color = palette_by_owner[owner]
			color.a = 1.0 if owner != "" else 0.0
		image.set_pixel(index, 0, color)
	_faction_texture = ImageTexture.create_from_image(image)


func _build_material() -> void:
	material = ShaderMaterial.new()
	material.shader = TERRAIN_SHADER
	material.set_shader_parameter("heightmap", _height_texture)
	material.set_shader_parameter("height_bpp", height_texture_mode())
	material.set_shader_parameter("height_little_endian", map_data.height_little_endian)
	material.set_shader_parameter("height_min_m", map_data.height_min_m)
	material.set_shader_parameter("height_max_m", map_data.height_max_m)
	material.set_shader_parameter("map_size", Vector2(map_data.size))
	material.set_shader_parameter("meters_per_px", map_data.meters_per_px)
	material.set_shader_parameter("province_ids", _ids_texture)
	material.set_shader_parameter("faction_colors", _faction_texture)
	material.set_shader_parameter("splat", _splat_texture)
	material.set_shader_parameter("has_splat", _splat_texture != null)
	material.set_shader_parameter("border_dist", _border_texture)
	material.set_shader_parameter("has_border_dist", _border_texture != null)
	material.set_shader_parameter("coast_dist", _coast_texture)
	material.set_shader_parameter("has_coast_dist", _coast_texture != null)
	# Lot V4 : lit des fleuves (les tuiles creusées sont réglées par RiversRenderer).
	material.set_shader_parameter("river_bed", _river_bed_texture)
	material.set_shader_parameter("has_river_bed", _river_bed_texture != null)
	# Occupation du sol (vigne, sécheresse, bocage) partagée avec la végétation (lot V2b).
	_landuse_texture = ImageTexture.create_from_image(VegetationFields.landuse(map_data))
	material.set_shader_parameter("landuse", _landuse_texture)
	material.set_shader_parameter("has_landuse", true)
	material.set_shader_parameter("has_textures", _albedo_array != null)
	ReliefLandcover.apply(material, map_data)  # lot R1 : relief fin, zones humides
	# ZG8 : roche sur les falaises du relief exagéré (désactivée avec le profil).
	var relief := ReliefExaggerationProfile.load_default()
	material.set_shader_parameter("cliff_slope_start", relief.cliff_slope_start if relief.enabled else 0.0)
	material.set_shader_parameter("cliff_slope_full", relief.cliff_slope_full if relief.enabled else 0.0)
	if _albedo_array != null:
		material.set_shader_parameter("albedo_array", _albedo_array)
		material.set_shader_parameter("normal_rough_array", _normal_array)
		material.set_shader_parameter("layer_mean", _layer_means)


## Maillage d'une tuile et grille de ses hauteurs : {"mesh": ArrayMesh, "grid": Dictionary}.
## PB1 : hauteurs prises dans les sommets calculés (plus de relecture `surface_get_arrays` du
## maillage) et indices partagés par toutes les tuiles de même pas.
func _build_chunk(cx: int, cy: int, step: int) -> Dictionary:
	var heights := PackedFloat32Array()
	var vertices := _chunk_vertices(cx, cy, step, heights)
	return _chunk_from(vertices, heights, step)


func _chunk_from(vertices: PackedVector3Array, heights: PackedFloat32Array, step: int) -> Dictionary:
	var quads := ceili(float(chunk_px) / step)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = _grid_indices(quads)
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return {"mesh": mesh, "grid": {"heights": heights, "side": quads + 1, "unit": float(step)}}


## Indices d'une grille `quads` × `quads` (face avant = sens horaire vu du dessus, convention
## Godot), mis en cache par taille.
func _grid_indices(quads: int) -> PackedInt32Array:
	if _grid_indices_cache.has(quads):
		return _grid_indices_cache[quads]
	var side := quads + 1
	var indices := PackedInt32Array()
	indices.resize(quads * quads * 6)
	var k := 0
	for j in quads:
		for i in quads:
			var a := j * side + i
			var b := a + 1
			var c := a + side
			var d := c + 1
			indices[k] = a
			indices[k + 1] = b
			indices[k + 2] = d
			indices[k + 3] = a
			indices[k + 4] = d
			indices[k + 5] = c
			k += 6
	_grid_indices_cache[quads] = indices
	return indices


## Sommets d'une tuile en coordonnées locales (origine = coin nord-ouest de la tuile) ; remplit
## `heights` avec leurs hauteurs (grille de `surface_height_at`).
func _chunk_vertices(cx: int, cy: int, step: int, heights: PackedFloat32Array) -> PackedVector3Array:
	var x0 := cx * chunk_px
	var y0 := cy * chunk_px
	var quads := ceili(float(chunk_px) / step)
	var side := quads + 1
	var width := map_data.size.x
	var height := map_data.size.y
	var bytes := map_data.height_bytes
	var bpp := map_data.height_bpp
	var little_endian := map_data.height_little_endian
	var h_min := map_data.height_min_m
	var h_range := map_data.height_max_m - map_data.height_min_m
	var scale := MapData.HEIGHT_SCALE
	# ZG8 : maillages cuits à l'échelle stratégique, relief local exagéré compris (gain lointain).
	var gain := _baked_gain
	var vertices := PackedVector3Array()
	vertices.resize(side * side)
	heights.resize(side * side)
	# LOD lointain : bilinéaire dans le mipmap 4×4 (filtre ≈ 8 px), si le pas est multiple de 4.
	var smooth := step >= far_step and step % 4 == 0 and not _smooth_bytes.is_empty()
	var sw := _smooth_size.x
	var sh := _smooth_size.y
	var k := 0
	for j in side:
		var py := mini(y0 + j * step, height - 1)
		var row := py * width
		for i in side:
			var px := mini(x0 + i * step, width - 1)
			var v01: float
			if smooth:
				# Centre du texel mip k = 4k + 1,5 : pour px ≡ 0 (mod 4), poids 0,375 / 0,625.
				var kx0 := clampi(px / 4 - 1, 0, sw - 1)
				var kx1 := clampi(px / 4, 0, sw - 1)
				var ky0 := clampi(py / 4 - 1, 0, sh - 1)
				var ky1 := clampi(py / 4, 0, sh - 1)
				var a := _smooth_bytes.decode_u16((ky0 * sw + kx0) * 2)
				var b := _smooth_bytes.decode_u16((ky0 * sw + kx1) * 2)
				var c := _smooth_bytes.decode_u16((ky1 * sw + kx0) * 2)
				var d := _smooth_bytes.decode_u16((ky1 * sw + kx1) * 2)
				v01 = ((a * 0.375 + b * 0.625) * 0.375 + (c * 0.375 + d * 0.625) * 0.625) / 65535.0
			elif bpp == 2:
				var o := (row + px) * 2
				if little_endian:
					v01 = float(bytes[o] | (bytes[o + 1] << 8)) / 65535.0
				else:
					v01 = float((bytes[o] << 8) | bytes[o + 1]) / 65535.0
			else:
				v01 = float(bytes[row + px]) / 255.0
			var y := MapData.display_height_with(h_min + v01 * h_range, px, py, scale, gain)
			vertices[k] = Vector3(px - x0, y, py - y0)
			heights[k] = y
			k += 1
	return vertices


static func chunk_px_for(map_size: Vector2i) -> int:
	return ceili(float(maxi(map_size.x, map_size.y)) / CHUNKS)
