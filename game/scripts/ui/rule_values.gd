class_name RuleValues
extends RefCounted

## Lot SV4 : valeurs de règles citées par les textes d'interface (aide, infobulles,
## encyclopédie), lues dans le cœur par `GameDataStore.get_rule_constants()` au lieu d'être
## recopiées dans les GDScript. Un texte écrit `{rule.nom}` là où il cite une valeur ;
## `format()` la remplace par le nombre au format français (« 1,5 », « 1 000 ») ;
## `{rule.nom:1}` impose une décimale (« 1,0 »).
## Sans données chargées (maquette, fixtures), une valeur inconnue s'affiche « ? ».

const UNKNOWN := "?"
static var _cache: Dictionary = {}
static var _pattern: RegEx = null


## Dictionnaire `{nom: valeur}` du cœur (vide si le magasin de données manque).
static func all() -> Dictionary:
	if _cache.is_empty():
		var tree := Engine.get_main_loop() as SceneTree
		var facade: Node = tree.root.get_node_or_null("SimFacade") if tree != null else null
		var store: Object = facade.get("store") if facade != null else null
		if store != null and store.has_method("get_rule_constants"):
			_cache = store.call("get_rule_constants")
	return _cache


## Valeur `name`, ou `fallback` si elle est inconnue (données absentes).
static func value(name: String, fallback: float = NAN) -> float:
	return float(all().get(name, fallback))


## Vrai si la valeur `name` est connue.
static func has(name: String) -> bool:
	return all().has(name)


## Valeur `name` au format français, « ? » si inconnue ; `decimals` ≥ 0 impose le nombre de
## décimales.
static func text(name: String, decimals: int = -1) -> String:
	if not has(name):
		return UNKNOWN
	return number(value(name), decimals)


## `8.0` → « 8 », `1.5` → « 1,5 », `1000.0` → « 1 000 » ; `(1.0, 1)` → « 1,0 ».
static func number(amount: float, decimals: int = -1) -> String:
	if decimals >= 0:
		return String.num(amount, decimals).pad_decimals(decimals).replace(".", ",")
	if is_equal_approx(amount, roundf(amount)):
		return Money.digits(int(roundf(amount)))
	return String.num(amount, 2).replace(".", ",")


## Remplace chaque `{rule.nom}` de `template` par la valeur du cœur.
static func format(template: String) -> String:
	if template.find("{rule.") < 0:
		return template
	if _pattern == null:
		_pattern = RegEx.create_from_string("\\{rule\\.([a-z0-9_]+)(?::([0-9]))?\\}")
	var out := ""
	var last := 0
	for found in _pattern.search_all(template):
		var decimals := int(found.get_string(2)) if found.get_string(2) != "" else -1
		out += template.substr(last, found.get_start() - last) + text(found.get_string(1), decimals)
		last = found.get_end()
	return out + template.substr(last)


## Oublie les valeurs lues (données rechargées, tests).
static func reset() -> void:
	_cache = {}
