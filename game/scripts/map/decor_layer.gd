class_name DecorLayer
extends Node3D

## Lots DN ME6/ME7/ME9 : décor ponctuel hors les villes de la carte de campagne (croix de chemin,
## gibets, péages, phares, mines, forges, mégalithes, ruines romaines, champs de bataille passés,
## pèlerins, foires, steppe…). Rendu seulement : aucune règle de jeu.
## - Placement : `DecorPlanner` (règles et sites de `data/map/map_landmarks_extra.json`), calculé
##   hors du fil principal dès `setup`, déterministe.
## - Rendu : un `MultiMesh` par type pour le voisinage de la caméra (au plus un appel de dessin par
##   type), reconstruit quand la caméra s'éloigne du centre du voisinage, que la distance change
##   de plus de 4 %, ou que l'année, la saison ou le brouillard de guerre changent. Pas de nœud
##   par instance.
## - Taille : réelle de près, puis tenue à l'écran (`fraction` du type) comme les bâtiments hors
##   les murs (TB3, ADR 0162) ; en style `maquette` (GC2, ADR 0158) taille monde constante, celle
##   d'un objet vu à `maquette.size_distance`.
## - Modèle d'un type : `model` = id du catalogue DN (`data/art/dn_manifest.json`, fichiers
##   `_lod0/1/2.glb`). Tant que le modèle manque, volume procédural (`shape`, `color`, `size_m`).
##   Brancher un nouveau modèle = rien à faire (le manifeste suffit) ou changer `model`.
## - Vue parchemin : rien n'est affiché au-delà de `view_range_units` (bien sous le palier 1200).
## - `--no-me6` coupe la couche (A/B).

const MANIFEST_FILE := "art/dn_manifest.json"
const MODEL_ROOT := "res://assets/models/"
const CELL_SEARCH_LIMIT := 4096
## Images entre deux lectures du brouillard de guerre.
const FOG_PERIOD := 20
## Images d'attente avant de recaler les hauteurs après un changement de surface du relief.
const REGROUND_DELAY := 20

var config: Dictionary = {}
var enabled := true
## Affiche et construit quelle que soit la distance du rig (tests headless).
var force_active := false
## Objet portant `hidden_provinces` (id -> true), par défaut les marqueurs d'armée voisins.
var fog_source: Object = null
var stats: Dictionary = {}

var _layer: Node
var _map: MapData
var _terrain: TerrainBuilder
var _data: SettlementData
var _mpu := 719.0
var _maquette := false
var _task := -1
var _planner: DecorPlanner
var _sites: Array = []
var _cells: Dictionary = {}  # Vector2i -> PackedInt32Array
var _manifest: Dictionary = {}
var _meshes: Dictionary = {}  # type -> Array de maillages (un par LOD)
var _sources: Dictionary = {}  # type -> "glb" ou "procedural"
var _materials: Dictionary = {}
var _root: Node3D
var _batches: Dictionary = {}  # type -> MultiMeshInstance3D
var _shown: Array = []  # indices des instances affichées
var _center := Vector2(INF, INF)
var _radius := 0.0
var _rig_distance := 1000.0
var _placed_distance := -1.0
var _dirty := true
var _year := 0
var _season := ""
var _fog_hash := 0
var _fog_in := 0
var _fade := 1.0
var _reground_in := -1
var _shadows := true


func setup(layer: Node, map: MapData, terrain: TerrainBuilder, data: SettlementData, meters_per_unit: float = 719.0) -> void:
	_layer = layer
	_map = map
	_terrain = terrain
	_data = data
	_mpu = meters_per_unit
	var data_dir := OutbuildingLayer.data_dir()
	config = DecorPlanner.load_config(data_dir)
	_maquette = TownMaquetteData.enabled()
	enabled = enabled and not config.is_empty() and not CmdArgs.has("--no-me6")
	visible = false
	_root = Node3D.new()
	_root.name = "Batches"
	add_child(_root)
	_manifest = _load_manifest(data_dir)
	stats = {"instances": 0, "nodes": 0, "sites": 0, "build_ms": 0.0}
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	if enabled:
		_task = WorkerThreadPool.add_task(_plan_task.bind(data_dir), false, "decor plan")


static func _load_manifest(data_dir: String) -> Dictionary:
	var path := data_dir.path_join(MANIFEST_FILE)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return (parsed as Dictionary).get("assets", {}) if parsed is Dictionary else {}


func _plan_task(data_dir: String) -> void:
	_planner = DecorPlanner.new()
	_planner.setup(config, _map, _data, DecorPlanner.load_province_info(data_dir), DecorPlanner.load_crossings(data_dir))
	_planner.plan()


## Vrai quand le placement est terminé (l'attend au besoin s'il est lancé).
func is_planned() -> bool:
	return _task < 0 and _planner != null


func _finish_plan(block: bool) -> bool:
	if _task < 0:
		return _planner != null
	if not block and not WorkerThreadPool.is_task_completed(_task):
		return false
	WorkerThreadPool.wait_for_task_completion(_task)
	_task = -1
	_sites = _planner.instances
	stats["sites"] = _sites.size()
	stats["plan_ms"] = _planner.stats.get("plan_ms", 0.0)
	var cell := float(_render("cell_px", 24.0))
	for i in _sites.size():
		var px: Vector2 = _sites[i]["px"]
		var key := Vector2i(floori(px.x / cell), floori(px.y / cell))
		var indices: PackedInt32Array = _cells.get(key, PackedInt32Array())
		indices.append(i)
		_cells[key] = indices
	_dirty = true
	return true


func _exit_tree() -> void:
	if _task >= 0:
		WorkerThreadPool.wait_for_task_completion(_task)
		_task = -1


# --- Données de rendu ----------------------------------------------------------------------


func _render(key: String, fallback: Variant) -> Variant:
	var render: Dictionary = config.get("render", {})
	if _maquette:
		var maquette: Dictionary = render.get("maquette", {})
		if maquette.has(key):
			return maquette[key]
	return render.get(key, fallback)


func view_range() -> float:
	return float(_render("view_range_units", 70.0))


## Multiplicateur des portées des types et des distances LOD.
func distance_scale() -> float:
	return float((config.get("render", {}) as Dictionary).get("maquette", {}).get("distance_scale", 1.0)) if _maquette else 1.0


func load_radius() -> float:
	return clampf(float(_render("load_factor", 2.6)) * _rig_distance, float(_render("load_min_units", 8.0)), float(_render("load_max_units", 140.0)))


## Distance du rig qui fixe la taille (constante en style maquette).
func _size_distance() -> float:
	return float(_render("size_distance", _rig_distance)) if _maquette else _rig_distance


## Facteur de grossissement d'un type : 1 à l'échelle réelle, puis largeur tenue à l'écran.
func factor_of(spec: Dictionary, rig_distance: float) -> float:
	var render: Dictionary = config.get("render", {})
	var screen: Dictionary = render.get("screen", {})
	var low := float(screen.get("real_below", 8.0))
	var blend := smoothstep(low, maxf(float(screen.get("full_from", 14.0)), low + 1e-3), rig_distance)
	var span := 2.0 * tan(deg_to_rad(float(screen.get("fov_deg", 55.0))) * 0.5) * rig_distance
	var width: float = (spec["size_m"] as Array)[0]
	var full := clampf(float(spec["fraction"]) * span * _mpu / maxf(width, 0.1), 1.0, float(screen.get("max_factor", 200.0)))
	return lerpf(1.0, full, blend)


func lod_of(rig_distance: float) -> int:
	var distances: Array = (config.get("render", {}) as Dictionary).get("lod_distances", [14.0, 38.0])
	var scale := distance_scale()
	if rig_distance < float(distances[0]) * scale:
		return 0
	return 1 if rig_distance < float(distances[1]) * scale else 2


# --- Maillages -----------------------------------------------------------------------------


## Maillage du type au niveau de détail `lod` : glb du manifeste DN s'il existe, volume de repli sinon.
func _mesh_of(type: String, lod: int) -> Mesh:
	if not _meshes.has(type):
		_meshes[type] = _build_meshes(type)
	var meshes: Array = _meshes[type]
	return meshes[mini(lod, meshes.size() - 1)]


func _build_meshes(type: String) -> Array:
	var spec: Dictionary = config["types"][type]
	var entry: Dictionary = _manifest.get(str(spec["model"]), {})
	var out: Array = []
	for file: String in entry.get("files", []):
		var mesh := OutbuildingLayer._load_glb_mesh(MODEL_ROOT + file)
		if mesh != null:
			out.append(mesh)
	_sources[type] = "glb" if not out.is_empty() else "procedural"
	if out.is_empty():
		out.append(procedural_mesh(str(spec["shape"]), spec["size_m"], Color(spec["color"][0], spec["color"][1], spec["color"][2])))
	return out


## Origine du maillage d'un type : "glb" (manifeste DN) ou "procedural" (repli).
func model_source(type: String) -> String:
	_mesh_of(type, 0)
	return str(_sources.get(type, ""))


## Remplace le manifeste DN (tests : brancher un glb sur un type) et jette les maillages construits.
func set_manifest(assets: Dictionary) -> void:
	_manifest = assets
	_meshes = {}
	_sources = {}
	_dirty = true
	for mmi: MultiMeshInstance3D in _batches.values():
		mmi.free()
	_batches = {}


func _material_of(type: String, mesh: Mesh) -> Material:
	if mesh.get_surface_count() > 0 and mesh.surface_get_material(0) != null:
		return null  # le glb porte son matériau
	if not _materials.has("flat"):
		var material := StandardMaterial3D.new()
		material.vertex_color_use_as_albedo = true
		material.roughness = 0.95
		material.cull_mode = BaseMaterial3D.CULL_DISABLED
		_materials["flat"] = material
	return _materials["flat"]


## Volume de repli (mètres, pied en y = 0) : `pillar`, `hut`, `tower`, `cone`, `slab`, `ring`, `arch`.
static func procedural_mesh(shape: String, size_m: Array, color: Color) -> Mesh:
	var width := float(size_m[0])
	var height := float(size_m[1])
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	tool.set_color(color)
	match shape:
		"pillar":
			_box(tool, Vector3.ZERO, Vector3(width, height * 0.8, width), color)
			_box(tool, Vector3(0.0, height * 0.8, 0.0), Vector3(width * 0.6, height * 0.2, width * 0.6), color.lightened(0.1))
		"hut":
			_box(tool, Vector3.ZERO, Vector3(width, height * 0.55, width * 0.6), color)
			_prism(tool, Vector3(0.0, height * 0.55, 0.0), Vector3(width * 1.08, height * 0.45, width * 0.66), color.darkened(0.35))
		"tower":
			_cylinder(tool, 0.0, height * 0.78, width * 0.5, 8, color)
			_cone(tool, height * 0.78, height * 0.22, width * 0.58, 8, color.darkened(0.3))
		"cone":
			_cone(tool, 0.0, height, width * 0.5, 8, color)
		"slab":
			_box(tool, Vector3.ZERO, Vector3(width, height, width * 0.35), color)
		"ring":
			for i in 8:
				var angle := TAU * float(i) / 8.0
				_box(tool, Vector3(cos(angle), 0.0, sin(angle)) * width * 0.5, Vector3(width * 0.12, height, width * 0.07), color)
		"arch":
			_box(tool, Vector3(-width * 0.4, 0.0, 0.0), Vector3(width * 0.16, height, width * 0.2), color)
			_box(tool, Vector3(width * 0.4, 0.0, 0.0), Vector3(width * 0.16, height, width * 0.2), color)
			_box(tool, Vector3(0.0, height * 0.82, 0.0), Vector3(width, height * 0.18, width * 0.2), color.darkened(0.08))
		_:
			_box(tool, Vector3.ZERO, Vector3(width, height, width), color)
	tool.generate_normals()
	return tool.commit()


## Boîte de centre de base `at` (pied en y de `at`), dimensions `size`.
static func _box(tool: SurfaceTool, at: Vector3, size: Vector3, color: Color) -> void:
	var h := size * 0.5
	var corners: Array[Vector3] = []
	for sx in [-1.0, 1.0]:
		for sy in [0.0, 1.0]:
			for sz in [-1.0, 1.0]:
				corners.append(at + Vector3(h.x * sx, size.y * sy, h.z * sz))
	# corners index = sx*4 + sy*2 + sz (bits)
	var faces := [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]]
	for face: Array in faces:
		_quad(tool, corners[face[0]], corners[face[1]], corners[face[2]], corners[face[3]], color)


static func _quad(tool: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3, color: Color) -> void:
	for p in [a, b, c, a, c, d]:
		tool.set_color(color)
		tool.add_vertex(p)


static func _prism(tool: SurfaceTool, at: Vector3, size: Vector3, color: Color) -> void:
	var h := size * 0.5
	var a := at + Vector3(-h.x, 0.0, -h.z)
	var b := at + Vector3(h.x, 0.0, -h.z)
	var c := at + Vector3(h.x, 0.0, h.z)
	var d := at + Vector3(-h.x, 0.0, h.z)
	var r0 := at + Vector3(-h.x, size.y, 0.0)
	var r1 := at + Vector3(h.x, size.y, 0.0)
	_quad(tool, b, a, r0, r1, color)
	_quad(tool, d, c, r1, r0, color)
	for p in [a, r0, d, b, c, r1]:
		tool.set_color(color)
		tool.add_vertex(p)


static func _cylinder(tool: SurfaceTool, y0: float, height: float, radius: float, segments: int, color: Color) -> void:
	for i in segments:
		var a0 := TAU * float(i) / float(segments)
		var a1 := TAU * float(i + 1) / float(segments)
		var p0 := Vector3(cos(a0) * radius, y0, sin(a0) * radius)
		var p1 := Vector3(cos(a1) * radius, y0, sin(a1) * radius)
		_quad(tool, p0, p0 + Vector3.UP * height, p1 + Vector3.UP * height, p1, color)


static func _cone(tool: SurfaceTool, y0: float, height: float, radius: float, segments: int, color: Color) -> void:
	var tip := Vector3(0.0, y0 + height, 0.0)
	for i in segments:
		var a0 := TAU * float(i) / float(segments)
		var a1 := TAU * float(i + 1) / float(segments)
		for p in [Vector3(cos(a0) * radius, y0, sin(a0) * radius), tip, Vector3(cos(a1) * radius, y0, sin(a1) * radius)]:
			tool.set_color(color)
			tool.add_vertex(p)


# --- Simulation (année, saison) ------------------------------------------------------------


## Lit l'année et la saison de la simulation ; une instance dont la période ou la saison ne
## convient pas n'est pas affichée. Rend vrai si quelque chose a changé.
func refresh(sim: Object) -> bool:
	if sim == null or not sim.has_method("get_date_label"):
		return false
	var label := str(sim.call("get_date_label"))
	var year := LandmarkModel.year_of(label)
	var season := SeasonVisuals.season_from_label(label)
	if year == _year and season == _season:
		return false
	_year = year
	_season = season
	_dirty = true
	return true


func set_date(year: int, season: String) -> void:
	if year != _year or season != _season:
		_year = year
		_season = season
		_dirty = true


## Instance visible à l'année et la saison courantes.
func _in_period(instance: Dictionary) -> bool:
	if _year > 0:
		if instance.has("from_year") and _year < int(instance["from_year"]):
			return false
		if instance.has("to_year") and _year > int(instance["to_year"]):
			return false
	if _season != "" and instance.has("seasons") and not (instance["seasons"] as Array).has(_season):
		return false
	return true


func _hidden_provinces() -> Dictionary:
	if fog_source == null and _layer != null and _layer.get_parent() != null:
		fog_source = _layer.get_parent().get_node_or_null("Armies")
	var hidden: Variant = fog_source.get("hidden_provinces") if fog_source != null else null
	return hidden if hidden is Dictionary else {}


# --- Vue -----------------------------------------------------------------------------------


func update_view(rig_distance: float) -> void:
	if not enabled or _data == null:
		visible = false
		return
	_rig_distance = rig_distance
	var shown := force_active or rig_distance < view_range()
	if shown != visible:
		visible = shown
	if not shown:
		return
	if not _finish_plan(false):
		return
	_fog_in -= 1
	if _fog_in <= 0:
		_fog_in = FOG_PERIOD
		var fog := _hidden_provinces().hash()
		if fog != _fog_hash:
			_fog_hash = fog
			_dirty = true
	var focus := _camera_ground()
	if _dirty or _center.x == INF or focus.distance_to(_center) > 0.3 * _radius or absf(rig_distance - _placed_distance) > 0.04 * maxf(_placed_distance, 0.1):
		rebuild(focus)
	elif _reground_in >= 0:
		_reground_in -= 1
		if _reground_in < 0:
			rebuild(_center)
	var opacity := 1.0 if force_active else clampf(inverse_lerp(view_range(), minf(float(_render("fade_from_units", view_range())), view_range() - 1e-3), rig_distance), 0.0, 1.0)
	if absf(opacity - _fade) > 0.02 or (opacity >= 1.0) != (_fade >= 1.0):
		_fade = opacity
		for mmi: MultiMeshInstance3D in _batches.values():
			mmi.transparency = 1.0 - _fade


## Point du sol visé par la caméra (unités carte) ; position de la caméra en repli.
func _camera_ground() -> Vector2:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return _center if _center.x != INF else Vector2.ZERO
	var origin := camera.global_position
	var forward := -camera.global_transform.basis.z
	if forward.y < -0.05:
		origin += forward * (origin.y / -forward.y)
	return Vector2(origin.x, origin.z)


## Termine tout de suite la construction autour de `center` (tests, captures).
func flush(center: Variant = null) -> void:
	if not enabled or _data == null or not (visible or force_active) or not _finish_plan(true):
		return
	rebuild(center if center is Vector2 else _camera_ground())


func _on_chunk_surface_changed(_index: int) -> void:
	if _terrain != null and _terrain.rescaling_vertical:
		return
	if visible and not _shown.is_empty():
		_reground_in = REGROUND_DELAY


## Instances du voisinage de `focus`, filtrées (période, portée du type, brouillard), les plus
## proches d'abord jusqu'à `max_instances`.
func gather(focus: Vector2) -> Array:
	var radius := load_radius()
	var cell := float(_render("cell_px", 24.0))
	var scale := distance_scale()
	var hidden := _hidden_provinces()
	var types: Dictionary = config["types"]
	var found: Array = []
	var low := Vector2i(floori((focus.x - radius) / cell), floori((focus.y - radius) / cell))
	var high := Vector2i(floori((focus.x + radius) / cell), floori((focus.y + radius) / cell))
	for cy in range(low.y, high.y + 1):
		for cx in range(low.x, high.x + 1):
			var indices: PackedInt32Array = _cells.get(Vector2i(cx, cy), PackedInt32Array())
			for i in indices:
				var instance: Dictionary = _sites[i]
				var px: Vector2 = instance["px"]
				var d := px.distance_to(focus)
				if d > radius:
					continue
				if _rig_distance > float((types[instance["type"]] as Dictionary)["max_distance"]) * scale and not force_active:
					continue
				if not _in_period(instance) or hidden.has(str(instance["province"])):
					continue
				found.append([d, i])
	found.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var cap := int(_render("max_instances", 5000))
	if found.size() > cap:
		found.resize(cap)
	return found.map(func(e: Array) -> int: return e[1])


func rebuild(focus: Vector2) -> void:
	var t0 := Time.get_ticks_usec()
	_dirty = false
	_reground_in = -1
	_center = focus
	_radius = load_radius()
	_placed_distance = _rig_distance
	_shown = gather(focus)
	var size_distance := _size_distance()
	var lod := lod_of(_rig_distance)
	var groups := {}
	for i: int in _shown:
		var type: String = _sites[i]["type"]
		if not groups.has(type):
			groups[type] = []
		(groups[type] as Array).append(i)
	for type: String in _batches.keys():
		if not groups.has(type):
			(_batches[type] as Node).free()
			_batches.erase(type)
	var total := 0
	for type: String in groups:
		var spec: Dictionary = config["types"][type]
		var factor := factor_of(spec, size_distance)
		var scale := factor / _mpu
		var indices: Array = groups[type]
		var mesh := _mesh_of(type, lod)
		var mmi: MultiMeshInstance3D = _batches.get(type)
		if mmi == null:
			mmi = MultiMeshInstance3D.new()
			mmi.name = type
			mmi.multimesh = MapInstancing.make(mesh, 0)
			mmi.transparency = 1.0 - _fade
			var material := _material_of(type, mesh)
			if material != null:
				mmi.material_override = material
			_root.add_child(mmi)
			_batches[type] = mmi
		elif mmi.multimesh.mesh != mesh:
			mmi.multimesh.mesh = mesh
			mmi.material_override = _material_of(type, mesh)
		mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if _shadows and _rig_distance < float(_render("shadow_range_units", 16.0)) * distance_scale() else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var multimesh := mmi.multimesh
		multimesh.instance_count = indices.size()
		for n in indices.size():
			var instance: Dictionary = _sites[indices[n]]
			var px: Vector2 = instance["px"]
			var y := _ground_y(px)
			var basis := Basis(Vector3.UP, float(instance["yaw"])).scaled(Vector3(scale, scale, scale))
			multimesh.set_instance_transform(n, Transform3D(basis, Vector3(px.x, y, px.y)))
		total += indices.size()
	stats["instances"] = total
	stats["nodes"] = _batches.size()
	stats["build_ms"] = float(Time.get_ticks_usec() - t0) / 1000.0


func _ground_y(px: Vector2) -> float:
	if _terrain != null:
		return _terrain.surface_height_at(px.x, px.y)
	return _map.surface_world_at(px.x, px.y) if _map != null else 0.0


# --- Lecture pour les tests ----------------------------------------------------------------


func sites() -> Array:
	return _sites


func shown_instances() -> Array:
	return _shown.map(func(i: int) -> Dictionary: return _sites[i])


func instance_count() -> int:
	return int(stats.get("instances", 0))


## Nombre de nœuds de rendu (un appel de dessin par nœud et par passe).
func node_count() -> int:
	return _batches.size()


## Positions (px carte) des sources de fumée proches de `center` (forges, charbonnières, verreries),
## pour `LifeEffects`.
func smoke_sources(center: Vector2, radius: float) -> Array:
	var out: Array = []
	if not is_planned():
		return out
	var types: Dictionary = config["types"]
	for instance: Dictionary in _sites:
		if bool((types[instance["type"]] as Dictionary).get("smoke", false)) and (instance["px"] as Vector2).distance_to(center) <= radius and _in_period(instance):
			out.append(instance["px"])
	return out
