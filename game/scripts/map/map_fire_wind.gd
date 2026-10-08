class_name MapFireWind
extends RefCounted

## Lot AS5 : paramètres des restes statiques passés en shader (flammes de la carte, vent des
## bannières de maquette, balancement des imposteurs d'arbres de bataille), lus dans
## `data/fx/map_fire_wind.json` (schéma `fx_map_fire_wind.schema.json`). Rendu seulement. Fichier
## absent (fixtures de test) : effets éteints. Éteint aussi par `enabled` et par `--no-as5` (A/B).

const DATA_PATH := "fx/map_fire_wind.json"

static var _lookup := JsonLookup.new(DATA_PATH)


static func data() -> Dictionary:
	return _lookup.data()


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
