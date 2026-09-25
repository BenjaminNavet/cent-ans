class_name BuildingKit
extends RefCounted

## Lot BR1 (ADR 0021) : bâtiments réalistes du kit Blender (`game/assets/models/buildings/`,
## `manifest.json` : type, ruine, emprise murs `length` × `depth`, hauteur, triangles).
## Choisit le modèle qui épouse le mieux une emprise de la simulation, et pose les bâtiments par
## `MultiMesh` (une instance par bâtiment, un appel de dessin par surface de modèle) à travers un
## `Batch`. Repère des modèles : origine au centre de l'emprise, sol à y = 0 (fondations jusqu'à
## -2 m pour les pentes), faîtage le long de +X, façade vers +Z. Purement visuel.

const DIR := "res://assets/models/buildings/"

static var _manifest: Dictionary = {}
static var _loaded := false
static var _source_meshes: Dictionary = {}  # nom → Mesh importé


static func clear_cache() -> void:
	_manifest.clear()
	_loaded = false
	_source_meshes.clear()


static func manifest() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := DIR + "manifest.json"
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_manifest = parsed
	return _manifest


static func available() -> bool:
	return not manifest().is_empty()


## Modèles d'un type (`cottage`, `timber`, `townhouse`…), intacts ou en ruine.
static func models_of(kind: String, ruined: bool = false) -> Array:
	var out := []
	var all := manifest()
	for model_name in all:
		var entry: Dictionary = all[model_name]
		if str(entry["kind"]) == kind and bool(entry["ruined"]) == ruined:
			out.append(model_name)
	out.sort()
	return out


## Le modèle de `kind` dont les proportions épousent le mieux `length` × `width` (tirage parmi
## les deux meilleurs pour varier) ; "" si aucun.
static func pick(kind: String, length: float, width: float, rng: RandomNumberGenerator, ruined: bool = false) -> String:
	var candidates := models_of(kind, ruined)
	if candidates.is_empty():
		return ""
	var target := length / maxf(width, 0.1)
	var all := manifest()
	candidates.sort_custom(func(a: String, b: String) -> bool:
		var ra := float(all[a]["length"]) / float(all[a]["depth"])
		var rb := float(all[b]["length"]) / float(all[b]["depth"])
		return absf(log(ra / target)) < absf(log(rb / target)))
	return candidates[rng.randi_range(0, mini(1, candidates.size() - 1))]


## Échelle (x, y, z) qui amène l'emprise murs du modèle sur `length` × `width` ; la hauteur suit
## la moyenne géométrique (bornée) pour ne pas écraser les toits.
static func fit_scale(model_name: String, length: float, width: float) -> Vector3:
	var entry: Dictionary = manifest().get(model_name, {})
	if entry.is_empty():
		return Vector3.ONE
	var sx := length / float(entry["length"])
	var sz := width / float(entry["depth"])
	var sy := clampf(sqrt(sx * sz), 0.75, 1.3)
	return Vector3(sx, sy, sz)


## Maillage importé du modèle (premier `MeshInstance3D` du GLB), matériaux non remplacés.
static func source_mesh(model_name: String) -> Mesh:
	if _source_meshes.has(model_name):
		return _source_meshes[model_name]
	var mesh: Mesh = null
	var path := DIR + model_name + ".glb"
	if ResourceLoader.exists(path):
		var scene := load(path) as PackedScene
		if scene != null:
			var root := scene.instantiate()
			for child in root.find_children("*", "MeshInstance3D", true, false):
				mesh = (child as MeshInstance3D).mesh
				break
			root.free()
	_source_meshes[model_name] = mesh
	return mesh


## Maillage prêt à poser (matériaux partagés de `BuildingMaterials`, variante "" / "snow" / "far").
static func mesh(model_name: String, variant: String = "") -> Mesh:
	return BuildingMaterials.remap_mesh(source_mesh(model_name), variant)


## Accumule des bâtiments puis les pose en `MultiMeshInstance3D` (un par modèle).
class Batch:
	extends RefCounted

	var variant := ""
	var visibility_end := 0.0
	var _items: Dictionary = {}  # nom → Array[Transform3D]
	var _instances: Dictionary = {}  # nom → MultiMeshInstance3D (après build)

	func _init(p_variant: String = "", p_visibility_end: float = 0.0) -> void:
		variant = p_variant
		visibility_end = p_visibility_end

	## Ajoute un bâtiment ; renvoie sa poignée [nom, indice] (pour le cacher plus tard).
	func add(model_name: String, xform: Transform3D) -> Array:
		if not _items.has(model_name):
			_items[model_name] = []
		(_items[model_name] as Array).append(xform)
		return [model_name, (_items[model_name] as Array).size() - 1]

	func count() -> int:
		var total := 0
		for model_name in _items:
			total += (_items[model_name] as Array).size()
		return total

	func build(parent: Node3D) -> void:
		for model_name in _items:
			var mesh := BuildingKit.mesh(model_name, variant)
			if mesh == null:
				continue
			var transforms: Array = _items[model_name]
			var mm := MultiMesh.new()
			mm.transform_format = MultiMesh.TRANSFORM_3D
			mm.mesh = mesh
			mm.instance_count = transforms.size()
			for i in transforms.size():
				mm.set_instance_transform(i, transforms[i])
			var mmi := MultiMeshInstance3D.new()
			mmi.name = "Kit_" + model_name
			mmi.multimesh = mm
			if visibility_end > 0.0:
				mmi.visibility_range_end = visibility_end
			parent.add_child(mmi)
			_instances[model_name] = mmi

	## Transformation actuelle d'une poignée (Transform3D() si inconnue).
	func get_transform(handle: Array) -> Transform3D:
		var mmi: MultiMeshInstance3D = _instances.get(handle[0])
		if mmi == null:
			return Transform3D()
		return mmi.multimesh.get_instance_transform(int(handle[1]))

	func set_transform(handle: Array, xform: Transform3D) -> void:
		var mmi: MultiMeshInstance3D = _instances.get(handle[0])
		if mmi != null:
			mmi.multimesh.set_instance_transform(int(handle[1]), xform)

	## Cache une instance (échelle nulle, sans modifier les autres).
	func hide(handle: Array) -> void:
		var xform := get_transform(handle)
		set_transform(handle, Transform3D(Basis().scaled(Vector3.ONE * 0.0001), xform.origin - Vector3(0, 50, 0)))
