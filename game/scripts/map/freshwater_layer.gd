class_name FreshwaterLayer
extends Node3D

## Lot DN-ME4 : eaux douces et zones humides de la carte de campagne (rendu seulement).
##
## - Roselières et mares : `data/map/freshwater_px.json` (sortie de `cent-ans geo freshwater-sites`,
##   ellipses de `wetlands.json` en pixels) et réglages `data/map/map_freshwater.json`. Deux
##   niveaux de blocs semés autour du point visé : « patch » (grosses touffes grossies par la
##   distance, vue moyenne) et « près » (touffes à l'échelle réelle, dense). Un `MultiMesh` par
##   bloc et par famille (touffe, mare), jamais un nœud par instance. Lagunes (Camargue, deltas) :
##   roselière sur les rives seulement, eau peu profonde turquoise-grise au centre.
## - Salins : bassins rectangulaires en maillage unique, teinte rose ou grise.
## - Torrents : rubans d'écume sur les tronçons de forte pente (écoulement animé par shader).
## - Cascades et gués : sites historiques ; glb de `dn_manifest.json` (ids du catalogue) sinon
##   substitut procédural. Brancher un modèle = une ligne de données.
## Rien sous la vue parchemin (fondu `fade_start` → `fade_end`). `--no-freshwater` : A/B.
## Purement visuel : aucune règle de jeu.

const CONFIG_FILE := "map/map_freshwater.json"
const SITES_FILE := "map/freshwater_px.json"
const MANIFEST_FILE := "art/dn_manifest.json"
const MODEL_ROOT := "res://assets/models/"
const REED_SHADER := preload("res://shaders/freshwater_reeds.gdshader")
const POOL_SHADER := preload("res://shaders/freshwater_pool.gdshader")
const SURFACE_SHADER := preload("res://shaders/freshwater_surface.gdshader")
const FLOATS := 16
const METERS_PER_UNIT := 719.0
const LEVEL_PATCH := 0
const LEVEL_NEAR := 1

@export var camera_rig_path: NodePath = ^"../CameraRig"

var enabled: bool = true
var map_data: MapData
var terrain: TerrainBuilder
var config: Dictionary = {}
var sites: Dictionary = {}
var render: Dictionary = {}
var exclusions: PackedVector3Array = PackedVector3Array()
var stats: Dictionary = {"blocks": 0, "reeds": 0, "pools": 0, "seed_ms_max": 0.0}

var _rig: Node3D
var _blocks: Dictionary = {}  # "level:x:y" → {"nodes": Array, "last_seen": int, "reeds": int, "pools": int}
var _dirty: Array = []
var _frame := 0
var _reed_materials: Array[ShaderMaterial] = []
var _pool_materials: Array[ShaderMaterial] = []
var _surface_materials: Array[ShaderMaterial] = []
var _reed_mesh: ArrayMesh
var _pool_mesh: ArrayMesh
var _static_root: Node3D
var _manifest: Dictionary = {}
var _total_instances := 0


func _ready() -> void:
	_rig = get_node_or_null(camera_rig_path) as Node3D
	if CmdArgs.has("--no-freshwater"):
		enabled = false


func setup(data: MapData, terrain_builder: TerrainBuilder, exclusion_circles := PackedVector3Array(), config_override := {}, sites_override := {}) -> void:
	clear()
	map_data = data
	exclusions = exclusion_circles
	config = config_override if not config_override.is_empty() else _read(CONFIG_FILE)
	sites = sites_override if not sites_override.is_empty() else _read(SITES_FILE)
	render = config.get("render", {})
	_manifest = _read(MANIFEST_FILE).get("assets", {})
	if terrain != null and terrain.surface_rect_changed.is_connected(_on_surface_rect_changed):
		terrain.surface_rect_changed.disconnect(_on_surface_rect_changed)
	terrain = terrain_builder
	if terrain != null:
		terrain.surface_rect_changed.connect(_on_surface_rect_changed)
	if config.is_empty() or sites.is_empty():
		enabled = false
		return
	_apply_glacier()
	_build_meshes()
	_build_materials()
	_build_static()


## Névés et glaciers : réglages `glacier` poussés dans le matériau du terrain (shader freshwater_ground).
func _apply_glacier() -> void:
	if terrain == null or terrain.material == null:
		return
	var glacier: Dictionary = config.get("glacier", {})
	if glacier.is_empty():
		return
	terrain.material.set_shader_parameter("fw_glacier", Vector4(float(glacier["line_m"]), float(glacier["full_m"]), float(glacier["max_slope"]), 1.0))
	var ice: Array = glacier["ice_color"]
	var crevasse: Array = glacier["crevasse_color"]
	terrain.material.set_shader_parameter("fw_ice_color", Vector3(ice[0], ice[1], ice[2]))
	terrain.material.set_shader_parameter("fw_crevasse_color", Vector3(crevasse[0], crevasse[1], crevasse[2]))


func clear() -> void:
	for key: String in _blocks.keys():
		_free_block(key)
	_blocks.clear()
	_dirty.clear()
	if _static_root != null:
		_static_root.queue_free()
		_static_root = null
	_reed_materials.clear()
	_pool_materials.clear()
	_surface_materials.clear()
	_total_instances = 0


static func _read(rel_path: String) -> Dictionary:
	var parsed: Variant = DataFile.read_json(rel_path)
	return parsed if parsed is Dictionary else {}


func active() -> bool:
	return enabled and map_data != null


# --- Géométrie commune ---------------------------------------------------------------------

func _build_meshes() -> void:
	# Deux plans croisés de 1 × 1 (pied en y = 0), UV.y = 1 au pied.
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for angle in [0.0, PI * 0.5]:
		var dir := Vector3(cos(angle), 0.0, sin(angle)) * 0.5
		var quad := [[-dir, Vector2(0, 1)], [dir, Vector2(1, 1)], [dir + Vector3.UP, Vector2(1, 0)], [-dir + Vector3.UP, Vector2(0, 0)]]
		for index in [0, 1, 2, 0, 2, 3]:
			st.set_uv(quad[index][1])
			st.set_normal(Vector3.UP)
			st.add_vertex(quad[index][0])
	_reed_mesh = st.commit()
	# Disque plat (16 secteurs), UV centrées.
	var pool := SurfaceTool.new()
	pool.begin(Mesh.PRIMITIVE_TRIANGLES)
	var sides := 16
	for i in sides:
		var a0 := TAU * i / sides
		var a1 := TAU * (i + 1) / sides
		for v: Vector2 in [Vector2.ZERO, Vector2(cos(a0), sin(a0)), Vector2(cos(a1), sin(a1))]:
			pool.set_uv(v * 0.5 + Vector2(0.5, 0.5))
			pool.set_normal(Vector3.UP)
			pool.add_vertex(Vector3(v.x * 0.5, 0.0, v.y * 0.5))
	_pool_mesh = pool.commit()


func _build_materials() -> void:
	var texture_path := str(config.get("reeds", {}).get("card_texture", ""))
	var card: Texture2D = load(texture_path) as Texture2D if texture_path != "" and ResourceLoader.exists(texture_path) else null
	for level in 2:
		var reeds := ShaderMaterial.new()
		reeds.shader = REED_SHADER
		reeds.set_shader_parameter("use_card", card != null)
		if card != null:
			reeds.set_shader_parameter("card", card)
		_reed_materials.append(reeds)
		var pools := ShaderMaterial.new()
		pools.shader = POOL_SHADER
		pools.render_priority = -1
		_pool_materials.append(pools)


func _surface_material(mode: int) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = SURFACE_SHADER
	material.set_shader_parameter("mode", mode)
	material.set_shader_parameter("fade_start", float(render.get("fade_start", 950.0)))
	material.set_shader_parameter("fade_end", float(render.get("fade_end", 1200.0)))
	material.render_priority = -1
	_surface_materials.append(material)
	return material


# --- Éléments fixes : salins, torrents, cascades, gués ---------------------------------------

func _build_static() -> void:
	_static_root = Node3D.new()
	_static_root.name = "FreshwaterStatic"
	add_child(_static_root)
	var pans := _salt_pan_mesh()
	if pans != null:
		_add_mesh("SaltPans", pans, _surface_material(0))
	var foam := _torrent_mesh()
	if foam != null:
		_add_mesh("Torrents", foam, _surface_material(1))
	_place_waterfalls()
	_place_fords()


func _add_mesh(node_name: String, mesh: ArrayMesh, material: ShaderMaterial) -> void:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	instance.material_override = material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.extra_cull_margin = 4096.0
	_static_root.add_child(instance)


func _ground(x: float, z: float) -> float:
	return map_data.surface_world_at(x, z)


## Bassins de salines : grille de rectangles (digues claires entre eux) dans chaque ellipse.
func _salt_pan_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array()
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	var rng := RandomNumberGenerator.new()
	rng.seed = int(render.get("seed", 1)) + 17
	for pan: Dictionary in sites.get("salt_pans", []):
		var center := Vector2(pan["center"][0], pan["center"][1])
		var radii := Vector2(pan["radii_px"][0], pan["radii_px"][1])
		var angle := deg_to_rad(float(pan.get("angle_deg", 0.0)))
		var cell := maxf(minf(radii.x, radii.y) / 5.0, 0.12)
		var rose: bool = str(pan.get("tint", "grey")) == "rose"
		var nx := int(ceil(radii.x * 2.0 / cell))
		var nz := int(ceil(radii.y * 2.0 / cell))
		for ix in nx:
			for iz in nz:
				var local := Vector2(-radii.x + (ix + 0.5) * cell, -radii.y + (iz + 0.5) * cell)
				if (local / radii).length() > 1.0:
					continue
				var world := center + Vector2(local.x * cos(angle) - local.y * sin(angle), local.x * sin(angle) + local.y * cos(angle))
				if not map_data.is_land_px(int(world.x), int(world.y)):
					continue
				var half := cell * 0.44
				var basis := Vector2(cos(angle), sin(angle))
				var perp := Vector2(-sin(angle), cos(angle))
				var tone := rng.randf()
				var color := Color(0.74, 0.46, 0.50, 0.9).lerp(Color(0.80, 0.62, 0.60, 0.9), tone) if rose and tone < 0.7 else Color(0.55, 0.62, 0.62, 0.9).lerp(Color(0.86, 0.86, 0.82, 0.9), tone)
				var base := vertices.size()
				var y := _ground(world.x, world.y) + 0.004
				for corner: Vector2 in [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, 1)]:
					var p := world + basis * corner.x * half + perp * corner.y * half
					vertices.append(Vector3(p.x, y, p.y))
					colors.append(color)
					uvs.append(corner * 0.5 + Vector2(0.5, 0.5))
				indices.append_array([base, base + 1, base + 2, base, base + 2, base + 3])
	return _commit(vertices, colors, uvs, indices)


## Rubans d'écume sur les tronçons de forte pente, élargis avec la pente.
func _torrent_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array()
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	var width := float(config.get("torrent", {}).get("foam_width_units", 0.18))
	for run: Dictionary in sites.get("torrents", []):
		var points: Array = run["points"]
		var slope := float(run.get("slope", 0.05))
		var half := width * 0.5 * clampf(0.7 + slope * 4.0, 0.7, 1.6)
		var along := 0.0
		var previous := -1
		for i in points.size():
			var p := Vector2(points[i][0], points[i][1])
			var q := Vector2(points[mini(i + 1, points.size() - 1)][0], points[mini(i + 1, points.size() - 1)][1])
			var r := Vector2(points[maxi(i - 1, 0)][0], points[maxi(i - 1, 0)][1])
			var dir := (q - r).normalized()
			var perp := Vector2(-dir.y, dir.x) * half
			if i > 0:
				along += p.distance_to(r) / (half * 4.0)
			var alpha := clampf(0.55 + slope * 3.0, 0.5, 1.0)
			var base := vertices.size()
			var y := _ground(p.x, p.y) + 0.01
			vertices.append(Vector3(p.x + perp.x, y, p.y + perp.y))
			vertices.append(Vector3(p.x - perp.x, y, p.y - perp.y))
			uvs.append(Vector2(along, 0.0))
			uvs.append(Vector2(along, 1.0))
			colors.append(Color(1, 1, 1, alpha))
			colors.append(Color(1, 1, 1, alpha))
			if previous >= 0:
				indices.append_array([previous, previous + 1, base, previous + 1, base + 1, base])
			previous = base
	return _commit(vertices, colors, uvs, indices)


static func _commit(vertices: PackedVector3Array, colors: PackedColorArray, uvs: PackedVector2Array, indices: PackedInt32Array) -> ArrayMesh:
	if vertices.is_empty():
		return null
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	var normals := PackedVector3Array()
	normals.resize(vertices.size())
	normals.fill(Vector3.UP)
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


## Chemin du glb d'un id de catalogue (manifeste `dn-ingest`, LOD 0) ; "" tant qu'il manque.
func model_path(model_key: String) -> String:
	var asset_id := str(config.get("models", {}).get(model_key, ""))
	var entry: Dictionary = _manifest.get(asset_id, {})
	if entry.is_empty():
		return ""
	var files: Array = entry.get("files", [])
	if files.is_empty():
		return ""
	var path := MODEL_ROOT + str(files[0])
	return path if ResourceLoader.exists(path) else ""


func _place_model(model_key: String, node_name: String, px: Vector2, yaw: float, magnify: float) -> bool:
	var path := model_path(model_key)
	if path == "":
		return false
	var scene := load(path) as PackedScene
	if scene == null:
		return false
	var node := scene.instantiate() as Node3D
	node.name = node_name
	node.position = Vector3(px.x, _ground(px.x, px.y), px.y)
	node.rotation.y = yaw
	node.scale = Vector3.ONE * magnify / METERS_PER_UNIT
	_static_root.add_child(node)
	return true


func _place_waterfalls() -> void:
	var magnify := float(render.get("model_magnify", 6.0))
	for site: Dictionary in sites.get("waterfalls", []):
		var px := Vector2(site["px"][0], site["px"][1])
		if _place_model("waterfall", "Waterfall_" + str(site["id"]), px, hash(site["id"]) % 628 / 100.0, magnify):
			continue
		# Substitut : colonne d'eau blanche et brume au pied (ruban vertical).
		var height := clampf(float(site.get("height_m", 30.0)) / METERS_PER_UNIT * 12.0, 0.05, 0.6)
		var yaw := float(hash(site["id"]) % 628) / 100.0
		var vertices := PackedVector3Array()
		var colors := PackedColorArray()
		var uvs := PackedVector2Array()
		var base_y := _ground(px.x, px.y)
		var half := 0.05
		var dir := Vector2(cos(yaw), sin(yaw)) * half
		vertices.append_array([Vector3(px.x - dir.x, base_y + height, px.y - dir.y), Vector3(px.x + dir.x, base_y + height, px.y + dir.y), Vector3(px.x + dir.x, base_y, px.y + dir.y), Vector3(px.x - dir.x, base_y, px.y - dir.y)])
		colors.append_array([Color(1, 1, 1, 1), Color(1, 1, 1, 1), Color(1, 1, 1, 1), Color(1, 1, 1, 1)])
		uvs.append_array([Vector2(0, 0), Vector2(0, 1), Vector2(1, 1), Vector2(1, 0)])
		var mesh := _commit(vertices, colors, uvs, PackedInt32Array([0, 1, 2, 0, 2, 3]))
		_add_mesh("Waterfall_" + str(site["id"]), mesh, _surface_material(1))


func _place_fords() -> void:
	for site: Dictionary in sites.get("fords", []):
		var px := Vector2(site["px"][0], site["px"][1])
		if _place_model("ford", "Ford_" + str(site["id"]), px, 0.0, 6.0):
			continue
		var half_length := float(site.get("width_m", 1500.0)) / METERS_PER_UNIT * 0.5
		var half_width := maxf(half_length * 0.08, 0.03)
		var y := _ground(px.x, px.y) + 0.006
		var vertices := PackedVector3Array([Vector3(px.x - half_length, y, px.y - half_width), Vector3(px.x + half_length, y, px.y - half_width), Vector3(px.x + half_length, y, px.y + half_width), Vector3(px.x - half_length, y, px.y + half_width)])
		var colors := PackedColorArray([Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE])
		var uvs := PackedVector2Array([Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(0, 1)])
		_add_mesh("Ford_" + str(site["id"]), _commit(vertices, colors, uvs, PackedInt32Array([0, 1, 2, 0, 2, 3])), _surface_material(2))


# --- Roselières et mares (blocs) --------------------------------------------------------------

## Rayon normalisé (0 au centre, 1 au bord) du point `p` dans l'ellipse `site`.
static func ellipse_radius(site: Dictionary, p: Vector2) -> float:
	var center := Vector2(site["center"][0], site["center"][1])
	var radii := Vector2(site["radii_px"][0], site["radii_px"][1])
	var angle := deg_to_rad(float(site.get("angle_deg", 0.0)))
	var d := p - center
	var dx := d.x
	var dy_north := -d.y
	var u := dx * cos(angle) + dy_north * sin(angle)
	var v := -dx * sin(angle) + dy_north * cos(angle)
	return sqrt(pow(u / radii.x, 2.0) + pow(v / radii.y, 2.0))


func block_units(level: int) -> float:
	return float(render.get("near_block_units" if level == LEVEL_NEAR else "patch_block_units", 16.0))


func _spacing(level: int) -> float:
	return float(render.get("near_spacing_units" if level == LEVEL_NEAR else "patch_spacing_units", 0.8))


## Grossissement des touffes du niveau « patch » à la distance caméra `d` (1 de près).
func scale_for(d: float) -> float:
	var near := float(render.get("scale_near_distance", 3.0))
	var far := maxf(float(render.get("scale_far_distance", 60.0)), near * 1.01)
	var t := clampf(log(maxf(d, 1e-3) / near) / log(far / near), 0.0, 1.0)
	return lerpf(1.0, float(render.get("far_scale", 36.0)), smoothstep(0.0, 1.0, t))


## Semis d'un bloc (sans hauteurs) : {"reeds": [Vector2 position, Color teinte, float rayon...] ...}.
## Retourne des listes plates : reeds = [x, z, size_factor, r, g, b, phase]*, pools = [x, z, diameter_m, r, g, b, seed]*.
func seed_block(level: int, key: Vector2i) -> Dictionary:
	var size := block_units(level)
	var rect := Rect2(Vector2(key) * size, Vector2(size, size))
	var local_sites: Array = []
	for site: Dictionary in sites.get("wetlands", []):
		var radius := maxf(site["radii_px"][0], site["radii_px"][1])
		var center := Vector2(site["center"][0], site["center"][1])
		if rect.grow(radius).has_point(center):
			local_sites.append(site)
	var reeds := PackedFloat32Array()
	var pools := PackedFloat32Array()
	if local_sites.is_empty() or map_data == null:
		return {"reeds": reeds, "pools": pools}
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(Vector3i(key.x, key.y, level)) + int(render.get("seed", 1))
	var spacing := _spacing(level)
	var cells := int(ceil(size / spacing))
	var kinds: Dictionary = config.get("reeds", {}).get("kinds", {})
	var pool_m := float(config.get("reeds", {}).get("pool_diameter_m", 60.0))
	for ix in cells:
		for iz in cells:
			var p := rect.position + Vector2((ix + rng.randf()) * spacing, (iz + rng.randf()) * spacing)
			if not rect.has_point(p):
				continue
			var best := 0.0
			var pick: Dictionary = {}
			var pick_r := 1.0
			for site: Dictionary in local_sites:
				var r := ellipse_radius(site, p)
				if r >= 1.0:
					continue
				var kind: Dictionary = kinds.get(str(site["kind"]), {})
				var weight := float(kind.get("cover", 0.3)) * float(site.get("density", 0.5)) * smoothstep(1.0, 0.6, r)
				if weight > best:
					best = weight
					pick = site
					pick_r = r
			var roll := rng.randf()
			if pick.is_empty() or roll > best:
				continue
			if not map_data.is_land_px(int(p.x), int(p.y)) or _in_circles(p, exclusions):
				continue
			var kind: Dictionary = kinds.get(str(pick["kind"]), {})
			var tint: Array = kind.get("tint", [0.55, 0.55, 0.35])
			var lagoon: bool = bool(pick.get("lagoon", false))
			var pool_share := float(kind.get("pool_share", 0.1))
			if lagoon:
				pool_share = 0.4 if pick_r < 0.62 else 0.0
			if rng.randf() < pool_share:
				var diameter := pool_m * rng.randf_range(0.6, 1.8) * (3.0 if lagoon else 1.0)
				var water := Color(0.30, 0.46, 0.46) if lagoon else Color(0.10, 0.17, 0.17)
				pools.append_array([p.x, p.y, diameter, water.r, water.g, water.b, rng.randf()])
			elif not lagoon or pick_r > 0.5:
				var jitter := rng.randf_range(0.85, 1.15)
				reeds.append_array([p.x, p.y, rng.randf_range(0.7, 1.4), tint[0] * jitter, tint[1] * jitter, tint[2] * jitter, rng.randf()])
	return {"reeds": reeds, "pools": pools}


static func _in_circles(p: Vector2, circles: PackedVector3Array) -> bool:
	for c in circles:
		if p.distance_squared_to(Vector2(c.x, c.y)) < c.z * c.z:
			return true
	return false


func _height_source(rect: Rect2) -> Dictionary:
	if terrain == null or terrain.quadtree == null or terrain.map_data == null:
		return {}
	return terrain.quadtree.surface_snapshot(rect, Vector2.ZERO)


func _surface_at(source: Dictionary, x: float, y: float) -> float:
	if source.is_empty():
		return map_data.surface_world_at(x, y)
	var h := ReliefQuadtree.sample_snapshot(source, x, y)
	return h if h > 0.0 else map_data.surface_world_at(x, y)


## Transformations et données personnalisées des instances d'une famille.
func _buffer(level: int, flat: PackedFloat32Array, stride: int, is_pool: bool, rect: Rect2) -> PackedFloat32Array:
	var count := flat.size() / stride
	var buffer := PackedFloat32Array()
	buffer.resize(count * FLOATS)
	var source := _height_source(rect.grow(1.0))
	var reed_cfg: Dictionary = config.get("reeds", {})
	var width := float(reed_cfg.get("clump_width_m", 7.0)) / METERS_PER_UNIT
	var height := float(reed_cfg.get("clump_height_m", 2.6)) / METERS_PER_UNIT
	for n in count:
		var o := n * stride
		var x := flat[o]
		var z := flat[o + 1]
		var y := _surface_at(source, x, z)
		var sx: float
		var sy: float
		if is_pool:
			sx = flat[o + 2] / METERS_PER_UNIT
			sy = 1.0
		else:
			sx = width * flat[o + 2]
			sy = height * flat[o + 2]
		var base := n * FLOATS
		buffer[base + 0] = sx
		buffer[base + 3] = x
		buffer[base + 5] = sy
		buffer[base + 7] = y + (0.002 if is_pool else 0.0)
		buffer[base + 10] = sx
		buffer[base + 11] = z
		buffer[base + 12] = flat[o + 3]
		buffer[base + 13] = flat[o + 4]
		buffer[base + 14] = flat[o + 5]
		buffer[base + 15] = flat[o + 6]
	return buffer


func _make_multimesh(mesh: Mesh, buffer: PackedFloat32Array, rect: Rect2, max_scale: float) -> MultiMesh:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = mesh
	multimesh.instance_count = buffer.size() / FLOATS
	if multimesh.instance_count > 0:
		multimesh.buffer = buffer
	var margin := 0.5 * max_scale
	multimesh.custom_aabb = AABB(Vector3(rect.position.x - margin, -50.0, rect.position.y - margin), Vector3(rect.size.x + margin * 2.0, 400.0, rect.size.y + margin * 2.0))
	return multimesh


func _block_key(level: int, key: Vector2i) -> String:
	return "%d:%d:%d" % [level, key.x, key.y]


func _install_block(level: int, key: Vector2i) -> void:
	var t0 := Time.get_ticks_usec()
	var seeded := seed_block(level, key)
	var size := block_units(level)
	var rect := Rect2(Vector2(key) * size, Vector2(size, size))
	var entry := {"nodes": [], "last_seen": _frame, "reeds": 0, "pools": 0}
	var reeds: PackedFloat32Array = seeded["reeds"]
	var pools: PackedFloat32Array = seeded["pools"]
	var far_scale := float(render.get("far_scale", 36.0))
	var pool_scale := float(render.get("pool_far_scale", 2.0))
	if reeds.size() > 0:
		var node := MultiMeshInstance3D.new()
		node.multimesh = _make_multimesh(_reed_mesh, _buffer(level, reeds, 7, false, rect), rect, far_scale if level == LEVEL_PATCH else 1.0)
		node.material_override = _reed_materials[level]
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		entry["nodes"].append(node)
		entry["reeds"] = reeds.size() / 7
	if pools.size() > 0:
		var node := MultiMeshInstance3D.new()
		node.multimesh = _make_multimesh(_pool_mesh, _buffer(level, pools, 7, true, rect), rect, pool_scale if level == LEVEL_PATCH else 1.0)
		node.material_override = _pool_materials[level]
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(node)
		entry["nodes"].append(node)
		entry["pools"] = pools.size() / 7
	_total_instances += int(entry["reeds"]) + int(entry["pools"])
	_blocks[_block_key(level, key)] = entry
	stats["blocks"] = _blocks.size()
	stats["seed_ms_max"] = maxf(float(stats["seed_ms_max"]), (Time.get_ticks_usec() - t0) / 1000.0)


func _free_block(block_key: String) -> void:
	var entry: Dictionary = _blocks.get(block_key, {})
	for node: Node in entry.get("nodes", []):
		node.queue_free()
	_total_instances -= int(entry.get("reeds", 0)) + int(entry.get("pools", 0))
	_blocks.erase(block_key)


func _on_surface_rect_changed(rect: Rect2) -> void:
	for block_key: String in _blocks:
		if not _dirty.has(block_key):
			var parts := block_key.split(":")
			var size := block_units(int(parts[0]))
			if Rect2(Vector2(int(parts[1]), int(parts[2])) * size, Vector2(size, size)).intersects(rect):
				_dirty.append(block_key)


func _process(_delta: float) -> void:
	if not active():
		visible = false
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	var focus_value: Variant = _rig.get("focus") if _rig != null else null
	var at := Vector2(focus_value.x, focus_value.z) if focus_value is Vector3 else Vector2(camera.global_position.x, camera.global_position.z)
	update_view(at, distance)


func update_view(at: Vector2, camera_distance: float) -> void:
	_frame += 1
	var patch_max := float(render.get("patch_max_distance", 120.0))
	var near_max := float(render.get("near_max_distance", 4.0))
	var fade_end := float(render.get("fade_end", 1200.0))
	_static_root.visible = camera_distance < fade_end
	if camera_distance >= patch_max:
		for block_key: String in _blocks.keys():
			(_blocks[block_key]["nodes"] as Array).map(func(n: Node3D) -> void: n.visible = false)
		return
	var patch_alpha := smoothstep(near_max * 0.8, near_max * 1.4, camera_distance) * (1.0 - smoothstep(patch_max * 0.75, patch_max, camera_distance))
	var near_alpha := 1.0 - smoothstep(near_max, near_max * 1.7, camera_distance)
	var scale := scale_for(camera_distance)
	for level in 2:
		var alpha := patch_alpha if level == LEVEL_PATCH else near_alpha
		_reed_materials[level].set_shader_parameter("fw_alpha", alpha)
		_reed_materials[level].set_shader_parameter("fw_scale", scale if level == LEVEL_PATCH else 1.0)
		_pool_materials[level].set_shader_parameter("fw_alpha", alpha)
		_pool_materials[level].set_shader_parameter("fw_scale", minf(scale, float(render.get("pool_far_scale", 2.0))) if level == LEVEL_PATCH else 1.0)
	var created := 0
	var wanted: Dictionary = {}
	for level in 2:
		var alpha := patch_alpha if level == LEVEL_PATCH else near_alpha
		if alpha <= 0.01:
			continue
		var size := block_units(level)
		var radius := clampf(camera_distance * 1.8, 20.0, 140.0) if level == LEVEL_PATCH else clampf(camera_distance * 1.6, 2.5, 5.0)
		var lo := Vector2i(floori((at.x - radius) / size), floori((at.y - radius) / size))
		var hi := Vector2i(floori((at.x + radius) / size), floori((at.y + radius) / size))
		var missing: Array = []
		for ty in range(lo.y, hi.y + 1):
			for tx in range(lo.x, hi.x + 1):
				var key := Vector2i(tx, ty)
				var block_key := _block_key(level, key)
				var rect := Rect2(Vector2(key) * size, Vector2(size, size))
				var nearest := Vector2(clampf(at.x, rect.position.x, rect.end.x), clampf(at.y, rect.position.y, rect.end.y))
				if nearest.distance_to(at) > radius:
					continue
				wanted[block_key] = true
				if not _blocks.has(block_key):
					missing.append([nearest.distance_to(at), level, key])
		missing.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
		for item: Array in missing:
			if created >= int(render.get("max_blocks_per_frame", 2)) or _total_instances > int(render.get("max_instances", 60000)):
				break
			_install_block(item[1], item[2])
			created += 1
	for block_key: String in _blocks:
		var entry: Dictionary = _blocks[block_key]
		var shown := wanted.has(block_key)
		if shown:
			entry["last_seen"] = _frame
		for node: Node3D in entry["nodes"]:
			node.visible = shown
	if not _dirty.is_empty():
		var dirty_key: String = _dirty.pop_front()
		if _blocks.has(dirty_key):
			var parts := dirty_key.split(":")
			_free_block(dirty_key)
			_install_block(int(parts[0]), Vector2i(int(parts[1]), int(parts[2])))
	_evict()
	stats["reeds"] = 0
	stats["pools"] = 0
	for block_key: String in _blocks:
		stats["reeds"] += int(_blocks[block_key]["reeds"])
		stats["pools"] += int(_blocks[block_key]["pools"])


func _evict() -> void:
	var limit := int(render.get("max_cached_blocks", 220))
	if _blocks.size() <= limit:
		return
	var keys := _blocks.keys()
	keys.sort_custom(func(a: String, b: String) -> bool: return int(_blocks[a]["last_seen"]) < int(_blocks[b]["last_seen"]))
	for i in _blocks.size() - limit:
		_free_block(keys[i])
	stats["blocks"] = _blocks.size()
