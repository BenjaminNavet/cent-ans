extends Node

## Autoload `CodexStore` (H2) : fiches du Codex (bundle `data/codex_bundle.json` généré depuis
## `data/codex/*.json`, schéma `data/schemas/codex.schema.json`), index des alias pour les auto-liens, et découvertes du
## joueur. Les découvertes sont une méta-progression (fichier `user://codex.json`), pas un état
## de partie : aucune règle de jeu ici.
##
## `data/` est résolu par `DataFile` : `MapPaths.data_dir` (éditeur ou export .app), avec repli
## sur `<racine du dépôt>/data` si le bundle y est absent (fixtures du smoke).

signal discovered(id: String)
signal loaded

const SAVE_PATH := "user://codex.json"
const TEST_SAVE_PATH := "user://codex_test.json"
const BUNDLE_FILE := "codex_bundle.json"  # relatif à data/

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
	["Armées, navires et bâtiments", ["unite", "batiment", "technique"], "Armées"],
	["Mécaniques du jeu", ["mecanique"], "Le jeu"],
]
const CATEGORY_LABELS := {
	"personnage": "Personnage", "dynastie": "Dynastie", "lieu": "Lieu", "bataille": "Bataille",
	"evenement": "Événement", "institution": "Institution", "societe": "Société",
	"guerre": "Guerre", "religion": "Religion", "economie": "Économie", "cuisine": "Cuisine",
	"ingredient": "Ingrédient", "recette": "Recette", "medecine": "Médecine", "plante": "Plante",
	"savoir": "Savoir", "vie_quotidienne": "Vie quotidienne", "mecanique": "Mécanique du jeu",
	"batiment": "Bâtiment", "unite": "Unité", "technique": "Technique",
}

var entries: Dictionary = {}  # id → fiche
var bundle_path: String = ""
var _aliases: Dictionary = {}  # alias en minuscules → id
var _by_entity: Dictionary = {}  # entité de jeu → id
var _exclusions: Dictionary = {}  # id → expressions en minuscules où l'alias n'est pas lié (B8)
var _discovered: Dictionary = {}  # id → true
var _save_path: String = SAVE_PATH
var _alias_regex: RegEx


func _ready() -> void:
	reload()
	_load_discoveries()


## Recharge les fiches depuis le bundle `bundle_path` (défaut : `data/codex_bundle.json` de
## `DataFile`). Le bundle est généré depuis `data/codex/cdx_*.json` (`cent-ans codex-bundle`,
## schéma `codex_bundle.schema.json`) : un seul fichier lu au démarrage au lieu de ~480.
func reload(path: String = "") -> void:
	bundle_path = path if path != "" else DataFile.path_of(BUNDLE_FILE)
	entries.clear()
	_aliases.clear()
	_by_entity.clear()
	_exclusions.clear()
	_alias_regex = null
	var parsed: Variant = DataFile.parse_file(bundle_path)
	if parsed is Dictionary:
		for entry in parsed.get("entries", []):
			if entry is Dictionary and entry.has("id"):
				_add(entry)
	loaded.emit()


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
	var contexts: Array = []
	for context in entry.get("exclude_contexts", []):
		var lowered := str(context).strip_edges().to_lower()
		if lowered != "":
			contexts.append(lowered)
	if not contexts.is_empty():
		_exclusions[id] = contexts


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


## Expressions (minuscules) de la fiche `id` dans lesquelles un alias n'est pas auto-lié
## (`exclude_contexts`, B8 : « Louis de Poitiers » pour la bataille de Poitiers).
func exclude_contexts(id: String) -> Array:
	return _exclusions.get(id, [])


## Vrai si l'occurrence `[start, end[` d'un alias de `id` dans `text` fait partie d'une de ses
## expressions exclues (casse ignorée).
func is_excluded(id: String, text: String, start: int, end: int) -> bool:
	var contexts: Array = _exclusions.get(id, [])
	if contexts.is_empty():
		return false
	var alias := text.substr(start, end - start).to_lower()
	for context: String in contexts:
		var offset := context.find(alias)
		while offset >= 0:
			var from := start - offset
			if from >= 0 and text.substr(from, context.length()).to_lower() == context:
				return true
			offset = context.find(alias, offset + 1)
	return false


## Expression régulière de tous les alias, ou null. Les plus longs d'abord : à une même
## position, l'alternance retient l'alias le plus long (« Philippe VI » avant « Philippe »).
## Mots entiers : ni lettre, chiffre ou `_` accolés, ni tiret entre deux mots (« Saint-Omer »
## ne lie pas « Omer », « Poitiers-sur-… » ne lie pas « Poitiers ») ; l'apostrophe sépare
## (« d'Artois » lie « Artois »).
func alias_regex() -> RegEx:
	if _alias_regex != null or _aliases.is_empty():
		return _alias_regex
	var keys: Array = _aliases.keys()
	keys.sort_custom(func(a: String, b: String) -> bool: return a.length() > b.length())
	var escaped := PackedStringArray()
	for key in keys:
		escaped.append(_escape(str(key)))
	_alias_regex = RegEx.create_from_string("(?i)(?<![\\p{L}\\p{N}_]|[\\p{L}\\p{N}]-)(%s)(?![\\p{L}\\p{N}_]|-[\\p{L}\\p{N}])" % "|".join(escaped))
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
## T2 : `test_path` isole ce fichier par exécution (smoke test en parallèle).
func use_test_file(test_path: String = TEST_SAVE_PATH) -> void:
	_save_path = test_path
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
