class_name MapBirdFlocks
extends Node3D

## Lot ME3 (chantier DN) : oiseaux de la carte de campagne, plusieurs espèces et comportements
## (rendu seulement). Tout vient de `data/map/map_birds.json` (espèces, habitats par biome /
## côte / zone humide / ville / altitude, saisons) ; un `MultiMesh` par espèce, mouvement et
## battement d'ailes dans `life_birds.gdshader`. Comportements : `orbit` (tourne autour d'un
## point : corbeaux, mouettes, étourneaux, rapaces), `transit` (vol en V qui traverse : oies),
## `perch` (posé sur un toit : cigognes), `wade` (patauge et dérive : flamants, grues).
## Les vols vivent autour du point visé et sont replacés quand la vue se déplace.

const BIRDS_SHADER := preload("res://shaders/life_birds.gdshader")
const BIRDS_FILE := "map_birds.json"
const WETLAND_FILE := "wetland_sites_px.json"
const BIOMES_FILE := "biomes.png"
## Indices FIGÉS de `data/map/biomes.yaml` (ADR 0143).
const BIOME_NAMES := ["sea", "oceanic", "continental", "mediterranean", "steppe", "boreal", "mountain", "semi_arid"]
const BIOME_DOWNSCALE := 8
const COAST_DIST_SCALE := 2.0
const MODE_ORBIT := 0
const MODE_TRANSIT := 1
const MODE_PERCH := 2
const MODE_WADE := 3
const HIDDEN_Y := -50.0

## Total d'oiseaux instanciés et par espèce.
var stats: Dictionary = {}
var flock_area := 70.0

var _map_data: MapData = null
var _terrain: TerrainBuilder = null
var _settlements: SettlementData = null
var _groups: Array = []
var _wetlands: Dictionary = {}
var _biome_bytes := PackedByteArray()
var _biome_size := Vector2i.ZERO
var _timer := 0.0
var _retarget_seconds := 1.5
var _rng := RandomNumberGenerator.new()


func setup(map_data: MapData, terrain: TerrainBuilder, settlements: SettlementData, data_dir: String = "") -> void:
	_map_data = map_data
	_terrain = terrain
	_settlements = settlements
	_rng.seed = 1337
	var dir := data_dir if data_dir != "" else (map_data.map_dir if map_data != null else "")
	var config: Variant = _read_json(dir.path_join(BIRDS_FILE))
	if not (config is Dictionary):
		push_warning("MapBirdFlocks: %s unreadable, no birds" % BIRDS_FILE)
		return
	var sites: Variant = _read_json(dir.path_join(WETLAND_FILE))
	if sites is Dictionary:
		_wetlands = (sites as Dictionary).get("sites", {})
	_load_biomes(dir)
	build(config as Dictionary)


## Construit les groupes depuis le contenu de `map_birds.json` (tests : dictionnaire synthétique).
func build(config: Dictionary) -> void:
	flock_area = float(config.get("flock_area", 70.0))
	_retarget_seconds = float(config.get("retarget_seconds", 1.5))
	var total := 0
	var per_species := {}
	for spec: Dictionary in config.get("species", []):
		var group := _make_group(spec)
		_groups.append(group)
		var count := int(spec["flocks"]) * int(spec["per_flock"])
		total += count
		per_species[str(spec["id"])] = count
	stats = {"total": total, "species": per_species}


# --- Fonctions pures (testées) -----------------------------------------------------------


## Poids de présence d'une espèce pour des poids de saison (printemps, été, automne, hiver).
static func season_weight(spec: Dictionary, season_weights: Vector4) -> float:
	var s: Dictionary = spec.get("seasons", {})
	return clampf(
		float(s.get("spring", 0.0)) * season_weights.x + float(s.get("summer", 0.0)) * season_weights.y
		+ float(s.get("autumn", 0.0)) * season_weights.z + float(s.get("winter", 0.0)) * season_weights.w,
		0.0, 1.0)


## Places d'un vol en V dans le repère du vol : x vers l'avant (négatif = derrière), y latéral.
static func v_slots(count: int, spacing: float, angle_deg: float) -> PackedVector2Array:
	var slots := PackedVector2Array()
	var tan_a := tan(deg_to_rad(angle_deg))
	for k in count:
		var row := (k + 1) / 2
		var side := 0.0 if k == 0 else (1.0 if k % 2 == 1 else -1.0)
		slots.append(Vector2(-row * spacing, side * row * spacing * tan_a))
	return slots


static func biome_name(index: int) -> String:
	return BIOME_NAMES[index] if index >= 0 and index < BIOME_NAMES.size() else "sea"


## Zones humides de `sites` (id -> {px, radius_px}) retenues pour `ids`, à portée de `focus`.
static func sites_near(sites: Dictionary, ids: Array, focus: Vector2, reach: float) -> Array:
	var found: Array = []
	for id: Variant in ids:
		var site: Dictionary = sites.get(str(id), {})
		if site.is_empty():
			continue
		var px: Array = site["px"]
		var center := Vector2(float(px[0]), float(px[1]))
		if center.distance_to(focus) <= reach + float(site["radius_px"]):
			found.append({"id": str(id), "center": center, "radius": float(site["radius_px"])})
	return found


# --- Construction -------------------------------------------------------------------------


static func _read_json(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		return null
	return JSON.parse_string(FileAccess.get_file_as_string(path))


## Carte des biomes réduite (le raster complet fait 44 Mo).
func _load_biomes(dir: String) -> void:
	var path := dir.path_join(BIOMES_FILE)
	if not FileAccess.file_exists(path):
		return
	var image := Image.load_from_file(path)
	if image == null or image.is_empty():
		return
	if image.get_format() != Image.FORMAT_L8:
		image.convert(Image.FORMAT_L8)
	image.resize(maxi(image.get_width() / BIOME_DOWNSCALE, 1), maxi(image.get_height() / BIOME_DOWNSCALE, 1), Image.INTERPOLATE_NEAREST)
	_biome_size = image.get_size()
	_biome_bytes = image.get_data()


func _make_group(spec: Dictionary) -> Dictionary:
	var mode := _mode_of(str(spec["behavior"]))
	var material := ShaderMaterial.new()
	material.shader = BIRDS_SHADER
	var color: Array = spec["color"]
	material.set_shader_parameter("bird_color", Color(color[0], color[1], color[2]))
	material.set_shader_parameter("mode", mode)
	material.set_shader_parameter("flap_rate", float(spec.get("flap_rate", 11.0)))
	material.set_shader_parameter("flap_amount", float(spec.get("flap_amount", 0.55)))
	material.set_shader_parameter("wing_fold", float(spec.get("wing_fold", 0.0)))
	material.set_shader_parameter("span", float(spec["habitat"].get("span", 60.0)))
	var mesh: Mesh = null
	var model := str(spec.get("model", ""))
	if model != "":
		mesh = LifeEffects._first_mesh(model)
	if mesh == null:
		mesh = bird_mesh()
	var count := int(spec["flocks"]) * int(spec["per_flock"])
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = mesh
	multimesh.instance_count = count
	var mmi := MultiMeshInstance3D.new()
	mmi.name = "Birds_" + str(spec["id"])
	mmi.multimesh = multimesh
	mmi.material_override = material
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mmi.extra_cull_margin = _world_margin()
	add_child(mmi)
	var centers: Array[Vector2] = []
	centers.resize(int(spec["flocks"]))
	centers.fill(Vector2(-1e6, -1e6))
	var group := {"spec": spec, "mode": mode, "mmi": mmi, "centers": centers, "placed": [], "key": []}
	(group["placed"] as Array).resize(int(spec["flocks"]))
	(group["key"] as Array).resize(int(spec["flocks"]))
	for f in int(spec["flocks"]):
		_hide_flock(group, f)
	return group


static func _mode_of(behavior: String) -> int:
	match behavior:
		"transit":
			return MODE_TRANSIT
		"perch":
			return MODE_PERCH
		"wade":
			return MODE_WADE
	return MODE_ORBIT


## V aplati : deux ailes en triangles, corps au centre (bec vers -Z).
static func bird_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array([
		Vector3(0, 0, -0.25), Vector3(-1.0, 0.08, 0.15), Vector3(0, 0, 0.2),
		Vector3(0, 0, -0.25), Vector3(0, 0, 0.2), Vector3(1.0, 0.08, 0.15),
	])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


# --- Placement ------------------------------------------------------------------------------


func _hide_flock(group: Dictionary, f: int) -> void:
	var spec: Dictionary = group["spec"]
	var per_flock := int(spec["per_flock"])
	var multimesh: MultiMesh = (group["mmi"] as MultiMeshInstance3D).multimesh
	for b in per_flock:
		multimesh.set_instance_transform(f * per_flock + b, Transform3D(Basis.from_scale(Vector3.ONE * 0.001), Vector3(0, HIDDEN_Y, 0)))
	group["centers"][f] = Vector2(-1e6, -1e6)
	group["placed"][f] = false
	group["key"][f] = ""


func _ground(p: Vector2) -> float:
	return _terrain.surface_height_at(p.x, p.y) if _terrain != null else 0.0


func _biome_at(p: Vector2) -> String:
	if _biome_bytes.is_empty() or _map_data == null:
		return ""
	var px := clampi(int(p.x * _biome_size.x / maxf(_map_data.size.x, 1.0)), 0, _biome_size.x - 1)
	var py := clampi(int(p.y * _biome_size.y / maxf(_map_data.size.y, 1.0)), 0, _biome_size.y - 1)
	return biome_name(int(_biome_bytes[py * _biome_size.x + px]))


## Distance signée à la côte (px carte, > 0 sur terre) ; grande sans raster.
func _coast_distance(p: Vector2) -> float:
	var image := _map_data.coast_dist_image if _map_data != null else null
	if image == null or image.is_empty():
		return 1e6
	var px := clampi(int(p.x * image.get_width() / maxf(_map_data.size.x, 1.0)), 0, image.get_width() - 1)
	var py := clampi(int(p.y * image.get_height() / maxf(_map_data.size.y, 1.0)), 0, image.get_height() - 1)
	return (image.get_pixel(px, py).r * 255.0 - 128.0) / COAST_DIST_SCALE


## L'emplacement `p` convient-il à l'habitat de l'espèce ?
func habitat_ok(spec: Dictionary, p: Vector2) -> bool:
	var habitat: Dictionary = spec["habitat"]
	var on_land := _map_data == null or _map_data.is_land_px(int(p.x), int(p.y))
	match str(habitat.get("surface", "land")):
		"land":
			if not on_land:
				return false
		"sea":
			if on_land:
				return false
	if habitat.has("biomes") and on_land:
		var biome := _biome_at(p)
		if biome != "" and not (habitat["biomes"] as Array).has(biome):
			return false
	if habitat.has("coast_max_px") and absf(_coast_distance(p)) > float(habitat["coast_max_px"]):
		return false
	if (habitat.has("min_height_m") or habitat.has("max_height_m")) and _map_data != null:
		var height := _map_data.height_m_at(p.x, p.y)
		if height < float(habitat.get("min_height_m", -1e9)) or height > float(habitat.get("max_height_m", 1e9)):
			return false
	return true


func _try_place(group: Dictionary, f: int, focus: Vector2) -> void:
	var spec: Dictionary = group["spec"]
	var habitat: Dictionary = spec["habitat"]
	if habitat.has("towns"):
		_place_on_town(group, f, focus)
	elif habitat.has("wetland_ids"):
		_place_on_wetland(group, f, focus)
	else:
		for _try in 8:
			var angle := _rng.randf() * TAU
			var place := focus + Vector2(cos(angle), sin(angle)) * flock_area * sqrt(_rng.randf())
			if habitat_ok(spec, place):
				_place_flock(group, f, place, "")
				return


func _place_on_town(group: Dictionary, f: int, focus: Vector2) -> void:
	if _settlements == null:
		return
	var spec: Dictionary = group["spec"]
	var towns: Array = spec["habitat"]["towns"]
	var start := _rng.randi() % maxi(towns.size(), 1)
	for k in towns.size():
		var id := str(towns[(start + k) % towns.size()])
		if (group["key"] as Array).has(id):
			continue
		var entry := _settlements.get_settlement(id)
		if entry.is_empty():
			continue
		var center: Vector2 = entry["px"]
		if center.distance_to(focus) > flock_area:
			continue
		var radius := float(spec["habitat"].get("town_radius", 3.0))
		var angle := _rng.randf() * TAU
		_place_flock(group, f, center + Vector2(cos(angle), sin(angle)) * radius * sqrt(_rng.randf()), id)
		return


func _place_on_wetland(group: Dictionary, f: int, focus: Vector2) -> void:
	var spec: Dictionary = group["spec"]
	var near := sites_near(_wetlands, spec["habitat"]["wetland_ids"], focus, flock_area)
	if near.is_empty():
		return
	var start := _rng.randi() % near.size()
	for k in near.size():
		var site: Dictionary = near[(start + k) % near.size()]
		if (group["key"] as Array).has(site["id"]):
			continue
		for _try in 6:
			var angle := _rng.randf() * TAU
			var place: Vector2 = site["center"] + Vector2(cos(angle), sin(angle)) * float(site["radius"]) * 0.6 * sqrt(_rng.randf())
			if habitat_ok(spec, place):
				_place_flock(group, f, place, site["id"])
				return


func _place_flock(group: Dictionary, f: int, center: Vector2, key: String) -> void:
	var spec: Dictionary = group["spec"]
	var mode: int = group["mode"]
	var per_flock := int(spec["per_flock"])
	var multimesh: MultiMesh = (group["mmi"] as MultiMeshInstance3D).multimesh
	group["centers"][f] = center
	group["placed"][f] = true
	group["key"][f] = key
	var ground := _ground(center)
	var height := _range(spec["height"])
	var radius := _range(spec["radius"])
	var direction := 1.0 if _rng.randf() < 0.5 else -1.0
	var heading := _rng.randf() * TAU
	var jitter := float(spec.get("jitter", 0.0))
	var height_jitter := float(spec.get("height_jitter", 0.0))
	var flock_speed := _range(spec["speed"])
	var slots := PackedVector2Array()
	if mode == MODE_TRANSIT:
		var habitat: Dictionary = spec["habitat"]
		slots = v_slots(per_flock, float(habitat.get("v_spacing", 1.6)), float(habitat.get("v_angle_deg", 32.0)))
	var basis := Basis.from_scale(Vector3.ONE * float(spec["scale"]))
	for b in per_flock:
		var n := f * per_flock + b
		var offset := Vector3(_rng.randf_range(-jitter, jitter), 0.0, _rng.randf_range(-jitter, jitter))
		var vertical := height + _rng.randf_range(-height_jitter, height_jitter)
		var custom := Color()
		match mode:
			MODE_TRANSIT:
				var slot := slots[b]
				var along := Vector2(cos(heading), sin(heading))
				var lateral := Vector2(-sin(heading), cos(heading))
				var planar := along * slot.x + lateral * slot.y
				offset = Vector3(planar.x, 0.0, planar.y)
				custom = Color(heading, flock_speed, fposmod(float(f) * 0.37 + 0.2, 1.0), 0.0)
			MODE_PERCH:
				custom = Color(_rng.randf() * TAU, 0.0, _rng.randf(), 0.0)
			_:
				# Même vitesse dans un vol, phases proches : ils volent ensemble ; rayon propre à
				# chaque oiseau quand `radius` varie (nuées).
				var r := radius + _rng.randf() * float(spec.get("radius_spread", 0.8))
				custom = Color(maxf(r, 0.2), direction * (flock_speed + 0.02 * _rng.randf()), _rng.randf() * float(spec.get("phase_spread", 0.1)) + 0.1 * f, _rng.randf() * 0.8)
		# La hauteur est portée par l'origine de l'instance (sol du centre du vol).
		var origin := Vector3(center.x + offset.x, ground + vertical, center.y + offset.z)
		multimesh.set_instance_transform(n, Transform3D(basis, origin))
		multimesh.set_instance_custom_data(n, custom)


func _range(pair: Array) -> float:
	return _rng.randf_range(float(pair[0]), float(pair[1]))


# --- Mise à jour ------------------------------------------------------------------------------


## `season_weights` : poids printemps, été, automne, hiver (`SeasonVisuals.weights`).
func update_view(focus: Vector2, visible_birds: bool, season_weights: Vector4) -> void:
	visible = visible_birds
	if not visible_birds:
		return
	_timer -= get_process_delta_time()
	if _timer > 0.0:
		return
	_timer = _retarget_seconds
	retarget(focus, season_weights)


## Replace les vols trop loin, hors saison ou hors habitat ; place les vols libres.
func retarget(focus: Vector2, season_weights: Vector4) -> void:
	for group: Dictionary in _groups:
		var spec: Dictionary = group["spec"]
		var weight := season_weight(spec, season_weights)
		var centers: Array[Vector2] = group["centers"]
		for f in centers.size():
			var placed: bool = group["placed"][f]
			if placed and (weight <= 0.0 or centers[f].distance_to(focus) > flock_area * 1.3):
				_hide_flock(group, f)
				placed = false
			if not placed and weight > 0.0 and _rng.randf() < weight:
				_try_place(group, f, focus)


func has_biomes() -> bool:
	return not _biome_bytes.is_empty()


## Nombre de vols posés actuellement (tests, statistiques).
func placed_count() -> int:
	var total := 0
	for group: Dictionary in _groups:
		for flag: bool in group["placed"]:
			total += 1 if flag else 0
	return total


func placed_count_of(species_id: String) -> int:
	for group: Dictionary in _groups:
		if str(group["spec"]["id"]) == species_id:
			var total := 0
			for flag: bool in group["placed"]:
				total += 1 if flag else 0
			return total
	return 0


## Marge d'élagage couvrant tout le monde (instances réparties sur la carte, ADR 0115).
func _world_margin() -> float:
	if _map_data == null:
		return 16384.0
	return float(maxi(_map_data.size.x, _map_data.size.y))
