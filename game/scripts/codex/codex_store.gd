extends Node

## Autoload `CodexStore` (H2) : fiches du Codex (`data/codex/*.json`, schéma
## `data/schemas/codex.schema.json`), index des alias pour les auto-liens, et découvertes du
## joueur. Les découvertes sont une méta-progression (fichier `user://codex.json`), pas un état
## de partie : aucune règle de jeu ici.
##
## `data/` est résolu comme `GameCatalog` : `MapPaths.data_dir` (éditeur ou export .app), avec
## repli sur `<racine du dépôt>/data/codex` si ce dossier n'a pas de fiches (fixtures du smoke).

signal discovered(id: String)
signal loaded

const SAVE_PATH := "user://codex.json"
const TEST_SAVE_PATH := "user://codex_test.json"

## Grandes familles de la fenêtre Codex (§1.4) : [libellé, catégories, libellé court d'onglet].
const FAMILIES := [
	["Personnages et dynasties", ["personnage", "dynastie"], "Personnages"],
	["Lieux", ["lieu"], "Lieux"],
	["Guerre et batailles", ["guerre", "bataille"], "Guerre"],
	["Société et institutions", ["institution", "societe", "economie", "evenement", "vie_quotidienne"], "Société"],
	["Religion", ["religion"], "Religion"],
	["Table", ["cuisine", "ingredient", "recette"], "Table"],
	["Médecine et herbier", ["medecine", "plante"], "Médecine"],
	["Savoirs", ["savoir"], "Savoirs"],
]
const CATEGORY_LABELS := {
	"personnage": "Personnage", "dynastie": "Dynastie", "lieu": "Lieu", "bataille": "Bataille",
	"evenement": "Événement", "institution": "Institution", "societe": "Société",
	"guerre": "Guerre", "religion": "Religion", "economie": "Économie", "cuisine": "Cuisine",
	"ingredient": "Ingrédient", "recette": "Recette", "medecine": "Médecine", "plante": "Plante",
	"savoir": "Savoir", "vie_quotidienne": "Vie quotidienne",
}

var entries: Dictionary = {}  # id → fiche
var codex_dir: String = ""
var _aliases: Dictionary = {}  # alias en minuscules → id
var _by_entity: Dictionary = {}  # entité de jeu → id
var _discovered: Dictionary = {}  # id → true
var _save_path: String = SAVE_PATH
var _alias_regex: RegEx


func _ready() -> void:
	reload()
	_load_discoveries()


## Recharge les fiches depuis `directory` (défaut : `data/codex` de `MapPaths`).
func reload(directory: String = "") -> void:
	codex_dir = directory if directory != "" else _default_dir()
	entries.clear()
	_aliases.clear()
	_by_entity.clear()
	_alias_regex = null
	var dir := DirAccess.open(codex_dir)
	if dir != null:
		for file_name in dir.get_files():
			if not file_name.begins_with("cdx_") or not file_name.ends_with(".json"):
				continue
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(codex_dir.path_join(file_name)))
			if parsed is Dictionary and parsed.has("id"):
				_add(parsed)
	loaded.emit()


func _default_dir() -> String:
	var paths: Node = get_node_or_null("/root/MapPaths")
	var data_dir: String = str(paths.get("data_dir")) if paths != null else ""
	var candidate := data_dir.path_join("codex")
	if data_dir != "" and _has_entries(candidate):
		return candidate
	return ProjectSettings.globalize_path("res://").path_join("../data/codex").simplify_path()


static func _has_entries(directory: String) -> bool:
	var dir := DirAccess.open(directory)
	if dir == null:
		return false
	for file_name in dir.get_files():
		if file_name.begins_with("cdx_") and file_name.ends_with(".json"):
			return true
	return false


func _add(entry: Dictionary) -> void:
	var id := str(entry["id"])
	entries[id] = entry
	for form in [str(entry.get("title", ""))] + Array(entry.get("aliases", [])):
		var key := str(form).strip_edges().to_lower()
		if key.length() >= 2 and not _aliases.has(key):
			_aliases[key] = id
	var entity := str(entry.get("entity", ""))
	if entity != "":
		_by_entity[entity] = id


func has_entry(id: String) -> bool:
	return entries.has(id)


func entry(id: String) -> Dictionary:
	return entries.get(id, {})


func title(id: String) -> String:
	return str(entry(id).get("title", id))


func category_label(id: String) -> String:
	var category := str(entry(id).get("category", ""))
	return str(CATEGORY_LABELS.get(category, category.capitalize()))


## « 1338 – 1380 », « 1346 » ou vide.
func era_label(id: String) -> String:
	var era: Dictionary = entry(id).get("era", {})
	var from := str(era.get("from", ""))
	var to := str(era.get("to", ""))
	if from != "" and to != "" and from != to:
		return "%s – %s" % [from, to]
	return from if from != "" else to


## Fiche liée à une entité de jeu (`chr_…`, `evt_…`, `tech_…`), vide sinon.
func entry_for_entity(entity_id: String) -> String:
	return str(_by_entity.get(entity_id, ""))


## Id désigné par un alias (casse ignorée), vide sinon.
func id_for_alias(alias: String) -> String:
	return str(_aliases.get(alias.strip_edges().to_lower(), ""))


## Expression régulière de tous les alias (les plus longs d'abord, mots entiers), ou null.
func alias_regex() -> RegEx:
	if _alias_regex != null or _aliases.is_empty():
		return _alias_regex
	var keys: Array = _aliases.keys()
	keys.sort_custom(func(a: String, b: String) -> bool: return a.length() > b.length())
	var escaped := PackedStringArray()
	for key in keys:
		escaped.append(_escape(str(key)))
	_alias_regex = RegEx.create_from_string("(?i)(?<![\\p{L}\\p{N}_])(%s)(?![\\p{L}\\p{N}_])" % "|".join(escaped))
	return _alias_regex


static func _escape(text: String) -> String:
	var out := ""
	for character in text:
		out += "\\" + character if "\\^$.|?*+()[]{}-/".contains(character) else character
	return out


func families() -> Array:
	return FAMILIES


## Ids d'une famille (index de `FAMILIES`, -1 = toutes), triés par titre.
func ids_in_family(family: int) -> Array:
	var categories: Array = FAMILIES[family][1] if family >= 0 and family < FAMILIES.size() else []
	var result: Array = []
	for id in entries:
		if family < 0 or categories.has(str(entries[id].get("category", ""))):
			result.append(id)
	result.sort_custom(func(a: String, b: String) -> bool: return title(a).naturalnocasecmp_to(title(b)) < 0)
	return result


# --- Découvertes ----------------------------------------------------------------------------


func is_discovered(id: String) -> bool:
	return _discovered.has(id)


## Marque la fiche découverte (sauvegarde immédiate) ; renvoie true si c'est une nouveauté.
func discover(id: String) -> bool:
	if not entries.has(id) or _discovered.has(id):
		return false
	_discovered[id] = true
	_save_discoveries()
	discovered.emit(id)
	return true


func discovered_count() -> int:
	var count := 0
	for id in _discovered:
		if entries.has(id):
			count += 1
	return count


func total_count() -> int:
	return entries.size()


## Tests : découvertes dans un fichier dédié, vidé (le fichier du joueur n'est pas touché).
func use_test_file() -> void:
	_save_path = TEST_SAVE_PATH
	_discovered.clear()
	_save_discoveries()


func reset_discoveries() -> void:
	_discovered.clear()
	_save_discoveries()


func _load_discoveries() -> void:
	_discovered.clear()
	if not FileAccess.file_exists(_save_path):
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(_save_path))
	if parsed is Dictionary:
		for id in parsed.get("discovered", []):
			_discovered[str(id)] = true


func _save_discoveries() -> void:
	var file := FileAccess.open(_save_path, FileAccess.WRITE)
	if file != null:
		file.store_string(JSON.stringify({"version": 1, "discovered": _discovered.keys()}, "\t"))
