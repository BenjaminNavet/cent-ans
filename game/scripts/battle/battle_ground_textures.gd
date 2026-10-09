class_name BattleGroundTextures
extends RefCounted

## TX T2c (ADR 0240) : sols de bataille régionaux générés. Un paquet par biome (13 couches, une par
## rôle de `data/fx/battle_ground_layers.json`, dans l'ordre fixe du shader `battle_ground`) ; seul le
## paquet du biome du lieu est chargé. Repli : biome, parent (`BiomeParents`), biome par défaut, puis
## `{}` (l'appelant retombe sur le jeu Poly Haven). Jamais de plantage : un paquet absent, non
## importé ou incohérent est ignoré avec un avertissement. Rendu seulement, aucune règle de jeu.
##
## Qualité : « haute » préfère la variante 2k locale (`hi/`, manifeste `*_2048.json`) si elle est
## importée, sinon le paquet 1k du dépôt (`TextureQuality`).

const FILE := "fx/battle_ground_layers.json"
const PROVINCES_FILE := "fx/battle_province_biomes.json"
const BIOME_FLAG := "--battle-biome"
const HI_SUFFIX := "_2048"


static func spec() -> Dictionary:
	var parsed: Variant = DataFile.load_cached(FILE) if DataFile.exists(FILE) else null
	return parsed if parsed is Dictionary else {}


## Rôles dans l'ordre des couches du tableau.
static func roles() -> Array:
	return spec().get("roles", [])


## Biome (1..14) d'une bataille : option `--battle-biome N` (captures), sinon celui de la province
## (`battle_province_biomes.json`), sinon celui du terrain de simulation, sinon le biome par défaut.
static func biome_for(province_id: String, terrain_key: String) -> int:
	var forced := CmdArgs.value(BIOME_FLAG, "")
	if forced.is_valid_int():
		return clampi(int(forced), 1, BiomeParents.COUNT - 1)
	var data := spec()
	if province_id != "" and DataFile.exists(PROVINCES_FILE):
		var table: Variant = DataFile.load_cached(PROVINCES_FILE)
		if table is Dictionary:
			var biome := int(((table as Dictionary).get("provinces", {}) as Dictionary).get(province_id, 0))
			if biome > 0:
				return biome
	var by_terrain: Dictionary = data.get("terrain_biomes", {})
	return int(by_terrain.get(terrain_key, data.get("default_biome", 2)))


## Biome, ses parents, puis le biome par défaut, sans doublon.
static func fallback_chain(biome: int) -> Array[int]:
	var chain: Array[int] = []
	var current := clampi(biome, 1, BiomeParents.COUNT - 1)
	for _i in 4:
		if current <= 0 or chain.has(current):
			break
		chain.append(current)
		current = BiomeParents.parent_of(current)
	var fallback := int(spec().get("default_biome", 2))
	if not chain.has(fallback):
		chain.append(fallback)
	return chain


## Matière du rôle pour le biome (repli par la chaîne) ; "" si aucune.
static func material_id(role: String, biome: int) -> String:
	var by_biome: Dictionary = (spec().get("materials", {}) as Dictionary).get(role, {})
	for candidate in fallback_chain(biome):
		var id := str(by_biome.get(str(candidate), ""))
		if id != "":
			return id
	return ""


## Paquet du biome : {"biome", "albedo", "normal", "layers": [{id, tile_size_m, role}]} ou {}.
static func pack_for(biome: int) -> Dictionary:
	for candidate in fallback_chain(biome):
		var pack := _load_pack(candidate)
		if not pack.is_empty():
			return pack
	push_warning("BattleGroundTextures: aucun paquet pour le biome %d, repli Poly Haven" % biome)
	return {}


static func _load_pack(biome: int) -> Dictionary:
	var rel := str(spec().get("pack_manifest", "")).replace("{biome}", "%02d" % biome)
	var manifest := _manifest(rel)
	if manifest.is_empty():
		return {}
	var role_list := roles()
	var layers_in: Array = manifest.get("layers", [])
	if layers_in.size() != role_list.size():
		push_warning("BattleGroundTextures: %s a %d couches, %d attendues" % [rel, layers_in.size(), role_list.size()])
		return {}
	var albedo := load(str(manifest["albedo"])) as TextureLayered
	var normal := load(str(manifest["normal"])) as TextureLayered
	if albedo == null or normal == null or albedo.get_layers() != role_list.size() or normal.get_layers() != role_list.size():
		push_warning("BattleGroundTextures: tableaux de %s absents ou invalides" % rel)
		return {}
	var layers: Array = []
	for i in role_list.size():
		var entry: Dictionary = layers_in[i]
		var expected := material_id(str(role_list[i]), biome)
		if expected != str(entry.get("id", "")) or int(entry.get("layer", -1)) != i:
			push_warning("BattleGroundTextures: %s couche %d = %s, attendu %s" % [rel, i, entry.get("id", ""), expected])
			return {}
		layers.append({"id": expected, "tile_size_m": float(entry.get("tile_m", 5.0)), "role": role_list[i]})
	return {"biome": biome, "albedo": albedo, "normal": normal, "layers": layers}


## Manifeste de `rel` (sous `data/`), variante 2k si la qualité est haute et que ses tableaux sont
## importés ; {} s'il manque ou si ses tableaux ne sont pas importés.
static func _manifest(rel: String) -> Dictionary:
	var candidates: Array[String] = []
	if TextureQuality.is_high():
		candidates.append(rel.get_basename() + HI_SUFFIX + "." + rel.get_extension())
	candidates.append(rel)
	for candidate in candidates:
		if not DataFile.exists(candidate):
			continue
		var parsed: Variant = DataFile.load_cached(candidate)
		if not (parsed is Dictionary):
			continue
		var doc: Dictionary = parsed
		if ResourceLoader.exists(str(doc.get("albedo", ""))) and ResourceLoader.exists(str(doc.get("normal", ""))):
			return doc
	return {}


## Grain fin du sol (paquet `micro_battle`) : {"albedo", "normal", "layer": PackedFloat32Array,
## "size": PackedFloat32Array} (une entrée par couche de sol, valeurs lues par le shader) ou {}.
static func micro_for(biome: int, ground_layers: Array) -> Dictionary:
	var micro: Dictionary = spec().get("micro", {})
	if micro.is_empty():
		return {}
	var manifest := _manifest(str(micro.get("manifest", "")))
	if manifest.is_empty():
		return {}
	var albedo := load(str(manifest["albedo"])) as TextureLayered
	var normal := load(str(manifest["normal"])) as TextureLayered
	if albedo == null or normal == null:
		return {}
	var index_of := {}
	for entry in manifest.get("layers", []):
		index_of[str((entry as Dictionary)["id"])] = entry
	var dry: bool = (micro.get("dry_biomes", []) as Array).has(biome)
	var layer_index := PackedFloat32Array()
	var tile_size := PackedFloat32Array()
	for ground in ground_layers:
		var role := str((ground as Dictionary).get("role", ""))
		var grain := str((micro.get("role_grain", {}) as Dictionary).get(role, micro.get("default", "")))
		if dry:
			grain = str((micro.get("dry_role_grain", {}) as Dictionary).get(role, grain))
		var layer: Dictionary = index_of.get(grain, index_of.get(str(micro.get("default", "")), {}))
		layer_index.append(float(layer.get("layer", 0)))
		tile_size.append(float(layer.get("tile_m", 0.5)))
	return {"albedo": albedo, "normal": normal, "layer": layer_index, "size": tile_size}
