class_name HouseArms
extends RefCounted

## DA1 : armoiries des maisons (`data/heraldry/houses.json`, écus dessinés par
## `tools/cent_ans_tools/heraldry.py` dans `res://assets/heraldry/houses/<id>.png`).
## Rendu seulement : nom de maison → écu, maison d'un personnage, bannerets d'une faction
## (champ `vassal_of`) et croix de livrée du commun (`badges`).

const DATA_FILE := "heraldry/houses.json"
const DIR := "res://assets/heraldry/houses/"

static var _lookup := JsonLookup.new(DATA_FILE)
static var _indexed: bool = false
static var _by_key: Dictionary = {}  # nom exact ou id → entrée
static var _houses: Array = []  # entrées triées par id
static var _character_house: Dictionary = {}  # id de personnage → maison (fichiers de données)


## Fichier de `data/` : dossier de données du jeu (`MapPaths.data_dir`), puis `data/` du dépôt.
static func data_path(relative: String) -> String:
	return DataFile.path_of(relative) if DataFile.exists(relative) else ""


static func _ensure_indexed() -> void:
	if _indexed:
		return
	_indexed = true
	for house: Dictionary in _lookup.value("houses", []):
		_by_key[str(house["name"])] = house
		_by_key[str(house["id"])] = house
		_houses.append(house)
	_houses.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return str(a["id"]) < str(b["id"]))


## Entrée de la maison `house` (nom affiché, ex. « Plantagenêt », ou id), vide si inconnue.
static func entry(house: String) -> Dictionary:
	_ensure_indexed()
	return _by_key.get(house, {})


static func id_of(house: String) -> String:
	return str(entry(house).get("id", ""))


## Écu de la maison, null si elle n'a pas d'armes (nom inconnu, fichier absent).
static func texture(house: String) -> Texture2D:
	var id := id_of(house)
	if id == "":
		return null
	return PortraitLoader.load_texture(DIR + id + ".png")


## « Maison de Valois — D'azur semé de fleurs de lis d'or, à la bordure de gueules. »
static func tooltip(house: String) -> String:
	var data := entry(house)
	if data.is_empty():
		return ""
	var text := "Armes de la maison %s : %s" % [str(data["name"]), str(data["blazon"])]
	if str(data.get("certainty", "")) == "substitution":
		text += " (armes de substitution)"
	return text


## Maison d'un personnage : `sim.get_character` si la campagne tourne (personnages nés en jeu),
## sinon `data/characters/<id>.json`.
static func house_of(character_id: String, sim: Object = null) -> String:
	if character_id == "":
		return ""
	if sim != null and sim.has_method("get_character"):
		var character: Variant = sim.call("get_character", character_id)
		if character is Dictionary and not (character as Dictionary).is_empty():
			return str((character as Dictionary).get("house", ""))
	if _character_house.has(character_id):
		return _character_house[character_id]
	var house := ""
	var path := data_path("characters/%s.json" % character_id)
	if path != "":
		var parsed: Variant = DataFile.parse_file(path)
		if parsed is Dictionary:
			house = str((parsed as Dictionary).get("house", ""))
	_character_house[character_id] = house
	return house


## Ids des maisons dont des bannerets servent dans les armées de `faction` (hors `exclude`).
static func vassals(faction: String, exclude: String = "") -> Array[String]:
	_ensure_indexed()
	var out: Array[String] = []
	var skip := id_of(exclude) if exclude != "" else ""
	for house in _houses:
		if str(house["id"]) != skip and (house.get("vassal_of", []) as Array).has(faction):
			out.append(str(house["id"]))
	return out


## Croix de livrée du commun de `faction` : {cross: Color, patch: Color ou null}, vide sinon.
static func badge(faction: String) -> Dictionary:
	_ensure_indexed()
	var data: Dictionary = _lookup.section("badges").get(faction, {})
	if data.is_empty():
		return {}
	var out := {"cross": tincture(str(data["cross"]))}
	out["patch"] = tincture(str(data["patch"])) if data.get("patch") != null else null
	return out


## Teintes canoniques (bible DA § 3.1, `TINCTURES` de heraldry.py).
static func tincture(name: String) -> Color:
	match name:
		"or":
			return Color8(242, 194, 48)
		"argent":
			return Color8(245, 241, 230)
		"gueules":
			return Color8(176, 24, 43)
		"azur":
			return Color8(31, 58, 147)
		"sable":
			return Color8(26, 26, 26)
		"sinople":
			return Color8(30, 120, 60)
		"pourpre":
			return Color8(107, 42, 122)
	return Color.WHITE
