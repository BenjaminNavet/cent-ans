class_name VegetationMask
extends RefCounted

## Masques de végétation de la carte de campagne (lot V3, rendu seulement).
##
## Source préférée : `data/map/splat.png` (lot V2), RGBA dans le repère de `province_ids.png` :
## R = prairie, G = cultures, B = forêt, A = roche/lande. Si le fichier manque (ou est illisible),
## repli procédural : terrain dominant de la province (`data/provinces/<id>.json`, champ
## `terrain`), altitude, pente et bruit. Les deux sources donnent des densités [0, 1].
##
## Lecture seule après `setup` : les fonctions d'échantillonnage peuvent être appelées depuis
## des tâches de `WorkerThreadPool` (le bruit est dupliqué par tâche, voir `make_noise`).

const SPLAT_FILE := "splat.png"
## Lot V4 : sources interchangeables de la couverture forestière et des essences.
const FOREST_COVER_FILE := "forest_cover.json"
const CHANNELS := {"r": 0, "g": 1, "b": 2, "a": 3}

## Densité de forêt de base par terrain dominant (repli procédural).
const FOREST_BY_TERRAIN := {
	"forest": 0.80,
	"bocage": 0.38,
	"hills": 0.40,
	"mountains": 0.42,
	"plains": 0.20,
	"heath": 0.08,
	"marsh": 0.14,
	"": 0.26,
}
## Part de terres cultivées par terrain dominant (repli procédural).
const CROPS_BY_TERRAIN := {
	"forest": 0.15,
	"bocage": 0.85,
	"hills": 0.45,
	"mountains": 0.10,
	"plains": 0.85,
	"heath": 0.10,
	"marsh": 0.20,
	"": 0.35,
}

## Voisinage (px) moyenné pour le terrain dominant, décalé de manière irrégulière.
const NEIGHBORHOOD: Array[Vector2] = [
	Vector2(0, 0), Vector2(17, 5), Vector2(-6, 16), Vector2(-16, -7), Vector2(5, -18),
	Vector2(30, -22), Vector2(-28, 26),
]

var map_data: MapData
## "splat" ou "procedural".
var source: String = "procedural"
var splat_size: Vector2i = Vector2i.ZERO

var _splat_bytes: PackedByteArray
## Terrain dominant par index raster de province.
var _terrain_by_index: PackedStringArray = PackedStringArray()
var _hedge_terrain: PackedByteArray = PackedByteArray()
var _landuse_bytes: PackedByteArray = PackedByteArray()
var _landuse_size: Vector2i = Vector2i.ZERO
## Lot V4 : couverture forestière (octets RGBA8, canal, seuils) et essences optionnelles.
var cover_source: String = ""
var _cover_bytes: PackedByteArray = PackedByteArray()
var _cover_size: Vector2i = Vector2i.ZERO
var _cover_channel: int = 2
var _cover_low: float = 0.22
var _cover_high: float = 0.68
var _essence_bytes: PackedByteArray = PackedByteArray()
var _essence_size: Vector2i = Vector2i.ZERO
var _essence_channels: Vector3i = Vector3i(0, 1, 2)
var _beech: Dictionary = {"altitude_low_m": 150.0, "altitude_high_m": 750.0, "patch_px": 22.0, "south_oak": 0.35}


func setup(data: MapData) -> void:
	map_data = data
	_load_splat()
	_load_forest_cover()
	_load_province_terrains()
	# Densité de haies partagée avec le shader de terrain (canal B, bocage flouté).
	var landuse := VegetationFields.landuse(data)
	_landuse_size = landuse.get_size()
	_landuse_bytes = landuse.get_data()


func has_splat() -> bool:
	return source == "splat"


func _load_splat() -> void:
	# Déjà chargée par `MapData` (lot V2) ; sinon lecture directe (compatibilité).
	var image: Image = map_data.get("splat_image") as Image
	if image == null:
		var path := map_data.map_dir.path_join(SPLAT_FILE)
		if not FileAccess.file_exists(path):
			return
		image = Image.load_from_file(path)
	if image != null and image.get_format() != Image.FORMAT_RGBA8:
		image = image.duplicate() as Image
	if image == null or image.is_empty():
		push_warning("VegetationMask: %s unreadable, procedural fallback" % SPLAT_FILE)
		return
	if image.get_format() != Image.FORMAT_RGBA8:
		image.convert(Image.FORMAT_RGBA8)
	splat_size = image.get_size()
	_splat_bytes = image.get_data()
	source = "splat"


## Lot V4 : `data/map/forest_cover.json` → raster de couverture (défaut : splat, canal B) et,
## en option, raster des essences (chêne, hêtre, conifères).
func _load_forest_cover() -> void:
	var path := map_data.map_dir.path_join(FOREST_COVER_FILE)
	var config: Variant = JSON.parse_string(FileAccess.get_file_as_string(path)) if FileAccess.file_exists(path) else null
	if not (config is Dictionary):
		return
	var cover: Dictionary = config.get("cover", {})
	var file := str(cover.get("file", SPLAT_FILE))
	_cover_channel = int(CHANNELS.get(str(cover.get("channel", "b")), 2))
	_cover_low = float(cover.get("low", _cover_low))
	_cover_high = float(cover.get("high", _cover_high))
	if file == SPLAT_FILE and not _splat_bytes.is_empty():
		_cover_bytes = _splat_bytes
		_cover_size = splat_size
		cover_source = file
	else:
		var image := _rgba8(map_data.map_dir.path_join(file))
		if image != null:
			_cover_bytes = image.get_data()
			_cover_size = image.get_size()
			cover_source = file
		else:
			push_warning("VegetationMask: forest cover %s unreadable" % file)
	var essences: Variant = config.get("essences")
	if essences is Dictionary:
		var image := _rgba8(map_data.map_dir.path_join(str(essences.get("file", ""))))
		if image != null:
			_essence_bytes = image.get_data()
			_essence_size = image.get_size()
			_essence_channels = Vector3i(int(CHANNELS.get(str(essences.get("oak", "r")), 0)),
				int(CHANNELS.get(str(essences.get("beech", "g")), 1)), int(CHANNELS.get(str(essences.get("conifer", "b")), 2)))
	var beech: Variant = config.get("beech")
	if beech is Dictionary:
		_beech.merge(beech, true)


static func _rgba8(path: String) -> Image:
	if not FileAccess.file_exists(path):
		return null
	var image := Image.load_from_file(path)
	if image == null or image.is_empty():
		return null
	if image.get_format() != Image.FORMAT_RGBA8:
		image.convert(Image.FORMAT_RGBA8)
	return image


static func _texel(bytes: PackedByteArray, size: Vector2i, map_size: Vector2i, x: float, y: float, channel: int) -> float:
	var px := clampi(int(x * size.x / map_size.x), 0, size.x - 1)
	var py := clampi(int(y * size.y / map_size.y), 0, size.y - 1)
	return bytes[(py * size.x + px) * 4 + channel] / 255.0


func has_forest_cover() -> bool:
	return not _cover_bytes.is_empty()


func _load_province_terrains() -> void:
	_terrain_by_index.resize(map_data.province_count + 1)
	_hedge_terrain.resize(map_data.province_count + 1)
	var provinces_dir := map_data.map_dir.get_base_dir().path_join("provinces")
	for index in map_data.provinces:
		var province: Dictionary = map_data.provinces[index]
		var terrain: String = str(province.get("terrain", ""))
		if terrain == "":
			var path := provinces_dir.path_join(str(province.get("id", "")) + ".json")
			if FileAccess.file_exists(path):
				var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
				if parsed is Dictionary:
					terrain = str(parsed.get("terrain", ""))
		if index >= 0 and index < _terrain_by_index.size():
			_terrain_by_index[index] = terrain
			_hedge_terrain[index] = 1 if terrain == "bocage" else 0


## Bruit basse fréquence des massifs forestiers (un par tâche : FastNoiseLite n'est pas
## garanti réentrant).
static func make_noise(seed_value: int = 1337) -> FastNoiseLite:
	var noise := FastNoiseLite.new()
	noise.seed = seed_value
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = 1.0 / 48.0
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 4
	noise.fractal_gain = 0.55
	return noise


func terrain_at(x: float, y: float) -> String:
	var index := map_data.province_index_at(x, y)
	if index <= 0 or index >= _terrain_by_index.size():
		return ""
	return _terrain_by_index[index]


## Échantillon de la splatmap (Color 0..1) au pixel carte le plus proche ; noir sans splat.
func splat_at(x: float, y: float) -> Color:
	if _splat_bytes.is_empty():
		return Color(0, 0, 0, 0)
	var px := clampi(int(x * splat_size.x / map_data.size.x), 0, splat_size.x - 1)
	var py := clampi(int(y * splat_size.y / map_data.size.y), 0, splat_size.y - 1)
	var o := (py * splat_size.x + px) * 4
	return Color8(_splat_bytes[o], _splat_bytes[o + 1], _splat_bytes[o + 2], _splat_bytes[o + 3])


## Pente (dénivelé / distance horizontale, sans unité) par différences finies sur ±2 px.
func slope_at(x: float, y: float) -> float:
	var dx := map_data.height_m_at(x + 2.0, y) - map_data.height_m_at(x - 2.0, y)
	var dy := map_data.height_m_at(x, y + 2.0) - map_data.height_m_at(x, y - 2.0)
	return Vector2(dx, dy).length() / (4.0 * map_data.meters_per_px)


## Limite des arbres (m) : ≈ 2000 m au sud, plus basse vers le nord (y carte petit).
func treeline_m(y: float) -> float:
	var north := 1.0 - clampf(y / maxf(map_data.size.y, 1.0), 0.0, 1.0)
	return lerpf(2100.0, 900.0, smoothstep(0.55, 0.95, north))


## Échantillon complet pour le semis : {forest, crops, conifer, hedge, height_m}.
## `noise` : bruit propre à l'appelant (`make_noise`).
func sample(x: float, y: float, noise: FastNoiseLite) -> Dictionary:
	var height_m := map_data.height_m_at(x, y)
	var result := {"forest": 0.0, "crops": 0.0, "conifer": 0.0, "beech": 0.0, "hedge": 0.0, "height_m": height_m}
	if height_m <= 1.0 or not map_data.is_land_px(int(x), int(y)):
		return result
	var slope := slope_at(x, y)
	var treeline := treeline_m(y)
	var altitude_factor := 1.0 - smoothstep(treeline - 350.0, treeline, height_m)
	var slope_factor := 1.0 - smoothstep(0.35, 0.8, slope)
	var n := noise.get_noise_2d(x, y)
	var forest: float
	var crops: float
	if has_splat():
		var splat := splat_at(x, y)
		# Poids de mélange du terrain → densité de semis : massifs nets là où la texture de
		# forêt domine, arbres isolés rares ailleurs.
		forest = smoothstep(0.22, 0.68, splat.b)
		# Campagne ouverte (cultures + prairies) : bosquets, arbres isolés et haies.
		crops = clampf(splat.g + 0.6 * splat.r, 0.0, 1.0)
	else:
		# Terrain moyenné sur un voisinage : les lisières ne suivent pas les frontières de province.
		var base := 0.0
		var crop_base := 0.0
		for offset in NEIGHBORHOOD:
			var t := terrain_at(x + offset.x, y + offset.y)
			base += float(FOREST_BY_TERRAIN.get(t, FOREST_BY_TERRAIN[""]))
			crop_base += float(CROPS_BY_TERRAIN.get(t, CROPS_BY_TERRAIN[""]))
		base /= NEIGHBORHOOD.size()
		crop_base /= NEIGHBORHOOD.size()
		# Massifs nets avec clairières : seuil du bruit d'autant plus bas que le terrain est boisé.
		var threshold := 0.55 - base * 1.25
		forest = smoothstep(threshold - 0.08, threshold + 0.10, n)
		# Les fonds de vallée et le littoral bas sont plus défrichés.
		forest *= lerpf(0.55, 1.0, smoothstep(20.0, 180.0, height_m))
		crops = crop_base * (1.0 - forest)
		crops *= 1.0 - smoothstep(600.0, 1100.0, height_m)
	if has_forest_cover():
		forest = smoothstep(_cover_low, _cover_high, _texel(_cover_bytes, _cover_size, map_data.size, x, y, _cover_channel))
	forest *= altitude_factor * slope_factor
	result["forest"] = forest
	result["crops"] = crops * slope_factor
	# Conifères : altitude (relative à la limite des arbres) et latitude, un peu de bruit.
	var north := 1.0 - clampf(y / maxf(map_data.size.y, 1.0), 0.0, 1.0)
	var conifer := smoothstep(treeline * 0.35, treeline * 0.7, height_m) + smoothstep(0.62, 0.85, north)
	result["conifer"] = clampf(conifer + n * 0.25, 0.0, 1.0)
	# Lot V4 : chênaie / hêtraie, par taches (le hêtre monte en altitude, recule dans le Midi).
	var patch: float = 48.0 / float(_beech["patch_px"])
	var n2 := noise.get_noise_2d(x * patch + 5171.0, y * patch - 3307.0)
	var south := smoothstep(0.55, 0.78, clampf(y / maxf(map_data.size.y, 1.0), 0.0, 1.0))
	var beech := 0.2 + 0.7 * smoothstep(float(_beech["altitude_low_m"]), float(_beech["altitude_high_m"]), height_m) + 0.9 * n2 - float(_beech["south_oak"]) * south
	result["beech"] = smoothstep(0.35, 0.65, beech)
	if not _essence_bytes.is_empty():
		var oak_share := _texel(_essence_bytes, _essence_size, map_data.size, x, y, _essence_channels.x)
		var beech_share := _texel(_essence_bytes, _essence_size, map_data.size, x, y, _essence_channels.y)
		var conifer_share := _texel(_essence_bytes, _essence_size, map_data.size, x, y, _essence_channels.z)
		var total := oak_share + beech_share + conifer_share
		if total > 0.01:
			result["conifer"] = conifer_share / total
			result["beech"] = beech_share / maxf(oak_share + beech_share, 0.001)
	result["hedge"] = hedge_at(x, y)
	return result


## Densité de haies (0..1) : canal B de l'occupation du sol, interpolé comme dans le shader ;
## repli sur le terrain de la province si l'image manque.
func hedge_at(x: float, y: float) -> float:
	if _landuse_bytes.is_empty():
		var index := map_data.province_index_at(x, y)
		return 1.0 if index > 0 and index < _hedge_terrain.size() and _hedge_terrain[index] == 1 else 0.0
	var fx := clampf(x / map_data.size.x * _landuse_size.x - 0.5, 0.0, _landuse_size.x - 1.001)
	var fy := clampf(y / map_data.size.y * _landuse_size.y - 0.5, 0.0, _landuse_size.y - 1.001)
	var i := int(fx)
	var j := int(fy)
	var tx := fx - i
	var ty := fy - j
	var w := _landuse_size.x
	var b00 := _landuse_bytes[(j * w + i) * 4 + 2]
	var b10 := _landuse_bytes[(j * w + i + 1) * 4 + 2]
	var b01 := _landuse_bytes[((j + 1) * w + i) * 4 + 2]
	var b11 := _landuse_bytes[((j + 1) * w + i + 1) * 4 + 2]
	return lerpf(lerpf(b00, b10, tx), lerpf(b01, b11, tx), ty) / 255.0
