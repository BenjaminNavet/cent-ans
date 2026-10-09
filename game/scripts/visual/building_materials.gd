class_name BuildingMaterials
extends RefCounted

## Matériaux PBR partagés des bâtiments du kit Blender
## (`tools/blender_scripts/building_kit.py`). Les GLB du kit ne portent que des matériaux
## *nommés* (`Plaster`, `Rubble`, `RoofTile`…), des UV en mètres réels et des couleurs de sommet
## (teinte du bâtiment × occlusion cuite × patine) ; ce script remplace chaque surface par un
## matériau texturé commun (textures Poly Haven CC0), albédo × couleur de sommet. Variantes :
## `snow` (toits enneigés) et `far` (campagne : sans carte normale, moins de lectures de texture).
## Purement visuel. Les teintes par matériau reprennent `MATERIAL_TINT` de `kit_export.py`.
##
## `SPECS`/`PLAIN`/`ROOFS`/`ATLAS_LAYERS` viennent de `data/art/building_materials.json`
## (schéma `art_building_materials.schema.json`), jamais codés en dur ici — même repli que
## `BattleTerrain.ground_layers()`.
##
## Panneaux des murs à pans de bois du kit Blender, deux couches ajoutées en fin d'atlas
## (indices déjà bakés dans les `.glb` inchangés), texturées malgré leur place après les couches
## unies (`first_plain`..`plain_last` du shader) : `TimberFrame` (14, torchis clair sous les
## poutres modelées, niveau `high`) et `TimberFrameFar` (15, lattis peint, niveau `low` sans
## poteaux modelés). L'atlas est plein (16 couches).

const TEX := "res://assets/textures/"
const DATA_FILE := "art/building_materials.json"

## nom → [albédo, normale, rugosité ("" = scalaire), tuile (m), teinte, rugosité]
static var SPECS: Dictionary = {}
## Matériaux unis (sans texture) : couleur sRGB, rugosité.
static var PLAIN: Dictionary = {}
static var ROOFS: Array = []
## Toutes les matières du kit fusionnées en un matériau `Building` (`building_atlas.gdshader`),
## couche = alpha de la couleur de sommet : maquettes de campagne (variante `far`, sans normales)
## et bâtiments de bataille. Même ordre que `kit_export.ATLAS_LAYERS` et que les tranches de
## `building_albedo_array.jpg` / `building_normal_array.jpg`.
static var ATLAS_LAYERS: Array = []
const ATLAS_SHADER := preload("res://shaders/building_atlas.gdshader")
## Matériau partagé des matières nommées (usure procédurale, `building_aging`).
const PBR_SHADER := preload("res://shaders/building_pbr.gdshader")
## Intensité par défaut de l'usure des bâtiments (0 : pas d'usure).
const SR5_AGING := 0.5
## Mousse sur les faces au ciel par matière (défaut : `SR5_WALL_MOSS`, sommets de mur).
const SR5_MOSS := {"RoofTile": 1.0, "Thatch": 0.8, "RoofSlate": 0.5, "RoofFlat": 0.6}
const SR5_WALL_MOSS := 0.25
## Poids des salissures de mur (boue, coulures) ; 0 pour les toits.
const SR5_GRIME := {"Door": 0.5, "Timber": 0.7, "Planks": 0.8, "TimberFrame": 0.8, "TimberFrameFar": 0.8}

## TX T4 (ADR 0241) : matières régionales générées (tableau de couches, `tx_building_pack.json`).
const PACK_FILE := "art/tx_building_pack.json"
const MICRO_MASONRY := "res://assets/textures/buildings/tx_micro_masonry.jpg"
## Régions et couches de l'atlas des shaders (`building_regional.gdshaderinc`).
const REGION_CAP := 40
const REGIONAL_TILE_CAP := 96
## Micro-détail de maçonnerie de près : force, fondu entre `MICRO_NEAR` et `MICRO_FAR` mètres.
const MICRO_STRENGTH := 0.6
const MICRO_NEAR := 2.0
const MICRO_FAR := 12.0

static var _materials: Dictionary = {}  # "variante|nom|région" → Material
static var _meshes: Dictionary = {}  # "variante|id du maillage|région" → Mesh
static var _data_loaded: bool = false
static var _regional_loaded: bool = false
static var _regional: Dictionary = {}  # {"table", "tile", "albedo", "normal"} ; {} : inactif
## Région de la bataille en cours (`set_region`) : ses matières remplacent celles de l'atlas.
static var _region: String = ""
static var _settlement_region: Dictionary = {}  # id de colonie → indice de région (campagne)


## Charge `SPECS`/`PLAIN`/`ROOFS`/`ATLAS_LAYERS` depuis les données (dossier de données
## du jeu, puis `data/` du dépôt). Sans repli chiffré : si le fichier est introuvable ou invalide,
## les tables restent vides et `material()`/`_atlas()` ne trouvent aucune matière (avertissement).
static func _ensure_data() -> void:
	if _data_loaded:
		return
	_data_loaded = true
	var parsed: Variant = DataFile.read_json(DATA_FILE) if DataFile.exists(DATA_FILE) else null
	if parsed is Dictionary:
		var doc := parsed as Dictionary
		for entry_v in doc.get("textured", []):
			var entry := entry_v as Dictionary
			var name := str(entry["name"])
			var rough_map := str(entry.get("roughness_map", ""))
			var tint: Array = entry["tint"]
			SPECS[name] = [
				str(entry["diffuse"]),
				str(entry["normal"]),
				rough_map,
				float(entry["tile_size_m"]),
				Color(tint[0], tint[1], tint[2]),
				float(entry["roughness"]),
			]
			if bool(entry.get("roof", false)):
				ROOFS.append(name)
		for entry_v in doc.get("plain", []):
			var entry := entry_v as Dictionary
			var color: Array = entry["color"]
			PLAIN[str(entry["name"])] = [Color(color[0], color[1], color[2]), float(entry["roughness"])]
		var layers: Array = doc.get("atlas_layers", [])
		ATLAS_LAYERS = layers.duplicate()
		return
	push_warning("BuildingMaterials: %s introuvable, aucune matière chargée" % DATA_FILE)


static func clear_cache() -> void:
	_materials.clear()
	_meshes.clear()
	_regional_loaded = false
	_regional = {}


## TX T4 : région des bâtiments de la bataille en cours ("" : matières par défaut). Les matériaux
## et maillages déjà remappés pour une autre région sont oubliés.
static func set_region(region: String) -> void:
	if region == _region:
		return
	_region = region
	_materials.clear()
	_meshes.clear()


static func region() -> String:
	return _region


## Campagne : région (nom) de chaque colonie, posée par `SettlementLayer` ; les villes en tirent
## leur paramètre d'instance `town_region`.
static func set_settlement_regions(regions: Dictionary) -> void:
	var ids := BuildingRegions.region_ids()
	_settlement_region.clear()
	for id: String in regions:
		var index := ids.find(str(regions[id]))
		if index >= 0 and index < REGION_CAP:
			_settlement_region[id] = index


## Indice de région de la colonie `id` pour les shaders, -1 sans matières régionales.
static func settlement_region_index(id: String) -> int:
	_ensure_regional()
	return int(_settlement_region.get(id, -1)) if not _regional.is_empty() else -1


## Tables régionales prêtes (paquet présent, `--legacy-textures` absent).
static func regional_ready() -> bool:
	_ensure_regional()
	return not _regional.is_empty()


## Charge le paquet régional et bâtit `region_table` (région × couche de l'atlas → couche du
## tableau régional ou -1) et `regional_tile` (1 / taille de tuile par couche du tableau).
static func _ensure_regional() -> void:
	if _regional_loaded:
		return
	_regional_loaded = true
	_regional = {}
	_ensure_data()
	if not TextureQuality.use_tx():
		return
	var pack_file := PACK_FILE
	var hi := PACK_FILE.get_basename() + "_%d.json" % TextureQuality.HI_SIZE
	if TextureQuality.is_high() and DataFile.exists(hi):
		pack_file = hi
	var parsed: Variant = DataFile.read_json(pack_file) if DataFile.exists(pack_file) else null
	if not parsed is Dictionary:
		return
	var pack := parsed as Dictionary
	var layers: Dictionary = {}
	var tiles := PackedFloat32Array()
	tiles.resize(REGIONAL_TILE_CAP)
	tiles.fill(1.0)
	for layer_v in pack.get("layers", []):
		var layer := layer_v as Dictionary
		var index := int(layer["layer"])
		if index < REGIONAL_TILE_CAP:
			layers[str(layer["id"])] = index
			tiles[index] = 1.0 / maxf(float(layer["tile_m"]), 0.1)
	var albedo := load(TextureQuality.texture_path(str(pack["albedo"]))) as Texture
	var normal := load(TextureQuality.texture_path(str(pack["normal"]))) as Texture
	# Un import raté (tableau trop grand pour Godot) donne un tableau sans tranche : ancien rendu.
	if albedo == null or layers.is_empty() or (albedo is TextureLayered and (albedo as TextureLayered).get_layers() == 0):
		return
	var table := PackedInt32Array()
	table.resize(REGION_CAP * 16)
	table.fill(-1)
	var ids := BuildingRegions.region_ids()
	for r in mini(ids.size(), REGION_CAP):
		var wanted := BuildingRegions.materials_for_region(str(ids[r]))
		for role: String in wanted:
			if not layers.has(str(wanted[role])):
				continue
			# `TimberFrameFar` (lattis lointain) prend aussi le colombage régional.
			var targets: Array = [role, "TimberFrameFar"] if role == "TimberFrame" else [role]
			for atlas_role: String in targets:
				var slot: int = ATLAS_LAYERS.find(atlas_role)
				if slot >= 0 and slot < 16:
					table[r * 16 + slot] = int(layers[str(wanted[role])])
	_regional = {"table": table, "tile": tiles, "albedo": albedo, "normal": normal}


## Pose les tables régionales sur un matériau `building_atlas`/`town_building` ; `region_index` -1
## laisse le choix de la région à l'instance (villes) ou aux matières par défaut.
static func apply_regional(mat: ShaderMaterial, region_index: int = -1) -> void:
	_ensure_regional()
	if mat == null or _regional.is_empty():
		return
	mat.set_shader_parameter("use_regional", true)
	mat.set_shader_parameter("region_table", _regional["table"])
	mat.set_shader_parameter("regional_tile", _regional["tile"])
	mat.set_shader_parameter("regional_albedo", _regional["albedo"])
	mat.set_shader_parameter("regional_normal", _regional["normal"])
	mat.set_shader_parameter("region_id", region_index)
	if ResourceLoader.exists(MICRO_MASONRY):
		mat.set_shader_parameter("micro_masonry", load(MICRO_MASONRY))
		mat.set_shader_parameter("micro_fade", Vector3(MICRO_STRENGTH, MICRO_NEAR, MICRO_FAR))


## Ordre des couches de l'atlas `Building` (`ATLAS_LAYERS`, chargé depuis les données).
static func atlas_layers() -> Array:
	_ensure_data()
	return ATLAS_LAYERS


## Matériau partagé `name` (variante "", "snow" ou "far") ; null si le nom est inconnu.
static func material(name: String, variant: String = "") -> Material:
	_ensure_data()
	var key := variant + "|" + name + "|" + _region
	if _materials.has(key):
		return _materials[key]
	if name == "Building":
		var atlas := _atlas(variant)
		_materials[key] = atlas
		return atlas
	var mat: Material = null
	if SPECS.has(name):
		mat = _textured(name, variant)
	elif PLAIN.has(name):
		var plain := StandardMaterial3D.new()
		plain.albedo_color = PLAIN[name][0]
		plain.roughness = PLAIN[name][1]
		plain.vertex_color_use_as_albedo = name != "Window"
		if name == "Canvas":
			plain.cull_mode = BaseMaterial3D.CULL_DISABLED
		mat = plain
	if mat != null:
		mat.resource_name = name
	_materials[key] = mat
	return mat


static func _atlas(variant: String) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = ATLAS_SHADER
	mat.resource_name = "Building"
	mat.set_shader_parameter("albedo_array", load(TEX + "buildings/building_albedo_array.jpg"))
	var tints := PackedVector4Array()
	var tiles := PackedFloat32Array()
	for i in 16:
		var layer_name: String = ATLAS_LAYERS[i] if i < ATLAS_LAYERS.size() else ""
		if SPECS.has(layer_name):
			var spec: Array = SPECS[layer_name]
			var tint: Color = spec[4]
			tints.append(Vector4(tint.r, tint.g, tint.b, float(spec[5])))
			tiles.append(1.0 / float(spec[3]))
		elif PLAIN.has(layer_name):
			var color: Color = (PLAIN[layer_name][0] as Color).srgb_to_linear()
			tints.append(Vector4(color.r, color.g, color.b, float(PLAIN[layer_name][1])))
			tiles.append(1.0)
		else:
			tints.append(Vector4(1, 1, 1, 0.9))
			tiles.append(1.0)
	mat.set_shader_parameter("layer_tint", tints)
	mat.set_shader_parameter("layer_tile", tiles)
	# Les couches unies forment un bloc contigu (`first_plain`..`plain_last`) ; les
	# couches texturées ajoutées après lui (`TimberFrame`) gardent leur texture.
	var plain_first := -1
	var plain_last := -2
	for i in ATLAS_LAYERS.size():
		if PLAIN.has(ATLAS_LAYERS[i]):
			plain_first = i if plain_first < 0 else plain_first
			plain_last = i
	mat.set_shader_parameter("first_plain", plain_first if plain_first >= 0 else 16)
	mat.set_shader_parameter("plain_last", plain_last)
	mat.set_shader_parameter("roof_first", ATLAS_LAYERS.find("RoofTile"))
	mat.set_shader_parameter("roof_last", ATLAS_LAYERS.find("Thatch"))
	mat.set_shader_parameter("snow", 1.0 if variant == "snow" else 0.0)
	mat.set_shader_parameter("aging", SR5_AGING)
	if variant != "far":
		mat.set_shader_parameter("normal_array", load(TEX + "buildings/building_normal_array.jpg"))
		mat.set_shader_parameter("use_normals", true)
	if regional_ready():
		apply_regional(mat, BuildingRegions.region_ids().find(_region) if _region != "" else -1)
	return mat


## Même rendu que `_textured_standard` (mêmes textures, teinte, UV en mètres, neige)
## dans `building_pbr.gdshader`, plus l'usure procédurale.
static func _textured(name: String, variant: String) -> ShaderMaterial:
	var spec: Array = SPECS[name]
	var mat := ShaderMaterial.new()
	mat.shader = PBR_SHADER
	var roof: bool = name in ROOFS
	var snow := variant == "snow" and roof
	mat.set_shader_parameter("albedo_color", Color(0.93, 0.95, 1.0) if snow else spec[4])
	mat.set_shader_parameter("use_albedo_texture", not snow)
	if not snow:
		mat.set_shader_parameter("albedo_texture", load(TEX + str(spec[0]) + ".jpg"))
	var rough := 0.8 if snow else float(spec[5])
	if variant != "far":
		mat.set_shader_parameter("use_normal", true)
		mat.set_shader_parameter("normal_texture", load(TEX + str(spec[1]) + ".jpg"))
		mat.set_shader_parameter("normal_scale", 0.6 if snow else 1.0)
		if str(spec[2]) != "":
			mat.set_shader_parameter("roughness_texture", load(TEX + str(spec[2]) + ".jpg"))
			mat.set_shader_parameter("use_roughness_texture", true)
			rough = 1.0
	mat.set_shader_parameter("roughness", rough)
	var tile := 1.0 / float(spec[3])
	mat.set_shader_parameter("uv_scale", Vector2(tile, tile))
	mat.set_shader_parameter("aging", SR5_AGING)
	mat.set_shader_parameter("aging_grime", 0.0 if roof else float(SR5_GRIME.get(name, 1.0)))
	mat.set_shader_parameter("aging_moss", 0.0 if snow else float(SR5_MOSS.get(name, SR5_WALL_MOSS)))
	return mat


## Copie de `mesh` dont chaque surface reçoit le matériau partagé de même nom (mise en cache).
## Les surfaces au nom inconnu gardent leur matériau d'origine.
static func remap_mesh(mesh: Mesh, variant: String = "") -> Mesh:
	if mesh == null:
		return null
	var key := variant + "|" + str(mesh.get_instance_id()) + "|" + _region
	if _meshes.has(key):
		return _meshes[key]
	var copy := mesh.duplicate() as Mesh
	for i in copy.get_surface_count():
		var original := copy.surface_get_material(i)
		if original == null:
			continue
		var shared := material(_base_name(original.resource_name), variant)
		if shared != null:
			copy.surface_set_material(i, shared)
	_meshes[key] = copy
	return copy


## Remplace les maillages de toutes les `MeshInstance3D` de `root` (modèle instancié).
static func remap_node(root: Node, variant: String = "") -> void:
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		mesh_instance.mesh = remap_mesh(mesh_instance.mesh, variant)


## Blender suffixe les doublons (« Plaster.001 ») ; on ne garde que la racine.
static func _base_name(name: String) -> String:
	var dot := name.find(".")
	return name if dot < 0 else name.substr(0, dot)
