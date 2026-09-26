class_name LandmarkModel
extends Node3D

## Ville emblématique sur la carte de campagne (lot L1, Paris d'abord), rendu seulement.
##
## Plan dans `data/landmarks/<id>.json` (schéma `landmark.schema.json`), maquette générée par
## `tools/blender_scripts/landmark_city.py` → `res://assets/models/landmarks/<id>.glb`. La
## maquette est construite à plat, en unités carte, nord vrai : ce nœud la tourne de
## `anchor.north_bearing_deg`, la pose à `anchor.px` et la drape sur le relief affiché (shader
## `landmark.gdshader`, hauteurs cuites dans une petite texture recalculée quand une tuile de
## terrain change de niveau).
##
## Couches (nœuds du glTF) : `ground` (Seine, îles, rues), `houses` (tissu détaillé, près),
## `blocks` (îlots simplifiés, loin), `landmarks` (monuments, murailles, ponts), puis un nœud par
## élément daté (`charles_v`, `bastille`…) ou variante (`louvre__charles_v`) : `set_year` choisit.

const MODELS_DIR := "res://assets/models/landmarks/"
const SHADER := preload("res://shaders/landmark.gdshader")
const HEIGHT_RES := 96
## Portées de visibilité (distance caméra → maquette, unités carte).
const HOUSES_END := 120.0
const BLOCKS_BEGIN := 105.0
const DETAIL_END := 1100.0
const GROUND_END := 2000.0

var landmark: Dictionary = {}
var zone_radius: float = 6.0
var core_radius: float = 5.0
var year: int = -1
var stats: Dictionary = {}

var _terrain: TerrainBuilder
var _height_texture: ImageTexture
var _materials: Array[ShaderMaterial] = []
var _dated: Dictionary = {}  # nom de nœud → {from, until}
var _extent := 1.0
var _origin := Vector2.ZERO
var _fade := 1.0


## Construit la maquette ; null si le modèle n'est pas importé.
static func create(data: Dictionary, terrain: TerrainBuilder) -> LandmarkModel:
	var path := MODELS_DIR + str(data.get("id", "")) + ".glb"
	if not ResourceLoader.exists(path):
		push_warning("LandmarkModel: %s absent (lancer tools/blender_scripts/landmark_city.py)" % path)
		return null
	var scene := load(path) as PackedScene
	if scene == null:
		return null
	var node := LandmarkModel.new()
	node.name = "Landmark_" + str(data.get("id", ""))
	node._setup(data, scene.instantiate() as Node3D, terrain)
	return node


func _setup(data: Dictionary, model: Node3D, terrain: TerrainBuilder) -> void:
	landmark = data
	_terrain = terrain
	var anchor: Dictionary = data.get("anchor", {})
	var px: Array = anchor.get("px", [0.0, 0.0])
	var scale_block: Dictionary = data.get("scale", {})
	zone_radius = float(scale_block.get("zone_radius_px", 6.0))
	core_radius = float(scale_block.get("core_radius_px", 5.0))
	position = Vector3(float(px[0]), 0.0, float(px[1]))
	rotation.y = -deg_to_rad(float(anchor.get("north_bearing_deg", 0.0)))
	model.name = "Model"
	add_child(model)
	_index_dated()
	_extent = zone_radius * 2.2
	_origin = Vector2(position.x, position.z) - Vector2(_extent, _extent) * 0.5
	_bake_heights()
	var triangles := 0
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		_dress(mesh_instance)
		if mesh_instance.mesh != null:
			for surface in mesh_instance.mesh.get_surface_count():
				triangles += mesh_instance.mesh.surface_get_array_len(surface) / 3
	stats = {"triangles": triangles, "layers": model.find_children("*", "MeshInstance3D", true, false).size()}
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)


## Éléments datés du plan : nom de couche → années de début et de fin de visibilité.
func _index_dated() -> void:
	_dated.clear()
	for key in ["walls", "bridges", "monuments", "open_spaces", "areas"]:
		for item in landmark.get(key, []):
			var item_id := str(item.get("id", ""))
			var from_year := int(item.get("from_year", -100000))
			var until_year := int(item.get("until_year", 100000))
			var variants: Dictionary = item.get("variant_from_year", {})
			if item.has("from_year") or item.has("until_year"):
				_dated[item_id] = Vector2i(from_year, until_year)
			for variant in variants:
				var start := int(variants[variant])
				_dated[item_id] = Vector2i(from_year, mini(until_year, start - 1))
				_dated["%s__%s" % [item_id, variant]] = Vector2i(maxi(from_year, start), until_year)


## Remplace les matériaux importés par le shader drapé (couleur et rugosité reprises).
func _dress(mesh_instance: MeshInstance3D) -> void:
	var layer := mesh_instance.name
	var mesh := mesh_instance.mesh
	if mesh == null:
		return
	for surface in mesh.get_surface_count():
		var source := mesh.surface_get_material(surface)
		var material := ShaderMaterial.new()
		material.shader = SHADER
		var color := Color(0.6, 0.6, 0.6)
		var rough := 0.9
		var name := ""
		if source is BaseMaterial3D:
			color = (source as BaseMaterial3D).albedo_color
			rough = (source as BaseMaterial3D).roughness
			name = source.resource_name
		material.set_shader_parameter("albedo", color)
		material.set_shader_parameter("roughness", rough)
		material.set_shader_parameter("water", 1.0 if name == "Water" else 0.0)
		material.set_shader_parameter("tint_strength", 1.0 if layer == "houses" or layer == "landmarks" else 0.0)
		# L3 : textures à leur taille réelle (unités de carte → mètres du plan au centre de la
		# loupe), pied des murs au niveau des plaques de sol du générateur (`Z_ISLAND`).
		var scale_block: Dictionary = landmark.get("scale", {})
		material.set_shader_parameter("meters_per_unit", float(scale_block.get("center_meters_per_unit", 200.0)))
		material.set_shader_parameter("ground_height", 0.010)
		_apply_height_params(material)
		mesh_instance.set_surface_override_material(surface, material)
		_materials.append(material)
	# Portées : tissu détaillé de près, îlots simplifiés de loin, sol et monuments toujours.
	match layer:
		"houses":
			mesh_instance.visibility_range_end = HOUSES_END
			mesh_instance.visibility_range_end_margin = 15.0
			mesh_instance.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
		"blocks":
			mesh_instance.visibility_range_begin = BLOCKS_BEGIN
			mesh_instance.visibility_range_begin_margin = 15.0
			mesh_instance.visibility_range_end = DETAIL_END
			mesh_instance.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
			mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		"ground":
			mesh_instance.visibility_range_end = GROUND_END
			mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		_:
			mesh_instance.visibility_range_end = DETAIL_END
	# Le drapé déplace les sommets : l'AABB doit couvrir le relief.
	mesh_instance.extra_cull_margin = 4.0


func _apply_height_params(material: ShaderMaterial) -> void:
	material.set_shader_parameter("height_map", _height_texture)
	material.set_shader_parameter("map_origin", _origin)
	material.set_shader_parameter("map_extent", _extent)
	material.set_shader_parameter("height_in_meters", true)


## Hauteurs de la surface affichée sur une grille couvrant la zone réservée, en mètres (lot ZG4 :
## le shader les met à l'échelle verticale courante, `campaign_vertical_scale`, sans nouvelle
## cuisson quand l'exagération change). Cuisson complète et synchrone (construction) ; ensuite,
## `_on_chunk_surface_changed` la relance par tranches de lignes étalées sur plusieurs images
## (`bake_budget_ms`), l'ancienne texture restant affichée jusqu'à la fin.
func _bake_heights() -> void:
	_start_bake()
	_continue_bake(INF)
	_finish_bake()


## Budget par image de la cuisson étalée (ms).
@export var bake_budget_ms: float = 1.5
var _bake_image: Image
var _bake_row: int = -1
var _bake_corners: PackedFloat32Array = PackedFloat32Array()
var _bake_pending: bool = false
var _bake_active_us: int = 0
## SZ6 : avec le relief quadtree, les recuissons se font d'un bloc dans un fil de travail sur un
## instantané des pages (`_bake_snapshot`, lu par `_surface_m`) au lieu de tranches de
## `bake_budget_ms` par maquette et par image (jusqu'à 30 ms par image avec plusieurs maquettes).
@export var bake_in_thread: bool = true
var _bake_task: int = -1
var _bake_snapshot: Dictionary = {}


func _start_bake() -> void:
	_bake_image = Image.create(HEIGHT_RES, HEIGHT_RES, false, Image.FORMAT_RF)
	_bake_row = 0
	_bake_active_us = 0
	_bake_corners = _corner_row(0)


## Hauteur (m) de la surface affichée au point carte ; 0 sans terrain.
func _surface_m(x: float, z: float) -> float:
	if not _bake_snapshot.is_empty():
		# SZ6 (fil de travail) : même valeur que `TerrainBuilder.surface_height_at` avec le quadtree.
		return MapData.height_from_display(maxf(ReliefQuadtree.sample_snapshot(_bake_snapshot, x, z), 0.0), x, z)
	if _terrain == null:
		return 0.0
	# ZG8 : inverse de la hauteur affichée (le shader la repose avec `campaign_display_height`).
	return MapData.height_from_display(_terrain.surface_height_at(x, z), x, z)


## Hauteurs (m) des coins de texels de la ligne `j` (HEIGHT_RES + 1 valeurs, partagées par les
## texels voisins : 2,5 fois moins d'appels que cinq échantillons par texel).
func _corner_row(j: int) -> PackedFloat32Array:
	var row := PackedFloat32Array()
	row.resize(HEIGHT_RES + 1)
	var z := _origin.y + float(j) / HEIGHT_RES * _extent
	for i in HEIGHT_RES + 1:
		row[i] = _surface_m(_origin.x + float(i) / HEIGHT_RES * _extent, z)
	return row


## Avance la cuisson dans le budget (ms) ; rend vrai quand toutes les lignes sont faites
## (`_finish_bake` installe alors la texture, sur le fil principal).
func _continue_bake(budget_ms: float) -> bool:
	if _bake_row < 0:
		return false
	var t0 := Time.get_ticks_usec()
	while _bake_row < HEIGHT_RES:
		var j := _bake_row
		var next := _corner_row(j + 1)
		var z := _origin.y + (float(j) + 0.5) / HEIGHT_RES * _extent
		for i in HEIGHT_RES:
			# L2 : maximum sur le texel (centre et coins) : le lit creusé d'une rivière de la carte
			# ne fait plus plonger les rives, l'eau et les quais de la maquette sous le relief.
			var h := _surface_m(_origin.x + (float(i) + 0.5) / HEIGHT_RES * _extent, z)
			h = maxf(maxf(h, maxf(_bake_corners[i], _bake_corners[i + 1])), maxf(next[i], next[i + 1]))
			_bake_image.set_pixel(i, j, Color(h, 0.0, 0.0))
		_bake_corners = next
		_bake_row += 1
		if (Time.get_ticks_usec() - t0) / 1000.0 >= budget_ms:
			break
	_bake_active_us += Time.get_ticks_usec() - t0
	return _bake_row >= HEIGHT_RES


func _finish_bake() -> void:
	if _bake_row < HEIGHT_RES:
		return
	_bake_row = -1
	if _height_texture == null:
		_height_texture = ImageTexture.create_from_image(_bake_image)
	else:
		_height_texture.update(_bake_image)
	_bake_image = null
	for material in _materials:
		_apply_height_params(material)
	stats["bakes"] = int(stats.get("bakes", 0)) + 1
	stats["bake_ms"] = _bake_active_us / 1000.0
	stats["bake_ms_total"] = float(stats.get("bake_ms_total", 0.0)) + _bake_active_us / 1000.0


func _process(_delta: float) -> void:
	if _bake_task >= 0:
		if not WorkerThreadPool.is_task_completed(_bake_task):
			return
		_join_bake()
	if _bake_row < 0 and _bake_pending:
		_bake_pending = false
		if bake_in_thread and _terrain != null and _terrain.quadtree != null:
			_bake_snapshot = _terrain.quadtree.surface_snapshot(Rect2(_origin, Vector2(_extent, _extent)).grow(1.0), Vector2.ZERO)
			_bake_task = WorkerThreadPool.add_task(_bake_rows, false, "landmark bake " + name)
			return
		_start_bake()
	if _bake_row >= 0:
		var t0 := Time.get_ticks_usec()
		if _continue_bake(bake_budget_ms):
			_finish_bake()
		stats["bake_frame_ms_max"] = maxf(float(stats.get("bake_frame_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)


## Fil de travail : toutes les lignes, sur l'instantané des pages.
func _bake_rows() -> void:
	_start_bake()
	_continue_bake(INF)


func _join_bake() -> void:
	WorkerThreadPool.wait_for_task_completion(_bake_task)
	_bake_task = -1
	_bake_snapshot = {}
	var t0 := Time.get_ticks_usec()
	_finish_bake()
	stats["bake_frame_ms_max"] = maxf(float(stats.get("bake_frame_ms_max", 0.0)), (Time.get_ticks_usec() - t0) / 1000.0)


func _exit_tree() -> void:
	if _bake_task >= 0:
		WorkerThreadPool.wait_for_task_completion(_bake_task)
		_bake_task = -1
		_bake_snapshot = {}


## Termine tout de suite la cuisson en cours ou en attente (captures, tests).
func flush_bake() -> void:
	if _bake_task >= 0:
		_join_bake()
	if _bake_row < 0 and _bake_pending:
		_bake_pending = false
		_start_bake()
	_continue_bake(INF)
	_finish_bake()


func _on_chunk_surface_changed(index: int) -> void:
	# ZG4 : un changement d'échelle verticale seul ne touche pas aux hauteurs en mètres.
	if _terrain == null or _terrain.chunk_px <= 0 or _terrain.rescaling_vertical:
		return
	var cx := index % TerrainBuilder.CHUNKS
	var cy := index / TerrainBuilder.CHUNKS
	var rect := Rect2(cx * _terrain.chunk_px, cy * _terrain.chunk_px, _terrain.chunk_px, _terrain.chunk_px)
	if rect.intersects(Rect2(_origin, Vector2(_extent, _extent))):
		# Cuisson en cours : relancée à la fin (la surface a pu changer sous les lignes déjà faites).
		_bake_pending = true


## Affiche les éléments datés de l'année (enceinte de Charles V, Bastille, Louvre de Charles V…).
## Option de capture `--landmark-year=<année>` : force l'année affichée.
func set_year(new_year: int) -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--landmark-year="):
			new_year = int(arg.trim_prefix("--landmark-year="))
	if new_year == year:
		return
	year = new_year
	var model := get_node_or_null("Model")
	if model == null:
		return
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var layer := str(child.name)
		if _dated.has(layer):
			var span: Vector2i = _dated[layer]
			(child as Node3D).visible = year >= span.x and year <= span.y


## VH4 (ADR 0078) : opacité de la maquette (tramage) pendant le fondu vers la ville 1:1 ;
## cachée sous 1 %.
func set_fade(alpha: float) -> void:
	if is_equal_approx(alpha, _fade):
		return
	_fade = alpha
	visible = alpha > 0.01
	for material in _materials:
		material.set_shader_parameter("fade", alpha)


## Hauteur de la surface au centre (pose des étiquettes et du picking).
func ground_height() -> float:
	return _terrain.surface_height_at(position.x, position.z) if _terrain != null else 0.0


## Vrai si le point carte (px) est dans la zone réservée.
func covers(px: Vector2) -> bool:
	return px.distance_to(Vector2(position.x, position.z)) <= zone_radius


## Année d'un libellé de date (« Printemps 1337 » → 1337), -1 sinon.
static func year_of(label: String) -> int:
	var digits := ""
	for character in label:
		if character >= "0" and character <= "9":
			digits += character
		elif digits.length() == 4:
			break
		else:
			digits = ""
	return int(digits) if digits.length() == 4 else -1
