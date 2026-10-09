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
	var region := region_of(province_id)
	var style: Dictionary = overrides[province_id] if overrides.has(province_id) else style_for_region(region)
	if style.is_empty():
		return style
	# TX T4 : la région (matières régionales) accompagne le style ; copie, le document reste intact.
	var result := style.duplicate()
	result["region"] = region
	return result


## TX T4 : matières régionales d'une région, `{rôle: id de couche}` du paquet `tx_building_pack.json`
## (champ facultatif `materials` de `data/art/building_regions.json`) ; {} sans entrée (matières par
## défaut de `building_materials.json`).
static func materials_for_region(region: String) -> Dictionary:
	var doc := document()
	var regions: Dictionary = doc.get("regions", {})
	var style: Dictionary = regions.get(region, doc.get("default", {}))
	return style.get("materials", {})


## Identifiants des régions du fichier, triés : l'indice d'une région dans cette liste est celui des
## tables `region_table` des shaders de bâtiments (`building_regional.gdshaderinc`).
static func region_ids() -> Array:
	var ids: Array = (document().get("regions", {}) as Dictionary).keys()
	ids.sort()
	return ids
