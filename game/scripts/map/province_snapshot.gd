class_name ProvinceSnapshot
extends RefCounted

## Lot PB3d : état des provinces lu en un seul appel (`CampaignSim.get_provinces_snapshot`,
## tableaux groupés) pour les calques de la carte — couleurs politiques, frontières, parchemin,
## hameaux, vie des campagnes — au lieu d'un `get_province_state` complet (garnison, gouverneur)
## par province et par boucle. Un instantané est partagé par tous les calques d'un même
## `refresh_all` (voir `invalidate`). Sans l'appel groupé (simulation factice des tests), il se
## remplit par `get_province_state`.

## Identifiants des provinces, dans l'ordre des index raster (index - 1).
var ids := PackedStringArray()
var known := PackedByteArray()
var owner := PackedStringArray()
var controller := PackedStringArray()
var devastation := PackedInt32Array()
var population_total := PackedInt64Array()
var besieged := PackedByteArray()
var _index_of: Dictionary = {}  # id → index raster - 1

static var _cached: ProvinceSnapshot = null
static var _cached_key: Array = []
static var _ids_cache: Dictionary = {}  # instance MapData → PackedStringArray


## Instantané courant pour `sim` et `map_data` : relu quand l'état de la simulation a changé
## (`CampaignSim.get_state_revision`, augmenté par tout ordre, fin de tour ou chargement) ; sans ce
## compteur (simulation factice), au plus une fois par image. `invalidate` force la relecture.
static func of(sim: Object, map_data: MapData) -> ProvinceSnapshot:
	var stamp: int = int(sim.call("get_state_revision")) if sim != null and sim.has_method("get_state_revision") else -Engine.get_process_frames() - 1
	var key := [sim.get_instance_id() if sim != null else 0, map_data.get_instance_id() if map_data != null else 0, stamp]
	if _cached != null and key == _cached_key:
		return _cached
	_cached = read(sim, province_ids(map_data))
	_cached_key = key
	return _cached


## À appeler après tout changement d'état de la simulation (début de `refresh_all`).
static func invalidate() -> void:
	_cached = null
	_cached_key = []


## Identifiants des provinces de `map_data` par index raster - 1 (mis en cache).
static func province_ids(map_data: MapData) -> PackedStringArray:
	if map_data == null:
		return PackedStringArray()
	var cache_key := map_data.get_instance_id()
	if _ids_cache.has(cache_key):
		return _ids_cache[cache_key]
	var out := PackedStringArray()
	out.resize(map_data.province_count)
	for index in range(1, map_data.province_count + 1):
		out[index - 1] = str(map_data.get_province(index).get("id", ""))
	_ids_cache[cache_key] = out
	return out


## Lit l'état des provinces `province_ids` (dans cet ordre).
static func read(sim: Object, province_ids: PackedStringArray) -> ProvinceSnapshot:
	var snap := ProvinceSnapshot.new()
	snap.ids = province_ids
	for i in province_ids.size():
		snap._index_of[province_ids[i]] = i
	var count := province_ids.size()
	if sim != null and sim.has_method("get_provinces_snapshot"):
		var data: Dictionary = sim.call("get_provinces_snapshot", province_ids)
		snap.known = data.get("known", PackedByteArray())
		snap.owner = data.get("owner", PackedStringArray())
		snap.controller = data.get("controller", PackedStringArray())
		snap.devastation = data.get("devastation", PackedInt32Array())
		snap.population_total = data.get("population_total", PackedInt64Array())
		snap.besieged = data.get("besieged", PackedByteArray())
		if snap.known.size() == count:
			return snap
	snap.known.resize(count)
	snap.owner.resize(count)
	snap.controller.resize(count)
	snap.devastation.resize(count)
	snap.population_total.resize(count)
	snap.besieged.resize(count)
	snap.known.fill(0)
	snap.devastation.fill(0)
	snap.population_total.fill(0)
	snap.besieged.fill(0)
	if sim == null or not sim.has_method("get_province_state"):
		return snap
	for i in count:
		var state: Dictionary = sim.call("get_province_state", province_ids[i])
		if state.is_empty():
			continue
		snap.known[i] = 1
		snap.owner[i] = str(state.get("owner", ""))
		snap.controller[i] = str(state.get("controller", ""))
		snap.devastation[i] = int(state.get("devastation", 0))
		snap.population_total[i] = int(state.get("population_total", 0))
		snap.besieged[i] = 1 if state.has("siege") else 0
	return snap


## Index (raster - 1) de la province `id`, ou -1.
func index_of(id: String) -> int:
	return int(_index_of.get(id, -1))


## Vrai si la simulation connaît la province d'index `i`.
func has(i: int) -> bool:
	return i >= 0 and i < known.size() and known[i] != 0
