class_name JsonLookup
extends RefCounted

## Lecture paresseuse, une seule fois, d'un JSON de `data/` (via `DataFile`) avec accès par clé.
## Une instance par fichier, tenue dans une `static var` du script appelant :
## `static var _lookup := JsonLookup.new("map/sea_basins.json")`.
## - `defaults` : valeurs de repli, complétées (fusion récursive) par le fichier ;
## - `block` : n'utiliser que ce bloc de premier niveau du fichier (doit être un dictionnaire) ;
## - `required_key` : clé qui doit exister, sinon le fichier est jugé invalide.
## Fichier absent ou invalide : un seul avertissement, `defaults` seul.

var rel_path: String
var defaults: Dictionary
var block: String
var required_key: String

var _data: Dictionary = {}
var _loaded := false


func _init(path: String, fallback: Dictionary = {}, block_name: String = "", required: String = "") -> void:
	rel_path = path
	defaults = fallback
	block = block_name
	required_key = required


## Contenu du fichier (lu au premier appel).
func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		_data = _read()
	return _data


## Bloc `name` du contenu ({} s'il manque ou n'est pas un dictionnaire).
func section(name: String) -> Dictionary:
	var value: Variant = data().get(name, {})
	return value if value is Dictionary else {}


## Valeur de `key` dans le contenu, `fallback` si absente.
func value(key: String, fallback: Variant = null) -> Variant:
	return data().get(key, fallback)


## Oublie le contenu lu : relu au prochain accès (tests, autre dossier de données).
func reload() -> void:
	_loaded = false
	_data = {}


func _read() -> Dictionary:
	var result := defaults.duplicate(true)
	var parsed: Variant = DataFile.read_json(rel_path) if DataFile.exists(rel_path) else null
	var content: Variant = parsed
	if parsed is Dictionary and block != "":
		content = (parsed as Dictionary).get(block)
	if not content is Dictionary or (required_key != "" and not (content as Dictionary).has(required_key)):
		push_warning("JsonLookup: %s missing or invalid" % rel_path)
		return result
	result.merge(content, true)
	return result
