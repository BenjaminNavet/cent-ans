class_name RockOutcrops
extends Node3D

## Lot HB5 (ADR 0143) : affleurements rocheux à l'échelle du paysage sur la carte de campagne
## (falaises calcaires, chaos granitiques, barres stratifiées, aiguilles alpines, grès rouges,
## blocs erratiques), lisibles de la vue moyenne (d ≈ 100-400) à la vue proche.
##
## - Catalogue `data/art/rock_outcrops.yaml` (modèles, biomes, pente et altitude, taille réelle,
##   densité, réglages de rendu) ; modèles `res://assets/models/rocks/hb/<id>_lod{0,1,2}.glb`.
## - Tuiles de `tile_units` unités semées autour du point visé (rayon lié à la distance caméra),
##   dans le `WorkerThreadPool` ; un `MultiMeshInstance3D` par modèle et par tuile (un appel de
##   dessin), cache borné des tuiles (les plus anciennes hors champ libérées), comme `Vegetation`
##   et `GroundClutter`.
## - Semis déterministe (graine = tuile) sur une grille tramée : aptitude par modèle = biome
##   (`data/map/biomes.png`, repli altitude / position), fenêtre d'altitude, pente forte, crêtes
##   (courbure convexe), au-dessus de la limite des arbres, lande / garrigue (splatmap, canal A),
##   modulée par un bruit de regroupement. Jamais dans l'eau (terre aux quatre coins de l'emprise,
##   berges des fleuves), les clairières des colonies ni sur les routes.
## - Posé sur la surface affichée (instantané du quadtree, pied au plus bas de l'emprise), recalé
##   sur `chunk_surface_changed` / `surface_rect_changed`.
## - Taille réelle de près, grossie jusqu'à `far_scale` au loin (shader `rock_outcrops`), hauteur
##   suivant en partie l'exagération du relief ; niveau de détail par tuile selon la distance ;
##   triangles affichés bornés (`max_visible_triangles`) ; effacement de `fade_start` à
##   `fade_end` (vue parchemin au-delà, ADR 0124). `--no-outcrops` coupe la couche (A/B).
## Purement visuel : aucune règle de jeu.

const CATALOGUE_FILE := "art/rock_outcrops.yaml"
const BIOMES_FILE := "biomes.png"
const MODEL_DIR := "res://assets/models/rocks/hb/"
const SHADER := preload("res://shaders/rock_outcrops.gdshader")
const FLOATS_PER_INSTANCE := 16  # transformation 3 × 4 + données personnalisées
const LODS := 3
## Grille grossière (par côté) du pré-examen d'une tuile : une tuile plate et basse est vide.
const PRECHECK := 12

@export var camera_rig_path: NodePath = ^"../CameraRig"
@export var threaded: bool = true
@export var max_concurrent_jobs: int = 4
@export var max_installs_per_frame: int = 2
@export var max_reground_per_frame: int = 2

var enabled: bool = true
var map_data: MapData
var terrain: TerrainBuilder
var catalogue: Dictionary = {}
## Réglages `render` du catalogue.
var settings: Dictionary = {}
## Modèles chargés : [{"id", "entry", "meshes": [Mesh ×3], "triangles": [int ×3], "material"}].
var models: Array = []
var exclusions: PackedVector3Array = PackedVector3Array()
## "biomes.png" ou "fallback".
var biome_source: String = "fallback"
var stats: Dictionary = {"tiles": 0, "instances": 0, "visible": 0, "triangles": 0, "draw_calls": 0, "seed_ms_max": 0.0, "install_ms_max": 0.0}

var _vegetation: Node
var _mask: VegetationMask
var _rig: Node3D
var _biome_bytes: PackedByteArray = PackedByteArray()
var _biome_size: Vector2i = Vector2i.ZERO
var _roads_by_tile: Dictionary = {}  # Vector2i → PackedVector2Array (paires de points)
var _tiles: Dictionary = {}  # Vector2i → {"parts": Array, "last_seen", "lod"}
var _jobs: Dictionary = {}  # Vector2i → {"task", "result"[, "stale"]}
var _ground_jobs: Dictionary = {}  # Vector2i → {"task", "result"[, "stale"]}
var _dirty: Dictionary = {}
var _frame: int = 0
var _focus := Vector2.ZERO
var _camera_distance: float = 1e9
var _scale: float = 1.0


func _ready() -> void:
	_rig = get_node_or_null(camera_rig_path) as Node3D
	if "--no-outcrops" in OS.get_cmdline_user_args():
		enabled = false


## Lit le catalogue (YAML en syntaxe de flux : lignes `#` retirées, puis JSON). {} si absent.
static func load_catalogue(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		return {}
	var lines := PackedStringArray()
	for line in FileAccess.get_file_as_string(path).split("\n"):
		if not line.strip_edges().begins_with("#"):
			lines.append(line)
	var parsed: Variant = JSON.parse_string("\n".join(lines))
	return parsed if parsed is Dictionary else {}


## Branche la couche. `road_lines` : polylignes des routes (px carte) ; `exclusion_circles` :
## clairières des colonies (Vector3(x, y, rayon)) ; `vegetation` : nœud dont on lit le masque.
func setup(data: MapData, terrain_builder: TerrainBuilder, vegetation: Node = null, exclusion_circles := PackedVector3Array(), road_lines: Array = [], catalogue_path: String = "") -> void:
	clear()
	map_data = data
	_vegetation = vegetation
	if catalogue_path == "":
		catalogue_path = MapPaths.default_data_dir().path_join(CATALOGUE_FILE)
	catalogue = load_catalogue(catalogue_path)
	settings = catalogue.get("render", {})
	var margin := float(settings.get("settlement_margin", 1.0))
	exclusions = PackedVector3Array()
	for c in exclusion_circles:
		exclusions.append(Vector3(c.x, c.y, c.z + margin))
	_load_models()
	_load_biomes()
	_index_roads(road_lines)
	if terrain != null:
		if terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
			terrain.chunk_surface_changed.disconnect(_on_chunk_surface_changed)
		if terrain.surface_rect_changed.is_connected(_on_surface_rect_changed):
			terrain.surface_rect_changed.disconnect(_on_surface_rect_changed)
	terrain = terrain_builder
	if terrain != null:
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
		terrain.surface_rect_changed.connect(_on_surface_rect_changed)


## Vrai si la couche peut poser quelque chose (catalogue et au moins un modèle).
func active() -> bool:
	return enabled and map_data != null and not models.is_empty()


func _exit_tree() -> void:
	_wait_jobs()


func _wait_jobs() -> void:
	for jobs: Dictionary in [_jobs, _ground_jobs]:
		for holder: Dictionary in jobs.values():
			WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
		jobs.clear()


func clear() -> void:
	_wait_jobs()
	for entry: Dictionary in _tiles.values():
		for part: Dictionary in entry["parts"]:
			(part["mmi"] as Node).queue_free()
	_tiles.clear()
	_dirty.clear()
	stats["tiles"] = 0
	stats["instances"] = 0
	stats["visible"] = 0


func pending_jobs() -> int:
	return _jobs.size() + _ground_jobs.size()


func tile_count() -> int:
	return _tiles.size()


func tile_units() -> float:
	return float(settings.get("tile_units", 128.0))


## Instances posées dans `rect` (px carte) : [{"id", "x", "y", "height", "size"}] (tests).
func instances_in(rect: Rect2) -> Array:
	var result: Array = []
	for key: Vector2i in _tiles:
		if not _tile_rect(key).intersects(rect):
			continue
		for part: Dictionary in _tiles[key]["parts"]:
			var buffer: PackedFloat32Array = part["buffer"]
			var id: String = models[int(part["model"])]["id"]
			for n in buffer.size() / FLOATS_PER_INSTANCE:
				var o := n * FLOATS_PER_INSTANCE
				var p := Vector2(buffer[o + 3], buffer[o + 11])
				if rect.has_point(p):
					result.append({"id": id, "x": p.x, "y": p.y, "height": buffer[o + 7], "size": Vector2(buffer[o], buffer[o + 8]).length()})
	return result


# --- Chargement ---------------------------------------------------------------------------


func _load_models() -> void:
	models.clear()
	for entry: Dictionary in catalogue.get("outcrops", []):
		var meshes: Array = []
		var triangles: Array = []
		for lod in LODS:
			var mesh := Ga3Vegetation._load_mesh(MODEL_DIR + "%s_lod%d.glb" % [entry["id"], lod])
			if mesh == null:
				break
			meshes.append(mesh)
			triangles.append(_triangles(mesh))
		if meshes.size() != LODS:
			continue
		var material := ShaderMaterial.new()
		material.shader = SHADER
		material.set_shader_parameter("meters_per_unit", map_data.meters_per_px if map_data != null else 719.0)
		material.set_shader_parameter("vertical_follow", float(settings.get("vertical_follow", 0.5)))
		var source := (meshes[0] as Mesh).surface_get_material(0) as BaseMaterial3D
		if source != null and source.albedo_texture != null:
			material.set_shader_parameter("albedo_texture", source.albedo_texture)
		models.append({"id": entry["id"], "entry": entry, "meshes": meshes, "triangles": triangles, "material": material, "biomes": _biome_mask(entry)})


static func _triangles(mesh: Mesh) -> int:
	var total := 0
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var index: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
		total += index.size() / 3 if not index.is_empty() else (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() / 3
	return total


static func _biome_mask(entry: Dictionary) -> int:
	var bits := 0
	for b: Variant in entry.get("biomes", []):
		bits |= 1 << int(b)
	return bits


func _load_biomes() -> void:
	_biome_bytes = PackedByteArray()
	biome_source = "fallback"
	if map_data == null:
		return
	var path := map_data.map_dir.path_join(BIOMES_FILE)
	if not FileAccess.file_exists(path):
		return
	var image := Image.load_from_file(path)
	if image == null or image.is_empty():
		return
	if image.get_format() != Image.FORMAT_L8:
		image.convert(Image.FORMAT_L8)
	_biome_size = image.get_size()
	_biome_bytes = image.get_data()
	biome_source = BIOMES_FILE


## Routes rangées par tuile (segments dont la boîte, élargie du dégagement, touche la tuile).
func _index_roads(road_lines: Array) -> void:
	_roads_by_tile.clear()
	var size := tile_units()
	var grow := float(settings.get("road_clearance", 1.2)) + _max_radius()
	for line: Variant in road_lines:
		var points: PackedVector2Array = line
		for i in points.size() - 1:
			var a := points[i]
			var b := points[i + 1]
			var lo := Vector2i(floori((minf(a.x, b.x) - grow) / size), floori((minf(a.y, b.y) - grow) / size))
			var hi := Vector2i(floori((maxf(a.x, b.x) + grow) / size), floori((maxf(a.y, b.y) + grow) / size))
			for ty in range(lo.y, hi.y + 1):
				for tx in range(lo.x, hi.x + 1):
					var key := Vector2i(tx, ty)
					var list: PackedVector2Array = _roads_by_tile.get(key, PackedVector2Array())
					list.append(a)
					list.append(b)
					_roads_by_tile[key] = list


## Plus grand rayon d'emprise (unités, taille réelle) du catalogue.
func _max_radius() -> float:
	var r := 0.0
	var mpp := map_data.meters_per_px if map_data != null else 719.0
	for entry: Dictionary in catalogue.get("outcrops", []):
		r = maxf(r, float(entry["size_m"][1]) * 0.5 / mpp)
	return r


# --- Vue ------------------------------------------------------------------------------------


func _process(_delta: float) -> void:
	if not active():
		visible = false
		return
	if _mask == null and _vegetation != null:
		_mask = _vegetation.get("mask") as VegetationMask
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		return
	var distance: float = _rig.get("distance") if _rig != null else camera.global_position.y
	var focus_value: Variant = _rig.get("focus") if _rig != null else null
	var at := Vector2(focus_value.x, focus_value.z) if focus_value is Vector3 else Vector2(camera.global_position.x, camera.global_position.z)
	update_view(at, distance)


## Grossissement (1 de près, `far_scale` au loin) à la distance caméra `d`.
func scale_for(d: float) -> float:
	var near := maxf(float(settings.get("near_distance", 15.0)), 1e-3)
	var far := maxf(float(settings.get("far_distance", 110.0)), near * 1.01)
	var t := clampf(log(maxf(d, 1e-3) / near) / log(far / near), 0.0, 1.0)
	return lerpf(1.0, float(settings.get("far_scale", 2.6)), smoothstep(0.0, 1.0, t))


## Niveau de détail d'une tuile à la distance `d` (point visé → tuile + distance caméra).
func lod_for(d: float) -> int:
	var lods: Array = settings.get("lod_distances", [45.0, 160.0])
	if d < float(lods[0]):
		return 0
	return 1 if d < float(lods[1]) else 2


func update_view(at: Vector2, camera_distance: float) -> void:
	_frame += 1
	var fade := 1.0 - smoothstep(float(settings.get("fade_start", 950.0)), float(settings.get("fade_end", 1200.0)), camera_distance)
	if not active() or fade <= 0.001:
		if visible:
			visible = false
			stats["visible"] = 0
		return
	visible = true
	_focus = at
	_camera_distance = camera_distance
	_scale = scale_for(camera_distance)
	for model: Dictionary in models:
		(model["material"] as ShaderMaterial).set_shader_parameter("outcrop_scale", _scale)
	var size := tile_units()
	var radius := clampf(camera_distance * float(settings.get("view_radius_factor", 2.2)), float(settings.get("min_view_radius", 96.0)), float(settings.get("max_view_radius", 1000.0)))
	var lo := Vector2i(floori((at.x - radius) / size), floori((at.y - radius) / size))
	var hi := Vector2i(floori((at.x + radius) / size), floori((at.y + radius) / size))
	var wanted: Dictionary = {}
	var missing: Array = []
	for ty in range(lo.y, hi.y + 1):
		for tx in range(lo.x, hi.x + 1):
			var key := Vector2i(tx, ty)
			if not _tile_on_map(key):
				continue
			var d := _rect_distance(_tile_rect(key), at)
			if d > radius:
				continue
			wanted[key] = d
			if not _tiles.has(key) and not _jobs.has(key):
				missing.append([d, key])
	missing.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	_collect_jobs()
	for item: Array in missing:
		if _jobs.size() >= max_concurrent_jobs:
			break
		_start_job(item[1])
	_update_regrounds()
	for key: Vector2i in _tiles:
		var entry: Dictionary = _tiles[key]
		var shown := wanted.has(key)
		for part: Dictionary in entry["parts"]:
			(part["mmi"] as MultiMeshInstance3D).visible = shown
		if shown:
			entry["last_seen"] = _frame
			_set_tile_lod(entry, lod_for(float(wanted[key]) + camera_distance))
	_evict()
	_apply_counts(fade)


func _tile_rect(key: Vector2i) -> Rect2:
	var size := tile_units()
	return Rect2(Vector2(key) * size, Vector2(size, size))


func _tile_on_map(key: Vector2i) -> bool:
	if key.x < 0 or key.y < 0 or map_data == null:
		return false
	return key.x * tile_units() < map_data.size.x and key.y * tile_units() < map_data.size.y


static func _rect_distance(rect: Rect2, point: Vector2) -> float:
	var nearest := Vector2(clampf(point.x, rect.position.x, rect.end.x), clampf(point.y, rect.position.y, rect.end.y))
	return nearest.distance_to(point)


# --- Semis (fil de travail) -----------------------------------------------------------------


func _start_job(key: Vector2i) -> void:
	var source := _height_source(_tile_rect(key).grow(_max_radius() * 2.0 + 2.0))
	if not threaded:
		_install_tile(key, _grounded(_seed_tile(key), source))
		return
	var holder := {"result": {}}
	holder["task"] = WorkerThreadPool.add_task(func() -> void: holder["result"] = _grounded(_seed_tile(key), source), false, "RockOutcrops")
	_jobs[key] = holder


func _collect_jobs() -> void:
	var installed := 0
	for key: Vector2i in _jobs.keys():
		if installed >= max_installs_per_frame:
			break
		var holder: Dictionary = _jobs[key]
		if not WorkerThreadPool.is_task_completed(int(holder["task"])):
			continue
		WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
		_jobs.erase(key)
		_install_tile(key, holder["result"])
		if holder.get("stale", false):
			_dirty[key] = true
		installed += 1


## Attend toutes les tâches et installe (tests, captures).
func flush() -> void:
	for jobs: Dictionary in [_jobs, _ground_jobs]:
		for holder: Dictionary in jobs.values():
			WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
	var saved := max_installs_per_frame
	max_installs_per_frame = 1 << 20
	_collect_jobs()
	_update_regrounds()
	max_installs_per_frame = saved


## Biome (indices ADR 0143) au point (x, y) : `biomes.png`, sinon repli altitude / position.
func biome_at(x: float, y: float, height_m: float) -> int:
	if not _biome_bytes.is_empty():
		var px := clampi(int(x * _biome_size.x / map_data.size.x), 0, _biome_size.x - 1)
		var py := clampi(int(y * _biome_size.y / map_data.size.y), 0, _biome_size.y - 1)
		return _biome_bytes[py * _biome_size.x + px]
	var fb: Dictionary = catalogue.get("fallback_biomes", {})
	var u := x / maxf(map_data.size.x, 1.0)
	var v := y / maxf(map_data.size.y, 1.0)
	if height_m >= float(fb.get("alpine_altitude_m", 1500.0)):
		return 6
	if v >= float(fb.get("south_y", 0.62)):
		return 7 if u >= float(fb.get("steppe_x", 0.78)) else 3
	if v <= float(fb.get("north_y", 0.26)):
		return 5
	if u >= float(fb.get("steppe_x", 0.78)):
		return 4
	return 2 if u >= float(fb.get("east_x", 0.52)) else 1


## Limite des arbres (m), comme `VegetationMask.treeline_m`.
func _treeline_m(y: float) -> float:
	var north := 1.0 - clampf(y / maxf(map_data.size.y, 1.0), 0.0, 1.0)
	return lerpf(2100.0, 900.0, smoothstep(0.55, 0.95, north))


## Relief local au point : {"h", "slope", "ridge", "grad": Vector2} (mètres, ±2 px).
func _terrain_probe(x: float, y: float) -> Dictionary:
	var h := map_data.height_m_at(x, y)
	var e := map_data.height_m_at(x + 2.0, y)
	var w := map_data.height_m_at(x - 2.0, y)
	var s := map_data.height_m_at(x, y + 2.0)
	var n := map_data.height_m_at(x, y - 2.0)
	var grad := Vector2(e - w, s - n) / (4.0 * map_data.meters_per_px)
	return {"h": h, "slope": grad.length(), "ridge": h - (e + w + s + n) * 0.25, "grad": grad}


## Aptitude [0, 1] du modèle `model` au point sondé `probe` (biome `biome`, lande `heath`).
func suitability(model: Dictionary, probe: Dictionary, biome: int, heath: float, y: float) -> float:
	if (int(model["biomes"]) & (1 << biome)) == 0:
		return 0.0
	var entry: Dictionary = model["entry"]
	var h: float = probe["h"]
	var window := smoothstep(float(entry["min_altitude_m"]) - 80.0, float(entry["min_altitude_m"]) + 80.0, h) * (1.0 - smoothstep(float(entry["max_altitude_m"]) - 150.0, float(entry["max_altitude_m"]) + 150.0, h))
	if window <= 0.0:
		return 0.0
	var slope: float = probe["slope"]
	var min_slope := float(entry["min_slope"])
	var slope_term := smoothstep(min_slope, min_slope * 2.5 + 0.05, slope)
	var ridge_term := float(entry["ridge"]) * smoothstep(15.0, 110.0, float(probe["ridge"])) * smoothstep(min_slope * 0.5, min_slope * 1.2, slope)
	var heath_term := float(entry["heath"]) * heath * smoothstep(0.012, 0.05, slope)
	var treeline := _treeline_m(y)
	var alpine := 0.8 * smoothstep(treeline - 200.0, treeline + 300.0, h) * smoothstep(min_slope * 0.5, min_slope * 1.5, slope)
	return clampf(maxf(maxf(slope_term, ridge_term), maxf(heath_term, alpine)), 0.0, 1.0) * window * float(entry["density"])


## Semis d'une tuile (sans nœud ni terrain : appelable depuis une tâche) :
## {"parts": {modèle: {"buffer", "points", "radii"}}, "ms"}.
func _seed_tile(key: Vector2i) -> Dictionary:
	var t0 := Time.get_ticks_usec()
	var rect := _tile_rect(key)
	var parts: Dictionary = {}
	if not _tile_may_hold(rect):
		return {"parts": parts, "ms": (Time.get_ticks_usec() - t0) / 1000.0}
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(Vector3i(key.x, key.y, 0x4B5))
	var noise := FastNoiseLite.new()
	noise.seed = 0x4B5
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = float(settings.get("cluster_scale", 0.035))
	noise.fractal_octaves = 2
	var step := float(settings.get("grid_step", 2.0))
	var p_max := float(settings.get("max_probability", 0.4))
	var bias := float(settings.get("cluster_bias", 0.15))
	var mpp := map_data.meters_per_px
	var circles := PackedVector3Array()
	var reach := _max_radius() + 1.0
	for c in exclusions:
		if rect.grow(c.z + reach).has_point(Vector2(c.x, c.y)):
			circles.append(c)
	var roads: PackedVector2Array = _roads_by_tile.get(key, PackedVector2Array())
	var road_clearance := float(settings.get("road_clearance", 1.2))
	var river_clearance := float(settings.get("river_clearance", 1.5))
	var cells := int(ceil(rect.size.x / step))
	var scores := PackedFloat32Array()
	scores.resize(models.size())
	for j in cells:
		for i in cells:
			var p := rect.position + (Vector2(i, j) + Vector2(rng.randf(), rng.randf())) * step
			var roll := rng.randf()
			var pick := rng.randf()
			var size_t := rng.randf()
			var yaw := rng.randf() * TAU
			var stretch := rng.randf_range(0.85, 1.15)
			var tint := rng.randf()
			if not map_data.is_land_px(int(p.x), int(p.y)):
				continue
			var cluster := clampf(noise.get_noise_2d(p.x, p.y) + 0.5 + bias, 0.0, 1.0)
			if roll >= p_max * cluster:
				continue
			var probe := _terrain_probe(p.x, p.y)
			var biome := biome_at(p.x, p.y, probe["h"])
			var heath := 0.0
			if _mask != null and _mask.has_splat():
				var splat := _mask.splat_at(p.x, p.y)
				heath = splat.a * (1.0 - splat.b * 0.6)
			var total := 0.0
			var best := 0.0
			for m in models.size():
				scores[m] = suitability(models[m], probe, biome, heath, p.y)
				total += scores[m]
				best = maxf(best, scores[m])
			if best <= 0.0 or roll >= p_max * cluster * best:
				continue
			var chosen := 0
			var acc := 0.0
			for m in models.size():
				acc += scores[m]
				if pick * total <= acc and scores[m] > 0.0:
					chosen = m
					break
			var entry: Dictionary = models[chosen]["entry"]
			var size_units := lerpf(float(entry["size_m"][0]), float(entry["size_m"][1]), size_t) / mpp
			var radius := size_units * 0.5
			if not _clear_of_water(p, radius, river_clearance):
				continue
			if not circles.is_empty() and _in_circles(p, radius, circles):
				continue
			if not roads.is_empty() and _near_roads(p, radius + road_clearance, roads):
				continue
			if bool(entry.get("along_contour", false)):
				var grad: Vector2 = probe["grad"]
				if grad.length() > 1e-4:
					# Grand côté (X du modèle) le long de la courbe de niveau, un peu de jeu.
					yaw = -atan2(grad.x, grad.y) + PI * 0.5 + (yaw - PI) * 0.08
			var part: Dictionary = parts.get(chosen, {"buffer": PackedFloat32Array(), "points": PackedVector2Array(), "radii": PackedFloat32Array()})
			var b := Basis(Vector3.UP, yaw) * Basis.from_scale(Vector3(size_units * stretch, size_units, size_units / stretch))
			var buffer: PackedFloat32Array = part["buffer"]
			var o := buffer.size()
			buffer.resize(o + FLOATS_PER_INSTANCE)
			buffer[o] = b.x.x
			buffer[o + 1] = b.y.x
			buffer[o + 2] = b.z.x
			buffer[o + 3] = p.x
			buffer[o + 4] = b.x.y
			buffer[o + 5] = b.y.y
			buffer[o + 6] = b.z.y
			buffer[o + 8] = b.x.z
			buffer[o + 9] = b.y.z
			buffer[o + 10] = b.z.z
			buffer[o + 11] = p.y
			buffer[o + 12] = tint
			part["buffer"] = buffer
			(part["points"] as PackedVector2Array).append(p)
			(part["radii"] as PackedFloat32Array).append(radius)
			parts[chosen] = part
	for m: int in parts:
		_shuffle(parts[m], rng)
	return {"parts": parts, "ms": (Time.get_ticks_usec() - t0) / 1000.0}


## Pré-examen grossier : une tuile sans relief, sans altitude ni lande ne porte rien.
func _tile_may_hold(rect: Rect2) -> bool:
	var min_slope := INF
	var min_alt := INF
	var any_heath := false
	for model: Dictionary in models:
		var entry: Dictionary = model["entry"]
		min_slope = minf(min_slope, float(entry["min_slope"]))
		min_alt = minf(min_alt, float(entry["min_altitude_m"]))
		any_heath = any_heath or float(entry["heath"]) > 0.0
	for j in PRECHECK:
		for i in PRECHECK:
			var p := rect.position + (Vector2(i, j) + Vector2(0.5, 0.5)) * rect.size / PRECHECK
			if not map_data.is_land_px(int(p.x), int(p.y)):
				continue
			var probe := _terrain_probe(p.x, p.y)
			if float(probe["h"]) < min_alt - 80.0:
				continue
			if float(probe["slope"]) >= min_slope * 0.5:
				return true
			if float(probe["h"]) > _treeline_m(p.y) - 250.0:
				return true
			if any_heath and _mask != null and _mask.has_splat() and _mask.splat_at(p.x, p.y).a > 0.3 and float(probe["slope"]) > 0.012:
				return true
	return false


## Terre aux quatre coins et au centre de l'emprise, loin des berges.
func _clear_of_water(p: Vector2, radius: float, river_clearance: float) -> bool:
	for offset: Vector2 in [Vector2.ZERO, Vector2(radius, 0), Vector2(-radius, 0), Vector2(0, radius), Vector2(0, -radius)]:
		var q := p + offset
		if not map_data.is_land_px(int(q.x), int(q.y)):
			return false
		if map_data.height_m_at(q.x, q.y) <= 0.5:
			return false
		if map_data.river_sd_at(q.x, q.y) < river_clearance:
			return false
	return true


static func _in_circles(p: Vector2, radius: float, circles: PackedVector3Array) -> bool:
	for c in circles:
		var r := c.z + radius
		if p.distance_squared_to(Vector2(c.x, c.y)) < r * r:
			return true
	return false


static func _near_roads(p: Vector2, clearance: float, segments: PackedVector2Array) -> bool:
	var c2 := clearance * clearance
	for i in range(0, segments.size() - 1, 2):
		var a := segments[i]
		var ab := segments[i + 1] - a
		var t := clampf((p - a).dot(ab) / maxf(ab.length_squared(), 1e-6), 0.0, 1.0)
		if p.distance_squared_to(a + ab * t) < c2:
			return true
	return false


## Ordre aléatoire des instances (la part affichée coupe la fin du tampon).
static func _shuffle(part: Dictionary, rng: RandomNumberGenerator) -> void:
	var buffer: PackedFloat32Array = part["buffer"]
	var points: PackedVector2Array = part["points"]
	var radii: PackedFloat32Array = part["radii"]
	var count := points.size()
	for i in range(count - 1, 0, -1):
		var j := rng.randi_range(0, i)
		if i == j:
			continue
		var pi := points[i]
		points[i] = points[j]
		points[j] = pi
		var ri := radii[i]
		radii[i] = radii[j]
		radii[j] = ri
		for k in FLOATS_PER_INSTANCE:
			var v := buffer[i * FLOATS_PER_INSTANCE + k]
			buffer[i * FLOATS_PER_INSTANCE + k] = buffer[j * FLOATS_PER_INSTANCE + k]
			buffer[j * FLOATS_PER_INSTANCE + k] = v
	part["buffer"] = buffer
	part["points"] = points
	part["radii"] = radii


# --- Pose sur la surface --------------------------------------------------------------------


## Source de hauteurs lisible depuis un fil de travail (instantané des pages du quadtree), ou {}
## (heightmap de la carte).
func _height_source(rect: Rect2) -> Dictionary:
	if terrain == null or terrain.quadtree == null or terrain.map_data == null:
		return {}
	return terrain.quadtree.surface_snapshot(rect, Vector2.ZERO)


func _surface_at(source: Dictionary, x: float, y: float) -> float:
	if source.is_empty():
		return map_data.surface_world_at(x, y)
	return maxf(ReliefQuadtree.sample_snapshot(source, x, y), 0.0)


## Pied de chaque instance au plus bas de son emprise (centre et quatre points à 0,6 rayon) :
## l'affleurement sort de la pente au lieu de flotter au-dessus du versant aval.
func _grounded(seeded: Dictionary, source: Dictionary) -> Dictionary:
	var parts: Dictionary = seeded["parts"]
	for m: int in parts:
		var part: Dictionary = parts[m]
		part["buffer"] = _regrounded(part, source)
	return seeded


func _regrounded(part: Dictionary, source: Dictionary) -> PackedFloat32Array:
	var buffer: PackedFloat32Array = (part["buffer"] as PackedFloat32Array).duplicate()
	var points: PackedVector2Array = part["points"]
	var radii: PackedFloat32Array = part["radii"]
	for n in points.size():
		var p := points[n]
		var r := radii[n] * 0.6
		var h := _surface_at(source, p.x, p.y)
		for offset: Vector2 in [Vector2(r, 0), Vector2(-r, 0), Vector2(0, r), Vector2(0, -r)]:
			h = minf(h, _surface_at(source, p.x + offset.x, p.y + offset.y))
		buffer[n * FLOATS_PER_INSTANCE + 7] = h - radii[n] * 0.04
	return buffer


## Pose (fil principal) : un `MultiMeshInstance3D` par modèle.
func _install_tile(key: Vector2i, seeded: Dictionary) -> void:
	var t0 := Time.get_ticks_usec()
	stats["seed_ms_max"] = maxf(float(stats["seed_ms_max"]), float(seeded.get("ms", 0.0)))
	var entry := {"parts": [], "last_seen": _frame, "lod": -1, "share": 0.0}
	var parts: Dictionary = seeded.get("parts", {})
	for m: int in parts:
		var part: Dictionary = parts[m]
		var buffer: PackedFloat32Array = part["buffer"]
		if buffer.is_empty():
			continue
		var mmi := MultiMeshInstance3D.new()
		mmi.name = "Outcrop_%s_%d_%d" % [models[m]["id"], key.x, key.y]
		mmi.material_override = models[m]["material"]
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		mmi.gi_mode = GeometryInstance3D.GI_MODE_DISABLED
		mmi.visible = false
		var record := {"mmi": mmi, "model": m, "buffer": buffer, "points": part["points"], "radii": part["radii"], "aabb": _aabb_of(buffer, part["radii"])}
		mmi.multimesh = _multimesh(models[m]["meshes"][2], buffer, record["aabb"])
		add_child(mmi)
		entry["parts"].append(record)
	_tiles[key] = entry
	stats["tiles"] = _tiles.size()
	stats["install_ms_max"] = maxf(float(stats["install_ms_max"]), (Time.get_ticks_usec() - t0) / 1000.0)


## Boîte englobante généreuse : grossissement lointain et exagération verticale maximale.
func _aabb_of(buffer: PackedFloat32Array, radii: PackedFloat32Array) -> AABB:
	var lo := Vector3(INF, INF, INF)
	var hi := Vector3(-INF, -INF, -INF)
	var grow := float(settings.get("far_scale", 2.6)) * 1.1
	for n in radii.size():
		var o := n * FLOATS_PER_INSTANCE
		var c := Vector3(buffer[o + 3], buffer[o + 7], buffer[o + 11])
		var r := radii[n] * 2.0 * grow
		lo = lo.min(c - Vector3(r, r, r))
		hi = hi.max(c + Vector3(r, r * 6.0, r))
	return AABB(lo, hi - lo)


static func _multimesh(mesh: Mesh, buffer: PackedFloat32Array, aabb: AABB) -> MultiMesh:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_custom_data = true
	multimesh.mesh = mesh
	multimesh.instance_count = buffer.size() / FLOATS_PER_INSTANCE
	if multimesh.instance_count > 0:
		multimesh.buffer = buffer
	multimesh.custom_aabb = aabb
	return multimesh


## Niveau de détail d'une tuile : MultiMesh recréés depuis la copie processeur (changer `mesh`
## après `buffer` relirait le tampon au GPU, comme `GroundClutter._update_rock_lod`).
func _set_tile_lod(entry: Dictionary, lod: int) -> void:
	if int(entry["lod"]) == lod:
		return
	entry["lod"] = lod
	var shadow_lod := int(settings.get("shadow_max_lod", 0))
	for part: Dictionary in entry["parts"]:
		var mmi: MultiMeshInstance3D = part["mmi"]
		var mesh: Mesh = models[int(part["model"])]["meshes"][lod]
		if mmi.multimesh.mesh != mesh:
			mmi.multimesh = _multimesh(mesh, part["buffer"], part["aabb"])
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if lod <= shadow_lod else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF


# --- Recalage -------------------------------------------------------------------------------


func _on_chunk_surface_changed(index: int) -> void:
	if terrain == null or terrain.chunk_px <= 0:
		return
	var origin := Vector2((index % terrain.chunks_x) * terrain.chunk_px, (index / terrain.chunks_x) * terrain.chunk_px)
	_on_surface_rect_changed(Rect2(origin, Vector2(terrain.chunk_px, terrain.chunk_px)))


func _on_surface_rect_changed(rect: Rect2) -> void:
	var reach := rect.grow(_max_radius() + 1.0)
	for key: Vector2i in _tiles:
		if _tile_rect(key).intersects(reach, true):
			_dirty[key] = true
	for jobs: Dictionary in [_jobs, _ground_jobs]:
		for key: Vector2i in jobs:
			if _tile_rect(key).intersects(reach, true):
				(jobs[key] as Dictionary)["stale"] = true


func _update_regrounds() -> void:
	for key: Vector2i in _ground_jobs.keys():
		var holder: Dictionary = _ground_jobs[key]
		if not WorkerThreadPool.is_task_completed(int(holder["task"])):
			continue
		WorkerThreadPool.wait_for_task_completion(int(holder["task"]))
		_ground_jobs.erase(key)
		var entry: Dictionary = _tiles.get(key, {})
		if not entry.is_empty() and is_same(entry["parts"], holder["parts"]):
			var buffers: Array = holder["result"]
			for n in buffers.size():
				var part: Dictionary = entry["parts"][n]
				part["buffer"] = buffers[n]
				var mmi: MultiMeshInstance3D = part["mmi"]
				mmi.multimesh.buffer = buffers[n]
		if holder.get("stale", false):
			_dirty[key] = true
	var started := 0
	for key: Vector2i in _dirty.keys():
		if started >= max_reground_per_frame:
			break
		if _ground_jobs.has(key):
			continue
		_dirty.erase(key)
		if not _tiles.has(key) or (_tiles[key]["parts"] as Array).is_empty():
			continue
		var parts: Array = _tiles[key]["parts"]
		var source := _height_source(_tile_rect(key).grow(_max_radius() * 2.0 + 2.0))
		var holder := {"parts": parts, "result": []}
		var job := func() -> void:
			var buffers: Array = []
			for part: Dictionary in parts:
				buffers.append(_regrounded(part, source))
			holder["result"] = buffers
		if threaded:
			holder["task"] = WorkerThreadPool.add_task(job, false, "RockOutcrops reground")
			_ground_jobs[key] = holder
		else:
			job.call()
			for n in parts.size():
				parts[n]["buffer"] = holder["result"][n]
				(parts[n]["mmi"] as MultiMeshInstance3D).multimesh.buffer = holder["result"][n]
		started += 1


# --- Comptes et cache -----------------------------------------------------------------------


## Instances affichées : `far_share` sur les tuiles en LOD2, `fade` à l'effacement, puis part
## commune réduite si les triangles dépassent `max_visible_triangles`.
func _apply_counts(fade: float) -> void:
	var far_share := float(settings.get("far_share", 0.55))
	var cap := float(settings.get("max_visible_triangles", 900000))
	var total := 0.0
	for entry: Dictionary in _tiles.values():
		var share := (far_share if int(entry["lod"]) >= 2 else 1.0) * fade
		entry["share"] = share
		for part: Dictionary in entry["parts"]:
			var mmi: MultiMeshInstance3D = part["mmi"]
			if mmi.visible:
				total += mmi.multimesh.instance_count * share * int(models[int(part["model"])]["triangles"][maxi(int(entry["lod"]), 0)])
	var factor := minf(1.0, cap / maxf(total, 1.0))
	var shown := 0
	var triangles := 0
	var calls := 0
	var instances := 0
	for entry: Dictionary in _tiles.values():
		var share := float(entry["share"]) * factor
		for part: Dictionary in entry["parts"]:
			var mmi: MultiMeshInstance3D = part["mmi"]
			var n := int(ceil(mmi.multimesh.instance_count * share))
			mmi.multimesh.visible_instance_count = n
			instances += mmi.multimesh.instance_count
			if mmi.visible:
				shown += n
				calls += 1 if n > 0 else 0
				triangles += n * int(models[int(part["model"])]["triangles"][maxi(int(entry["lod"]), 0)])
	stats["visible"] = shown
	stats["triangles"] = triangles
	stats["draw_calls"] = calls
	stats["instances"] = instances
	stats["share_factor"] = factor
	stats["scale"] = _scale


func _evict() -> void:
	var cap := int(settings.get("max_cached_tiles", 96))
	if _tiles.size() <= cap:
		return
	var stale: Array = []
	for key: Vector2i in _tiles:
		if int(_tiles[key]["last_seen"]) < _frame:
			stale.append(key)
	stale.sort_custom(func(a: Vector2i, b: Vector2i) -> bool: return int(_tiles[a]["last_seen"]) < int(_tiles[b]["last_seen"]))
	for n in mini(_tiles.size() - cap, stale.size()):
		var key: Vector2i = stale[n]
		for part: Dictionary in _tiles[key]["parts"]:
			(part["mmi"] as Node).queue_free()
		_tiles.erase(key)
		_dirty.erase(key)
	stats["tiles"] = _tiles.size()
