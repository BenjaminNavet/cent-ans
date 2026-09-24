class_name SettlementGrowth
extends RefCounted

## Lot CV1 : colonies qui grandissent avec leur niveau (rendu seulement). Niveau visuel :
## 0 village, 1 bourg (ouvert, halle), 2 ville (murailles), 3 cité (cathédrale, halle,
## château, faubourgs). Calculé depuis le type, les bâtiments et la fortification
## (`CampaignSim.settlement_detail`) et la population de la province (`get_province_state`) :
## - village → bourg : marché bâti ou province très peuplée ;
## - bourg → ville : murailles (`bld_stone_walls`, `bld_palisade`) ou fortification ≥ 2 ;
## - ville → cité : cité épiscopale (cathédrale) ou capitale de province très peuplée ;
## - cité : château de Kenney Castle Kit (CC0) en plus si fortification ≥ 6.
## Châteaux et abbayes gardent leur maquette. Paris (monument dédié, lot L1) est exclu.

const LEVEL_NAMES: Array[String] = ["village", "bourg", "ville", "cité"]
## Colonies exclues de la croissance générique (monuments dédiés, lot L1).
const EXCLUDED: Array[String] = ["set_paris"]
## Maquettes par niveau (`game/assets/models/settlements/`).
const LEVEL_MODELS := [["village_a", "village_b"], ["bourg_a", "bourg_b"], ["town_a", "town_b"], ["cite_a", "cite_b"]]
## Échelle monde par niveau (unités Blender → pixels carte), cf. `ModelLibrary.SETTLEMENT_SCALE`.
const LEVEL_SCALE := [3.8, 4.1, 4.4, 4.6]
const WALL_BUILDINGS: Array[String] = ["bld_stone_walls", "bld_palisade", "bld_walls"]
## Population de province (habitants) au-delà de laquelle un village devient bourg, une ville cité.
const BOURG_POPULATION := 420000.0
const CITE_POPULATION := 600000.0
const CASTLE_FORTIFICATION := 6
## Pièces Kenney du château (tour carrée + toit, tours hexagonales).
const KENNEY_DIR := "res://assets/third_party/buildings/kenney_castle_kit/"
const KENNEY_TINT := Color(0.5, 0.5, 0.5)


## Niveau visuel d'une colonie, -1 si elle n'en a pas (château, abbaye) ou est exclue.
## `detail` : `settlement_detail` (buildings, fortification_level, is_city).
static func level_of(entry: Dictionary, detail: Dictionary, province_population: float) -> int:
	if EXCLUDED.has(str(entry.get("id", ""))):
		return -1
	var kind := str(entry.get("kind", ""))
	var buildings: Array = detail.get("buildings", [])
	var fortification := int(detail.get("fortification_level", entry.get("fortification_level", 0)))
	var walled := fortification >= 2
	for building in WALL_BUILDINGS:
		if buildings.has(building):
			walled = true
	match kind:
		"village":
			return 1 if buildings.has("bld_market") or province_population >= BOURG_POPULATION else 0
		"town":
			return 2 if walled else 1
		"city":
			if buildings.has("bld_cathedral") or (bool(detail.get("is_city", false)) and province_population >= CITE_POPULATION):
				return 3
			return 2 if walled else 1
	return -1


## Château (Kenney) à côté de la cité.
static func has_castle(detail: Dictionary) -> bool:
	return int(detail.get("fortification_level", 0)) >= CASTLE_FORTIFICATION


## Maquette à l'échelle monde pour un niveau (variante déterministe), château éventuel ;
## null si les modèles manquent.
static func build_model(level: int, variant_seed: int, castle: bool) -> Node3D:
	if level < 0 or level >= LEVEL_MODELS.size():
		return null
	var variants: Array = LEVEL_MODELS[level]
	var name := str(variants[absi(variant_seed) % variants.size()])
	var model := ModelLibrary.instantiate("settlements/" + name, LEVEL_SCALE[level])
	if model == null:
		return null
	var root := Node3D.new()
	root.name = "CV1_%s" % name
	root.add_child(model)
	if castle:
		var chateau := kenney_castle()
		if chateau != null:
			var angle := float(absi(variant_seed / 3) % 628) / 100.0
			chateau.position = Vector3(cos(angle), 0.0, sin(angle)) * LEVEL_SCALE[level] * 1.65
			chateau.rotation.y = angle
			root.add_child(chateau)
	return root


## Château de Kenney Castle Kit : donjon carré coiffé, deux tours hexagonales, courtine ;
## teinté pierre (texture d'origine conservée pour les toits bleus → ardoise).
static func kenney_castle() -> Node3D:
	var pieces := [
		["tower-square", Vector3(0, 0, 0)],
		["tower-square-top-roof-high", Vector3(0, 1, 0)],
		["tower-hexagon-base", Vector3(1.4, 0, 0.9)],
		["tower-hexagon-roof", Vector3(1.4, 1, 0.9)],
		["tower-hexagon-base", Vector3(-1.3, 0, 1.0)],
		["tower-hexagon-roof", Vector3(-1.3, 1, 1.0)],
		["wall", Vector3(0.05, 0, 1.1)],
	]
	var root := Node3D.new()
	root.name = "KenneyCastle"
	for piece in pieces:
		var path := KENNEY_DIR + str(piece[0]) + ".glb"
		if not ResourceLoader.exists(path):
			return null
		var node := (load(path) as PackedScene).instantiate() as Node3D
		node.position = piece[1]
		root.add_child(node)
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		# Une seule matière (texture commune du kit) : surcharge d'instance, pas de surface.
		var base := mesh_instance.mesh.surface_get_material(0) as StandardMaterial3D if mesh_instance.mesh != null else null
		if base == null:
			continue
		var tinted := base.duplicate() as StandardMaterial3D
		tinted.albedo_color = KENNEY_TINT
		mesh_instance.material_override = tinted
	root.scale = Vector3.ONE * 0.85
	return root
