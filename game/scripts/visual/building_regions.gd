class_name BuildingRegions
extends RefCounted

## Lot TF : style régional des maisons du kit (`data/art/building_regions.json`, schéma
## `art_building_regions.schema.json`). Par région de province (`data/provinces/<id>.json`,
## champ `region`) : part des maisons à colombage (`framed`, 0..1) et variantes du Midi
## (`southern` : enduit ou pierre, tuiles canal, faible pente). Normandie, Île-de-France, Picardie,
## Flandre, Angleterre : colombage courant ; Guyenne, Languedoc, Provence : pierre et enduit.
## `provinces` : exceptions par province, prioritaires sur la région (ex. Lorraine en pierre).
## Purement visuel ; lu une fois, sans repli chiffré (fichier absent : style vide = choix d'avant).

const DATA_FILE := "art/building_regions.json"

static var _doc: Dictionary = {}
static var _loaded := false
static var _province_region: Dictionary = {}  # id de province → région (cache)


static func _data_dirs() -> Array[String]:
	var dirs: Array[String] = []
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		dirs.append(str(paths.get("data_dir")))
	dirs.append(ProjectSettings.globalize_path("res://").path_join("../data").simplify_path())
	return dirs


static func _read_json(relative: String) -> Variant:
	for dir in _data_dirs():
		var path := dir.path_join(relative)
		if FileAccess.file_exists(path):
			return JSON.parse_string(FileAccess.get_file_as_string(path))
	return null


static func document() -> Dictionary:
	if not _loaded:
		_loaded = true
		var parsed: Variant = _read_json(DATA_FILE)
		if parsed is Dictionary:
			_doc = parsed
		else:
			push_warning("BuildingRegions: %s introuvable, style régional désactivé" % DATA_FILE)
	return _doc


## Région (`region` du fichier de province) ; "" si inconnue.
static func region_of(province_id: String) -> String:
	if province_id == "":
		return ""
	if not _province_region.has(province_id):
		var parsed: Variant = _read_json("provinces/%s.json" % province_id)
		_province_region[province_id] = str((parsed as Dictionary).get("region", "")) if parsed is Dictionary else ""
	return _province_region[province_id]


## Style d'une région : entrée de `regions`, sinon `default` ; {} si les données manquent.
static func style_for_region(region: String) -> Dictionary:
	var doc := document()
	if doc.is_empty():
		return {}
	var regions: Dictionary = doc.get("regions", {})
	return regions.get(region, doc.get("default", {}))


## Style de la province d'une bataille ; {} sans province (démo, banc d'essai) : choix d'avant TF.
static func style_for_province(province_id: String) -> Dictionary:
	if province_id == "":
		return {}
	var overrides: Dictionary = document().get("provinces", {})
	if overrides.has(province_id):
		return overrides[province_id]
	return style_for_region(region_of(province_id))
