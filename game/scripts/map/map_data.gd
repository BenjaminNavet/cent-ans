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
var meters_per_px: float = 800.0
var height_min_m: float = -200.0
var height_max_m: float = 4800.0

## Image L8 (8 bits, repli) ou LA8 (16 bits, deux octets par pixel). L'ordre des octets
## dépend de la source : `GameDataStore.load_heightmap_u16` livre du little-endian
## (L = octet faible, A = octet fort), `Png16` du big-endian (L = octet fort).
var height_image: Image
var height_bytes: PackedByteArray
var height_bpp: int = 1
var height_little_endian: bool = false
## "rust" (GameDataStore), "png16" (GDScript) ou "8bit" (repli Image.load_from_file).
var height_decoder: String = ""
## Fichier d'altitude chargé : `heightmap_render.png` (lot R1 : Copernicus 90 m moyenné, relief
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
## Occupation du sol par province (vigne, sécheresse, bocage), calculée à la demande par
## `VegetationFields.landuse` (lot V2b) ; null tant qu'elle n'a pas été demandée.
var landuse_image: Image
var province_ids_image: Image
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
	land_mask = _load_image_optional("land_mask.png")
	splat_image = _load_image_optional("splat.png", Image.FORMAT_RGBA8)
	border_dist_image = _load_image_optional("province_border_dist.png", Image.FORMAT_RGB8)
	coast_dist_image = _load_image_optional("coast_dist.png", Image.FORMAT_L8)
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
	var text := FileAccess.get_file_as_string(path)
	var parsed: Variant = JSON.parse_string(text)
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
	meters_per_px = float(meta.get("meters_per_px", 800.0))
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
		raw16 = _load_heightmap_16(path)
	if raw16.is_empty():
		var img := Image.load_from_file(path)
		if img == null:
			return _fail("%s unreadable" % height_file)
		if img.get_format() != Image.FORMAT_L8:
			img.convert(Image.FORMAT_L8)
		height_image = img
		height_bpp = 1
		height_decoder = "8bit"
		push_warning("MapData: heightmap loaded as 8-bit (precision ~%.1f m)" % ((height_max_m - height_min_m) / 255.0))
	else:
		height_image = Image.create_from_data(raw16.width, raw16.height, false, Image.FORMAT_LA8, raw16.data)
		height_bpp = 2
		height_little_endian = raw16.get("little_endian", false)
		height_decoder = raw16.get("decoder", "png16")
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


## Décodage 16 bits en GDScript (repli) avec cache disque (`user://cache/`) : le défiltrage PNG en
## GDScript est lent sur 4096², mais le tampon brut se relit instantanément.
func _load_heightmap_16(path: String) -> Dictionary:
	var mtime := FileAccess.get_modified_time(path)
	var cache_dir := "user://cache"
	var cache_path := cache_dir.path_join("heightmap_%s_%d.u16be" % [path.md5_text(), mtime])
	if FileAccess.file_exists(cache_path):
		var cached := FileAccess.get_file_as_bytes(cache_path)
		if cached.size() == size.x * size.y * 2:
			return {"width": size.x, "height": size.y, "data": cached, "decoder": "png16"}
	var decoded := Png16.load_gray16(path)
	if decoded.is_empty():
		return {}
	DirAccess.make_dir_recursive_absolute(cache_dir)
	var file := FileAccess.open(cache_path, FileAccess.WRITE)
	if file != null:
		file.store_buffer(decoded.data)
		file.close()
	return decoded


func _load_image_optional(file_name: String, format: int = -1) -> Image:
	var path := map_dir.path_join(file_name)
	if not FileAccess.file_exists(path):
		return null
	var img := Image.load_from_file(path)
	if img != null and format >= 0 and img.get_format() != format:
		img.convert(format)
	return img


func _load_province_ids() -> bool:
	var img := _load_image_optional("province_ids.png")
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
	var geo: Variant = _read_json("provinces.geojson")
	if geo == null or not (geo is Dictionary):
		return _fail("provinces.geojson missing or invalid")
	var features: Array = geo.get("features", [])
	var position := 0
	for feature in features:
		position += 1
		var props: Dictionary = feature.get("properties", {})
		var index := int(props.get("index", position))
		var id: String = str(props.get("id", feature.get("id", "prov_%d" % index)))
		var centroid := _to_vec2(props.get("centroid", [0, 0]))
		var entry := {
			"index": index,
			"id": id,
			"name": str(props.get("name", id)),
			"owner": str(props.get("owner", "")),
			"terrain": str(props.get("terrain", "")),
			"capital_name": str(props.get("capital_name", "")),
			"centroid": centroid,
			"capital_px": _to_vec2(props.get("capital_px", [centroid.x, centroid.y])),
			"neighbors": props.get("neighbors", []),
			"area_px": float(props.get("area_px", 0.0)),
			"rings": _outer_rings(feature.get("geometry", {})),
		}
		provinces[index] = entry
		province_index_by_id[id] = index
		province_count = maxi(province_count, index)
	return true


func _load_rivers() -> void:
	var geo: Variant = _read_json("rivers.geojson")
	if geo == null or not (geo is Dictionary):
		return
	for feature in geo.get("features", []):
		var props: Dictionary = feature.get("properties", {})
		var importance: int
		if props.has("strahler"):
			importance = clampi(int(props["strahler"]), 1, 6)
		else:
			importance = clampi(12 - int(props.get("scalerank", 10)), 0, 6)
		for line in _linestrings(feature.get("geometry", {})):
			rivers.append({"name": str(props.get("name", "")), "importance": importance, "points": line})


func _load_coastline() -> void:
	var geo: Variant = _read_json("coastline.geojson")
	if geo == null or not (geo is Dictionary):
		return
	for feature in geo.get("features", []):
		for line in _linestrings(feature.get("geometry", {})):
			coastlines.append(line)


static func _to_vec2(value: Variant) -> Vector2:
	if value is Array and value.size() >= 2:
		return Vector2(float(value[0]), float(value[1]))
	return Vector2.ZERO


static func _to_line(coords: Array) -> PackedVector2Array:
	var line := PackedVector2Array()
	line.resize(coords.size())
	for i in coords.size():
		line[i] = _to_vec2(coords[i])
	return line


static func _linestrings(geometry: Dictionary) -> Array[PackedVector2Array]:
	var lines: Array[PackedVector2Array] = []
	var kind: String = geometry.get("type", "")
	var coords: Array = geometry.get("coordinates", [])
	if kind == "LineString":
		lines.append(_to_line(coords))
	elif kind == "MultiLineString":
		for part in coords:
			lines.append(_to_line(part))
	return lines


static func _outer_rings(geometry: Dictionary) -> Array[PackedVector2Array]:
	var rings: Array[PackedVector2Array] = []
	var kind: String = geometry.get("type", "")
	var coords: Array = geometry.get("coordinates", [])
	if kind == "Polygon" and coords.size() > 0:
		rings.append(_to_line(coords[0]))
	elif kind == "MultiPolygon":
		for polygon in coords:
			if polygon.size() > 0:
				rings.append(_to_line(polygon[0]))
	return rings


# --- Accès aux échantillons ---------------------------------------------------


## Altitude normalisée [0, 1] au pixel entier (clampé aux bords).
func height01_px(px: int, py: int) -> float:
	px = clampi(px, 0, size.x - 1)
	py = clampi(py, 0, size.y - 1)
	var offset := py * size.x + px
	if height_bpp == 2:
		offset *= 2
		if height_little_endian:
			return float(height_bytes[offset] | (height_bytes[offset + 1] << 8)) / 65535.0
		return float((height_bytes[offset] << 8) | height_bytes[offset + 1]) / 65535.0
	return float(height_bytes[offset]) / 255.0


## Altitude en mètres, interpolée bilinéairement en coordonnées carte.
func height_m_at(x: float, y: float) -> float:
	var fx := clampf(x, 0.0, size.x - 1.0)
	var fy := clampf(y, 0.0, size.y - 1.0)
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


## Altitude monde (Y Godot) en coordonnées carte.
func height_world_at(x: float, y: float) -> float:
	return height_m_at(x, y) * HEIGHT_SCALE


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


func is_land_px(px: int, py: int) -> bool:
	if land_mask == null:
		return height_m_at(px, py) > 0.0
	if px < 0 or py < 0 or px >= size.x or py >= size.y:
		return false
	return land_mask.get_pixel(px, py).r > 0.5


## Position (coordonnées carte) d'une colonie (lot C4, `settlements_px.json`) ;
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
