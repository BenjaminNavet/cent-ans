class_name WaterDetail
extends RefCounted

## Lot RC5 (ADR 0141) : branche les textures de détail d'eau générées (mer, océan, fleuve,
## rivière) sur un `ShaderMaterial` qui inclut `res://shaders/water_detail.gdshaderinc`.
## Réglages (force, échelle, écoulement) dans `data/fx/water_detail.json`, jamais codés en dur.
## Sans texture (`<id>_normal.png` absente) ou sans données : `water_detail_strength` reste à 0
## et le shader garde son rendu procédural.

const SPEC_FILE := "fx/water_detail.json"
const TEXTURE_DIR := "res://assets/textures/water/"
## TX (ADR 0241) : surfaces d'eau générées par biome (`<id>_albedo.jpg` / `<id>_normal.jpg`, 2k, paquet
## `data/art/tx_water_pack.json`). Une matière de `water_detail.json` peut porter `tx` (id de la surface
## générée qui remplace sa texture 512 px) ; sans fichier (surface lisse restée procédurale, paquet
## absent, `--legacy-textures`) la texture d'avant reste.
const SEA_ARRAY_SIZE := 1024

static var _lookup := JsonLookup.new(SPEC_FILE, {}, "materials")


## Données RC5 (dossier de données du jeu, puis `data/` du dépôt) ; {} si introuvables.
static func spec() -> Dictionary:
	return _lookup.data()


## Applique la matière `id` à `material` (uniformes `<prefix>_normal`, `_albedo`, `_scale`,
## `_strength` ; `ocean_detail` pour le large de la mer) ; renvoie true si le détail est actif.
## Renvoie false (force laissée à 0, repli procédural) si la texture ou les données manquent.
static func apply(material: ShaderMaterial, id: String, prefix: String = "water_detail") -> bool:
	if material == null:
		return false
	material.set_shader_parameter(prefix + "_strength", 0.0)
	var entry: Variant = spec().get(id)
	if not entry is Dictionary:
		return false
	var paths := _paths(id, entry)
	var normal_path: String = paths[0]
	if not ResourceLoader.exists(normal_path):
		return false
	var normal := load(normal_path) as Texture2D
	if normal == null:
		return false
	material.set_shader_parameter(prefix + "_normal", normal)
	var albedo_path: String = paths[1]
	if ResourceLoader.exists(albedo_path):
		material.set_shader_parameter(prefix + "_albedo", load(albedo_path) as Texture2D)
	material.set_shader_parameter(prefix + "_scale", float(entry.get("scale", 1.0)))
	material.set_shader_parameter(prefix + "_strength", float(entry.get("strength", 0.0)))
	return true


## Vitesse d'écoulement de la normale de détail (tuiles par seconde) de la matière `id`, 0 sinon.
static func flow_speed(id: String) -> float:
	var entry: Variant = spec().get(id)
	return float((entry as Dictionary).get("flow_speed", 0.0)) if entry is Dictionary else 0.0


## Turbidité (0 clair, 1 limoneux) de la matière `id` : uniforme `turbidity` des shaders de fleuve.
static func turbidity(id: String) -> float:
	var entry: Variant = spec().get(id)
	return float((entry as Dictionary).get("turbidity", 0.0)) if entry is Dictionary else 0.0


## [normale, albédo] de la matière `id` : surface générée (`tx`) si ses fichiers existent, sinon les PNG d'avant.
static func _paths(id: String, entry: Dictionary) -> Array:
	var tx := str(entry.get("tx", ""))
	if tx != "" and TextureQuality.use_tx():
		var normal := TEXTURE_DIR + tx + "_normal.jpg"
		if ResourceLoader.exists(normal):
			return [normal, TEXTURE_DIR + tx + "_albedo.jpg"]
	return [TEXTURE_DIR + id + "_normal.png", TEXTURE_DIR + id + "_albedo.png"]


## Image 1024 px (mipmaps) d'une texture importée, décompressée ; null si absente.
static func _image(path: String) -> Image:
	var texture := load(path) as Texture2D if ResourceLoader.exists(path) else null
	if texture == null:
		return null
	var image := texture.get_image()
	if image.is_compressed():
		image.decompress()
	image.convert(Image.FORMAT_RGBA8)
	if image.get_width() != SEA_ARRAY_SIZE:
		image.resize(SEA_ARRAY_SIZE, SEA_ARRAY_SIZE, Image.INTERPOLATE_LANCZOS)
	return image


## TX : détail de la mer par bassin. `basins` de `water_detail.json` associe l'emplacement de
## `SeaBasins` ("default", puis l'id de chaque bassin) à une matière (`tx` -> surface générée) ;
## les surfaces sont réunies en deux tableaux de couches (normales, albédos) lus par `water.gdshader`
## (`sea_basin_params[emplacement]` = couche, force, échelle). Renvoie vrai si au moins un
## emplacement a une surface générée ; sinon le détail commun reste.
static func apply_sea_basins(material: ShaderMaterial, basin_ids: Array) -> bool:
	if material == null or not TextureQuality.use_tx():
		return false
	var document: Variant = DataFile.read_json(SPEC_FILE) if DataFile.exists(SPEC_FILE) else {}
	var wanted: Dictionary = (document as Dictionary).get("basins", {}) if document is Dictionary else {}
	var slots: Array = ["default"]
	slots.append_array(basin_ids)
	var layers: Dictionary = {}  # id de surface -> couche
	var normals: Array[Image] = []
	var albedos: Array[Image] = []
	var params := PackedVector4Array()
	for slot_index in 5:
		var params_entry := Vector4(-1.0, 0.0, 1.0, 0.0)
		var slot: String = slots[slot_index] if slot_index < slots.size() else ""
		var id := str(wanted.get(slot, wanted.get("default", "")))
		var entry: Variant = spec().get(id)
		if slot != "" and entry is Dictionary and str((entry as Dictionary).get("tx", "")) != "":
			var tx := str((entry as Dictionary)["tx"])
			if not layers.has(tx):
				var normal := _image(TEXTURE_DIR + tx + "_normal.jpg")
				var albedo := _image(TEXTURE_DIR + tx + "_albedo.jpg")
				if normal != null and albedo != null:
					layers[tx] = normals.size()
					normals.append(normal)
					albedos.append(albedo)
			if layers.has(tx):
				params_entry = Vector4(float(layers[tx]), float((entry as Dictionary).get("strength", 0.3)), float((entry as Dictionary).get("scale", 2.0)), 0.0)
		params.append(params_entry)
	if normals.is_empty():
		return false
	var normal_array := Texture2DArray.new()
	var albedo_array := Texture2DArray.new()
	for image in normals:
		image.generate_mipmaps()
	for image in albedos:
		image.generate_mipmaps()
	normal_array.create_from_images(normals)
	albedo_array.create_from_images(albedos)
	material.set_shader_parameter("sea_basin_normal", normal_array)
	material.set_shader_parameter("sea_basin_albedo", albedo_array)
	material.set_shader_parameter("sea_basin_params", params)
	material.set_shader_parameter("sea_basin_detail_on", true)
	return true
