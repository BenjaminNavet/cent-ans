class_name CmdArgs
extends RefCounted

## Arguments utilisateur de la ligne de commande (après `--`), analysés une fois :
## `--flag` -> `""`, `--clé=valeur` -> `"valeur"`. Lecture seule, sans état de jeu.

static var _parsed: Dictionary = {}
static var _raw: PackedStringArray = PackedStringArray()
static var _ready := false


static func _table() -> Dictionary:
	if not _ready:
		_ready = true
		_raw = OS.get_cmdline_user_args()
		for arg in _raw:
			var eq := arg.find("=")
			if eq < 0:
				_parsed[arg] = ""
			else:
				_parsed[arg.substr(0, eq)] = arg.substr(eq + 1)
	return _parsed


## `--flag` (ou `--clé=…`) présent.
static func has(flag: String) -> bool:
	return _table().has(flag)


## Valeur de `--clé=valeur`, `fallback` si absente.
static func value(key: String, fallback: String = "") -> String:
	return str(_table().get(key, fallback))


## Valeur numérique de `--clé=nombre`, `fallback` si absente.
static func number(key: String, fallback: float = 0.0) -> float:
	var table := _table()
	return float(table[key]) if table.has(key) and str(table[key]) != "" else fallback


## Arguments bruts, dans l'ordre (pour les scripts qui parcourent la ligne entière).
static func args() -> PackedStringArray:
	_table()
	return _raw


## Valeurs de `--clé=a,b,c` découpées sur `separator` (sans éléments vides), vide si absente.
static func list(key: String, separator: String = ",") -> PackedStringArray:
	return value(key).split(separator, false)


## Script principal passé au moteur (`--script` / `-s`), vide sans script.
static func main_script() -> String:
	var engine_args := OS.get_cmdline_args()
	var at := engine_args.find("--script")
	if at < 0:
		at = engine_args.find("-s")
	return str(engine_args[at + 1]) if at >= 0 and at + 1 < engine_args.size() else ""


## Vrai pour un script de `res://tests/` (tests, sondes, captures) : réglages par défaut.
static func is_test_run() -> bool:
	return main_script().begins_with("res://tests/")
