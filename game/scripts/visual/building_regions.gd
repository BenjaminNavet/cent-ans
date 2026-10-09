class_name BuildingRegions
extends RefCounted

## Style régional des maisons du kit (`data/art/building_regions.json`, schéma
## `art_building_regions.schema.json`). Par région de province (`data/provinces/<id>.json`,
## champ `region`) : part des maisons à colombage (`framed`, 0..1) et variantes du Midi
## (`southern` : enduit ou pierre, tuiles canal, faible pente). Normandie, Île-de-France, Picardie,
## Flandre, Angleterre : colombage courant ; Guyenne, Languedoc, Provence : pierre et enduit.
## `provinces` : exceptions par province, prioritaires sur la région (ex. Lorraine en pierre).
## Purement visuel ; lu une fois, sans repli chiffré (fichier absent : style vide = choix d'avant).

const DATA_FILE := "art/building_regions.json"

static var _lookup := JsonLookup.new(DATA_FILE)
static var _province_region: Dictionary = {}  # id de province → région (cache)


static func _read_json(relative: String) -> Variant:
	return DataFile.read_json(relative) if DataFile.exists(relative) else null


static func document() -> Dictionary:
	return _lookup.data()


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
