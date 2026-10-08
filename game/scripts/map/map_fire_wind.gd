class_name MapFireWind
extends RefCounted

## Lot AS5 : paramètres des restes statiques passés en shader (flammes de la carte, vent des
## bannières de maquette, balancement des imposteurs d'arbres de bataille), lus dans
## `data/fx/map_fire_wind.json` (schéma `fx_map_fire_wind.schema.json`). Rendu seulement. Fichier
## absent (fixtures de test) : effets éteints. Éteint aussi par `enabled` et par `--no-as5` (A/B).

const DATA_PATH := "fx/map_fire_wind.json"

static var _data: Dictionary = {}
static var _loaded := false


static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		var parsed: Variant = DataFile.read_json(DATA_PATH) if DataFile.exists(DATA_PATH) else null
		if parsed is Dictionary:
			_data = parsed
	return _data


static func enabled() -> bool:
	return bool(data().get("enabled", false)) and not CmdArgs.has("--no-as5")


## Section `name` (`fire`, `maquette_banner`, `battle_tree_impostor`), {} si éteint ou absente.
static func section(name: String) -> Dictionary:
	if not enabled():
		return {}
	return data().get(name, {})


static func color(value: Variant) -> Color:
	var rgb: Array = value
	return Color(float(rgb[0]), float(rgb[1]), float(rgb[2]))
