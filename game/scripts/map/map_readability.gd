class_name MapReadability
extends RefCounted

## Lot TB2 (plan `docs/design/2026-10-02-campagne-tob.md` § 3) : réglages de lisibilité de la carte
## de campagne, lus dans `data/ui/campaign_map.json` (schéma `campaign_map_ui.schema.json`) :
## voile du brouillard de guerre (`fog_of_war`), signes de ville (`signs`), étiquettes (`labels`),
## noms de région (`region_labels`), nuées (`clouds`). Les valeurs de repli sont celles du fichier.
## Purement visuel : aucune règle de jeu.

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const FILE := "ui/campaign_map.json"

static var _catalog: Dictionary = {}
static var _loaded: bool = false


## Bloc `name` du fichier (dictionnaire vide si absent : chaque appelant garde ses replis).
static func section(name: String) -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := MAP_PATHS_SCRIPT.default_data_dir().path_join(FILE)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(FILE)
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
		if parsed is Dictionary:
			_catalog = parsed
		else:
			push_warning("MapReadability: %s missing or invalid" % path)
	var block: Variant = _catalog.get(name, {})
	return block if block is Dictionary else {}


static func number(name: String, key: String, fallback: float) -> float:
	return float(section(name).get(key, fallback))


static func clear_cache() -> void:
	_catalog = {}
	_loaded = false


## Couleur « #rrggbb » d'un bloc (repli si absente).
static func color(name: String, key: String, fallback: Color) -> Color:
	var value: Variant = section(name).get(key, null)
	return Color.html(str(value)) if value is String and Color.html_is_valid(str(value)) else fallback


## Voile de parchemin du brouillard de guerre : uniformes `fog_*` du matériau du terrain.
## Renvoie les valeurs posées (tests).
static func apply_fog(material: ShaderMaterial) -> Dictionary:
	var fog := section("fog_of_war")
	var applied := {}
	for key: String in ["desaturation", "dim", "veil_strength", "mist_max", "mist_min", "mist_glow"]:
		if fog.has(key):
			applied["fog_" + key] = float(fog[key])
	for key: String in ["veil_color", "mist_color"]:
		if fog.has(key) and Color.html_is_valid(str(fog[key])):
			applied["fog_" + key] = Color.html(str(fog[key]))
	if material != null:
		for param: String in applied:
			material.set_shader_parameter(param, applied[param])
	return applied
