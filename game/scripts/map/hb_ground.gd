class_name HbGround
extends RefCounted

## Lot HB3 (ADR 0143) : habillage du sol par biome dans `terrain.gdshader`
## (`hb_ground.gdshaderinc`). Pose les tableaux de matières fal.ai (`GroundMaterials`), la carte
## des biomes (`data/map/biomes.png`) et la table du mélange par biome
## (`data/art/ground_biome_mix.json`) ; `has_hb` reste faux s'il manque l'un d'eux (rendu SS).
## Purement visuel.

const MIX_FILE := "art/ground_biome_mix.json"
const BIOMES_FILE := "map/biomes.png"
const TABLE_WIDTH := 17
const TABLE_HEIGHT := 8
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


## Pose les paramètres du matériau ; rend vrai si l'habillage est actif.
static func apply(material: ShaderMaterial, data_dir: String) -> bool:
	material.set_shader_parameter("has_hb", false)
	var arrays := GroundMaterials.load_arrays()
	if arrays.is_empty():
		return false
	var biomes := _load_biomes(data_dir.path_join(BIOMES_FILE))
	var mix_text := FileAccess.get_file_as_string(data_dir.path_join(MIX_FILE))
	var mix: Variant = JSON.parse_string(mix_text) if not mix_text.is_empty() else null
	if biomes == null or not mix is Dictionary:
		push_warning("HbGround: biomes.png ou ground_biome_mix.json absent, habillage désactivé")
		return false
	var table := build_table(mix as Dictionary, arrays["layers"] as Dictionary)
	if table == null:
		return false
	material.set_shader_parameter("hb_albedo", arrays["albedo"])
	material.set_shader_parameter("hb_biomes", ImageTexture.create_from_image(biomes))
	material.set_shader_parameter("hb_table", ImageTexture.create_from_image(table))
	material.set_shader_parameter("has_hb", true)
	return true


static func _load_biomes(path: String) -> Image:
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
	return image


static func _tile_m(id: String) -> float:
	for layer in GroundMaterials.manifest().get("layers", []):
		if str((layer as Dictionary).get("id", "")) == id:
			return float((layer as Dictionary).get("tile_m", 120.0))
	return 120.0
