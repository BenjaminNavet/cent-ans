class_name ReliefLandcover
extends RefCounted

## Lot R1 (ADR 0019) : rasters de relief fin et de zones humides du shader de terrain
## (`game/shaders/relief_landcover.gdshaderinc`), produits hors ligne par
## `uv run --project tools cent-ans geo relief-shade` et `cent-ans geo landcover`.
## Rendu seulement ; fichiers optionnels (le shader garde son rendu V2 sans eux).
##
## - `relief_shade.png` (LA8, 8192²) : L détail d'altitude, A occlusion/courbure (mipmaps).
## - `wetlands.png` (RGB8, 4096²) : R marais, G étangs, B prés humides.
## - `forest_kind.png` (L8, 2048², part de résineux) n'est pas lu ici : il est destiné au rendu
##   des forêts (lot V4), comme `splat.png`.

const SHADE_FILE := "relief_shade.png"
const WETLANDS_FILE := "wetlands.png"


## Pose les textures et réglages sur le matériau du terrain ; durée consignée dans
## `map_data.timings["relief_landcover_ms"]`.
static func apply(material: ShaderMaterial, map_data: MapData) -> void:
	var t0 := Time.get_ticks_msec()
	var shade := _load(map_data, SHADE_FILE, Image.FORMAT_LA8)
	if shade != null:
		shade.generate_mipmaps()
		material.set_shader_parameter("relief_shade", ImageTexture.create_from_image(shade))
		material.set_shader_parameter("rl_detail_scale_m", _detail_scale(map_data))
	material.set_shader_parameter("has_relief_shade", shade != null)
	var wet := _load(map_data, WETLANDS_FILE, Image.FORMAT_RGB8)
	if wet != null:
		material.set_shader_parameter("wetlands", ImageTexture.create_from_image(wet))
	material.set_shader_parameter("has_wetlands", wet != null)
	map_data.timings["relief_landcover_ms"] = Time.get_ticks_msec() - t0


static func _load(map_data: MapData, file_name: String, format: int) -> Image:
	var path := map_data.map_dir.path_join(file_name)
	if not FileAccess.file_exists(path):
		return null
	var image := Image.load_from_file(path)
	if image == null:
		push_warning("ReliefLandcover: %s unreadable" % path)
		return null
	if image.get_format() != format:
		image.convert(format)
	return image


static func _detail_scale(map_data: MapData) -> float:
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(map_data.map_dir.path_join("map.json")))
	if meta is Dictionary and (meta as Dictionary).get("relief_shade") is Dictionary:
		return float(meta["relief_shade"].get("detail_scale_m", 1.5))
	return 1.5
