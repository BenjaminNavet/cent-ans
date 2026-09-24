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


## Hauteurs de la surface affichée sur une grille couvrant la zone réservée.
func _bake_heights() -> void:
	var image := Image.create(HEIGHT_RES, HEIGHT_RES, false, Image.FORMAT_RF)
	for j in HEIGHT_RES:
		for i in HEIGHT_RES:
			var x := _origin.x + (float(i) + 0.5) / HEIGHT_RES * _extent
			var z := _origin.y + (float(j) + 0.5) / HEIGHT_RES * _extent
			var h := _terrain.surface_height_at(x, z) if _terrain != null else 0.0
			image.set_pixel(i, j, Color(h, 0.0, 0.0))
	if _height_texture == null:
		_height_texture = ImageTexture.create_from_image(image)
	else:
		_height_texture.update(image)
	for material in _materials:
		_apply_height_params(material)


func _on_chunk_surface_changed(index: int) -> void:
	if _terrain == null or _terrain.chunk_px <= 0:
		return
	var cx := index % TerrainBuilder.CHUNKS
	var cy := index / TerrainBuilder.CHUNKS
	var rect := Rect2(cx * _terrain.chunk_px, cy * _terrain.chunk_px, _terrain.chunk_px, _terrain.chunk_px)
	if rect.intersects(Rect2(_origin, Vector2(_extent, _extent))):
		_bake_heights()


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
