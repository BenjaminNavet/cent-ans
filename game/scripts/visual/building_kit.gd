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


## Lot TF : style régional des maisons de la bataille en cours (`BuildingRegions.style_for_province`,
## posé par `BattleTerrain.build`) : `framed` (part de colombage), `southern` (variantes du Midi).
## Vide : choix d'avant TF (colombage selon les modèles, sans variantes du Midi).
static var region_style: Dictionary = {}
## Types dont le kit exporte des variantes à colombage et du Midi (`manifest.json` : `framed`,
## `southern`).
const REGIONAL_KINDS: Array[String] = ["cottage", "timber", "townhouse"]


## Modèles d'un type (`cottage`, `timber`, `townhouse`…), intacts ou en ruine. Lot TF : les
## variantes du Midi (`southern`) ne sortent que si `southern` le demande.
static func models_of(kind: String, ruined: bool = false, southern: bool = false) -> Array:
	var out := []
	var all := manifest()
	for model_name in all:
		var entry: Dictionary = all[model_name]
		if str(entry["kind"]) == kind and bool(entry["ruined"]) == ruined and bool(entry.get("southern", false)) == southern:
			out.append(model_name)
	out.sort()
	return out


## Lot TF : candidats d'un type selon le style régional (`region_style`) ; tire à pied de colombage
## ou non (`framed`), puis garde les modèles de ce genre (variantes du Midi si `southern`). Sans
## modèle qui convienne : maisons de pierre pour `timber`/`townhouse`, sinon tous les modèles.
static func regional_candidates(kind: String, rng: RandomNumberGenerator, style: Dictionary) -> Array:
	var want_framed := rng.randf() < float(style.get("framed", 0.5))
	var southern := bool(style.get("southern", false))
	var all := manifest()
	var out := []
	var pool := models_of(kind, false, southern and not want_framed)
	for model_name in pool:
		if bool((all[model_name] as Dictionary).get("framed", false)) == want_framed:
			out.append(model_name)
	if out.is_empty() and not want_framed and kind != "cottage":
		out = models_of("stonehouse")
	return out if not out.is_empty() else models_of(kind)


## Le modèle de `kind` dont les proportions épousent le mieux `length` × `width` (tirage parmi
## les deux meilleurs pour varier) ; "" si aucun. Lot TF : maisons intactes des types régionaux
## choisies selon `region_style` (colombage au nord, enduit et pierre au Midi).
static func pick(kind: String, length: float, width: float, rng: RandomNumberGenerator, ruined: bool = false) -> String:
	var candidates := models_of(kind, ruined)
	if not ruined and not region_style.is_empty() and kind in REGIONAL_KINDS:
		candidates = regional_candidates(kind, rng, region_style)
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


## BR3 : modèle du kit pour une pièce de mobilier du cœur (`kind` : `stall`, `cart`, `barrels`,
## `woodpile`, `well`), choisi par son indice (déterministe) ; "" si aucun.
static func prop_model(kind: String, index: int) -> String:
	var models := models_of(kind)
	if models.is_empty():
		return ""
	return models[int(hash01(index, 29) * models.size()) % models.size()]


## BR3 : transformation d'une pièce de mobilier du cœur `{x, z, yaw}` (lacet du cœur : longueur
## le long de (cos, sin), face avant (+Z du modèle) vers (-sin, cos)), posée à la hauteur `y`.
## Taille réelle : les emprises des données (`data/rules/siege_town.json`) sont celles du manifeste.
static func prop_transform(prop: Dictionary, y: float) -> Transform3D:
	var basis := Basis(Vector3.UP, -float(prop["yaw"]))
	return Transform3D(basis, Vector3(float(prop["x"]), y, float(prop["z"])))


## Tirage déterministe dans [0, 1) à partir d'un entier et d'un sel (variété du rendu seulement).
static func hash01(key: int, salt: int) -> float:
	var h := hash(Vector2i(key, salt))
	return float(h & 0xFFFFFF) / float(0x1000000)


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
	var _ga3: Ga3Kit.Batch = null  # variantes générées (GA3-L1), null sans elles

	func _init(p_variant: String = "", p_visibility_end: float = 0.0) -> void:
		variant = p_variant
		visibility_end = p_visibility_end

	## Ajoute un bâtiment ; renvoie sa poignée [nom, indice] (pour le cacher plus tard). Lot
	## GA3-L1 : le modèle peut être remplacé par sa variante générée (`Ga3Kit.variant_for`, pas
	## sous la neige) ; la poignée désigne alors l'instance GA3 (nom en `ga3_`).
	func add(model_name: String, xform: Transform3D) -> Array:
		if variant != "snow":
			var ga3 := Ga3Kit.variant_for(model_name, xform)
			if ga3 != "":
				if _ga3 == null:
					_ga3 = Ga3Kit.Batch.new(visibility_end)
				return _ga3.add(ga3, Ga3Kit.fitted_transform(ga3, model_name, xform))
		if not _items.has(model_name):
			_items[model_name] = []
		(_items[model_name] as Array).append(xform)
		return [model_name, (_items[model_name] as Array).size() - 1]

	func count() -> int:
		var total := 0 if _ga3 == null else _ga3.count()
		for model_name in _items:
			total += (_items[model_name] as Array).size()
		return total

	## Nombre d'instances GA3 (variantes générées) de ce lot.
	func ga3_count() -> int:
		return 0 if _ga3 == null else _ga3.count()

	func build(parent: Node3D) -> void:
		if _ga3 != null:
			_ga3.build(parent)
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
		if _ga3 != null and str(handle[0]).begins_with("ga3_"):
			return _ga3.get_transform(handle)
		var mmi: MultiMeshInstance3D = _instances.get(handle[0])
		if mmi == null:
			return Transform3D()
		return mmi.multimesh.get_instance_transform(int(handle[1]))

	func set_transform(handle: Array, xform: Transform3D) -> void:
		if _ga3 != null and str(handle[0]).begins_with("ga3_"):
			_ga3.set_transform(handle, xform)
			return
		var mmi: MultiMeshInstance3D = _instances.get(handle[0])
		if mmi != null:
			mmi.multimesh.set_instance_transform(int(handle[1]), xform)

	## Cache une instance (échelle nulle, sans modifier les autres).
	func hide(handle: Array) -> void:
		var xform := get_transform(handle)
		set_transform(handle, Transform3D(Basis().scaled(Vector3.ONE * 0.0001), xform.origin - Vector3(0, 50, 0)))
