class_name FaunaLayer
extends Node3D

## Lot DN-ME2 : troupeaux et faune de la carte de campagne (rendu seulement, aucune règle de jeu).
##
## - Données : `data/map/map_fauna.json` (schéma `map_fauna.schema.json`) : espèces (ids des animaux
##   de `data/art/dn_catalog_map_extra.json`), zones géographiques (ellipses lon/lat) et densités de
##   troupeaux par région et par biome (splatmap : prairie, cultures, forêt, lande), pente, altitude,
##   proximité de la mer ou d'un fleuve, poids par saison (transhumance : alpage l'été, plaine
##   l'hiver). Brancher un nouveau modèle = déposer le glb ingéré (`dn/fauna/<id>_lod{0,1,2}.glb`)
##   ou éditer une entrée `species` (le glb prime sur `placeholder`).
## - Semis : par cellule de `cell_px` pixels, à la demande autour du point visé, déterministe
##   (graine = zone, espèce, cellule). Les troupeaux évitent l'eau (cercle d'errance entièrement sur
##   la terre), les colonies, les fleuves, les pentes fortes, hors du terrain propre à l'espèce.
## - Rendu : un `MultiMesh` par espèce et par cellule (un appel de dessin), jamais un nœud par bête.
##   Errance lente, pas, rebond et tête basse à l'arrêt dans `fauna.gdshader` (pas d'armature).
##   Taille tenue à l'écran (`length_k` × distance^`length_exponent`), jamais sous la taille réelle.
## - Visible sous `max_distance` (donc jamais sur le parchemin) ; amincissement par rang au-delà de
##   `max_visible_instances`.

const SHADER := preload("res://shaders/fauna.gdshader")
const DATA_FILE := "map/map_fauna.json"
const MODEL_DIR := "res://assets/models/"
const DN_DIR := "res://assets/models/dn/fauna/"
const FLOATS_PER_INSTANCE := 16
const MAX_PROFILES := 24
const SEASONS: Array[String] = ["spring", "summer", "autumn", "winter"]
const MAX_LODS := 3
const LAKE_GRID := 16.0
const WETLANDS_FILE := "wetlands.png"
const WETLANDS_SHRINK := 4

var config: Dictionary = {}
var enabled := true
## Affiche et construit quelle que soit la distance du rig (tests headless).
var force_active := false
var stats: Dictionary = {"cells": 0, "visible_cells": 0, "draw_calls": 0, "instances": 0, "visible": 0, "build_ms_max": 0.0}

var _map_data: MapData
var _mpp := 719.0
var _rig: Node3D
var _zones: Array = []  # {id, center: Vector2 (px), radius: Vector2 (px), species: Array}
var _profiles: Array[Vector4] = []
var _profile_index: Dictionary = {}  # clé "a|b|c|d" → indice
var _species: Dictionary = {}  # id → état de rendu (meshes, matériaux, échelle)
var _cells: Dictionary = {}  # Vector2i → {"nodes": Array, "count": int, "last_seen": int}
var _town_grid: Dictionary = {}  # Vector2i → PackedVector2Array
var _frame := 0
var _lod := -1
var _turn_phase := 0.0
## Eau affichée hors du masque de terre : lacs (polygones, index par case de `LAKE_GRID` px) et
## zones humides (`wetlands.png` réduite au quart).
var _lakes: Array = []  # {rect: Rect2, polygon: PackedVector2Array}
var _lake_grid: Dictionary = {}  # Vector2i → PackedInt32Array
var _wet: Image


## Dossier `data/` -> configuration (vide si le fichier manque).
static func load_config() -> Dictionary:
	var parsed: Variant = DataFile.read_json(DATA_FILE)
	return parsed if parsed is Dictionary else {}


## Branche la couche sur la carte. `towns` : positions (px carte) des colonies à éviter.
func setup(map_data: MapData, rig: Node3D = null, towns: PackedVector2Array = PackedVector2Array(), lake_polygons: Array = []) -> void:
	clear()
	_map_data = map_data
	_mpp = map_data.meters_per_px if map_data != null else 719.0
	_rig = rig
	config = load_config()
	_species.clear()
	_zones.clear()
	_profiles.clear()
	_profile_index.clear()
	_town_grid.clear()
	_lakes.clear()
	_lake_grid.clear()
	_wet = null
	if config.is_empty() or map_data == null:
		return
	_index_lakes(lake_polygons)
	if FileAccess.file_exists(map_data.map_dir.path_join(WETLANDS_FILE)):
		_wet = _load_wetlands(map_data.map_dir.path_join(WETLANDS_FILE))
	for town in towns:
		var key := Vector2i(floori(town.x / 8.0), floori(town.y / 8.0))
		var list: PackedVector2Array = _town_grid.get(key, PackedVector2Array())
		list.append(town)
		_town_grid[key] = list
	for zone: Dictionary in config.get("zones", []):
		var center := lonlat_to_px(float(zone["center"][0]), float(zone["center"][1]), map_data)
		var radius_km: Array = zone["radius_km"]
		var items: Array = []
		for item: Dictionary in zone["species"]:
			var entry := item.duplicate()
			var weights := _season_vector(item.get("seasons", {}))
			entry["profile"] = _profile_of(weights)
			items.append(entry)
		_zones.append({
			"id": str(zone["id"]), "center": center,
			"radius": Vector2(float(radius_km[0]), float(radius_km[1])) * 1000.0 / _mpp,
			"species": items,
		})


## Zones humides réduites (R marais, G étangs, B prés humides).
static func _load_wetlands(path: String) -> Image:
	var image := Image.load_from_file(path)
	if image == null:
		return null
	image.resize(maxi(image.get_width() / WETLANDS_SHRINK, 1), maxi(image.get_height() / WETLANDS_SHRINK, 1), Image.INTERPOLATE_BILINEAR)
	return image


func _index_lakes(polygons: Array) -> void:
	for polygon: PackedVector2Array in polygons:
		if polygon.size() < 3:
			continue
		var low := polygon[0]
		var high := polygon[0]
		for point in polygon:
			low = Vector2(minf(low.x, point.x), minf(low.y, point.y))
			high = Vector2(maxf(high.x, point.x), maxf(high.y, point.y))
		var index := _lakes.size()
		_lakes.append({"rect": Rect2(low, high - low), "polygon": polygon})
		for gy in range(floori(low.y / LAKE_GRID), floori(high.y / LAKE_GRID) + 1):
			for gx in range(floori(low.x / LAKE_GRID), floori(high.x / LAKE_GRID) + 1):
				var key := Vector2i(gx, gy)
				var list: PackedInt32Array = _lake_grid.get(key, PackedInt32Array())
				list.append(index)
				_lake_grid[key] = list


## Le point (px carte) est-il sur un lac affiché ?
func in_lake(p: Vector2) -> bool:
	var list: PackedInt32Array = _lake_grid.get(Vector2i(floori(p.x / LAKE_GRID), floori(p.y / LAKE_GRID)), PackedInt32Array())
	for index in list:
		var lake: Dictionary = _lakes[index]
		if (lake["rect"] as Rect2).has_point(p) and Geometry2D.is_point_in_polygon(p, lake["polygon"]):
			return true
	return false


## Zones humides (R, G, B de 0 à 1) au point ; noir sans le fichier.
func wetness_at(p: Vector2) -> Color:
	if _wet == null:
		return Color(0, 0, 0, 0)
	return _wet.get_pixel(clampi(int(p.x * _wet.get_width() / _map_data.size.x), 0, _wet.get_width() - 1), clampi(int(p.y * _wet.get_height() / _map_data.size.y), 0, _wet.get_height() - 1))


func clear() -> void:
	for entry: Dictionary in _cells.values():
		for node: Node in entry["nodes"]:
			node.queue_free()
	_cells.clear()
	stats["cells"] = 0
	stats["visible_cells"] = 0
	stats["draw_calls"] = 0
	stats["instances"] = 0
	stats["visible"] = 0


func _render(key: String, fallback: float) -> float:
	return float((config.get("render", {}) as Dictionary).get(key, fallback))


func cell_count() -> int:
	return _cells.size()


func zone_count() -> int:
	return _zones.size()


# --- Projection ---------------------------------------------------------------------------------


## (longitude, latitude) WGS84 → pixel de la carte (EPSG:3035, projection azimutale équivalente de
## Lambert sur GRS80, puis emprise de `map.json`).
static func lonlat_to_px(lon: float, lat: float, map_data: MapData) -> Vector2:
	var en := laea_3035(lon, lat)
	var bounds: Array = map_data.bounds_projected
	return Vector2((en.x - float(bounds[0])) / map_data.meters_per_px, (float(bounds[3]) - en.y) / map_data.meters_per_px)


static func laea_3035(lon_deg: float, lat_deg: float) -> Vector2:
	const A := 6378137.0
	const E2 := 0.00669438002290
	var e := sqrt(E2)
	var phi := deg_to_rad(lat_deg)
	var phi0 := deg_to_rad(52.0)
	var dlam := deg_to_rad(lon_deg - 10.0)
	var q := _laea_q(phi, e)
	var q0 := _laea_q(phi0, e)
	var qp := _laea_q(PI * 0.5, e)
	var beta := asin(clampf(q / qp, -1.0, 1.0))
	var beta0 := asin(clampf(q0 / qp, -1.0, 1.0))
	var rq := A * sqrt(qp * 0.5)
	var d := A * cos(phi0) / (sqrt(1.0 - E2 * sin(phi0) * sin(phi0)) * rq * cos(beta0))
	var b := rq * sqrt(2.0 / (1.0 + sin(beta0) * sin(beta) + cos(beta0) * cos(beta) * cos(dlam)))
	var x := b * d * cos(beta) * sin(dlam)
	var y := b / d * (cos(beta0) * sin(beta) - sin(beta0) * cos(beta) * cos(dlam))
	return Vector2(4321000.0 + x, 3210000.0 + y)


static func _laea_q(phi: float, e: float) -> float:
	var s := sin(phi)
	var e2 := e * e
	return (1.0 - e2) * (s / (1.0 - e2 * s * s) - log((1.0 - e * s) / (1.0 + e * s)) / (2.0 * e))


# --- Saisons -----------------------------------------------------------------------------------


static func _season_vector(seasons: Dictionary) -> Vector4:
	var v := Vector4.ONE
	for n in SEASONS.size():
		if seasons.has(SEASONS[n]):
			v[n] = float(seasons[SEASONS[n]])
	return v


func _profile_of(weights: Vector4) -> int:
	var key := "%.3f|%.3f|%.3f|%.3f" % [weights.x, weights.y, weights.z, weights.w]
	if _profile_index.has(key):
		return int(_profile_index[key])
	if _profiles.size() >= MAX_PROFILES:
		push_warning("FaunaLayer: plus de %d profils de saison" % MAX_PROFILES)
		return 0
	_profiles.append(weights)
	_profile_index[key] = _profiles.size() - 1
	return _profiles.size() - 1


## Tour courant : change la graine d'errance (les troupeaux se déplacent d'un tour à l'autre).
func set_turn(turn: int) -> void:
	_turn_phase = float(turn % 997) * 0.618
	for state: Dictionary in _species.values():
		_set_uniform(state, "turn_phase", _turn_phase)


# --- Semis ------------------------------------------------------------------------------------


func _cell_px() -> float:
	return _render("cell_px", 96.0)


func _cell_rect(key: Vector2i) -> Rect2:
	var side := _cell_px()
	return Rect2(Vector2(key) * side, Vector2(side, side))


## Troupeaux de la cellule : Array de {species, center: Vector2, size, rank, profile, seed,
## members: PackedVector2Array (écarts, px)}. Déterministe.
func cell_herds(key: Vector2i) -> Array:
	var herds: Array = []
	if _map_data == null:
		return herds
	var rect := _cell_rect(key)
	var cell_km2 := pow(rect.size.x * _mpp / 1000.0, 2.0)
	var sigma_px := _render("herd_sigma_m", 90.0) / _mpp
	for zone: Dictionary in _zones:
		var center: Vector2 = zone["center"]
		var radius: Vector2 = zone["radius"]
		if not Rect2(center - radius, radius * 2.0).intersects(rect):
			continue
		for item: Dictionary in zone["species"]:
			var species_id := str(item["species"])
			var species: Dictionary = (config["species"] as Dictionary).get(species_id, {})
			if species.is_empty():
				continue
			var rng := RandomNumberGenerator.new()
			rng.seed = hash(Vector4i(key.x, key.y, str(zone["id"]).hash() & 0xffff, species_id.hash() & 0xffff))
			var expected := float(item["per_1000km2"]) * cell_km2 / 1000.0
			var candidates := int(expected) + (1 if rng.randf() < expected - floorf(expected) else 0)
			for n in candidates:
				var p := rect.position + Vector2(rng.randf(), rng.randf()) * rect.size
				var herd_size_range: Array = species["herd_size"]
				var size := rng.randi_range(int(herd_size_range[0]), int(herd_size_range[1]))
				var rank := rng.randf()
				var herd_seed := rng.randf() * 0.98
				var members := PackedVector2Array()
				for m in size:
					members.append(Vector2(clampf(rng.randfn(0.0, sigma_px), -2.5 * sigma_px, 2.5 * sigma_px), clampf(rng.randfn(0.0, sigma_px), -2.5 * sigma_px, 2.5 * sigma_px)))
				var d := (p - center) / radius
				if d.length_squared() > 1.0 or not site_ok(p, species, item):
					continue
				herds.append({"species": species_id, "center": p, "size": size, "rank": rank, "profile": int(item["profile"]), "seed": herd_seed, "members": members})
	return herds


func _filter(species: Dictionary, item: Dictionary, name: String) -> float:
	var defaults: Dictionary = (config.get("render", {}) as Dictionary).get("defaults", {})
	return float(item.get(name, species.get(name, defaults.get(name, 0.0))))


## Le site (px carte) convient-il au troupeau : terre, hors colonie, fleuve, pente, altitude, biome,
## et cercle d'errance entièrement sur la terre (sauf espèces de rivage).
func site_ok(p: Vector2, species: Dictionary, item: Dictionary) -> bool:
	var map := _map_data
	if p.x < 1.0 or p.y < 1.0 or p.x >= map.size.x - 1 or p.y >= map.size.y - 1:
		return false
	if not map.is_land_px(int(p.x), int(p.y)):
		return false
	var clearance := _render("town_clearance_px", 2.5)
	var tk := Vector2i(floori(p.x / 8.0), floori(p.y / 8.0))
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var list: PackedVector2Array = _town_grid.get(tk + Vector2i(dx, dy), PackedVector2Array())
			for town in list:
				if town.distance_squared_to(p) < clearance * clearance:
					return false
	var height := map.height_m_at(p.x, p.y)
	if height < _filter(species, item, "alt_min_m") or height > _filter(species, item, "alt_max_m"):
		return false
	var gx := map.height_m_at(p.x + 1.0, p.y) - map.height_m_at(p.x - 1.0, p.y)
	var gy := map.height_m_at(p.x, p.y + 1.0) - map.height_m_at(p.x, p.y - 1.0)
	if rad_to_deg(atan(Vector2(gx, gy).length() / (2.0 * _mpp))) > _filter(species, item, "slope_max_deg"):
		return false
	var river := map.river_sd_at(p.x, p.y)
	if river < 0.3:
		return false
	if item.get("near_water", false) and river > 2.0:
		return false
	if in_lake(p):
		return false
	var wet := wetness_at(p)
	if wet.r > _filter(species, item, "marsh_max") or wet.g > 0.4:
		return false
	var open_min := _filter(species, item, "open_min")
	var forest_min := _filter(species, item, "forest_min")
	if (open_min > 0.0 or forest_min > 0.0) and map.splat_image != null:
		var splat := map.splat_image.get_pixel(clampi(int(p.x * map.splat_image.get_width() / map.size.x), 0, map.splat_image.get_width() - 1), clampi(int(p.y * map.splat_image.get_height() / map.size.y), 0, map.splat_image.get_height() - 1))
		if splat.r + splat.g + splat.a * 0.5 < open_min or splat.b < forest_min:
			return false
	var shore := int(item.get("near_sea_px", species.get("near_sea_px", 0)))
	if shore > 0:
		var touches_sea := false
		for k in 8:
			var a := TAU * k / 8.0
			if not map.is_land_px(int(p.x + cos(a) * shore), int(p.y + sin(a) * shore)):
				touches_sea = true
		return touches_sea
	# Cercle d'errance (et marge d'eau) entièrement sur la terre ; au moins 1 px (résolution du masque).
	var ring := maxf(1.0, (float(species["wander_m"]) + 3.0 * _render("herd_sigma_m", 90.0) + _render("water_margin_m", 60.0)) / _mpp)
	for k in 8:
		var a := TAU * k / 8.0
		var q := p + Vector2(cos(a), sin(a)) * ring
		if not map.is_land_px(int(q.x), int(q.y)) or in_lake(q):
			return false
	return true


## Tampon de MultiMesh d'une espèce pour une liste de troupeaux : Array de 16 flottants par bête
## (transformation 3 × 4, données personnalisées).
func pack_herds(herds: Array, species_id: String) -> PackedFloat32Array:
	var state := _species_state(species_id)
	var scale: float = state["base_scale"]
	var buffer := PackedFloat32Array()
	var count := 0
	for herd: Dictionary in herds:
		if herd["species"] == species_id:
			count += int((herd["members"] as PackedVector2Array).size())
	buffer.resize(count * FLOATS_PER_INSTANCE)
	var o := 0
	for herd: Dictionary in herds:
		if herd["species"] != species_id:
			continue
		var centre: Vector2 = herd["center"]
		for off: Vector2 in herd["members"]:
			var at := centre + off
			var h := maxf(_map_data.height_m_at(at.x, at.y), 0.0)
			buffer[o + 0] = scale
			buffer[o + 3] = at.x
			buffer[o + 5] = scale
			buffer[o + 7] = h
			buffer[o + 10] = scale
			buffer[o + 11] = at.y
			buffer[o + 12] = off.x
			buffer[o + 13] = off.y
			buffer[o + 14] = float(herd["profile"]) + float(herd["seed"])
			buffer[o + 15] = float(herd["rank"])
			o += FLOATS_PER_INSTANCE
	return buffer


# --- Modèles ----------------------------------------------------------------------------------


## État de rendu d'une espèce : maillages par niveau de détail (surfaces recouvertes de
## `fauna.gdshader`), échelle de base (unités monde par unité de maillage).
func _species_state(species_id: String) -> Dictionary:
	if _species.has(species_id):
		return _species[species_id]
	var species: Dictionary = (config["species"] as Dictionary)[species_id]
	var lods: Array = []
	for level in MAX_LODS:
		var path := DN_DIR + "%s_lod%d.glb" % [species_id, level]
		if not ResourceLoader.exists(path):
			break
		var mesh := flat_mesh_of(path)
		if mesh == null:
			break
		lods.append(mesh)
	var from_dn := not lods.is_empty()
	if lods.is_empty():
		var placeholder := MODEL_DIR + str(species.get("placeholder", ""))
		var mesh := flat_mesh_of(placeholder) if ResourceLoader.exists(placeholder) else null
		lods.append(mesh if mesh != null else _box_mesh())
	var aabb := (lods[0] as ArrayMesh).get_aabb()
	var long_x := aabb.size.x >= aabb.size.z
	var length := maxf(aabb.size.x, aabb.size.z)
	var state := {
		"id": species_id, "lods": [], "materials": [], "from_dn": from_dn,
		"base_scale": float(species["length_m"]) / _mpp / maxf(length, 1e-4),
		"real_units": float(species["length_m"]) / _mpp,
	}
	var gait: Dictionary = ((config["render"] as Dictionary)["gaits"] as Dictionary).get(str(species["gait"]), {})
	for mesh: ArrayMesh in lods:
		var covered := mesh.duplicate() as ArrayMesh
		for s in covered.get_surface_count():
			var material := ShaderMaterial.new()
			material.shader = SHADER
			var source := mesh.surface_get_material(s) as BaseMaterial3D
			if source != null:
				material.set_shader_parameter("albedo", source.albedo_color)
				if source.albedo_texture != null:
					material.set_shader_parameter("albedo_texture", source.albedo_texture)
					material.set_shader_parameter("has_texture", true)
			material.set_shader_parameter("mesh_origin", Vector3(aabb.get_center().x, aabb.position.y, aabb.get_center().z))
			material.set_shader_parameter("mesh_size", Vector2(length, aabb.size.y))
			material.set_shader_parameter("long_x", long_x)
			material.set_shader_parameter("wander_radius", float(species["wander_m"]) / _mpp)
			material.set_shader_parameter("wander_omega", float(gait.get("speed_m_s", 0.4)) / maxf(float(species["wander_m"]), 1.0))
			material.set_shader_parameter("gait", Vector4(float(gait.get("cadence", 1.2)), float(gait.get("swing", 0.15)), float(gait.get("lift", 0.06)), float(gait.get("bounce", 0.02))))
			material.set_shader_parameter("graze", float(gait.get("graze", 0.2)))
			material.set_shader_parameter("turn_phase", _turn_phase)
			var table := PackedVector4Array()
			for n in MAX_PROFILES:
				table.append(Vector4(_profiles[n]) if n < _profiles.size() else Vector4.ONE)
			material.set_shader_parameter("season_profiles", table)
			covered.surface_set_material(s, material)
			(state["materials"] as Array).append(material)
		(state["lods"] as Array).append(covered)
	_species[species_id] = state
	return state


func _set_uniform(state: Dictionary, uniform_name: String, value: Variant) -> void:
	for material: ShaderMaterial in state["materials"]:
		material.set_shader_parameter(uniform_name, value)


## Premier niveau d'un glb en un seul `ArrayMesh` (nœuds aplatis, transformations appliquées,
## une surface par surface d'origine avec son matériau). Null si le fichier est illisible.
static func flat_mesh_of(path: String) -> ArrayMesh:
	var scene := load(path) as PackedScene
	if scene == null:
		return null
	var root := scene.instantiate()
	var result := ArrayMesh.new()
	for node in root.find_children("*", "MeshInstance3D", true, false):
		var mi := node as MeshInstance3D
		if mi.mesh == null:
			continue
		var xf := mi.transform
		var parent := mi.get_parent()
		while parent != null and parent != root and parent is Node3D:
			xf = (parent as Node3D).transform * xf
			parent = parent.get_parent()
		for s in mi.mesh.get_surface_count():
			var arrays := mi.mesh.surface_get_arrays(s)
			if arrays.is_empty() or arrays[Mesh.ARRAY_VERTEX] == null:
				continue
			var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			for n in vertices.size():
				vertices[n] = xf * vertices[n]
			arrays[Mesh.ARRAY_VERTEX] = vertices
			if arrays[Mesh.ARRAY_NORMAL] != null:
				var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
				for n in normals.size():
					normals[n] = (xf.basis * normals[n]).normalized()
				arrays[Mesh.ARRAY_NORMAL] = normals
			arrays[Mesh.ARRAY_TANGENT] = null
			result.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
			result.surface_set_material(result.get_surface_count() - 1, mi.mesh.surface_get_material(s))
	root.free()
	return result if result.get_surface_count() > 0 else null


## Repli sans glb : corps et quatre pattes en boîtes (+Z devant, pieds à y = 0).
static func _box_mesh() -> ArrayMesh:
	var mesh := ArrayMesh.new()
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var indices := PackedInt32Array()
	var boxes: Array = [[Vector3(0, 0.9, 0), Vector3(0.7, 0.6, 1.8)], [Vector3(0, 1.25, 1.1), Vector3(0.35, 0.5, 0.5)]]
	for z in [-0.7, 0.7]:
		for x in [-0.22, 0.22]:
			boxes.append([Vector3(x, 0.3, z), Vector3(0.18, 0.6, 0.18)])
	for box: Array in boxes:
		var c: Vector3 = box[0]
		var h: Vector3 = (box[1] as Vector3) * 0.5
		var base := vertices.size()
		for sx in [-1.0, 1.0]:
			for sy in [-1.0, 1.0]:
				for sz in [-1.0, 1.0]:
					vertices.append(c + Vector3(sx * h.x, sy * h.y, sz * h.z))
					normals.append(Vector3(sx, sy, sz).normalized())
		for tri: Array in [[0, 1, 3], [0, 3, 2], [4, 6, 7], [4, 7, 5], [0, 4, 5], [0, 5, 1], [2, 3, 7], [2, 7, 6], [0, 2, 6], [0, 6, 4], [1, 5, 7], [1, 7, 3]]:
			indices.append_array([base + tri[0], base + tri[1], base + tri[2]])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var material := StandardMaterial3D.new()
	material.albedo_color = Color(0.5, 0.42, 0.34)
	mesh.surface_set_material(0, material)
	return mesh


# --- Mise à jour ------------------------------------------------------------------------------


func _process(_delta: float) -> void:
	if _map_data == null or config.is_empty():
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	var focus_value: Variant = _rig.get("focus") if _rig != null else null
	var at := Vector2(focus_value.x, focus_value.z) if focus_value is Vector3 else Vector2(camera.global_position.x, camera.global_position.z)
	update_view(at, distance)


## Cellules du disque autour de `at` construites (`cells_per_frame` par appel), visibilité, taille.
func update_view(at: Vector2, rig_distance: float) -> void:
	_frame += 1
	var max_distance := _render("max_distance", 110.0)
	var fade := 1.0 - smoothstep(_render("fade_from", 70.0), max_distance, rig_distance)
	var active := (enabled and fade > 0.001 and not _zones.is_empty()) or force_active
	if force_active:
		fade = 1.0
	if not active:
		if visible:
			visible = false
			stats["visible"] = 0
			stats["visible_cells"] = 0
			stats["draw_calls"] = 0
		return
	visible = true
	var radius := clampf(rig_distance * _render("load_factor", 2.6), _render("min_radius_px", 24.0), _render("max_radius_px", 260.0))
	var side := _cell_px()
	var lo := Vector2i(floori((at.x - radius) / side), floori((at.y - radius) / side))
	var hi := Vector2i(floori((at.x + radius) / side), floori((at.y + radius) / side))
	var wanted: Dictionary = {}
	var missing: Array[Vector2i] = []
	for cy in range(lo.y, hi.y + 1):
		for cx in range(lo.x, hi.x + 1):
			var key := Vector2i(cx, cy)
			var rect := _cell_rect(key)
			var nearest := Vector2(clampf(at.x, rect.position.x, rect.end.x), clampf(at.y, rect.position.y, rect.end.y))
			if nearest.distance_squared_to(at) > radius * radius or key.x < 0 or key.y < 0 or rect.position.x >= _map_data.size.x or rect.position.y >= _map_data.size.y:
				continue
			wanted[key] = true
			if not _cells.has(key):
				missing.append(key)
	missing.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return _cell_rect(a).get_center().distance_squared_to(at) < _cell_rect(b).get_center().distance_squared_to(at))
	for n in mini(missing.size(), int(_render("cells_per_frame", 1.0))):
		_install_cell(missing[n])
	var total := 0
	var shown_cells := 0
	var draws := 0
	for key: Vector2i in _cells:
		var entry: Dictionary = _cells[key]
		var shown := wanted.has(key)
		for node: MultiMeshInstance3D in entry["nodes"]:
			node.visible = shown
		if shown:
			entry["last_seen"] = _frame
			total += int(entry["count"])
			shown_cells += 1
			draws += (entry["nodes"] as Array).size()
	_evict()
	var budget := _render("max_visible_instances", 3000.0)
	var thin := 1.0 if total <= budget else budget / float(total)
	_apply_view(rig_distance, fade, thin)
	stats["cells"] = _cells.size()
	stats["visible_cells"] = shown_cells
	stats["draw_calls"] = draws
	stats["visible"] = int(minf(float(total), budget))
	stats["instances"] = _total_instances()


func _total_instances() -> int:
	var sum := 0
	for entry: Dictionary in _cells.values():
		sum += int(entry["count"])
	return sum


## Taille tenue à l'écran, fondu, amincissement et niveau de détail pour la distance du rig.
func _apply_view(rig_distance: float, fade: float, thin: float) -> void:
	var length := _render("length_k", 0.0105) * pow(maxf(rig_distance, 0.01), _render("length_exponent", 0.8))
	var spread_exponent := _render("spread_exponent", 0.5)
	var lod_distances: Array = (config["render"] as Dictionary)["lod_distances"]
	var lod := 0 if rig_distance < float(lod_distances[0]) else (1 if rig_distance < float(lod_distances[1]) else 2)
	for state: Dictionary in _species.values():
		var mult := maxf(1.0, length / float(state["real_units"]))
		_set_uniform(state, "size_mult", mult)
		_set_uniform(state, "spread_mult", pow(mult, spread_exponent))
		_set_uniform(state, "fade", fade)
		_set_uniform(state, "thin", thin)
	if lod != _lod:
		_lod = lod
		for entry: Dictionary in _cells.values():
			for node: MultiMeshInstance3D in entry["nodes"]:
				var state: Dictionary = _species[str(node.get_meta("species"))]
				node.multimesh.mesh = (state["lods"] as Array)[mini(lod, (state["lods"] as Array).size() - 1)]


## Construit les MultiMesh d'une cellule (un par espèce présente).
func _install_cell(key: Vector2i) -> void:
	var t0 := Time.get_ticks_usec()
	var herds := cell_herds(key)
	var entry := {"nodes": [], "count": 0, "last_seen": _frame}
	var by_species: Dictionary = {}
	for herd: Dictionary in herds:
		by_species[herd["species"]] = true
	var side := _cell_px()
	var rect := _cell_rect(key)
	for species_id: String in by_species:
		var state := _species_state(species_id)
		var buffer := pack_herds(herds, species_id)
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.use_custom_data = true
		var lods: Array = state["lods"]
		multimesh.mesh = lods[mini(maxi(_lod, 0), lods.size() - 1)]
		multimesh.instance_count = buffer.size() / FLOATS_PER_INSTANCE
		multimesh.buffer = buffer
		var margin := 8.0
		multimesh.custom_aabb = AABB(Vector3(rect.position.x - margin, -2.0, rect.position.y - margin), Vector3(side + 2.0 * margin, 80.0, side + 2.0 * margin))
		var mmi := MultiMeshInstance3D.new()
		mmi.name = "Fauna_%s_%d_%d" % [species_id, key.x, key.y]
		mmi.multimesh = multimesh
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		mmi.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
		mmi.visible = false
		mmi.set_meta("species", species_id)
		add_child(mmi)
		(entry["nodes"] as Array).append(mmi)
		entry["count"] = int(entry["count"]) + multimesh.instance_count
	_cells[key] = entry
	stats["build_ms_max"] = maxf(float(stats["build_ms_max"]), (Time.get_ticks_usec() - t0) / 1000.0)


func _evict() -> void:
	var limit := int(_render("max_cached_cells", 48.0))
	if _cells.size() <= limit:
		return
	var keys: Array = _cells.keys()
	keys.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return int(_cells[a]["last_seen"]) < int(_cells[b]["last_seen"]))
	for n in _cells.size() - limit:
		var entry: Dictionary = _cells[keys[n]]
		if int(entry["last_seen"]) == _frame:
			break
		for node: Node in entry["nodes"]:
			node.queue_free()
		_cells.erase(keys[n])


## NA (ADR 0219) : bêtes des troupeaux visibles près de `ground` (px carte), calculées d'après les
## cellules installées (le mouvement est dans le shader : on vise la position de repos du
## troupeau). Renvoie [{species, position: Vector3, height, radius}] ; `reach` : rayon de
## recherche (px). Seules les cellules dont le MultiMesh est affiché comptent.
func decor_candidates(ground: Vector2, reach: float = 1.0) -> Array:
	var found: Array = []
	var side := _cell_px()
	var lo := Vector2i(floori((ground.x - reach) / side), floori((ground.y - reach) / side))
	var hi := Vector2i(floori((ground.x + reach) / side), floori((ground.y + reach) / side))
	for cy in range(lo.y, hi.y + 1):
		for cx in range(lo.x, hi.x + 1):
			var key := Vector2i(cx, cy)
			if not _cells.has(key):
				continue
			var shown := {}
			for node: MultiMeshInstance3D in (_cells[key]["nodes"] as Array):
				if is_instance_valid(node) and node.is_visible_in_tree():
					shown[str(node.get_meta("species", ""))] = true
			if shown.is_empty():
				continue
			for herd: Dictionary in cell_herds(key):
				if not shown.has(herd["species"]):
					continue
				var centre: Vector2 = herd["center"]
				if absf(centre.x - ground.x) > reach or absf(centre.y - ground.y) > reach:
					continue
				var state := _species_state(str(herd["species"]))
				found.append({"species": herd["species"], "position": Vector3(centre.x, maxf(_map_data.height_m_at(centre.x, centre.y), 0.0), centre.y), "height": float(state["base_scale"]), "radius": float(state["base_scale"])})
	return found
