class_name HbGround
extends RefCounted

## Habillage du sol par biome dans `terrain.gdshader` (ADR 0143)
## (`hb_ground.gdshaderinc`). Pose les tableaux de matières fal.ai (`GroundMaterials`), la carte
## des biomes (`data/map/biomes.png`) et la table du mélange par biome
## (`data/art/ground_biome_mix.json`) ; `has_hb` reste faux s'il manque l'un d'eux (rendu SS).
## Purement visuel.

const MIX_FILE := "art/ground_biome_mix.json"
const BIOMES_FILE := "map/biomes.png"
## Même carte, sous-classes 8-14 repliées sur leur parent (ADR 0242) : pour les lecteurs qui ne
## connaissent que les biomes 1-7 (`_load_biomes`, donc `FieldPlan`).
const BIOMES_BASE_FILE := "map/biomes_base.png"
const AGRI_FILE := "map/agri_landscapes.json"
const AGRI_MASK_FILE := "map/agri_regions.png"
## Lignes 1..7 = biomes, lignes 8..15 = paysages agricoles régionaux ; ADR 0242 : lignes
## 16..22 = biomes régionaux 8..14 (`BiomeParents.table_row`).
const FIRST_LANDSCAPE_ROW := 8
const TABLE_WIDTH := 18
const TABLE_HEIGHT := 24
const CROP_SLOTS := 12
## Valeurs de la table ramenées dans [0, 1] (une texture peut borner ses canaux) : couche / 255,
## poids cumulé × 0,5 (case vide : 1), tuile / 1000 m, taille de parcelle / 5000 m, haie / 2,
## allongement / 10, teinte / 2 ; même échelle dans `hb_ground.gdshaderinc`.
const LAYER_NORM := 255.0
const TILE_NORM := 1000.0
const CELL_NORM := 5000.0
const HEDGE_NORM := 2.0
const STRIP_NORM := 10.0
const TINT_NORM := 2.0

## Bancs seulement : impose la source du parcellaire (« hb » / « tx ») avant un nouvel `apply`.
static var forced_source := ""


## Pose les paramètres du matériau ; rend vrai si l'habillage est actif.
static func apply(material: ShaderMaterial, data_dir: String) -> bool:
	material.set_shader_parameter("has_hb", false)
	var mix: Variant = DataFile.try_parse(data_dir.path_join(MIX_FILE))
	GroundMaterials.source = parcels_source(mix as Dictionary if mix is Dictionary else {})
	var arrays := GroundMaterials.load_arrays()
	if arrays.is_empty() and GroundMaterials.source != GroundMaterials.SOURCE_HB:
		push_warning("HbGround: paquet TX de parcellaire indisponible, retour au parcellaire HB")
		GroundMaterials.source = GroundMaterials.SOURCE_HB
		arrays = GroundMaterials.load_arrays()
	if arrays.is_empty():
		return false
	var biomes := _load_raster(data_dir.path_join(BIOMES_FILE))
	if biomes == null or not mix is Dictionary:
		push_warning("HbGround: biomes.png ou ground_biome_mix.json absent, habillage désactivé")
		return false
	if GroundMaterials.source == GroundMaterials.SOURCE_TX:
		mix = with_tx_overrides(mix as Dictionary)
	var layer_ids: Dictionary = arrays["layers"]
	var agri_doc := _read_json(data_dir.path_join(AGRI_FILE))
	var landscapes := landscape_rows(agri_doc)
	var merged: Dictionary = mix_rows(mix as Dictionary)
	if not landscapes.is_empty():
		var biomes_mix: Dictionary = merged["biomes"]
		var resolved := resolve_landscapes(agri_doc, layer_ids)
		for key in resolved:
			biomes_mix[key] = resolved[key]
	var table := build_table(merged, layer_ids)
	if table == null:
		return false
	var agri_mask := _load_biomes(data_dir.path_join(AGRI_MASK_FILE))
	material.set_shader_parameter("has_agri", agri_mask != null and not landscapes.is_empty())
	if agri_mask != null:
		material.set_shader_parameter("hb_agri", ImageTexture.create_from_image(agri_mask))
		material.set_shader_parameter("hb_agri_texel", float(biomes.get_width()) / float(agri_mask.get_width()))
	material.set_shader_parameter("hb_albedo", arrays["albedo"])
	material.set_shader_parameter("hb_biomes", ImageTexture.create_from_image(biomes))
	material.set_shader_parameter("hb_table", ImageTexture.create_from_image(table))
	apply_view((mix as Dictionary).get("view", {}), material)
	material.set_shader_parameter("has_hb", true)
	return true


## ADR 0253 : réglages de lisibilité des champs peints (bloc `view` du mélange) -> uniforms
## `hb_*` ; une clé absente laisse la valeur par défaut du shader.
const VIEW_UNIFORMS := {
	"fade_start": "hb_fade_start",
	"fade_end": "hb_fade_end",
	"hue_keep": "hb_hue_keep",
	"cell_jitter": "hb_cell_jitter",
	"cell_hue": "hb_cell_hue",
	"patch_gain": "hb_patch_gain",
	"patch_start": "hb_patch_start",
	"patch_end": "hb_patch_end",
	"far_hedge": "hb_far_hedge",
	"open_farm": "hb_open_farm",
	"cell_scale": "hb_cell_scale",
}


static func apply_view(view: Variant, material: ShaderMaterial) -> void:
	if not view is Dictionary:
		return
	for key in VIEW_UNIFORMS:
		if (view as Dictionary).has(key):
			material.set_shader_parameter(VIEW_UNIFORMS[key], float(view[key]))


## TX (ADR 0243) : source du parcellaire demandée par le mélange (`parcels_source`, « hb » par
## défaut) ; toujours « hb » avec `--legacy-textures`.
static func parcels_source(mix: Dictionary) -> String:
	if not TextureQuality.use_tx():
		return GroundMaterials.SOURCE_HB
	if forced_source != "":
		return forced_source
	var forced := CmdArgs.value("--parcels-source", "")  # bancs et planches de comparaison
	return forced if forced != "" else str(mix.get("parcels_source", GroundMaterials.SOURCE_HB))


## TX : le mélange avec les entrées `tx_overrides` (par biome, champs remplacés) posées sur les
## biomes ; ces entrées emploient les matières régionales du paquet TX (oasis, dunes, toundra...).
static func with_tx_overrides(mix: Dictionary) -> Dictionary:
	var merged := mix.duplicate(true)
	var biomes: Dictionary = merged.get("biomes", {})
	var overrides: Dictionary = mix.get("tx_overrides", {})
	for key in overrides:
		if not biomes.has(key):
			continue
		var entry: Dictionary = biomes[key]
		for field in overrides[key]:
			entry[field] = (overrides[key] as Dictionary)[field]
	return merged


static func _read_json(path: String) -> Dictionary:
	return DataFile.try_dict(path)


## Ligne de la table de chaque paysage (ordre du fichier, à partir de `FIRST_LANDSCAPE_ROW`).
static func landscape_rows(doc: Dictionary) -> Dictionary:
	var rows := {}
	var row := FIRST_LANDSCAPE_ROW
	for id in doc.get("landscapes", {}):
		rows[str(id)] = row
		row += 1
	return rows


## Paysages du fichier ME8 au format du mélange par biome (clé = ligne), matières absentes
## de `layers` remplacées par leur repli (`material_fallbacks`) ; les poids d'une même matière
## s'additionnent. Un paysage au-delà de la ligne 15 est ignoré.
static func resolve_landscapes(doc: Dictionary, layers: Dictionary) -> Dictionary:
	var result := {}
	var fallbacks: Dictionary = doc.get("material_fallbacks", {})
	var rows := landscape_rows(doc)
	var landscapes: Dictionary = doc.get("landscapes", {})
	for id in rows:
		var row := int(rows[id])
		if row >= BiomeParents.REGIONAL_ROW_BASE:
			push_warning("HbGround: trop de paysages agricoles, %s ignoré" % id)
			continue
		var entry: Dictionary = (landscapes[id] as Dictionary).duplicate(true)
		var crops := {}
		for crop in entry["crops"]:
			var material := str(crop)
			if not layers.has(material):
				material = str(fallbacks.get(material, material))
			crops[material] = float(crops.get(material, 0.0)) + float(entry["crops"][crop])
		entry["crops"] = crops
		var wild: Array = []
		for material in entry["wild"]:
			wild.append(str(fallbacks.get(material, material)) if not layers.has(material) else material)
		entry["wild"] = wild
		result[str(row)] = entry
	return result


## Clés du mélange (indices de biomes) -> lignes de la table : 1..7 inchangées, 8..14 en 16..22.
static func mix_rows(mix: Dictionary) -> Dictionary:
	var merged := mix.duplicate(true)
	var rows := {}
	var biomes: Dictionary = mix.get("biomes", {})
	for key in biomes:
		rows[str(BiomeParents.table_row(int(key)))] = biomes[key]
	merged["biomes"] = rows
	return merged


## Carte des biomes lue par les consommateurs qui ne connaissent que 1-7 : pour `biomes.png`,
## la version repliée sur les parents si elle existe (ADR 0242) ; le masque ME8 est lu tel quel.
static func _load_biomes(path: String) -> Image:
	if path.ends_with(BIOMES_FILE):
		var base_path := path.trim_suffix(BIOMES_FILE) + BIOMES_BASE_FILE
		var base := _load_raster(base_path)
		if base != null:
			return base
	return _load_raster(path)


static func _load_raster(path: String) -> Image:
	if not FileAccess.file_exists(path):
		return null
	var image := Image.load_from_file(path)
	if image == null or image.is_empty():
		return null
	if image.get_format() != Image.FORMAT_L8:
		image.convert(Image.FORMAT_L8)
	return image


## Table 16 × 8 (voir l'en-tête de `hb_ground.gdshaderinc`) ; null si un identifiant est inconnu.
static func build_table(mix: Dictionary, layers: Dictionary) -> Image:
	var image := Image.create(TABLE_WIDTH, TABLE_HEIGHT, false, Image.FORMAT_RGBAF)
	for x in TABLE_WIDTH:
		for y in TABLE_HEIGHT:
			image.set_pixel(x, y, Color(0.0, 1.0, 120.0 / TILE_NORM, 0.0))
	var biomes: Dictionary = mix.get("biomes", {})
	for key in biomes:
		var row := int(key)
		var entry: Dictionary = biomes[key]
		var crops: Dictionary = entry.get("crops", {})
		var total := 0.0
		for id in crops:
			total += float(crops[id])
		var cumulative := 0.0
		var slot := 0
		for id in crops:
			if slot >= CROP_SLOTS or not layers.has(id):
				if not layers.has(id):
					push_warning("HbGround: matière inconnue %s" % id)
					return null
				continue
			cumulative += float(crops[id]) / maxf(total, 1e-6)
			image.set_pixel(slot, row, Color(float(layers[id]) / LAYER_NORM, minf(cumulative, 1.0) * 0.5, _tile_m(id) / TILE_NORM, 0.0))
			slot += 1
		var wild: Array = entry.get("wild", [])
		for id in [entry.get("canopy"), entry.get("rock")] + wild:
			if not layers.has(id):
				push_warning("HbGround: matière inconnue %s" % id)
				return null
		image.set_pixel(12, row, Color(float(layers[entry["canopy"]]) / LAYER_NORM, 0.0, _tile_m(entry["canopy"]) / TILE_NORM, float(entry["cell_m"]) / CELL_NORM))
		image.set_pixel(13, row, Color(float(layers[entry["rock"]]) / LAYER_NORM, 0.0, _tile_m(entry["rock"]) / TILE_NORM, float(entry["hedge"]) / HEDGE_NORM))
		image.set_pixel(14, row, Color(float(layers[wild[0]]) / LAYER_NORM, 0.0, _tile_m(wild[0]) / TILE_NORM, float(entry["strip"]) / STRIP_NORM))
		image.set_pixel(15, row, Color(float(layers[wild[1]]) / LAYER_NORM, 0.0, _tile_m(wild[1]) / TILE_NORM, float(entry["farm"])))
		var tint: Array = entry.get("tint", [1.0, 1.0, 1.0])
		image.set_pixel(16, row, Color(float(tint[0]) / TINT_NORM, float(tint[1]) / TINT_NORM, float(tint[2]) / TINT_NORM, 1.0))
		image.set_pixel(17, row, Color(float(entry.get("open_to_farm", 0.0)), 0.0, 0.0, 0.0))
	# ADR 0242 : un biome régional sans entrée prend la ligne de son parent.
	for biome in range(BiomeParents.FIRST_REGIONAL, BiomeParents.COUNT):
		var row := BiomeParents.table_row(biome)
		if biomes.has(str(row)):
			continue
		var parent_row := BiomeParents.table_row(BiomeParents.parent_of(biome))
		for x in TABLE_WIDTH:
			image.set_pixel(x, row, image.get_pixel(x, parent_row))
	return image


static func _tile_m(id: String) -> float:
	for layer in GroundMaterials.manifest().get("layers", []):
		if str((layer as Dictionary).get("id", "")) == id:
			return float((layer as Dictionary).get("tile_m", 120.0))
	return 120.0
