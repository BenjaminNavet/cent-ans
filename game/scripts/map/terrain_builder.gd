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

signal chunk_surface_changed(index: int)

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
@export var fine_step: int = 1
@export var max_fine_chunks: int = 4
@export var fine_radius: float = 150.0
@export var max_fine_jobs: int = 2
@export var max_cached_fine: int = 10
@export var fine_enabled: bool = true

var map_data: MapData
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
## Relief fin : index → {"mesh", "grid", "last_used"} ; tâches en cours : index → {"task", "job"}.
var _fine_cache: Dictionary = {}
var _fine_jobs: Dictionary = {}
var _fine_indices: PackedInt32Array = PackedInt32Array()
var _fine_tiles_dir: String = ""
var _fine_pattern: String = ""
var _fine_tile_px: int = 0
var _fine_store: Object = null
var _lod_frame: int = 0
var _height_texture: ImageTexture
var _splat_texture: ImageTexture
var _border_texture: ImageTexture
var _coast_texture: ImageTexture
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


## Palette de repli (couleurs héraldiques), utilisée seulement sans `GameDataStore`
## (ex. fixtures de test) ; sinon `set_province_colors` fournit les vraies couleurs.
const FALLBACK_PALETTE: Array[Color] = [
	Color(0.20, 0.32, 0.75), Color(0.78, 0.18, 0.18), Color(0.85, 0.65, 0.15),
	Color(0.25, 0.55, 0.30), Color(0.55, 0.25, 0.60), Color(0.85, 0.45, 0.15),
	Color(0.20, 0.60, 0.65), Color(0.60, 0.50, 0.30), Color(0.75, 0.30, 0.50),
	Color(0.35, 0.35, 0.35), Color(0.45, 0.70, 0.25), Color(0.90, 0.80, 0.55),
]


func build(data: MapData) -> void:
	var t0 := Time.get_ticks_msec()
	clear_terrain()
	map_data = data
	chunk_px = ceili(float(maxi(data.size.x, data.size.y)) / CHUNKS)
	_build_textures()
	_build_material()
	_is_near.resize(CHUNKS * CHUNKS)
	_is_near.fill(0)
	var vertex_count := 0
	for cy in CHUNKS:
		for cx in CHUNKS:
			var built := _build_chunk(cx, cy, far_step)
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
	build_stats = {
		"fine_tiles": _fine_tiles_dir != "",
		"chunks": _chunks.size(),
		"far_vertices": vertex_count,
		"far_step": far_step,
		"near_step": near_step,
		"build_ms": Time.get_ticks_msec() - t0,
	}


func clear_terrain() -> void:
	_wait_fine_jobs()
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
	var index := chunk_index_at(x, y)
	if index < 0:
		return map_data.surface_world_at(x, y)
	var grid: Dictionary = _grid_for(index)
	if grid.is_empty():
		return map_data.surface_world_at(x, y)
	return maxf(grid_height(grid, x - (index % CHUNKS) * chunk_px, y - (index / CHUNKS) * chunk_px), 0.0)


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
static func grid_height(grid: Dictionary, lx: float, ly: float) -> float:
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
	_collect_fine_jobs()
	var wanted_fine := _wanted_fine(camera_distance, view_center, fine_distance)
	var builds := 0
	var half := chunk_px * 0.5
	for i in _chunks.size():
		var chunk := _chunks[i]
		if wanted_fine.has(i) and _fine_cache.has(i):
			var entry: Dictionary = _fine_cache[i]
			entry["last_used"] = _lod_frame
			if _is_near[i] != 2:
				chunk.mesh = entry["mesh"]
				_is_near[i] = 2
				chunk_surface_changed.emit(i)
			continue
		var center := chunk.position + Vector3(half, 0.0, half)
		var is_near := camera_position.distance_to(center) < near_distance or wanted_fine.has(i)
		if is_near and _is_near[i] != 1:
			if not _near_meshes.has(i):
				if builds >= max_near_builds_per_frame:
					continue
				var built := _build_chunk(i % CHUNKS, i / CHUNKS, near_step)
				_near_meshes[i] = built["mesh"]
				_near_grids[i] = built["grid"]
				builds += 1
			chunk.mesh = _near_meshes[i]
			_is_near[i] = 1
			chunk_surface_changed.emit(i)
		elif not is_near and _is_near[i] != 0:
			chunk.mesh = _far_meshes[i]
			_is_near[i] = 0
			chunk_surface_changed.emit(i)
	for index in wanted_fine:
		if not _fine_cache.has(index) and not _fine_jobs.has(index) and _fine_jobs.size() < max_fine_jobs:
			_start_fine_job(index)
	_evict_fine()


## Tuiles voulues en relief fin (les plus proches du point visé), triées par distance.
func _wanted_fine(camera_distance: float, view_center: Vector3, fine_distance: float) -> Array:
	var result: Array = []
	if not fine_enabled or _fine_tiles_dir == "" or view_center == Vector3.INF or camera_distance >= fine_distance:
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
		return
	job.origin_px = Vector2i(col * chunk_px, row * chunk_px)
	job.chunk_px = chunk_px
	job.step = fine_step
	job.tile_side = _fine_tile_px
	job.h_min = map_data.height_min_m
	job.h_range = map_data.height_max_m - map_data.height_min_m
	job.height_scale = MapData.HEIGHT_SCALE
	job.map_bytes = map_data.height_bytes
	job.map_bpp = map_data.height_bpp
	job.map_little_endian = map_data.height_little_endian
	job.map_size = map_data.size
	job.edge_step = near_step
	var task := WorkerThreadPool.add_task(job.run, false, "fine terrain %d" % index)
	_fine_jobs[index] = {"task": task, "job": job}


func _collect_fine_jobs() -> void:
	for index in _fine_jobs.keys():
		var item: Dictionary = _fine_jobs[index]
		if not WorkerThreadPool.is_task_completed(item["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(item["task"])
		_fine_jobs.erase(index)
		var job: FineTerrainJob = item["job"]
		if not job.ok:
			continue
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = job.vertices
		arrays[Mesh.ARRAY_INDEX] = _fine_indices
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		var grid := {"heights": job.heights, "side": job.side, "unit": float(chunk_px) / float(job.side - 1)}
		_fine_cache[index] = {"mesh": mesh, "grid": grid, "last_used": _lod_frame}
		build_stats["fine_build_ms_max"] = maxf(float(build_stats.get("fine_build_ms_max", 0.0)), job.build_ms)


## Attend les tâches de relief fin en cours (captures, sortie).
func wait_fine_jobs() -> void:
	for index in _fine_jobs.keys():
		WorkerThreadPool.wait_for_task_completion(_fine_jobs[index]["task"])


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


func _build_textures() -> void:
	_height_texture = ImageTexture.create_from_image(_height_image_for_gpu())
	_ids_texture = ImageTexture.create_from_image(map_data.province_ids_image)
	_splat_texture = _optional_texture(map_data.splat_image)
	_border_texture = _optional_texture(map_data.border_dist_image)
	_coast_texture = _optional_texture(map_data.coast_dist_image)
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
	material.set_shader_parameter("height_scale", MapData.HEIGHT_SCALE)
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
	# Occupation du sol (vigne, sécheresse, bocage) partagée avec la végétation (lot V2b).
	_landuse_texture = ImageTexture.create_from_image(VegetationFields.landuse(map_data))
	material.set_shader_parameter("landuse", _landuse_texture)
	material.set_shader_parameter("has_landuse", true)
	material.set_shader_parameter("has_textures", _albedo_array != null)
	if _albedo_array != null:
		material.set_shader_parameter("albedo_array", _albedo_array)
		material.set_shader_parameter("normal_rough_array", _normal_array)
		material.set_shader_parameter("layer_mean", _layer_means)


## Maillage d'une tuile et grille de ses hauteurs : {"mesh": ArrayMesh, "grid": Dictionary}.
func _build_chunk(cx: int, cy: int, step: int) -> Dictionary:
	var mesh := _build_chunk_mesh(cx, cy, step)
	var vertices: PackedVector3Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
	var heights := PackedFloat32Array()
	heights.resize(vertices.size())
	for k in vertices.size():
		heights[k] = vertices[k].y
	var side := ceili(float(chunk_px) / step) + 1
	return {"mesh": mesh, "grid": {"heights": heights, "side": side, "unit": float(step)}}


## Maillage d'une tuile en coordonnées locales (origine = coin nord-ouest de la tuile).
## Face avant = sens horaire vu du dessus (convention Godot).
func _build_chunk_mesh(cx: int, cy: int, step: int) -> ArrayMesh:
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
	var vertices := PackedVector3Array()
	vertices.resize(side * side)
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
			vertices[k] = Vector3(px - x0, (h_min + v01 * h_range) * scale, py - y0)
			k += 1
	var indices := PackedInt32Array()
	indices.resize(quads * quads * 6)
	k = 0
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
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


static func chunk_px_for(map_size: Vector2i) -> int:
	return ceili(float(maxi(map_size.x, map_size.y)) / CHUNKS)
