class_name DnTreeModels
extends RefCounted

## Lot DN-FORET (ADR 0216) : maillages décimés des arbres générés (glb DN, `lod2`, ≈ 300
## triangles) pour les arbres proches de la carte de campagne.
##
## Une ligne d'atlas d'imposteurs (`TreeSpecies`, ordre du catalogue) = une essence = un glb
## `dn/vegetation/<dn_id>_lod2.glb` du paquet de modèles (ADR 0212). Le maillage est ramené à
## une hauteur de 1, pied en 0, largeur de houppier 1 (l'instance fixe taille et élancement comme
## pour les maillages procéduraux), normales arrondies vers le centre du houppier (ombrage doux de
## feuillage), COLOR.a = poids du vent, UV2 = UV du glb. Matériau : `foliage_model.gdshader`
## (texture du glb × `tree_model_gains.json`, teinte saisonnière et éclaircissement du feuillage).
## Sans paquet de modèles, `mesh(row)` renvoie null et l'appelant garde les imposteurs. Rendu
## seulement, aucune règle de jeu.

const MODEL_SHADER := preload("res://shaders/foliage_model.gdshader")
const MODEL_WINTER_SHADER := preload("res://shaders/foliage_model_winter.gdshader")
const GAINS_FILE := "art/tree_model_gains.json"
const MODEL_LOD := "lod1"
## Éclaircissement des textures du modèle (éclairage et ombrage propres du maillage, plus sombres que
## l'imposteur cuit à plat) pour que la bascule modèle / imposteur ne se voie pas.
const MODEL_BOOST := 1.55

var _species: TreeSpecies
var _gains: Dictionary = {}
var _meshes: Dictionary = {}  # ligne → ArrayMesh ou null
var _textures: Dictionary = {}  # ligne → Texture2D
var _materials: Dictionary = {}  # ligne → ShaderMaterial
var _winter := false


func _init(species: TreeSpecies) -> void:
	_species = species
	if DataFile.exists(GAINS_FILE):
		var data: Variant = DataFile.load_cached(GAINS_FILE)
		if data is Dictionary:
			_gains = (data as Dictionary).get("gains", {})


## Vrai si au moins un modèle d'essence est disponible (paquet installé).
func available() -> bool:
	if _species == null or not _species.ok:
		return false
	for row in _species.count:
		if mesh(row) != null:
			return true
	return false


func mesh(row: int) -> ArrayMesh:
	if _meshes.has(row):
		return _meshes[row]
	var built: ArrayMesh = _build(row)
	_meshes[row] = built
	return built


## Matériau de l'essence `row` (null sans modèle). Les uniformes communs du feuillage y sont
## reportés par `apply_param`.
func material(row: int) -> ShaderMaterial:
	if _materials.has(row):
		return _materials[row]
	var result: ShaderMaterial = null
	if mesh(row) != null and _textures.has(row):
		result = ShaderMaterial.new()
		result.shader = MODEL_WINTER_SHADER if _winter else MODEL_SHADER
		result.set_shader_parameter("model_texture", _textures[row])
		var gain: Array = _gains.get(_species.ids[row], [1.0, 1.0, 1.0])
		result.set_shader_parameter("model_gain", Vector3(float(gain[0]), float(gain[1]), float(gain[2])) * MODEL_BOOST)
		for param: String in _params:
			result.set_shader_parameter(param, _params[param])
	_materials[row] = result
	return result


var _params: Dictionary = {}


## Uniforme commune du feuillage (`Vegetation._set_foliage_param`) reportée sur tous les matériaux.
func apply_param(param: String, value: Variant) -> void:
	_params[param] = value
	for row: int in _materials:
		if _materials[row] != null:
			(_materials[row] as ShaderMaterial).set_shader_parameter(param, value)


func set_winter(winter: bool) -> void:
	if winter == _winter:
		return
	_winter = winter
	for row: int in _materials:
		if _materials[row] != null:
			(_materials[row] as ShaderMaterial).shader = MODEL_WINTER_SHADER if winter else MODEL_SHADER


func _build(row: int) -> ArrayMesh:
	if _species == null or row < 0 or row >= _species.count:
		return null
	var dn_id := _species.dn_ids[row]
	if dn_id == "":
		return null
	var scene := ModelLibrary.get_scene("dn/vegetation/%s_%s" % [dn_id, MODEL_LOD])
	if scene == null:
		return null
	var root := scene.instantiate() as Node3D
	if root == null:
		return null
	var source: MeshInstance3D = null
	for child in root.find_children("*", "MeshInstance3D", true, false):
		source = child as MeshInstance3D
		break
	if source == null or source.mesh == null or source.mesh.get_surface_count() == 0:
		root.free()
		return null
	var xform := Transform3D.IDENTITY
	var node: Node3D = source
	while node != null and node != root:
		xform = node.transform * xform
		node = node.get_parent() as Node3D
	var arrays := source.mesh.surface_get_arrays(0)
	var material := source.mesh.surface_get_material(0)
	var texture: Texture2D = (material as BaseMaterial3D).albedo_texture if material is BaseMaterial3D else null
	root.free()
	if texture == null or arrays[Mesh.ARRAY_VERTEX] == null:
		return null
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uvs: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV] if arrays[Mesh.ARRAY_TEX_UV] != null else PackedVector2Array()
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
	if vertices.is_empty() or uvs.size() != vertices.size():
		return null
	var low := Vector3(INF, INF, INF)
	var high := Vector3(-INF, -INF, -INF)
	var moved := PackedVector3Array()
	moved.resize(vertices.size())
	for i in vertices.size():
		var v := xform * vertices[i]
		moved[i] = v
		low = low.min(v)
		high = high.max(v)
	var extent := high - low
	if extent.y <= 1e-6:
		return null
	var crown := maxf(maxf(extent.x, extent.z), 1e-6)
	var centre_x := (low.x + high.x) * 0.5
	var centre_z := (low.z + high.z) * 0.5
	var out_vertices := PackedVector3Array()
	out_vertices.resize(vertices.size())
	var colors := PackedColorArray()
	colors.resize(vertices.size())
	var normals := PackedVector3Array()
	normals.resize(vertices.size())
	var uv_main := PackedVector2Array()
	uv_main.resize(vertices.size())
	var uv_tex := PackedVector2Array()
	uv_tex.resize(vertices.size())
	var crown_centre := Vector3(0.0, 0.62, 0.0)
	for i in vertices.size():
		var v := moved[i]
		var p := Vector3((v.x - centre_x) / crown, (v.y - low.y) / extent.y, (v.z - centre_z) / crown)
		out_vertices[i] = p
		var wind := smoothstep(0.25, 1.0, p.y)
		colors[i] = Color(1.0, 1.0, 1.0, wind)
		var radial := (p - crown_centre).normalized()
		var up_weight := smoothstep(0.2, 0.5, p.y) * 0.65
		normals[i] = radial.lerp(Vector3.UP, 0.15) if up_weight > 0.0 and radial.length() > 0.0 else Vector3.UP
		normals[i] = normals[i].normalized()
		uv_main[i] = Vector2(0.0, 1.0)
		uv_tex[i] = uvs[i]
	var final_arrays := []
	final_arrays.resize(Mesh.ARRAY_MAX)
	final_arrays[Mesh.ARRAY_VERTEX] = out_vertices
	final_arrays[Mesh.ARRAY_NORMAL] = normals
	final_arrays[Mesh.ARRAY_COLOR] = colors
	final_arrays[Mesh.ARRAY_TEX_UV] = uv_main
	final_arrays[Mesh.ARRAY_TEX_UV2] = uv_tex
	if not indices.is_empty():
		final_arrays[Mesh.ARRAY_INDEX] = indices
	var mesh_out := ArrayMesh.new()
	mesh_out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, final_arrays)
	mesh_out.custom_aabb = AABB(Vector3(-0.7, -0.1, -0.7), Vector3(1.4, 1.3, 1.4))
	_textures[row] = texture
	return mesh_out
