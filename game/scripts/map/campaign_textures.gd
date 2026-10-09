class_name CampaignTextures
extends RefCounted

## Lot GA4 (ADR 0105) : textures 2k du terrain de campagne, macro-variation à l'échelle de la
## carte et eau (normales animées, couleur de profondeur). Identité des couches et réglages dans
## `data/fx/campaign_terrain_textures.json` (schéma `fx_campaign_terrain_textures.schema.json`),
## jamais codés en dur ici ; seul l'ordre des couches est un contrat du shader (index fixes de
## `terrain.gdshader`). Si les tableaux manquent,
## `TerrainBuilder` retombe sur l'ancien chemin (JPEG 1k par couche).

const SPEC_FILE := "fx/campaign_terrain_textures.json"
## Contrat d'index de `terrain.gdshader` (sample_layer(0..6)).
const SHADER_LAYER_ORDER: Array[String] = ["grass", "farmland", "forest", "rock", "heath", "snow", "sand"]
const TEXTURE_DIR := "res://assets/textures/terrain/"
const ALBEDO_ARRAY_PATH := TEXTURE_DIR + "terrain_albedo_array.jpg"
const NORMAL_ARRAY_PATH := TEXTURE_DIR + "terrain_normal_array.jpg"
const WATER_NORMAL_PATH := TEXTURE_DIR + "water_normal.png"

static var _lookup := JsonLookup.new(SPEC_FILE, {}, "", "layers")


## Données GA4 (dossier de données du jeu, puis `data/` du dépôt) ; {} si introuvables.
static func spec() -> Dictionary:
	return _lookup.data()


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
	var albedo := load(ALBEDO_ARRAY_PATH) as TextureLayered
	var normal := load(NORMAL_ARRAY_PATH) as TextureLayered
	if albedo == null or normal == null or albedo.get_layers() != SHADER_LAYER_ORDER.size() or normal.get_layers() != SHADER_LAYER_ORDER.size():
		push_warning("CampaignTextures: tableaux GA4 invalides, repli sur les couches 1k")
		return {}
	var means := PackedVector3Array()
	for layer in data["layers"]:
		var m: Array = (layer as Dictionary).get("mean_linear", [0.1, 0.1, 0.1])
		means.append(Vector3(float(m[0]), float(m[1]), float(m[2])))
	return {"albedo": albedo, "normal": normal, "means": means}


## Taille fixe du tableau `layer_mean` du shader (couches d'origine : 7 ; fond régional : 45).
const MEAN_SLOTS := 48
## TX (ADR 0239) : table `bg_layers` = 15 biomes (0 = mer, repli biome 1) x 7 rôles.
const BIOME_COUNT := 15


## TX : bloc `regional` des données (fond par biome) ; {} sans lui.
static func regional_spec() -> Dictionary:
	return spec().get("regional", {})


## Manifeste d'un paquet (chemin relatif à `data/`), variante 2k `_2048` si « Qualité des
## textures » est haute et qu'elle existe (`cent-ans textures pack <famille> --size 2048`).
static func pack_manifest(rel_path: String) -> Dictionary:
	var path := rel_path
	if TextureQuality.is_high():
		var hi := rel_path.get_basename() + "_%d.%s" % [TextureQuality.HI_SIZE, rel_path.get_extension()]
		if DataFile.exists(hi):
			path = hi
	var doc: Variant = DataFile.read_json(path)
	return doc as Dictionary if doc is Dictionary else {}


## Tableaux d'un paquet depuis son manifeste : {"albedo", "normal", "manifest"} ou {}.
static func _load_pack(manifest: Dictionary) -> Dictionary:
	if manifest.is_empty():
		return {}
	var albedo_path := str(manifest.get("albedo", ""))
	var normal_path := str(manifest.get("normal", ""))
	if not ResourceLoader.exists(albedo_path) or not ResourceLoader.exists(normal_path):
		push_warning("CampaignTextures: tableaux absents (%s)" % albedo_path)
		return {}
	var albedo := load(albedo_path) as TextureLayered
	var normal := load(normal_path) as TextureLayered
	var count: int = (manifest["layers"] as Array).size()
	if albedo == null or normal == null or albedo.get_layers() != count or normal.get_layers() != count:
		push_warning("CampaignTextures: tableaux invalides (couches != %d)" % count)
		return {}
	return {"albedo": albedo, "normal": normal, "manifest": manifest}


## Couche du paquet pour (biome, rôle du paquet) en remontant les parents ; -1 si aucune.
static func _find_layer(layers: Array, biome: int, pack_role: String) -> int:
	var current := biome
	for _hop in BIOME_COUNT:
		for entry in layers:
			var layer := entry as Dictionary
			if str(layer.get("role", "")) != pack_role:
				continue
			for listed in layer.get("biomes", []):
				if int(listed) == current:
					return int(layer["layer"])
		var parent := BiomeParents.parent_of(current)
		if parent == current:
			break
		current = parent
	return -1


## Table `bg_layers` (BIOME_COUNT x 7, indice biome * 7 + rôle du shader) : couche du paquet
## par biome et par rôle, depuis le bloc `regional` (rôles, surcharges par biome) ; repli sur les
## parents, puis sur la couche 0. Testable sans fichier.
static func build_layer_table(layers: Array, regional: Dictionary) -> PackedInt32Array:
	var role_count := SHADER_LAYER_ORDER.size()
	var table := PackedInt32Array()
	table.resize(BIOME_COUNT * role_count)
	table.fill(0)
	var roles: Dictionary = regional.get("roles", {})
	var overrides: Dictionary = regional.get("overrides", {})
	for biome in range(1, BIOME_COUNT):
		var local: Dictionary = overrides.get(str(biome), {})
		for role_index in role_count:
			var role: String = SHADER_LAYER_ORDER[role_index]
			var layer := _find_layer(layers, biome, str(local.get(role, roles.get(role, ""))))
			table[biome * role_count + role_index] = maxi(layer, 0)
	for role_index in role_count:
		table[role_index] = table[role_count + role_index]
	return table


## Moyennes linéaires des couches, complétées à MEAN_SLOTS entrées.
static func _padded_means(layers: Array) -> PackedVector3Array:
	var means := PackedVector3Array()
	means.resize(MEAN_SLOTS)
	means.fill(Vector3(0.1, 0.1, 0.1))
	for entry in layers:
		var layer := entry as Dictionary
		var m: Array = layer.get("mean_linear", [0.1, 0.1, 0.1])
		means[int(layer["layer"])] = Vector3(float(m[0]), float(m[1]), float(m[2]))
	return means


## Teinte de chaque couche relative à la moyenne des couches de même rôle dans le paquet (rapport
## des moyennes, ramené à la luminance 1) : un sol régional colore le fond sans changer la
## couleur moyenne de l'ensemble des biomes.
static func layer_hues(layers: Array) -> PackedVector3Array:
	var hues := PackedVector3Array()
	hues.resize(MEAN_SLOTS)
	hues.fill(Vector3.ONE)
	var sums := {}
	var counts := {}
	for entry in layers:
		var layer := entry as Dictionary
		var role := str(layer.get("role", ""))
		var m: Array = layer.get("mean_linear", [0.1, 0.1, 0.1])
		sums[role] = (sums.get(role, Vector3.ZERO) as Vector3) + Vector3(float(m[0]), float(m[1]), float(m[2]))
		counts[role] = int(counts.get(role, 0)) + 1
	for entry in layers:
		var layer := entry as Dictionary
		var role := str(layer.get("role", ""))
		var m: Array = layer.get("mean_linear", [0.1, 0.1, 0.1])
		var reference: Vector3 = (sums[role] as Vector3) / float(counts[role])
		var ratio := Vector3(float(m[0]) / maxf(reference.x, 0.01), float(m[1]) / maxf(reference.y, 0.01), float(m[2]) / maxf(reference.z, 0.01))
		var luminance := ratio.dot(Vector3(0.2126, 0.7152, 0.0722))
		hues[int(layer["layer"])] = ratio / maxf(luminance, 0.01)
	return hues


static func _load_l8_texture(rel_path: String) -> ImageTexture:
	var path := DataFile.path_of(rel_path)
	if not FileAccess.file_exists(path):
		return null
	var image := Image.load_from_file(path)
	if image == null or image.is_empty():
		return null
	if image.get_format() != Image.FORMAT_L8:
		image.convert(Image.FORMAT_L8)
	return ImageTexture.create_from_image(image)


## TX : fond régional {"albedo", "normal", "means", "layers" (bg_layers), "ab", "dist"} ; {} avec
## `--legacy-textures`, sans le bloc `regional` ou si un fichier manque (repli : couches GA4).
static func load_regional() -> Dictionary:
	var regional := regional_spec()
	if regional.is_empty() or not TextureQuality.use_tx():
		return {}
	var pack := _load_pack(pack_manifest(str(regional["pack"])))
	var ab := _load_l8_texture(str(regional["blend_ab"]))
	var dist := _load_l8_texture(str(regional["blend_dist"]))
	if pack.is_empty() or ab == null or dist == null:
		push_warning("CampaignTextures: fond régional indisponible, repli sur les couches GA4")
		return {}
	var layers: Array = pack["manifest"]["layers"]
	return {
		"albedo": pack["albedo"], "normal": pack["normal"], "means": _padded_means(layers),
		"layers": build_layer_table(layers, regional), "hues": layer_hues(layers), "ab": ab, "dist": dist,
	}


## TX : pose le fond régional sur le matériau du terrain (voir `load_regional`).
static func apply_regional(material: ShaderMaterial, regional: Dictionary, meters_per_px: float) -> void:
	material.set_shader_parameter("bg_on", not regional.is_empty())
	if regional.is_empty():
		return
	var block := regional_spec()
	material.set_shader_parameter("bg_layers", regional["layers"])
	material.set_shader_parameter("layer_hue", regional["hues"])
	material.set_shader_parameter("bg_hue_keep", float(block.get("hue_keep", 0.0)))
	material.set_shader_parameter("bg_ab", regional["ab"])
	material.set_shader_parameter("bg_dist", regional["dist"])
	var texel_km := float(block["blend_texel_px"]) * meters_per_px / 1000.0
	material.set_shader_parameter("bg_blend_texels", float(block["blend_km"]) / maxf(texel_km, 1e-3))
	material.set_shader_parameter("bg_min_w", float(block.get("min_blend_weight", 0.03)))


## TX : grain de sol {"albedo", "layers" (micro_layers), "means"} ; {} avec `--legacy-textures`.
static func load_micro() -> Dictionary:
	var block: Dictionary = spec().get("micro", {})
	if block.is_empty() or not TextureQuality.use_tx():
		return {}
	var manifest := pack_manifest(str(block["pack"]))
	var pack := _load_pack(manifest)
	if pack.is_empty():
		return {}
	var by_id := {}
	var means := PackedFloat32Array()
	means.resize(8)
	means.fill(0.18)
	for entry in manifest["layers"]:
		var layer := entry as Dictionary
		by_id[str(layer["id"])] = int(layer["layer"])
		var m: Array = layer.get("mean_linear", [0.18, 0.18, 0.18])
		means[int(layer["layer"])] = 0.2126 * float(m[0]) + 0.7152 * float(m[1]) + 0.0722 * float(m[2])
	var table := PackedInt32Array()
	for role in SHADER_LAYER_ORDER:
		table.append(int(by_id.get(str((block["roles"] as Dictionary).get(role, "")), 0)))
	return {"albedo": pack["albedo"], "layers": table, "means": means}


## TX : pose le grain de sol ; absent, le shader ne l'applique pas.
static func apply_micro(material: ShaderMaterial, micro: Dictionary) -> void:
	material.set_shader_parameter("micro_on", not micro.is_empty())
	if micro.is_empty():
		return
	var block: Dictionary = spec()["micro"]
	material.set_shader_parameter("micro_albedo", micro["albedo"])
	material.set_shader_parameter("micro_layers", micro["layers"])
	material.set_shader_parameter("micro_mean", micro["means"])
	material.set_shader_parameter("micro_tile_m", float(block["tile_m"]))
	material.set_shader_parameter("micro_strength", float(block["strength"]))
	var fade: Array = block["fade_footprint"]
	material.set_shader_parameter("micro_fade", Vector2(float(fade[0]), float(fade[1])))


static func _vec4(values: Array) -> Vector4:
	return Vector4(float(values[0]), float(values[1]), float(values[2]), float(values[3]))


static func _vec3(values: Array) -> Vector3:
	return Vector3(float(values[0]), float(values[1]), float(values[2]))


## Réglages GA4 du shader terrain : macro-variation, échelle de tuilage, couleur de profondeur de
## la mer peinte (mêmes valeurs que `water.gdshader`).
static func apply_terrain(material: ShaderMaterial) -> void:
	var data := spec()
	var on := not data.is_empty()
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
## couches défilantes et couleur de profondeur à trois paliers.
static func apply_water(material: ShaderMaterial) -> void:
	var data := spec()
	var on := not data.is_empty() and ResourceLoader.exists(WATER_NORMAL_PATH)
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
