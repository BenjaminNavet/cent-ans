class_name CustomBattlePresets
extends RefCounted

## Compositions nommées de la bataille personnalisée : une armée (faction, budget, technologies,
## unités) gardée sous un nom dans `user://custom_battle_presets.json` (dictionnaire nom → armée).
## Aucune règle : la validité de l'armée rechargée reste jugée par le cœur (`validate_custom`).

const MAX_NAME := 40
static var path: String = "user://custom_battle_presets.json"


## Toutes les compositions : nom → {faction, budget, technologies, units} ({} si fichier absent ou illisible).
static func load_all() -> Dictionary:
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	var out := {}
	if parsed is Dictionary:
		for key in parsed:
			var army := sanitize(parsed[key])
			if not army.is_empty() and str(key) != "":
				out[str(key)] = army
	return out


## Armée normalisée ({} si `value` n'en est pas une).
static func sanitize(value: Variant) -> Dictionary:
	if not value is Dictionary or str((value as Dictionary).get("faction", "")) == "":
		return {}
	var army: Dictionary = value
	return {
		"faction": str(army["faction"]),
		"budget": int(army.get("budget", 0)),
		"technologies": Array(army.get("technologies", [])).map(func(t: Variant) -> String: return str(t)),
		"units": Array(army.get("units", [])).map(func(u: Variant) -> String: return str(u)),
	}


## Nom nettoyé (espaces retirés, longueur bornée).
static func clean_name(raw: String) -> String:
	return raw.strip_edges().left(MAX_NAME)


static func names() -> Array:
	var all := load_all().keys()
	all.sort()
	return all


## Enregistre (ou remplace) la composition `preset_name` ; faux si le nom est vide ou l'écriture échoue.
static func save(preset_name: String, army: Dictionary) -> bool:
	var clean := clean_name(preset_name)
	var normalized := sanitize(army)
	if clean == "" or normalized.is_empty():
		return false
	var all := load_all()
	all[clean] = normalized
	return _write(all)


static func remove(preset_name: String) -> bool:
	var all := load_all()
	if not all.erase(preset_name):
		return false
	return _write(all)


static func _write(all: Dictionary) -> bool:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return false
	file.store_string(JSON.stringify(all, "\t"))
	return true
