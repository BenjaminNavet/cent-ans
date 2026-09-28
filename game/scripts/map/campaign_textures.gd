class_name CampaignTextures
extends RefCounted

## Lot GA4 (ADR 0105) : textures 2k du terrain de campagne, macro-variation à l'échelle de la
## carte et eau (normales animées, couleur de profondeur). Identité des couches et réglages dans
## `data/fx/campaign_terrain_textures.json` (schéma `fx_campaign_terrain_textures.schema.json`),
## jamais codés en dur ici ; seul l'ordre des couches est un contrat du shader (index fixes de
## `terrain.gdshader`). `--no-ga4` restaure l'ancien chemin (JPEG 1k par couche, tableaux RGBA8
## non compressés, houle procédurale) pour la comparaison A/B.

const SPEC_FILE := "fx/campaign_terrain_textures.json"
## Contrat d'index de `terrain.gdshader` (sample_layer(0..6)).
const SHADER_LAYER_ORDER: Array[String] = ["grass", "farmland", "forest", "rock", "heath", "snow", "sand"]
const TEXTURE_DIR := "res://assets/textures/terrain/"
const ALBEDO_ARRAY_PATH := TEXTURE_DIR + "terrain_albedo_array.jpg"
const NORMAL_ARRAY_PATH := TEXTURE_DIR + "terrain_normal_array.jpg"
const WATER_NORMAL_PATH := TEXTURE_DIR + "water_normal.png"

static var _spec: Dictionary = {}
static var _spec_loaded: bool = false


static func enabled() -> bool:
	return not OS.get_cmdline_user_args().has("--no-ga4")


## Données GA4 (dossier de données du jeu, puis `data/` du dépôt) ; {} si introuvables.
static func spec() -> Dictionary:
	if _spec_loaded:
		return _spec
	_spec_loaded = true
	var candidates: Array[String] = []
	var tree := Engine.get_main_loop() as SceneTree
	var paths: Node = tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	if paths != null:
		candidates.append(str(paths.get("data_dir")))
	candidates.append(ProjectSettings.globalize_path("res://").path_join("../data").simplify_path())
	for dir in candidates:
		var path := dir.path_join(SPEC_FILE)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary and (parsed as Dictionary).get("layers") is Array:
				_spec = parsed
				return _spec
	push_warning("CampaignTextures: %s introuvable, textures GA4 désactivées" % SPEC_FILE)
	return _spec


## Identifiants des couches dans l'ordre des données (repli : contrat du shader).
static func layer_ids() -> Array[String]:
	var ids: Array[String] = []
	for layer in spec().get("layers", []):
		ids.append(str((layer as Dictionary).get("id", "")))
	return ids if not ids.is_empty() else SHADER_LAYER_ORDER.duplicate()


## Tableaux importés (albédo 2k, normale + rugosité, compressés en VRAM) et moyennes linéaires
## des albédos (données) : {"albedo", "normal", "means"}, {} si indisponibles (repli 1k).
static func load_arrays() -> Dictionary:
	var data := spec()
	if data.is_empty():
		return {}
	if layer_ids() != SHADER_LAYER_ORDER:
		push_warning("CampaignTextures: ordre des couches %s différent du contrat du shader" % [layer_ids()])
		return {}
	if not ResourceLoader.exists(ALBEDO_ARRAY_PATH) or not ResourceLoader.exists(NORMAL_ARRAY_PATH):
		push_warning("CampaignTextures: tableaux GA4 absents, repli sur les couches 1k")
		return {}
	var albedo := load(ALBEDO_ARRAY_PATH) as Texture2DArray
	var normal := load(NORMAL_ARRAY_PATH) as Texture2DArray
	if albedo == null or normal == null or albedo.get_layers() != SHADER_LAYER_ORDER.size() or normal.get_layers() != SHADER_LAYER_ORDER.size():
		push_warning("CampaignTextures: tableaux GA4 invalides, repli sur les couches 1k")
		return {}
	var means := PackedVector3Array()
	for layer in data["layers"]:
		var m: Array = (layer as Dictionary).get("mean_linear", [0.1, 0.1, 0.1])
		means.append(Vector3(float(m[0]), float(m[1]), float(m[2])))
	return {"albedo": albedo, "normal": normal, "means": means}


static func _vec4(values: Array) -> Vector4:
	return Vector4(float(values[0]), float(values[1]), float(values[2]), float(values[3]))


static func _vec3(values: Array) -> Vector3:
	return Vector3(float(values[0]), float(values[1]), float(values[2]))


## Réglages GA4 du shader terrain : macro-variation, échelle de tuilage, couleur de profondeur de
## la mer peinte (mêmes valeurs que `water.gdshader`). Sans effet sous `--no-ga4`.
static func apply_terrain(material: ShaderMaterial) -> void:
	var data := spec()
	var on := enabled() and not data.is_empty()
	material.set_shader_parameter("ga4_on", 1.0 if on else 0.0)
	if not on:
		return
	var macro: Dictionary = data["macro"]
	material.set_shader_parameter("ga4_macro_periods", _vec4(macro["periods_px"]))
	material.set_shader_parameter("ga4_macro_weights", _vec4(macro["weights"]))
	material.set_shader_parameter("ga4_tint_amp", float(macro["tint_amp"]))
	material.set_shader_parameter("ga4_lum_amp", float(macro["lum_amp"]))
	material.set_shader_parameter("tile_screen_px", float(data["tile_screen_px"]))
	var water: Dictionary = data["water"]
	material.set_shader_parameter("sea_shallow_color", _vec3(water["shallow_color"]))
	material.set_shader_parameter("sea_mid_color", _vec3(water["mid_color"]))
	material.set_shader_parameter("sea_deep_color", _vec3(water["deep_color"]))
	material.set_shader_parameter("sea_mid_depth_m", float(water["mid_depth_m"]))
	material.set_shader_parameter("sea_depth_falloff_m", float(water["depth_falloff_m"]))


## Réglages GA4 de la mer (`water.gdshader`) : normale CC0 (procédurale, cf. README) en deux
## couches défilantes et couleur de profondeur à trois paliers. Sans effet sous `--no-ga4`.
static func apply_water(material: ShaderMaterial) -> void:
	var data := spec()
	var on := enabled() and not data.is_empty() and ResourceLoader.exists(WATER_NORMAL_PATH)
	material.set_shader_parameter("ga4_on", 1.0 if on else 0.0)
	if not on:
		return
	var water: Dictionary = data["water"]
	material.set_shader_parameter("water_normal", load(WATER_NORMAL_PATH))
	material.set_shader_parameter("ga4_scale_a", float(water["scale_a_px"]))
	material.set_shader_parameter("ga4_scale_b", float(water["scale_b_px"]))
	var va: Array = water["velocity_a_px"]
	var vb: Array = water["velocity_b_px"]
	material.set_shader_parameter("ga4_velocity_a", Vector2(float(va[0]), float(va[1])))
	material.set_shader_parameter("ga4_velocity_b", Vector2(float(vb[0]), float(vb[1])))
	material.set_shader_parameter("ga4_normal_strength", float(water["strength"]))
	material.set_shader_parameter("shallow_color", _vec3(water["shallow_color"]))
	material.set_shader_parameter("mid_color", _vec3(water["mid_color"]))
	material.set_shader_parameter("deep_color", _vec3(water["deep_color"]))
	material.set_shader_parameter("mid_depth_m", float(water["mid_depth_m"]))
	material.set_shader_parameter("depth_falloff_m", float(water["depth_falloff_m"]))
