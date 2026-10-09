class_name DnWaterModels
extends RefCounted

## Lot DN-FLEUVE : glb générés de ce qui est sur ou au bord de l'eau (navires, bateaux de fleuve,
## ponts, moulins, ports, chantiers, épaves). Registre et règles dans `data/art/dn_water_models.json`
## (schéma `art_dn_water_models.schema.json`) : les règles désignent des identifiants du registre,
## ce module choisit (bassin, culture, fleuve, structure, année) et prépare maillage + repère de
## pose. Fichier absent, identifiant inconnu ou glb non importé : l'appelant garde son ancien
## rendu. Purement visuel.

const DATA_FILE := "art/dn_water_models.json"

## Année courante pour `from_year` / `until_year` (0 : `defaults.year`). La carte ne la connaît pas
## encore ; l'écran de campagne pourra la renseigner.
static var year_override := 0

static var _doc: Dictionary = {}
static var _loaded := false
static var _infos: Dictionary = {}


static func clear_cache() -> void:
	_doc = {}
	_loaded = false
	_infos = {}


static func document() -> Dictionary:
	if not _loaded:
		_loaded = true
		if DataFile.exists(DATA_FILE):
			var parsed: Variant = DataFile.read_json(DATA_FILE)
			if parsed is Dictionary:
				_doc = parsed
	return _doc


## Tests : remplace le document.
static func set_document(doc: Dictionary) -> void:
	_doc = doc
	_loaded = true
	_infos = {}


static func is_empty() -> bool:
	return (document().get("models", {}) as Dictionary).is_empty()


static func default_value(key: String, fallback: float) -> float:
	return float((document().get("defaults", {}) as Dictionary).get(key, fallback))


static func year() -> int:
	return year_override if year_override != 0 else int(default_value("year", 1337.0))


static func section(name: String) -> Dictionary:
	var value: Variant = document().get(name, {})
	return value if value is Dictionary else {}


static func model_entry(id: String) -> Dictionary:
	var entry: Variant = (document().get("models", {}) as Dictionary).get(id, {})
	return entry if entry is Dictionary else {}


## L'identifiant existe-t-il et est-il en service cette année-là ?
static func in_service(id: String) -> bool:
	var entry := model_entry(id)
	if entry.is_empty():
		return false
	var y := year()
	return y >= int(entry.get("from_year", -9999)) and y <= int(entry.get("until_year", 9999))


## Identifiant tiré dans `ids` par `index` (modulo), parmi ceux en service ; "" si aucun.
static func pick(ids: Variant, index: int) -> String:
	if not (ids is Array):
		return ""
	var usable: Array = []
	for id in ids:
		if in_service(str(id)):
			usable.append(str(id))
	if usable.is_empty():
		return ""
	return usable[absi(index) % usable.size()]


# --- Règles -----------------------------------------------------------------------------


## Navires d'une flotte : culture de la faction d'abord, sinon bassin, sinon repli.
static func fleet_ids(basin: String, culture: String) -> Array:
	var fleet := section("fleet")
	var by_culture: Dictionary = fleet.get("by_culture", {})
	if culture != "" and by_culture.has(culture):
		return by_culture[culture]
	var by_basin: Dictionary = fleet.get("by_basin", {})
	if basin != "" and by_basin.has(basin):
		return by_basin[basin]
	return fleet.get("default", [])


static func lane_ids(basin: String) -> Array:
	var lanes := section("sea_lanes")
	var by_basin: Dictionary = lanes.get("by_basin", {})
	if basin != "" and by_basin.has(basin):
		return by_basin[basin]
	return lanes.get("default", [])


static func river_ids(river_name: String) -> Array:
	var boats := section("river_boats")
	for rule: Dictionary in boats.get("rules", []):
		if (rule.get("rivers", []) as Array).has(river_name):
			return rule["ships"]
	return boats.get("default", [])


## Identifiant de pont : `by_id` (sous-chaîne de l'id ou du nom, minuscules) d'abord, sinon la
## liste de la structure tirée par `variant`. "" sans modèle (gué, structure inconnue).
static func bridge_id(structure: String, crossing_id: String, crossing_name: String, variant: int) -> String:
	var bridges := section("bridges")
	var haystack := (crossing_id + " " + crossing_name).to_lower()
	for key: String in bridges.get("by_id", {}):
		if haystack.contains(key) and in_service(str(bridges["by_id"][key])):
			return str(bridges["by_id"][key])
	return pick((bridges.get("by_structure", {}) as Dictionary).get(structure), variant)


# --- Maillage et repère de pose -------------------------------------------------------------


## Données d'un modèle au niveau de détail `lod` (cache) : {id, mesh, base, length, size,
## fit}. `base` ramène le maillage dans un repère à proue sur +X, centré en X/Z, posé sur y = 0,
## à `length` unités carte (ou à une longueur de 1 pour un modèle sans `length`, ex. pont) ;
## `size` : dimensions dans ce repère (avant `fit`). Dictionnaire vide si le glb manque.
static func info(id: String, lod: int) -> Dictionary:
	var key := "%s|%d" % [id, lod]
	if _infos.has(key):
		return _infos[key]
	var result := _measure(id, lod)
	_infos[key] = result
	return result


static func _measure(id: String, lod: int) -> Dictionary:
	var entry := model_entry(id)
	if entry.is_empty():
		return {}
	var scene := ModelLibrary.get_scene("%s_lod%d" % [entry["path"], lod])
	if scene == null:
		return {}
	var root := scene.instantiate() as Node3D
	if root == null:
		return {}
	var found := root.find_children("*", "MeshInstance3D", true, false)
	if root is MeshInstance3D:
		found.push_front(root)
	var result := {}
	if not found.is_empty():
		var mesh_instance := found[0] as MeshInstance3D
		var local := Transform3D.IDENTITY
		var node: Node3D = mesh_instance
		while node != null and node != root:
			local = node.transform * local
			node = node.get_parent() as Node3D
		var mesh := mesh_instance.mesh
		var box := local * mesh.get_aabb()
		# Axe long sur X (puis rotation propre au modèle) : proue sur +X.
		var turn := deg_to_rad(float(entry.get("yaw_deg", 0.0)))
		if box.size.z > box.size.x:
			turn += PI * 0.5
		var rotation := Basis(Vector3.UP, turn)
		var rotated := Transform3D(rotation, Vector3.ZERO) * local
		var oriented := rotated * mesh.get_aabb()
		var long_side := maxf(oriented.size.x, 0.001)
		var target := float(entry.get("length", 1.0))
		var fit := target / long_side
		var center := oriented.get_center()
		var recentre := Transform3D(Basis.IDENTITY, Vector3(-center.x, -oriented.position.y, -center.z))
		var base := Transform3D(Basis.from_scale(Vector3.ONE * fit), Vector3.ZERO) * recentre * rotated
		result = {"id": id, "mesh": mesh, "base": base, "length": target, "size": oriented.size, "fit": fit}
	root.free()
	return result


## Transformation monde d'un modèle : proue vers `heading` (vecteur carte x/z), au point
## `position`, grossi de `scale_multiplier`, relevé de `lift` unités du modèle.
static func pose(model_info: Dictionary, position: Vector3, heading: Vector2, scale_multiplier: float = 1.0) -> Transform3D:
	var entry := model_entry(str(model_info.get("id", "")))
	var raise := float(entry.get("lift", 0.0)) * scale_multiplier
	var yaw := -atan2(heading.y, heading.x)
	var basis := Basis(Vector3.UP, yaw).scaled(Vector3.ONE * scale_multiplier)
	return Transform3D(basis, position + Vector3(0.0, raise, 0.0)) * (model_info["base"] as Transform3D)
