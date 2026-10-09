class_name ForestStands
extends RefCounted

## Lot DN-FORET (ADR 0219) : peuplements forestiers de la carte de campagne.
##
## `data/art/forest_stands.json` décrit ce dont est faite chaque forêt (mélange d'essences, teinte,
## densité, hauteur) ; les massifs nommés de `data/map/historical_forests.json` reçoivent leur
## peuplement, ailleurs une cellule d'écorégion tire parmi les peuplements admissibles au lieu.
## Ce fichier ne fait qu'aplatir les données pour le semis natif (`VegetationScatter.set_species`,
## clé `stands`) ; la règle de choix est en Rust (`core/crates/vegetation/src/stands.rs`).
## Rendu seulement, aucune règle de jeu.

const DATA_FILE := "art/forest_stands.json"
const FORESTS_FILE := "map/historical_forests.json"


## Table aplatie pour le Rust, ou {} (données absentes, invalides ou carte sans projection).
static func table(species: TreeSpecies, map_data: MapData) -> Dictionary:
	if species == null or not species.ok or map_data == null or not DataFile.exists(DATA_FILE):
		return {}
	var data: Variant = DataFile.load_cached(DATA_FILE)
	if not (data is Dictionary) or map_data.bounds_projected.size() < 4 or map_data.meters_per_px <= 0.0:
		return {}
	var list: Array = (data as Dictionary).get("stands", [])
	if list.is_empty():
		return {}
	var dist: Dictionary = (data as Dictionary).get("distribution", {})
	var other := float(dist.get("other_mult", 0.05))
	var n := list.size()
	var count := species.count
	var mult := PackedFloat32Array()
	mult.resize(n * count)
	var tint := PackedFloat32Array()
	var density := PackedFloat32Array()
	var height := PackedFloat32Array()
	var biome_mask := PackedInt32Array()
	var altitude := PackedFloat32Array()
	var lat := PackedFloat32Array()
	var lon := PackedFloat32Array()
	var conifer := PackedFloat32Array()
	var weight := PackedFloat32Array()
	var index := {}
	for i in n:
		var st: Dictionary = list[i]
		index[str(st.get("id", ""))] = i
		var mix: Dictionary = st.get("species", {})
		for s in count:
			mult[i * count + s] = float(mix.get(species.ids[s], other))
		var t: Array = st.get("tint", [1.0, 1.0, 1.0])
		tint.append_array(PackedFloat32Array([float(t[0]), float(t[1]), float(t[2])]))
		density.append(float(st.get("density", 1.0)))
		height.append(float(st.get("height", 1.0)))
		var mask := 0
		for b: Variant in st.get("biomes", []):
			mask |= 1 << int(b)
		biome_mask.append(mask)
		_append_range(altitude, st.get("altitude_m", [0.0, 9000.0]))
		_append_range(lat, st.get("lat", [-90.0, 90.0]))
		_append_range(lon, st.get("lon", [-180.0, 180.0]))
		_append_range(conifer, st.get("conifer", [0.0, 1.0]))
		weight.append(float(st.get("weight", 1.0)))
	var regions := PackedFloat32Array()
	var mpp := map_data.meters_per_px
	var areas := _historical_areas()
	for r: Dictionary in (data as Dictionary).get("regions", []):
		var stand_index: int = index.get(str(r.get("stand", "")), -1)
		if stand_index < 0:
			continue
		var ellipse: Dictionary = r.get("ellipse", {})
		if r.has("area"):
			ellipse = areas.get(str(r["area"]), {})
		if ellipse.is_empty():
			continue
		var c: Array = ellipse["center"]
		var radii: Array = ellipse["radii_km"]
		var centre := FaunaLayer.lonlat_to_px(float(c[0]), float(c[1]), map_data)
		var rx := float(radii[0]) * 1000.0 / mpp
		var ry := float(radii[1]) * 1000.0 / mpp
		# Angle antihoraire depuis l'est sur la carte « nord en haut » ; y pixel vers le sud.
		var angle := -deg_to_rad(float(ellipse.get("angle_deg", 0.0)))
		var priority := float(r.get("priority", 0.0)) + 1000.0 / maxf(rx * ry, 1.0)
		regions.append_array(PackedFloat32Array([centre.x, centre.y, rx, ry, angle, float(stand_index), priority]))
	var bounds: Array = map_data.bounds_projected
	return {
		"count": n, "species_count": count, "mult": mult, "tint": tint, "density": density,
		"height": height, "biome_mask": biome_mask, "altitude": altitude, "lat": lat, "lon": lon,
		"conifer": conifer, "weight": weight, "regions": regions,
		"cell_px": float(dist.get("cell_px", 36.0)), "jitter": float(dist.get("jitter", 0.8)),
		"min_x_m": float(bounds[0]), "max_y_m": float(bounds[3]), "m_per_px": mpp,
	}


static func _append_range(target: PackedFloat32Array, range_v: Array) -> void:
	target.append(float(range_v[0]))
	target.append(float(range_v[1]))


## `id -> ellipse` des massifs nommés (`historical_forests.json`).
static func _historical_areas() -> Dictionary:
	var result := {}
	if not DataFile.exists(FORESTS_FILE):
		return result
	var data: Variant = DataFile.load_cached(FORESTS_FILE)
	if data is Dictionary:
		for a: Dictionary in (data as Dictionary).get("areas", []):
			result[str(a.get("id", ""))] = a.get("ellipse", {})
	return result
