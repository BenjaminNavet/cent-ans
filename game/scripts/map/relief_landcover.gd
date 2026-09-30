class_name ReliefLandcover
extends RefCounted

## Lot R1 (ADR 0019) : rasters de relief fin et de zones humides du shader de terrain
## (`game/shaders/relief_landcover.gdshaderinc`), produits hors ligne par
## `uv run --project tools cent-ans geo relief-shade` et `cent-ans geo landcover`.
## Rendu seulement ; fichiers optionnels (le shader garde son rendu V2 sans eux).
##
## - `relief_shade.png` (LA8, 2 × la carte) : L détail d'altitude, A occlusion/courbure (mipmaps).
##   Depuis OM2 (ADR 0115), écrit en bandes horizontales `relief_shade_<i>.png`
##   (`map.json.relief_shade.bands`) empilées ici en une seule image.
##   OMR-R2 : copie GPU `map.json.relief_shade.bc5` (BC5 = RGTC RG, mipmaps précalculés, parts
##   zlib `relief_shade_bc5_<i>.bin`, écrite par `cent-ans geo gpu-textures`, ADR 0118) lue en priorité :
##   235 Mo au lieu de 470, ni décodage PNG ni calcul de mipmaps. Canaux R = L, G = A
##   (`relief_shade_rg` dans le shader).
## - `wetlands.png` (RGB8, 4096²) : R marais, G étangs, B prés humides. OMR-R2 : copie BC1
##   `map.json.wetlands_gpu` lue en priorité (22 Mo au lieu de 126).
## - `forest_kind.png` (L8, 2048², part de résineux) n'est pas lu ici : il est destiné au rendu
##   des forêts (lot V4), comme `splat.png`.
##
## Le décodage (≈ 1 s pour le PNG 8192² et ses mipmaps) se fait dans `WorkerThreadPool` : le
## terrain s'affiche aussitôt, les textures sont posées sur le matériau à la frame suivante la
## fin du chargement (`pending()` faux ensuite).

const SHADE_FILE := "relief_shade.png"
const WETLANDS_FILE := "wetlands.png"

## Chargements en cours (garde les chargeurs en vie jusqu'à leur fin).
static var _loaders: Array = []

var _material: ShaderMaterial
var _map_data: MapData
var _map_dir: String
var _detail_scale: float = 1.5
var _shade: Image
var _wet: Image
var _colormap: Image
var _t0: int = 0
var _task: int = -1


## Lance le chargement des textures du matériau du terrain ; durée consignée dans
## `map_data.timings["relief_landcover_ms"]` à la fin.
static func apply(material: ShaderMaterial, map_data: MapData) -> void:
	material.set_shader_parameter("has_relief_shade", false)
	material.set_shader_parameter("has_wetlands", false)
	material.set_shader_parameter("has_colormap", false)
	var loader := ReliefLandcover.new()
	loader._material = material
	loader._map_data = map_data
	loader._map_dir = map_data.map_dir
	loader._detail_scale = _detail_scale_of(map_data.map_dir)
	loader._t0 = Time.get_ticks_msec()
	_loaders.append(loader)
	loader._task = WorkerThreadPool.add_task(loader._load, false, "relief landcover")


## Vrai tant qu'un chargement n'a pas posé ses textures (captures, mesures).
static func pending() -> bool:
	return not _loaders.is_empty()


func _load() -> void:
	_shade = load_bc5(_map_dir)
	if _shade == null:
		_shade = _load_bands(_map_dir, Image.FORMAT_LA8)
		if _shade == null:
			_shade = _load_image(_map_dir, SHADE_FILE, Image.FORMAT_LA8)
		if _shade != null:
			_shade.generate_mipmaps()
	_wet = load_wetlands_gpu(_map_dir)
	if _wet == null:
		_wet = _load_image(_map_dir, WETLANDS_FILE, Image.FORMAT_RGB8)
	_colormap = load_colormap(_map_dir)
	_finish.call_deferred()


func _finish() -> void:
	if _task >= 0:
		WorkerThreadPool.wait_for_task_completion(_task)
	if _shade != null:
		_material.set_shader_parameter("relief_shade", ImageTexture.create_from_image(_shade))
		_material.set_shader_parameter("rl_detail_scale_m", _detail_scale)
		_material.set_shader_parameter("relief_shade_rg", _shade.get_format() != Image.FORMAT_LA8)
	_material.set_shader_parameter("has_relief_shade", _shade != null)
	if _wet != null:
		_material.set_shader_parameter("wetlands", ImageTexture.create_from_image(_wet))
	_material.set_shader_parameter("has_wetlands", _wet != null)
	if _colormap != null:
		_material.set_shader_parameter("colormap", ImageTexture.create_from_image(_colormap))
	_material.set_shader_parameter("has_colormap", _colormap != null)
	_map_data.timings["relief_landcover_ms"] = Time.get_ticks_msec() - _t0
	_shade = null
	_wet = null
	_colormap = null
	_loaders.erase(self)


static func _load_image(map_dir: String, file_name: String, format: int) -> Image:
	var path := map_dir.path_join(file_name)
	if not FileAccess.file_exists(path):
		return null
	var image := Image.load_from_file(path)
	if image == null:
		push_warning("ReliefLandcover: %s unreadable" % path)
		return null
	if image.get_format() != format:
		image.convert(format)
	return image


## Formats des copies GPU (`tools/cent_ans_tools/geo/block_compress.py`).
const GPU_FORMATS := {"rgtc_rg": Image.FORMAT_RGTC_RG, "dxt1": Image.FORMAT_DXT1}


## OMR-R2 : image BC5 (RGTC RG) avec mipmaps de `map.json.relief_shade.bc5` ; null sans copie GPU
## ou si une part manque ou ne correspond pas (repli sur les bandes PNG).
static func load_bc5(map_dir: String) -> Image:
	var relief: Variant = _map_meta(map_dir).get("relief_shade")
	return load_gpu_copy(map_dir, relief.get("bc5") if relief is Dictionary else null)


## OMR-R2 : zones humides en BC1 (`map.json.wetlands_gpu`) ; null sans copie GPU (repli PNG).
static func load_wetlands_gpu(map_dir: String) -> Image:
	return load_gpu_copy(map_dir, _map_meta(map_dir).get("wetlands_gpu"))


## SS2 (ADR 0141) : carte de couleur du sol en BC1 avec mipmaps (`map.json.colormap.bc1`) ; null
## sans copie (ancien rendu procédural).
static func load_colormap(map_dir: String) -> Image:
	var colormap: Variant = _map_meta(map_dir).get("colormap")
	return load_gpu_copy(map_dir, colormap.get("bc1") if colormap is Dictionary else null)


## Copie GPU décrite par `entry` (format, taille, mipmaps, parts zlib) : parts lues et décompressées
## une à une (pic mémoire ≈ image + une part) ; null si absente, incomplète ou incohérente.
static func load_gpu_copy(map_dir: String, entry: Variant) -> Image:
	if not entry is Dictionary or not GPU_FORMATS.has(str(entry.get("format", ""))) or str(entry.get("compression", "")) != "deflate":
		return null
	var size: Array = entry.get("size_px", [])
	var part_bytes: Array = entry.get("part_bytes", [])
	var pattern := str(entry.get("pattern", ""))
	if size.size() != 2 or part_bytes.is_empty() or pattern == "":
		return null
	var data := PackedByteArray()
	for i in part_bytes.size():
		var path := map_dir.path_join(pattern.replace("{part}", str(i)))
		if not FileAccess.file_exists(path):
			return null
		var raw := FileAccess.get_file_as_bytes(path).decompress(int(part_bytes[i]), FileAccess.COMPRESSION_DEFLATE)
		if raw.size() != int(part_bytes[i]):
			push_warning("ReliefLandcover: %s unreadable" % path)
			return null
		data.append_array(raw)
	var format: int = GPU_FORMATS[str(entry["format"])]
	var image := Image.create_from_data(int(size[0]), int(size[1]), bool(entry.get("mipmaps", false)), format, data)
	if image.is_empty():
		push_warning("ReliefLandcover: GPU copy %s %s does not match %d bytes" % [entry["format"], size, data.size()])
		return null
	return image


static func _map_meta(map_dir: String) -> Dictionary:
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(map_dir.path_join("map.json")))
	return meta if meta is Dictionary else {}


## Bandes horizontales de `map.json.relief_shade.bands` empilées de haut en bas ; null sans bandes.
static func _load_bands(map_dir: String, format: int) -> Image:
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(map_dir.path_join("map.json")))
	if not (meta is Dictionary and (meta as Dictionary).get("relief_shade") is Dictionary):
		return null
	var bands: Variant = meta["relief_shade"].get("bands")
	if not bands is Dictionary:
		return null
	var pattern := String(bands.get("pattern", ""))
	var images: Array[Image] = []
	var height := 0
	for i in int(bands.get("count", 0)):
		var band := _load_image(map_dir, pattern.replace("{band}", str(i)), format)
		if band == null:
			return null
		images.append(band)
		height += band.get_height()
	if images.is_empty():
		return null
	var full := Image.create_empty(images[0].get_width(), height, false, format)
	var y := 0
	for band in images:
		full.blit_rect(band, Rect2i(Vector2i.ZERO, band.get_size()), Vector2i(0, y))
		y += band.get_height()
	return full


static func _detail_scale_of(map_dir: String) -> float:
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(map_dir.path_join("map.json")))
	if meta is Dictionary and (meta as Dictionary).get("relief_shade") is Dictionary:
		return float(meta["relief_shade"].get("detail_scale_m", 1.5))
	return 1.5
