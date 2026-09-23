class_name TerrainBuilder
extends Node3D

## Terrain de campagne : grille de CHUNKS × CHUNKS tuiles (MeshInstance3D) construites
## depuis la heightmap. Deux niveaux de détail : le LOD lointain (`far_step`) est
## construit au chargement ; le LOD proche (`near_step`) est construit à la demande
## quand la caméra s'approche, puis mis en cache.
##
## Les normales ne sont pas stockées dans le maillage : le shader les dérive de la
## heightmap (aucune couture entre tuiles ni entre LOD).

const CHUNKS := 16
const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")

## Pas (en pixels) entre deux sommets pour le LOD proche / lointain.
@export var near_step: int = 4
@export var far_step: int = 8
## Distance caméra → centre de tuile en dessous de laquelle le LOD proche est utilisé.
@export var near_distance: float = 600.0
@export var max_near_builds_per_frame: int = 4

var map_data: MapData
var material: ShaderMaterial
var chunk_px: int = 0
var build_stats: Dictionary = {}

var _chunks: Array[MeshInstance3D] = []
var _far_meshes: Array[ArrayMesh] = []
var _near_meshes: Dictionary = {}
var _is_near: PackedByteArray = PackedByteArray()
var _height_texture: ImageTexture
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
			var mesh := _build_chunk_mesh(cx, cy, far_step)
			vertex_count += mesh.surface_get_array_len(0)
			var instance := MeshInstance3D.new()
			instance.name = "Chunk_%d_%d" % [cx, cy]
			instance.position = Vector3(cx * chunk_px, 0.0, cy * chunk_px)
			instance.mesh = mesh
			instance.material_override = material
			add_child(instance)
			_chunks.append(instance)
			_far_meshes.append(mesh)
	build_stats = {
		"chunks": _chunks.size(),
		"far_vertices": vertex_count,
		"far_step": far_step,
		"near_step": near_step,
		"build_ms": Time.get_ticks_msec() - t0,
	}


func clear_terrain() -> void:
	for chunk in _chunks:
		chunk.queue_free()
	_chunks.clear()
	_far_meshes.clear()
	_near_meshes.clear()


func chunk_count() -> int:
	return _chunks.size()


func near_chunk_count() -> int:
	return _is_near.count(1)


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
## `max_near_builds_per_frame` tuiles proches par appel.
func update_lod(camera_position: Vector3) -> void:
	var builds := 0
	var half := chunk_px * 0.5
	for i in _chunks.size():
		var chunk := _chunks[i]
		var center := chunk.position + Vector3(half, 0.0, half)
		var is_near := camera_position.distance_to(center) < near_distance
		if is_near and _is_near[i] == 0:
			if not _near_meshes.has(i):
				if builds >= max_near_builds_per_frame:
					continue
				_near_meshes[i] = _build_chunk_mesh(i % CHUNKS, i / CHUNKS, near_step)
				builds += 1
			chunk.mesh = _near_meshes[i]
			_is_near[i] = 1
		elif not is_near and _is_near[i] == 1:
			chunk.mesh = _far_meshes[i]
			_is_near[i] = 0


func _build_textures() -> void:
	_height_texture = ImageTexture.create_from_image(map_data.height_image)
	_ids_texture = ImageTexture.create_from_image(map_data.province_ids_image)
	_build_faction_texture()


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
	material.set_shader_parameter("height_bpp", map_data.height_bpp)
	material.set_shader_parameter("height_little_endian", map_data.height_little_endian)
	material.set_shader_parameter("height_min_m", map_data.height_min_m)
	material.set_shader_parameter("height_max_m", map_data.height_max_m)
	material.set_shader_parameter("height_scale", MapData.HEIGHT_SCALE)
	material.set_shader_parameter("map_size", Vector2(map_data.size))
	material.set_shader_parameter("province_ids", _ids_texture)
	material.set_shader_parameter("faction_colors", _faction_texture)


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
	var k := 0
	for j in side:
		var py := mini(y0 + j * step, height - 1)
		var row := py * width
		for i in side:
			var px := mini(x0 + i * step, width - 1)
			var v01: float
			if bpp == 2:
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
