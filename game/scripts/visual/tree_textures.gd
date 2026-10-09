class_name TreeTextures
extends RefCounted

## TX T3 (ADR 0241) : écorces raccordables et planches de feuilles générées par essence.
##
## Chaque essence de `data/art/tree_species.json` peut porter `textures: {bark, leaves}` (ids des
## couches des paquets `data/art/tx_bark_pack.json` et `tx_leaves_pack.json`, produits par
## `cent-ans textures pack foliage_bark`). Les modèles 3D ne changent pas : les arbres procéduraux
## de bataille (`BattleTrees`) lisent ces tableaux de couches à la place des textures d'avant ;
## essence sans entrée, paquet absent ou `--legacy-textures` : `bark_layer`/`leaves_layer` valent -1
## et le rendu reste celui d'avant. Micro-détail d'écorce (`micro_bark`) fondu de près.
## Purement visuel.

const SPECIES_FILE := "art/tree_species.json"
const BARK_PACK := "art/tx_bark_pack.json"
const LEAVES_PACK := "art/tx_leaves_pack.json"
const MICRO_BARK := "res://assets/textures/vegetation/tx_micro_bark.jpg"
## Micro-détail d'écorce : force, distance pleine (m), distance nulle (m).
const MICRO_FADE := Vector3(0.7, 3.0, 14.0)

static var _loaded := false
static var _species: Dictionary = {}  # id d'essence → {"bark": id, "leaves": id}
static var _bark: Dictionary = {}  # {"layers": {id → couche}, "albedo": Texture, "normal": Texture}
static var _leaves: Dictionary = {}  # {"layers": {id → couche}, "albedo": Texture}


static func clear_cache() -> void:
	_loaded = false
	_species = {}
	_bark = {}
	_leaves = {}


static func _pack(file: String) -> Dictionary:
	var path := file
	var hi := file.get_basename() + "_%d.json" % TextureQuality.HI_SIZE
	if TextureQuality.is_high() and DataFile.exists(hi):
		path = hi
	var parsed: Variant = DataFile.read_json(path) if DataFile.exists(path) else null
	if not parsed is Dictionary:
		return {}
	var layers: Dictionary = {}
	for layer_v in (parsed as Dictionary).get("layers", []):
		layers[str((layer_v as Dictionary)["id"])] = int((layer_v as Dictionary)["layer"])
	var out := {"layers": layers}
	for key in ["albedo", "normal"]:
		if (parsed as Dictionary).has(key):
			var texture := load(TextureQuality.texture_path(str(parsed[key]))) as Texture
			if texture == null:
				return {}
			out[key] = texture
	return out


static func _ensure() -> void:
	if _loaded:
		return
	_loaded = true
	if not TextureQuality.use_tx():
		return
	var parsed: Variant = DataFile.read_json(SPECIES_FILE) if DataFile.exists(SPECIES_FILE) else null
	if parsed is Dictionary:
		for entry_v in (parsed as Dictionary).get("species", []):
			var entry := entry_v as Dictionary
			if entry.has("textures"):
				_species[str(entry["id"])] = entry["textures"]
	_bark = _pack(BARK_PACK)
	_leaves = _pack(LEAVES_PACK)


## Couche d'écorce de l'essence `species` dans `bark_albedo()` / `bark_normal()`, -1 sans texture.
static func bark_layer(species: String) -> int:
	_ensure()
	var textures: Dictionary = _species.get(species, {})
	return int((_bark.get("layers", {}) as Dictionary).get(str(textures.get("bark", "")), -1))


## Couche de la planche de feuilles de l'essence dans `leaves_albedo()`, -1 sans texture.
static func leaves_layer(species: String) -> int:
	_ensure()
	var textures: Dictionary = _species.get(species, {})
	return int((_leaves.get("layers", {}) as Dictionary).get(str(textures.get("leaves", "")), -1))


static func bark_albedo() -> Texture:
	_ensure()
	return _bark.get("albedo")


static func bark_normal() -> Texture:
	_ensure()
	return _bark.get("normal")


static func leaves_albedo() -> Texture:
	_ensure()
	return _leaves.get("albedo")


## Pose l'écorce régionale de `species` sur un matériau `battle_tree_bark` ; faux sans texture.
static func apply_bark(material: ShaderMaterial, species: String) -> bool:
	var layer := bark_layer(species)
	if layer < 0 or bark_albedo() == null:
		return false
	material.set_shader_parameter("bark_array", bark_albedo())
	material.set_shader_parameter("bark_normal_array", bark_normal())
	material.set_shader_parameter("bark_layer", layer)
	if ResourceLoader.exists(MICRO_BARK):
		material.set_shader_parameter("micro_bark", load(MICRO_BARK))
		material.set_shader_parameter("micro_fade", MICRO_FADE)
	return true


## Pose la planche de feuilles de `species` sur un matériau `battle_tree_foliage` ; faux sans texture.
static func apply_leaves(material: ShaderMaterial, species: String) -> bool:
	var layer := leaves_layer(species)
	if layer < 0 or leaves_albedo() == null:
		return false
	material.set_shader_parameter("leaves_array", leaves_albedo())
	material.set_shader_parameter("leaves_layer", layer)
	return true
