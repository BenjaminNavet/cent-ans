class_name MapData
extends RefCounted

## Chargement de `data/map/` (contrat : docs/design/m1-campaign-map.md).
##
## Coordonnées carte : X = colonne de pixel, Y (carte) = ligne de pixel, origine
## au coin nord-ouest. Dans Godot : X monde = x carte, Z monde = y carte,
## Y monde = altitude (m) × HEIGHT_SCALE.
##
## Les index de province (texture `province_ids.png`, 0 = mer) sont, par
## convention, la position 1-based de la feature dans `provinces.geojson`,
## sauf si la feature porte une propriété explicite `index`.

const HEIGHT_SCALE := 0.006

var map_dir: String = ""
var load_error: String = ""
var timings: Dictionary = {}

var crs: String = ""
var bounds_projected: Array = []
var size: Vector2i = Vector2i.ZERO
var meters_per_px: float = 718.9765625  # map.json (échelle ADR 0082)
var height_min_m: float = -200.0
var height_max_m: float = 4800.0

## Image LA8 (16 bits little-endian, L = octet faible, A = octet fort) livrée par
## `GameDataStore.load_heightmap_u16` (seul décodeur, obligatoire).
var height_image: Image
var height_bytes: PackedByteArray
const height_bpp: int = 2
const height_little_endian: bool = true
const height_decoder: String = "rust"
## Fichier d'altitude chargé : `heightmap_render.png` (Copernicus 90 m moyenné, relief
## local rehaussé pour le rendu, même trait de côte) s'il existe (`map.json.render_heightmap`),
## sinon `heightmap.png` (source des règles : grille de navigation). Surface unique du rendu :
## terrain, armées, villes, fleuves et sélection passent tous par `MapData`.
var height_file: String = "heightmap.png"

var land_mask: Image
## Rasters optionnels du shader de terrain (`cent-ans geo splat`), null si absents :
## splat RGBA8 (R prairie, G cultures, B forêt, A roche/lande), distances signées aux
## frontières (RGB8) et à la côte (L8). Voir `tools/cent_ans_tools/geo/splat.py`.
var splat_image: Image
var border_dist_image: Image
var coast_dist_image: Image
## Lit des fleuves (`cent-ans geo rivers-render`), L8, distance signée à la berge
## ((valeur − 128) / 16 px, négative dans le lit) ; null si absent.
var river_bed_image: Image
## Occlusion de vallée (`cent-ans geo relief-occlusion`), L8 demi-grille, 128 neutre,
## > 128 vallée, < 128 crête ; null si absente (le terrain s'en passe).
var relief_occlusion_image: Image
## Occupation du sol par province (vigne, sécheresse, bocage), calculée à la demande par
## `VegetationFields.landuse` ; null tant qu'elle n'a pas été demandée.
var landuse_image: Image
var province_ids_image: Image
var _preloaded_ids: Image = null
var ids_bytes: PackedByteArray
var ids_bpp: int = 3

## Provinces indexées par index raster (1..n). Chaque entrée :
## {index, id, name, owner, terrain, capital_name, centroid: Vector2,
##  capital_px: Vector2, neighbors: Array, rings: Array[PackedVector2Array]}
var provinces: Dictionary = {}
var province_count: int = 0
## id de province → index raster.
var province_index_by_id: Dictionary = {}

## Rivières : {name, importance: int (0..6, 6 = fleuve majeur), points: PackedVector2Array}
## `importance` = ordre de Strahler si présent, sinon 12 − scalerank (Natural Earth).
var rivers: Array[Dictionary] = []
var coastlines: Array[PackedVector2Array] = []


# --- Relief exagéré (ZG4, ZG8, SZ1) : état dans `ReliefState`, API conservée par délégation --


static func vertical_scale() -> float:
	return ReliefState.vertical_scale()


static func set_vertical_scale(value: float) -> bool:
	return ReliefState.set_vertical_scale(value)


static func near_vertical_scale() -> float:
	return ReliefState.near_vertical_scale()


static func relief_gain_for_scale(scale: float) -> float:
	return ReliefState.relief_gain_for_scale(scale)


static func relief_gain() -> float:
	return ReliefState.relief_gain()


static func relief_max_gain() -> float:
	return ReliefState.relief_max_gain()


static func squash_full_vertical_scale() -> float:
	return ReliefState.squash_full_vertical_scale()


static func relief_squash_for_scale(scale: float) -> float:
	return ReliefState.relief_squash_for_scale(scale)


static func relief_squash() -> float:
	return ReliefState.relief_squash()


static func relief_squash_max_for_scale(scale: float) -> float:
	return ReliefState.relief_squash_max_for_scale(scale)


static func set_relief_floor(grid: Dictionary) -> void:
	ReliefState.set_relief_floor(grid)


static func relief_floor_grid() -> Dictionary:
	return ReliefState.relief_floor_grid()


static func has_relief_floor() -> bool:
	return ReliefState.has_relief_floor()


static func relief_floor_at(x: float, z: float) -> float:
	return ReliefState.relief_floor_at(x, z)


static func relief_fields_at(x: float, z: float) -> Vector3:
	return ReliefState.relief_fields_at(x, z)


static func display_height(h_m: float, x: float, z: float) -> float:
	return ReliefState.display_height(h_m, x, z)


static func display_height_with(h_m: float, x: float, z: float, scale: float, gain: float, squash: float) -> float:
	return ReliefState.display_height_with(h_m, x, z, scale, gain, squash)


static func display_height_fields(h_m: float, fields: Vector3, scale: float, gain: float, squash: float) -> float:
	return ReliefState.display_height_fields(h_m, fields, scale, gain, squash)


static func height_from_display(y: float, x: float, z: float) -> float:
	return ReliefState.height_from_display(y, x, z)


static func height_from_display_with(y: float, x: float, z: float, scale: float, gain: float, squash: float) -> float:
	return ReliefState.height_from_display_with(y, x, z, scale, gain, squash)


## Taille du monde (`size_px` de `<map_dir>/map.json`, unités carte), sans charger la carte ;
## Vector2i.ZERO si illisible (ADR 0115 : aucune taille codée en dur).
static func read_world_size(map_dir: String) -> Vector2i:
	var path := map_dir.path_join("map.json")
	if not FileAccess.file_exists(path):
		return Vector2i.ZERO
	var meta: Variant = DataFile.parse_file(path)
	if not (meta is Dictionary):
		return Vector2i.ZERO
	var size_px: Variant = (meta as Dictionary).get("size_px", [])
	if not (size_px is Array) or (size_px as Array).size() != 2:
		return Vector2i.ZERO
	return Vector2i(int(size_px[0]), int(size_px[1]))


## Taille du monde de la carte des données par défaut (`MapPaths.default_data_dir()`), pour les tests.
static func default_world_size() -> Vector2i:
	var map_paths: GDScript = load("res://scripts/map/map_paths.gd")
	return read_world_size(str(map_paths.call("default_data_dir")).path_join("map"))


static func load_from_dir(dir: String) -> MapData:
	var data := MapData.new()
	data.map_dir = dir
	data._load()
	return data


func _load() -> void:
	var t0 := Time.get_ticks_msec()
	if not _load_map_json():
		return
	var t1 := Time.get_ticks_msec()
	if not _load_heightmap():
		return
	var t2 := Time.get_ticks_msec()
	# PB1 : les six masques de la carte sont décodés en parallèle (~100 ms chacun en série).
	var masks := _load_images_parallel([["land_mask.png", -1], ["splat.png", Image.FORMAT_RGBA8],
		["province_border_dist.png", Image.FORMAT_RGB8], ["coast_dist.png", Image.FORMAT_L8],
		["river_bed.png", Image.FORMAT_L8], ["province_ids.png", -1],
		["relief_occlusion.png", Image.FORMAT_L8]])
	land_mask = masks[0]
	splat_image = masks[1]
	border_dist_image = masks[2]
	coast_dist_image = masks[3]
	river_bed_image = masks[4]
	_preloaded_ids = masks[5]
	relief_occlusion_image = masks[6]  # RV-D
	if not _load_province_ids():
		return
	var t3 := Time.get_ticks_msec()
	if not _load_provinces():
		return
	var t4 := Time.get_ticks_msec()
	_load_rivers()
	_load_coastline()
	var t5 := Time.get_ticks_msec()
	timings = {
		"map_json_ms": t1 - t0,
		"heightmap_ms": t2 - t1,
		"masks_ms": t3 - t2,
		"provinces_ms": t4 - t3,
		"lines_ms": t5 - t4,
		"total_ms": t5 - t0,
	}


func _fail(message: String) -> bool:
	load_error = message
	push_error("MapData: " + message)
	return false


func _read_json(file_name: String) -> Variant:
	var path := map_dir.path_join(file_name)
	if not FileAccess.file_exists(path):
		return null
	var parsed: Variant = DataFile.try_parse(path)
	if parsed == null:
		push_error("MapData: invalid JSON in %s" % path)
	return parsed


func _load_map_json() -> bool:
	var meta: Variant = _read_json("map.json")
	if meta == null or not (meta is Dictionary):
		return _fail("map.json missing or invalid in %s" % map_dir)
	crs = str(meta.get("crs", ""))
	bounds_projected = meta.get("bounds_projected", [])
	var size_px: Array = meta.get("size_px", [0, 0])
	size = Vector2i(int(size_px[0]), int(size_px[1]))
	meters_per_px = float(meta.get("meters_per_px", meters_per_px))
	height_min_m = float(meta.get("height_min_m", -200.0))
	height_max_m = float(meta.get("height_max_m", 4800.0))
	var render: Variant = meta.get("render_heightmap")
	if render is Dictionary and FileAccess.file_exists(map_dir.path_join(str(render.get("file", "")))):
		height_file = str(render["file"])
	if size.x <= 0 or size.y <= 0:
		return _fail("map.json: invalid size_px")
	return true


func _load_heightmap() -> bool:
	var path := map_dir.path_join(height_file)
	if not FileAccess.file_exists(path):
		return _fail("%s missing" % height_file)
	var raw16 := _load_heightmap_rust(path)
	if raw16.is_empty():
		return _fail("%s unreadable (GameDataStore.load_heightmap_u16 required)" % height_file)
	height_image = Image.create_from_data(raw16.width, raw16.height, false, Image.FORMAT_LA8, raw16.data)
	if height_image.get_size() != size:
		return _fail("%s size %s != map.json size_px %s" % [height_file, height_image.get_size(), size])
	height_bytes = height_image.get_data()
	return true


## Décodage 16 bits par la GDExtension (`GameDataStore.load_heightmap_u16`, crate `png`
## côté Rust, little-endian). `{}` si la GDExtension manque ou refuse le fichier.
func _load_heightmap_rust(path: String) -> Dictionary:
	if not ClassDB.class_exists("GameDataStore"):
		return {}
	var store: Object = ClassDB.instantiate("GameDataStore")
	if not store.has_method("load_heightmap_u16"):
		return {}
	var data: PackedByteArray = store.call("load_heightmap_u16", path)
	if data.is_empty():
		return {}
	var image_size: Vector2i = store.call("get_last_image_size")
	if data.size() != image_size.x * image_size.y * 2:
		push_warning("MapData: load_heightmap_u16 returned %d bytes for %s" % [data.size(), image_size])
		return {}
	return {"width": image_size.x, "height": image_size.y, "data": data, "little_endian": true, "decoder": "rust"}


func _load_image_optional(file_name: String, format: int = -1) -> Image:
	var path := map_dir.path_join(file_name)
	if not FileAccess.file_exists(path):
		return null
	var img := Image.load_from_file(path)
	if img != null and format >= 0 and img.get_format() != format:
		img.convert(format)
	return img


## `[[nom de fichier, format ou -1], …]` → images (null si absente), décodées par le pool de fils.
func _load_images_parallel(specs: Array) -> Array:
	var images: Array = []
	images.resize(specs.size())
	var task := WorkerThreadPool.add_group_task(func(i: int) -> void:
		images[i] = _load_image_optional(str(specs[i][0]), int(specs[i][1])), specs.size(), -1, true, "map masks")
	WorkerThreadPool.wait_for_group_task_completion(task)
	return images


func _load_province_ids() -> bool:
	var img := _preloaded_ids if _preloaded_ids != null else _load_image_optional("province_ids.png")
	_preloaded_ids = null
	if img == null:
		return _fail("province_ids.png missing")
	if img.get_format() != Image.FORMAT_RGB8:
		img.convert(Image.FORMAT_RGB8)
	if img.get_size() != size:
		return _fail("province_ids.png size mismatch")
	province_ids_image = img
	ids_bytes = img.get_data()
	ids_bpp = 3
	return true


func _load_provinces() -> bool:
	var entries := MapDataLoader.provinces(map_dir)
	if entries.is_empty():
		return _fail("provinces.geojson missing or invalid")
	for entry: Dictionary in entries:
		var index: int = entry["index"]
		provinces[index] = entry
		province_index_by_id[entry["id"]] = index
		province_count = maxi(province_count, index)
	return true


func _load_rivers() -> void:
	rivers.assign(MapDataLoader.rivers(map_dir))


func _load_coastline() -> void:
	coastlines.assign(MapDataLoader.coastline(map_dir))


# --- Accès aux échantillons ---------------------------------------------------


## Altitude normalisée [0, 1] au pixel entier (clampé aux bords).
func height01_px(px: int, py: int) -> float:
	px = clampi(px, 0, size.x - 1)
	py = clampi(py, 0, size.y - 1)
	var offset := py * size.x + px
	offset *= 2
	return float(height_bytes[offset] | (height_bytes[offset + 1] << 8)) / 65535.0


## Altitude en mètres, interpolée bilinéairement en coordonnées carte (SZ2b, ADR 0086 : le
## pixel i de la heightmap couvre [i, i + 1], centré en x = i + 0,5, comme dans les outils).
func height_m_at(x: float, y: float) -> float:
	var fx := clampf(x - 0.5, 0.0, size.x - 1.0)
	var fy := clampf(y - 0.5, 0.0, size.y - 1.0)
	var x0 := int(fx)
	var y0 := int(fy)
	var tx := fx - x0
	var ty := fy - y0
	var h00 := height01_px(x0, y0)
	var h10 := height01_px(x0 + 1, y0)
	var h01 := height01_px(x0, y0 + 1)
	var h11 := height01_px(x0 + 1, y0 + 1)
	var h := lerpf(lerpf(h00, h10, tx), lerpf(h01, h11, tx), ty)
	return height_min_m + h * (height_max_m - height_min_m)


## `height_m_at` pour une série de points (mêmes valeurs), les champs de la carte
## et les bornes lus une seule fois ; à préférer aux boucles de `height_m_at` point par point.
func heights_m_at(points: PackedVector2Array) -> PackedFloat32Array:
	var out := PackedFloat32Array()
	out.resize(points.size())
	var span := height_max_m - height_min_m
	var max_x := size.x - 1
	var max_y := size.y - 1
	var width := size.x
	var bytes := height_bytes
	var wide := height_bpp == 2
	var little := height_little_endian
	for n in points.size():
		var fx := clampf(points[n].x - 0.5, 0.0, max_x)
		var fy := clampf(points[n].y - 0.5, 0.0, max_y)
		var x0 := int(fx)
		var y0 := int(fy)
		var x1 := mini(x0 + 1, max_x)
		var y1 := mini(y0 + 1, max_y)
		var tx := fx - x0
		var ty := fy - y0
		var h00 := _height01_at(bytes, (y0 * width + x0), wide, little)
		var h10 := _height01_at(bytes, (y0 * width + x1), wide, little)
		var h01 := _height01_at(bytes, (y1 * width + x0), wide, little)
		var h11 := _height01_at(bytes, (y1 * width + x1), wide, little)
		out[n] = height_min_m + lerpf(lerpf(h00, h10, tx), lerpf(h01, h11, tx), ty) * span
	return out


static func _height01_at(bytes: PackedByteArray, offset: int, wide: bool, little: bool) -> float:
	if not wide:
		return float(bytes[offset]) / 255.0
	offset *= 2
	if little:
		return float(bytes[offset] | (bytes[offset + 1] << 8)) / 65535.0
	return float((bytes[offset] << 8) | bytes[offset + 1]) / 65535.0


## Altitude monde (Y Godot) en coordonnées carte.
func height_world_at(x: float, y: float) -> float:
	return display_height(height_m_at(x, y), x, y)


## Altitude monde, jamais sous le niveau de la mer (pour poser des objets).
func surface_world_at(x: float, y: float) -> float:
	return maxf(height_world_at(x, y), 0.0)


## Index de province au pixel (0 = mer / aucune).
func province_index_at(x: float, y: float) -> int:
	var px := int(floorf(x))
	var py := int(floorf(y))
	if px < 0 or py < 0 or px >= size.x or py >= size.y:
		return 0
	var offset := (py * size.x + px) * ids_bpp
	return ids_bytes[offset] | (ids_bytes[offset + 1] << 8)


func get_province(index: int) -> Dictionary:
	return provinces.get(index, {})


## Index raster d'une province par id (0 si inconnue).
func index_of_id(id: String) -> int:
	return int(province_index_by_id.get(id, 0))


## Centroïde (coordonnées carte) d'une province par id ; Vector2(-1, -1) si inconnue.
func centroid_of_id(id: String) -> Vector2:
	var province := get_province(index_of_id(id))
	if province.is_empty():
		return Vector2(-1, -1)
	return province["centroid"]


## Distance signée (px carte) à la berge du fleuve le plus proche, négative dans le lit ;
## 8 (loin de tout fleuve) sans `river_bed.png`. Plus proche voisin (semis de la végétation).
func river_sd_at(x: float, y: float) -> float:
	if river_bed_image == null:
		return 8.0
	var px := clampi(int(x), 0, river_bed_image.get_width() - 1)
	var py := clampi(int(y), 0, river_bed_image.get_height() - 1)
	return (river_bed_image.get_pixel(px, py).r * 255.0 - 128.0) / 16.0


func is_land_px(px: int, py: int) -> bool:
	if land_mask == null:
		return height_m_at(px, py) > 0.0
	if px < 0 or py < 0 or px >= size.x or py >= size.y:
		return false
	return land_mask.get_pixel(px, py).r > 0.5


## Position (coordonnées carte) d'une colonie (`settlements_px.json`) ;
## Vector2(-1, -1) si inconnue ou si le fichier manque.
func settlement_px(id: String) -> Vector2:
	if _settlements_px == null:
		var parsed: Variant = _read_json("settlements_px.json")
		_settlements_px = parsed if parsed is Dictionary else {}
	var entry: Variant = (_settlements_px as Dictionary).get(id)
	if entry is Array and (entry as Array).size() >= 2:
		return Vector2(float(entry[0]), float(entry[1]))
	return Vector2(-1, -1)


var _settlements_px: Variant = null
