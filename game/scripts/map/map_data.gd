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
## Facteur vertical courant (unités monde par mètre), propriétaire unique de l'échelle verticale
## (ADR 0036, lot ZG4) : `HEIGHT_SCALE` (×4,3) en vue stratégique, ramené vers ×1,5 au zoom le
## plus rapproché quand la pyramide de relief est en cache (`CloseCameraProfile`). Lu par les
## shaders (paramètre global `campaign_vertical_scale`), le quadtree et `surface_height_at`.
## Les maillages E0 du repli sans cache restent cuits à `HEIGHT_SCALE` (l'échelle ne bouge pas).
static var _vertical_scale: float = HEIGHT_SCALE
## Lot ZG8 : relief exagéré (hauteur affichée = s·(h + g·max(h − fond, 0)), voir
## `ReliefExaggerationProfile`). Gain de relief local courant (fonction de l'échelle, publié avec
## elle), fond de vallée (m, grille `ReliefFloor`, lecture seule une fois publiée : lisible depuis
## les fils de travail). Sans fond publié, le gain est nul : comportement du lot ZG4.
static var _relief_gain: float = 0.0
static var _floor: PackedFloat32Array = PackedFloat32Array()
## Lot SZ1 : base (fond non plafonné, m) et facteur d'écrasement des montagnes k (0..1) par cellule
## de la même grille ; poids courant c de l'écrasement (fonction de l'échelle, publié avec elle).
static var _floor_base: PackedFloat32Array = PackedFloat32Array()
static var _floor_squash: PackedFloat32Array = PackedFloat32Array()
static var _floor_squash_max: float = 0.0
static var _relief_squash: float = 0.0
static var _floor_side: Vector2i = Vector2i.ZERO
static var _floor_cell: float = 8.0
## Lot PB2 : incrémenté à chaque publication du fond (le semis natif le recopie alors).
static var _floor_version: int = 0


var map_dir: String = ""
var load_error: String = ""
var timings: Dictionary = {}

var crs: String = ""
var bounds_projected: Array = []
var size: Vector2i = Vector2i.ZERO
var meters_per_px: float = 718.9765625  # map.json (échelle ADR 0082)
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
## Lot V4 : lit des fleuves (`cent-ans geo rivers-render`), L8, distance signée à la berge
## ((valeur − 128) / 16 px, négative dans le lit) ; null si absent.
var river_bed_image: Image
## Lot RV-D : occlusion de vallée (`cent-ans geo relief-occlusion`), L8 demi-grille, 128 neutre,
## > 128 vallée, < 128 crête ; null si absente (le terrain s'en passe).
var relief_occlusion_image: Image
## Occupation du sol par province (vigne, sécheresse, bocage), calculée à la demande par
## `VegetationFields.landuse` (lot V2b) ; null tant qu'elle n'a pas été demandée.
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


static func vertical_scale() -> float:
	return _vertical_scale


## Lot ZG4 : exagération courante (× par rapport au relief vrai, 1 unité = `meters_per_px` m).
static func vertical_exaggeration(meters_per_unit: float = 719.0) -> float:
	return _vertical_scale * meters_per_unit


## Lot ZG4 : change l'échelle verticale (propriétaire unique) et la publie aux shaders par le
## paramètre global `campaign_vertical_scale` (terrain, quadtree, fleuves, maquettes). Rend vrai
## si la valeur a changé. Appelé par `TerrainBuilder.set_vertical_scale`, qui recale les calques ;
## ne pas l'appeler directement ailleurs (sinon les objets posés ne suivent pas).
static func set_vertical_scale(value: float) -> bool:
	value = maxf(value, 1e-6)
	if is_equal_approx(value, _vertical_scale):
		return false
	_vertical_scale = value
	RenderingServer.global_shader_parameter_set("campaign_vertical_scale", value)
	_publish_gain()
	return true


# --- Relief exagéré (lot ZG8) ------------------------------------------------------------


## Plancher de l'échelle verticale de près (unités monde par mètre), selon les profils.
static func near_vertical_scale() -> float:
	var camera := CloseCameraProfile.load_default()
	return HEIGHT_SCALE * camera.near_exaggeration() / camera.exaggeration_far


## Gain de relief local pour une échelle donnée (nul sans fond publié).
static func relief_gain_for_scale(scale: float) -> float:
	if _floor.is_empty():
		return 0.0
	return ReliefExaggerationProfile.load_default().gain_for_scale(scale, near_vertical_scale())


## Gain de relief local courant.
static func relief_gain() -> float:
	return _relief_gain


## Plus grand gain possible (boîtes englobantes conservatrices : y ≤ s·(1 + g_max)·h).
static func relief_max_gain() -> float:
	return 0.0 if _floor.is_empty() else ReliefExaggerationProfile.load_default().max_gain()


static func _publish_gain() -> void:
	var gain := relief_gain_for_scale(_vertical_scale)
	_relief_gain = gain
	RenderingServer.global_shader_parameter_set("campaign_relief_gain", gain)
	var squash := relief_squash_for_scale(_vertical_scale)
	_relief_squash = squash
	RenderingServer.global_shader_parameter_set("campaign_relief_squash", squash)


## Lot SZ1 : échelle (unités monde par mètre) à partir de laquelle l'écrasement des montagnes est
## entier (`ReliefExaggerationProfile.mountain_squash_full_exaggeration`).
static func squash_full_vertical_scale() -> float:
	var camera := CloseCameraProfile.load_default()
	var relief := ReliefExaggerationProfile.load_default()
	return HEIGHT_SCALE * relief.mountain_squash_full_exaggeration / camera.exaggeration_far


## Lot SZ1 : poids c de l'écrasement des montagnes pour une échelle (nul sans fond publié).
static func relief_squash_for_scale(scale: float) -> float:
	if _floor.is_empty() or _floor_squash_max <= 0.0:
		return 0.0
	return ReliefExaggerationProfile.load_default().squash_weight_for_scale(scale, squash_full_vertical_scale())


## Lot SZ1 : poids courant de l'écrasement des montagnes.
static func relief_squash() -> float:
	return _relief_squash


## Lot SZ1 : plus grand écrasement c·k pour une échelle (boîtes englobantes : y ≥ s·(1 − c·k)·h).
static func relief_squash_max_for_scale(scale: float) -> float:
	return relief_squash_for_scale(scale) * _floor_squash_max


## Publie le fond de vallée (`ReliefFloor.compute`) aux shaders et au double GDScript ; grille
## vide : relief exagéré désactivé (gain nul).
static func set_relief_floor(grid: Dictionary) -> void:
	var data: PackedFloat32Array = grid.get("data", PackedFloat32Array())
	var side: Vector2i = grid.get("side", Vector2i.ZERO)
	if data.is_empty() or side.x * side.y != data.size():
		_floor = PackedFloat32Array()
		_floor_base = PackedFloat32Array()
		_floor_squash = PackedFloat32Array()
		_floor_squash_max = 0.0
		_floor_side = Vector2i.ZERO
	else:
		_floor = data
		var base: PackedFloat32Array = grid.get("base", PackedFloat32Array())
		var squash: PackedFloat32Array = grid.get("squash", PackedFloat32Array())
		_floor_base = base if base.size() == data.size() else data
		if squash.size() != data.size():
			squash = PackedFloat32Array()
			squash.resize(data.size())
		_floor_squash = squash
		_floor_squash_max = 0.0
		for v in squash:
			_floor_squash_max = maxf(_floor_squash_max, v)
		_floor_side = side
		_floor_cell = float(grid.get("cell", 8.0))
		RenderingServer.global_shader_parameter_set("campaign_relief_floor", ReliefFloor.texture_of(grid))
		RenderingServer.global_shader_parameter_set("campaign_relief_floor_info", Vector4(_floor_cell, 0.5 * (_floor_cell - 1.0), 0.0, 0.0))
	_floor_version += 1
	_publish_gain()


## Lot PB2 : fond de vallée publié {data, side, cell, version} (lecture seule ; semis natif).
static func relief_floor_grid() -> Dictionary:
	return {"data": _floor, "base": _floor_base, "squash": _floor_squash, "side": _floor_side, "cell": _floor_cell, "version": _floor_version}


## Vrai si un fond de vallée est publié.
static func has_relief_floor() -> bool:
	return not _floor.is_empty()


## Fond de vallée (m) au point carte (x, z) : bilinéaire entre centres de cellules, bords
## répliqués. Même calcul que `campaign_relief_floor_m` (texelFetch, pas de filtrage matériel).
static func relief_floor_at(x: float, z: float) -> float:
	return relief_fields_at(x, z).x


## Lot SZ1 : champs du relief au point carte (x, z) : (fond, base, facteur d'écrasement k), même
## interpolation que `campaign_relief_fields` (campaign_relief.gdshaderinc). Sans fond : zéro.
static func relief_fields_at(x: float, z: float) -> Vector3:
	if _floor.is_empty():
		return Vector3.ZERO
	var fx := clampf((x - 0.5 * (_floor_cell - 1.0)) / _floor_cell, 0.0, _floor_side.x - 1.0)
	var fz := clampf((z - 0.5 * (_floor_cell - 1.0)) / _floor_cell, 0.0, _floor_side.y - 1.0)
	var i := mini(int(fx), _floor_side.x - 2)
	var j := mini(int(fz), _floor_side.y - 2)
	var tx := fx - i
	var tz := fz - j
	var o := j * _floor_side.x + i
	var w := _floor_side.x
	return Vector3(
		_bilerp(_floor, o, w, tx, tz), _bilerp(_floor_base, o, w, tx, tz), _bilerp(_floor_squash, o, w, tx, tz))


static func _bilerp(grid: PackedFloat32Array, o: int, w: int, tx: float, tz: float) -> float:
	return lerpf(lerpf(grid[o], grid[o + 1], tx), lerpf(grid[o + w], grid[o + w + 1], tx), tz)


## Hauteur affichée (unités monde) d'une altitude `h_m` (m) au point carte (x, z), à l'échelle, au
## gain et à l'écrasement courants. Double GDScript de `campaign_display_height`
## (campaign_relief.gdshaderinc) : tout ce qui pose un objet au sol passe par ici.
##     y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0)), K = c·k
static func display_height(h_m: float, x: float, z: float) -> float:
	return display_height_with(h_m, x, z, _vertical_scale, _relief_gain, _relief_squash)


## `display_height` à une échelle, un gain et un écrasement donnés (maillages cuits, fils de travail).
static func display_height_with(h_m: float, x: float, z: float, scale: float, gain: float, squash: float) -> float:
	if gain == 0.0 and squash == 0.0:
		return h_m * scale
	return display_height_fields(h_m, relief_fields_at(x, z), scale, gain, squash)


## `display_height_with` avec les champs (fond, base, k) déjà lus au même point.
static func display_height_fields(h_m: float, fields: Vector3, scale: float, gain: float, squash: float) -> float:
	var k := squash * fields.z
	return scale * (h_m - k * maxf(h_m - fields.y, 0.0) + gain * (1.0 - k) * maxf(h_m - fields.x, 0.0))


## Inverse de `display_height` : altitude (m) dont la hauteur affichée en (x, z) vaut `y`.
static func height_from_display(y: float, x: float, z: float) -> float:
	return height_from_display_with(y, x, z, _vertical_scale, _relief_gain, _relief_squash)


## Inverse par morceaux (fonction croissante de h, coudes en base ≤ fond).
static func height_from_display_with(y: float, x: float, z: float, scale: float, gain: float, squash: float) -> float:
	var v := y / maxf(scale, 1e-9)
	if gain == 0.0 and squash == 0.0:
		return v
	var fields := relief_fields_at(x, z)
	var f := fields.x
	var b := minf(fields.y, f)
	var k := squash * fields.z
	if v <= b:
		return v
	var v_f := f - k * (f - b)
	if v <= v_f:
		return (v - k * b) / (1.0 - k)
	var g := gain * (1.0 - k)
	return (v - k * b + g * f) / (1.0 - k + g)


## Taille du monde (`size_px` de `<map_dir>/map.json`, unités carte), sans charger la carte ;
## Vector2i.ZERO si illisible (lot OM1, ADR 0115 : aucune taille codée en dur).
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


## Lot V4 : distance signée (px carte) à la berge du fleuve le plus proche, négative dans le lit ;
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
