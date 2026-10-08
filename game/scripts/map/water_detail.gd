class_name WaterDetail
extends RefCounted

## Lot RC5 (ADR 0141) : branche les textures de détail d'eau générées (mer, océan, fleuve,
## rivière) sur un `ShaderMaterial` qui inclut `res://shaders/water_detail.gdshaderinc`.
## Réglages (force, échelle, écoulement) dans `data/fx/water_detail.json`, jamais codés en dur.
## Sans texture (`<id>_normal.png` absente) ou sans données : `water_detail_strength` reste à 0
## et le shader garde son rendu procédural.

const SPEC_FILE := "fx/water_detail.json"
const TEXTURE_DIR := "res://assets/textures/water/"

static var _spec: Dictionary = {}
static var _spec_loaded: bool = false


## Données RC5 (dossier de données du jeu, puis `data/` du dépôt) ; {} si introuvables.
static func spec() -> Dictionary:
	if _spec_loaded:
		return _spec
	_spec_loaded = true
	if DataFile.exists(SPEC_FILE):
		var parsed: Variant = DataFile.read_json(SPEC_FILE)
		if parsed is Dictionary and (parsed as Dictionary).get("materials") is Dictionary:
			_spec = (parsed as Dictionary).get("materials")
			return _spec
	return _spec


## Applique la matière `id` à `material` (uniformes `<prefix>_normal`, `_albedo`, `_scale`,
## `_strength` ; `ocean_detail` pour le large de la mer) ; renvoie true si le détail est actif.
## Renvoie false (force laissée à 0, repli procédural) si la texture ou les données manquent.
static func apply(material: ShaderMaterial, id: String, prefix: String = "water_detail") -> bool:
	if material == null:
		return false
	material.set_shader_parameter(prefix + "_strength", 0.0)
	var entry: Variant = spec().get(id)
	var normal_path := TEXTURE_DIR + id + "_normal.png"
	if not entry is Dictionary or not ResourceLoader.exists(normal_path):
		return false
	var normal := load(normal_path) as Texture2D
	if normal == null:
		return false
	material.set_shader_parameter(prefix + "_normal", normal)
	var albedo_path := TEXTURE_DIR + id + "_albedo.png"
	if ResourceLoader.exists(albedo_path):
		material.set_shader_parameter(prefix + "_albedo", load(albedo_path) as Texture2D)
	material.set_shader_parameter(prefix + "_scale", float(entry.get("scale", 1.0)))
	material.set_shader_parameter(prefix + "_strength", float(entry.get("strength", 0.0)))
	return true


## Vitesse d'écoulement de la normale de détail (tuiles par seconde) de la matière `id`, 0 sinon.
static func flow_speed(id: String) -> float:
	var entry: Variant = spec().get(id)
	return float((entry as Dictionary).get("flow_speed", 0.0)) if entry is Dictionary else 0.0
