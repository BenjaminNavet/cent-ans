class_name TownData
extends RefCounted

## Lot ZG6 (ADR 0036) : lecture de `data/map/towns_1340.json` (généré par `cent-ans geo towns`),
## emprise vers 1340 des villes ordinaires. Lecture seule, rendu seulement.


## id de colonie → entrée (voir `data/schemas/towns_1340.schema.json`).
var towns: Dictionary = {}
## Paramètres communs : `{"plan": ..., "walls": ...}` (entrée de `TownPlan.generate`).
var params: Dictionary = {}
var meters_per_unit: float = 719.0


static func load_from(map_dir: String) -> TownData:
	var result := TownData.new()
	var path := map_dir.path_join("towns_1340.json")
	if not FileAccess.file_exists(path):
		return result
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		return result
	result.towns = parsed.get("towns", {})
	result.params = {"plan": parsed.get("plan", {}), "walls": parsed.get("walls", {})}
	result.meters_per_unit = float(parsed.get("meters_per_unit", 719.0))
	return result


func has_town(id: String) -> bool:
	return towns.has(id)


## Ancrage (unités monde) de la ville.
func anchor_of(id: String) -> Vector2:
	var px: Array = towns[id]["px"]
	return Vector2(float(px[0]), float(px[1]))


## Rayon (unités monde) de ce que la ville couvre : enceinte, faubourgs, moulin.
func extent_units(id: String) -> float:
	var town: Dictionary = towns[id]
	var r := 0.0
	for v in town["radii"]:
		r = maxf(r, float(v))
	for f in town["faubourgs"]:
		r = maxf(r, float(f["start_m"]) + float(f["length_m"]) + 140.0)
	for m in town["monuments"]:
		r = maxf(r, Vector2(float(m["at"][0]), float(m["at"][1])).length() + float(m["size_m"]))
	return (r + 80.0) / meters_per_unit


## Cercles de finage (x, y, rayon en unités monde) de toutes les villes, pour le parcellaire du
## lot ZG5b : ce qui est à l'intérieur est le finage (champs) de la ville.
func finage_zones() -> PackedVector3Array:
	var out := PackedVector3Array()
	for id in towns:
		var a := anchor_of(id)
		out.append(Vector3(a.x, a.y, float(towns[id]["finage_radius_m"]) / meters_per_unit))
	return out
