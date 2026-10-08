class_name OutbuildingLayer
extends Node3D

## Lot TB3 (ADR 0162) : bâtiments hors les murs de la carte de campagne (ferme, moulin, vignoble,
## mine, saline, abbaye, marché, port), posés sur le terroir autour des colonies selon les
## bâtiments construits que le moteur expose (`CampaignSim.get_settlements_live`, sinon
## `settlement_detail`), au niveau 1 à 3 donné par `data/map/building_models.json`. Rendu
## seulement : aucune règle de jeu.
## - Maquettes : `game/assets/models/outbuildings/` (une surface `Building` par maquette, même
##   matériau atlas que les villes 1:1, `TownBuilder.material`).
## - Rendu : un `MultiMesh` par maillage pour tout le voisinage de la caméra (au plus un appel de
##   dessin par maquette distincte), reconstruit par petites étapes quand la caméra s'éloigne du
##   centre du voisinage ou que l'état de la simulation change. Hauteurs de base en mètres, posées
##   par le shader (`campaign_display_height`).
## - **Taille tenue à l'écran** (ADR 0162, `render.screen`) : échelle réelle de près (ADR 0138),
##   puis chaque maquette garde une largeur d'écran selon son niveau (grossissement proportionnel
##   à la distance du rig), jusqu'au fondu de sortie. Les maquettes s'écartent alors de la ville
##   et les unes des autres dans la même proportion : mise en place par palier de distance
##   (`band_top`), sans recouvrement, hors mer, fleuves, routes principales et autres colonies ;
##   celles qui ne tiennent pas sont retirées à ce palier.
## - Brouillard de guerre : rien n'est posé dans une province hors de vue.
## - Les autres pièces de la croissance (faubourgs, enceinte, lot TB3 point 4) passent par le même
##   rendu : `TownGrowth` fournit ses instances par colonie.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const DATA_FILE := "map/building_models.json"
const MODEL_DIR := "res://assets/models/outbuildings/"
const WORKSITE_FAMILY := "worksite"
const WORKSITE_MODEL := "worksite_1"
## Écarts d'angle essayés autour du secteur d'une famille (en tiers de secteur), du plus proche
## au plus lointain.
const ANGLE_STEPS: Array[int] = [0, -1, 1, -2, 2, -4, 4, -6, 6, -9, 9, -12, 12]
const SHORE_DIRECTIONS := 16
## Temps (µs) donné à la reconstruction du voisinage par image.
const STEP_BUDGET_USEC := 1500
## Images d'attente avant de recaler les hauteurs après un changement de surface du relief.
const REGROUND_DELAY := 20
## Côté (unités) des cases de la grille de voisinage des colonies (emprises voisines à éviter).
const GRID_UNITS := 8.0
## Demi-angle (rad) laissé libre autour de la route de chaque porte.
const GATE_CORRIDOR := 0.2
## Côté (unités) des cases de la grille des emprises tenues à l'écran, et des routes principales.
const DISC_CELL := 18.0
const ROAD_CELL := 2.0
## Écart entre anneaux successifs de la mise en place de loin, en rayons de la maquette.
const RING_STEP := 2.15
## Ordre de mise en place par genre de colonie (les cités d'abord).
const KIND_RANK := {"city": 0, "town": 1}

var config: Dictionary = {}
var manifest: Dictionary = {}
var stats: Dictionary = {}
var enabled := true
## Affiche et construit quelle que soit la distance du rig (tests headless).
var force_active := false
## États imposés (captures de contrôle, comme `CampaignLife.forced_devastation`) : id de colonie
## → `{buildings: Array, building: bool}` à la place de l'état de la simulation ; province →
## rapport de population imposé. Appeler `invalidate()` après un changement.
var forced_states: Dictionary = {}
var forced_population_ratio: Dictionary = {}
## Brouillard de guerre : objet portant `hidden_provinces` (id → true), par défaut les marqueurs
## d'armée de la carte (`MinimapController.refresh_fog`). Null : rien n'est caché.
var fog_source: Object = null

var _layer: SettlementLayer
var _map: MapData
var _terrain: TerrainBuilder
var _data: SettlementData
var _mpu := 719.0
var _sim: Object = null
var _revision := -2
var _states: Dictionary = {}  # id de colonie → {buildings, fortification, building}
var _resources: Dictionary = {}  # province → Array d'ids de ressource
var _provinces: Dictionary = {}  # province → {population, devastation, besieged, constructing}
var _live: Dictionary = {}  # `get_settlements_live` de l'état courant (tableaux alignés)
var _live_index: Dictionary = {}  # id de colonie → rang dans `_live`
var _stale := true
## Distance de mise à l'échelle : celle du rig (style `real`), ou la distance fixe
## `render.maquette.size_distance` (style `maquette`, ADR 0158 : taille monde constante).
var _rig_distance := 10.0
var _view_distance := 10.0
var _maquette := false
var _loaded_radius := 0.0
var _baseline: Dictionary = {}  # province → population de 1337 (données)
var _soot: Dictionary = {}  # indice de colonie → suie 0-1
var _anchors: Dictionary = {}  # "i|famille" → {px, yaw} ou {} (aucun site)
var _root: Node3D
var _batches: Dictionary = {}  # clé de maillage → MultiMeshInstance3D
var _instances: Array = []  # instances affichées
var _center := Vector2(INF, INF)
var _dirty := true
var _pending: Array = []  # indices de colonies à traiter (reconstruction en cours)
var _building: Array = []  # instances de la reconstruction en cours
var _build_center := Vector2.ZERO
var _reground_in := -1
var _fov := 55.0
var _placed_distance := -1.0
var _fade := 1.0
var _build_top := 0.0  # palier de la reconstruction en cours (0 : échelle réelle)
var _built_top := -1.0
var _discs: Dictionary = {}  # Vector2i → Array d'emprises {c, dir, edge, off, r} au palier
var _layouts: Dictionary = {}  # indice de colonie → {sig, top, spots: {famille → emprise}}
var _road_cells: Dictionary = {}
var _roads_ready := false
var _fog_signature := 0
var _drape_mat: ShaderMaterial = null
var _drape_ready := false
var _vertical_scale := -1.0
var _shadows := true
var _meshes: Dictionary = {}
var _family_order: Array = []
var _grid: Dictionary = {}  # Vector2i → PackedInt32Array (indices de colonies)
## Maillages `out:` préparés hors du fil principal dès `setup` (chargement du `.glb` et pièces
## sommet par sommet : jusqu'à 80 ms par maquette sinon, à sa première apparition).
var _warm_task := -1
var _warmed: Dictionary = {}
var _warmed_resources: Dictionary = {}


## Dossier `data/` du jeu : celui de l'autoload `MapPaths`, sinon le dossier par défaut (scripts
## de test lancés par `--script`, où l'identifiant de l'autoload n'est pas compilé).
static func data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var paths := tree.root.get_node_or_null("/root/MapPaths") if tree != null else null
	return str(paths.get("data_dir")) if paths != null else MAP_PATHS.default_data_dir()


## Charge la correspondance (`data/map/building_models.json`) et le manifeste des maquettes.
static func load_config(data_dir: String) -> Dictionary:
	var path := data_dir.path_join(DATA_FILE)
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


static func load_manifest() -> Dictionary:
	var path := MODEL_DIR + "manifest.json"
	if not FileAccess.file_exists(path):
		return {}
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


## Vrai si la condition `when` d'un niveau est remplie : chaque groupe `require` compte au moins
## `min` bâtiments construits, et la province produit une des `resources` (si la liste existe).
static func level_reached(when: Dictionary, buildings: Array, resources: Array) -> bool:
	for group: Dictionary in when.get("require", []):
		var built := 0
		for id in group.get("of", []):
			if buildings.has(id):
				built += 1
		if built < int(group.get("min", 1)):
			return false
	var wanted: Array = when.get("resources", [])
	if wanted.is_empty():
		return true
	for id in wanted:
		if resources.has(id):
			return true
	return false


## Niveau (0 à 3) d'une famille pour des bâtiments construits et les ressources de la province.
static func family_level(family: Dictionary, buildings: Array, resources: Array) -> int:
	var best := 0
	for level: Dictionary in family.get("levels", []):
		if level_reached(level.get("when", {}), buildings, resources):
			best = maxi(best, int(level.get("level", 0)))
	return best


## Maquettes à poser autour d'une colonie : `[{family, level, model, site, slot}]`, une par
## famille au plus haut niveau atteint, au plus `render.max_per_settlement` (les plus hauts
## niveaux d'abord, puis l'ordre des familles dans les données).
static func models_for(cfg: Dictionary, buildings: Array, resources: Array) -> Array:
	var out: Array = []
	var families: Dictionary = cfg.get("families", {})
	var slot := 0
	for family_name: String in families:
		var family: Dictionary = families[family_name]
		var level := family_level(family, buildings, resources)
		if level > 0:
			var model := ""
			for entry: Dictionary in family["levels"]:
				if int(entry["level"]) == level:
					model = str(entry["model"])
			out.append({"family": family_name, "level": level, "model": model, "site": str(family.get("site", "field")), "slot": slot})
		slot += 1
	var cap := int((cfg.get("render", {}) as Dictionary).get("max_per_settlement", 4))
	if out.size() > cap:
		out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			if int(a["level"]) != int(b["level"]):
				return int(a["level"]) > int(b["level"])
			return int(a["slot"]) < int(b["slot"]))
		out.resize(cap)
		out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return int(a["slot"]) < int(b["slot"]))
	return out


static func screen_config(cfg: Dictionary) -> Dictionary:
	return (cfg.get("render", {}) as Dictionary).get("screen", {})


## Part (0-1) de la taille tenue à l'écran : 0 sous `real_below` (échelle réelle), 1 au-delà de
## `full_from`.
static func screen_blend(cfg: Dictionary, rig_distance: float) -> float:
	var screen := screen_config(cfg)
	var lo := float(screen.get("real_below", 10.0))
	return smoothstep(lo, maxf(float(screen.get("full_from", 15.0)), lo + 1e-3), rig_distance)


## Hauteur de terrain (unités) couverte par l'écran à la distance `rig_distance`.
static func view_span(rig_distance: float, fov_deg: float) -> float:
	return 2.0 * tan(deg_to_rad(fov_deg) * 0.5) * maxf(rig_distance, 0.0)


## Largeur (unités) qu'une maquette de niveau `level` tient à l'écran à pleine tenue :
## `fractions[level - 1]` de la hauteur de l'écran.
static func held_width(cfg: Dictionary, level: int, rig_distance: float, fov_deg: float) -> float:
	var fractions: Array = screen_config(cfg).get("fractions", [])
	if fractions.is_empty():
		return 0.0
	return float(fractions[clampi(level - 1, 0, fractions.size() - 1)]) * view_span(rig_distance, fov_deg)


## Grossissement d'une maquette large de `width_m` : 1 à l'échelle réelle, jamais moins.
static func model_factor(cfg: Dictionary, width_m: float, level: int, rig_distance: float, mpu: float, fov_deg: float) -> float:
	var full := maxf(1.0, held_width(cfg, level, rig_distance, fov_deg) * mpu / maxf(width_m, 0.1))
	return lerpf(1.0, full, screen_blend(cfg, rig_distance))


## Opacité (0-1) du fondu de sortie entre `fade_from_units` et `view_range_units`.
static func fade(cfg: Dictionary, rig_distance: float) -> float:
	var render: Dictionary = cfg.get("render", {})
	var hi := float(render.get("view_range_units", 160.0))
	var lo := minf(float(render.get("fade_from_units", hi)), hi - 1e-3)
	return clampf(inverse_lerp(hi, lo, rig_distance), 0.0, 1.0)


## Palier de mise en place de la distance `rig_distance` : 0 à l'échelle réelle, sinon la borne
## haute du palier géométrique (`full_from` × `band_ratio`^n) qui la contient. Les écarts sont
## calculés à cette borne puis réduits en proportion de la distance à l'intérieur du palier.
static func band_top(cfg: Dictionary, rig_distance: float) -> float:
	var screen := screen_config(cfg)
	if screen.is_empty() or rig_distance <= float(screen.get("real_below", 10.0)):
		return 0.0
	var from := maxf(float(screen.get("full_from", 15.0)), 1e-3)
	if rig_distance <= from:
		return from
	var ratio := maxf(float(screen.get("band_ratio", 1.5)), 1.05)
	return from * pow(ratio, ceilf(log(rig_distance / from) / log(ratio) - 1e-6))


func setup(layer: SettlementLayer, map: MapData, terrain: TerrainBuilder, data: SettlementData, meters_per_unit: float = 719.0) -> void:
	_layer = layer
	_map = map
	_terrain = terrain
	_data = data
	_mpu = meters_per_unit
	config = load_config(data_dir())
	# Carte généralisée (GC2, ADR 0158) : les villes sont des maquettes à taille monde constante ;
	# les bâtiments hors les murs suivent la même règle (rien ne « respire » au zoom) et les signes
	# de colonie du lot sont inutiles (la maquette GC est le signe de la ville).
	_maquette = TownMaquetteData.enabled() and not (config.get("render", {}) as Dictionary).get("maquette", {}).is_empty()
	manifest = load_manifest()
	_family_order = (config.get("families", {}) as Dictionary).keys()
	_grid.clear()
	for i in data.settlements.size():
		var px: Vector2 = data.settlements[i]["px"]
		var cell := Vector2i(floori(px.x / GRID_UNITS), floori(px.y / GRID_UNITS))
		if not _grid.has(cell):
			_grid[cell] = PackedInt32Array()
		(_grid[cell] as PackedInt32Array).append(i)
	_root = Node3D.new()
	_root.name = "Batches"
	add_child(_root)
	enabled = enabled and not config.is_empty() and not CmdArgs.has("--no-tb3")
	visible = false
	if terrain != null and not terrain.chunk_surface_changed.is_connected(_on_chunk_surface_changed):
		terrain.chunk_surface_changed.connect(_on_chunk_surface_changed)
	stats = {"instances": 0, "nodes": 0, "settlements": 0, "build_ms": 0.0}
	if enabled and _warm_task < 0:
		_warm_task = WorkerThreadPool.add_task(_warm_meshes.bind(manifest.duplicate(true), data_dir().path_join("provinces")), false, "outbuilding meshes")


## Fil de travail : maillages `out:` de toutes les maquettes du manifeste, dans `_warmed` (lu par
## le fil principal seulement après la fin de la tâche).
## Ressources des provinces aussi (`data/provinces/*.json`, statiques) : `get_province_city`
## coûtait 20 à 40 ms par province à sa première apparition (économie complète de la ville).
func _warm_meshes(models: Dictionary, provinces_dir: String) -> void:
	for model: String in models:
		_warmed["out:" + model] = with_pieces(_load_glb_mesh(MODEL_DIR + model + ".glb"), (models[model] as Dictionary).get("pieces", []))
	for file in DirAccess.get_files_at(provinces_dir):
		if file.get_extension() != "json":
			continue
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(provinces_dir.path_join(file)))
		if parsed is Dictionary and (parsed as Dictionary).has("resources"):
			_warmed_resources[str((parsed as Dictionary).get("id", file.get_basename()))] = Array((parsed as Dictionary)["resources"])


func _finish_warm() -> void:
	if _warm_task < 0:
		return
	WorkerThreadPool.wait_for_task_completion(_warm_task)
	_warm_task = -1
	for key: String in _warmed:
		if not _meshes.has(key):
			_meshes[key] = _warmed[key]
	_warmed = {}
	for province: String in _warmed_resources:
		if not _resources.has(province):
			_resources[province] = _warmed_resources[province]
	_warmed_resources = {}


func _exit_tree() -> void:
	_finish_warm()


## Note que l'état de la simulation a changé (`get_state_revision`) : il sera relu à la demande
## (voisinage à reconstruire, `models_of`…), pas tant que la couche est hors de portée. Rend
## vrai si l'état a changé.
func refresh(sim: Object) -> bool:
	_sim = sim
	var revision := -1
	if sim != null and sim.has_method("get_state_revision"):
		revision = int(sim.call("get_state_revision"))
	elif sim != null and sim.has_method("get_turn"):
		revision = int(sim.call("get_turn"))
	if revision == _revision and revision >= 0:
		if _fog_hash() != _fog_signature:  # réglage du brouillard changé sans changement d'état
			_fog_signature = _fog_hash()
			_dirty = true
		return false
	_revision = revision
	_fog_signature = _fog_hash()
	_states.clear()
	_provinces.clear()
	_live = {}
	_stale = true
	_dirty = true
	return true


## Reconstruit le voisinage à la prochaine mise à jour (états imposés changés).
func invalidate() -> void:
	_dirty = true
	_layouts.clear()


## Provinces hors de vue (id → true) : `fog_source.hidden_provinces`, par défaut celles des
## marqueurs d'armée de la carte (nœud `Armies` voisin de la couche des colonies).
func _hidden_provinces() -> Dictionary:
	if fog_source == null and _layer != null and _layer.get_parent() != null:
		fog_source = _layer.get_parent().get_node_or_null("Armies")
	var hidden: Variant = fog_source.get("hidden_provinces") if fog_source != null else null
	return hidden if hidden is Dictionary else {}


func _fog_hash() -> int:
	var hidden := _hidden_provinces()
	return hidden.size() * 31 + (hash(hidden.keys()) if hidden.size() < 64 else 0)


## Vrai si la colonie `id` est sous le brouillard de guerre (rien n'y est posé).
func is_hidden(id: String) -> bool:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	return i >= 0 and _hidden_provinces().has(str(_data.settlements[i]["province"]))


## État d'une province : `{population, devastation, besieged, constructing}` (vide si la
## simulation ne la connaît pas), lu dans `ProvinceSnapshot` et gardé jusqu'au prochain
## changement d'état.
func province_live(province: String) -> Dictionary:
	if _provinces.has(province):
		return _provinces[province]
	var out := {}
	if _sim != null and _map != null and (_sim.has_method("get_provinces_snapshot") or _sim.has_method("get_province_state")):
		var snapshot := ProvinceSnapshot.of(_sim, _map)
		var p := snapshot.index_of(province)
		if snapshot.has(p):
			out = {
				"population": float(snapshot.population_total[p]),
				"devastation": float(snapshot.devastation[p]),
				"besieged": snapshot.besieged[p] != 0,
				"constructing": snapshot.constructing.size() == snapshot.ids.size() and snapshot.constructing[p] != 0,
			}
	_provinces[province] = out
	return out


## TB3, point 5 : suie (0-1) des faubourgs, de l'enceinte et des bâtiments hors les murs de la
## colonie `id` (par instance de `MultiMesh`, `INSTANCE_CUSTOM.b`).
func set_soot(id: String, amount: float) -> void:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	if i < 0 or is_equal_approx(float(_soot.get(i, 0.0)), amount):
		return
	if amount > 0.0:
		_soot[i] = amount
	else:
		_soot.erase(i)
	for inst: Dictionary in _instances:
		if int(inst["settlement"]) == i:
			_write_batches()
			return


## État de toutes les colonies en un appel groupé (`get_settlements_live`), relu une fois par
## changement d'état et seulement à la demande ; sans cet appel, lecture par `settlement_detail`.
func _ensure_states() -> void:
	if not _stale:
		return
	_stale = false
	if _sim == null or not _sim.has_method("get_settlements_live"):
		return
	var t0 := Time.get_ticks_usec()
	_live = _sim.call("get_settlements_live")
	var ids: PackedStringArray = _live.get("id", PackedStringArray())
	if ids.size() != _live_index.size():
		_live_index.clear()
		for k in ids.size():
			_live_index[ids[k]] = k
	stats["read_ms"] = float(Time.get_ticks_usec() - t0) / 1000.0


## État d'une colonie : `{buildings: Array, fortification: int, is_city: bool, building: bool}`
## (`building` : chantier en cours ; pour la cité, celui de `ProvinceSnapshot.constructing`).
func _state_of(i: int) -> Dictionary:
	var id := str(_data.settlements[i]["id"])
	if forced_states.has(id):
		var forced: Dictionary = forced_states[id]
		return {"buildings": forced.get("buildings", []), "fortification": 0, "is_city": true, "building": bool(forced.get("building", false))}
	if _states.has(id):
		return _states[id]
	_ensure_states()
	var state := {"buildings": [], "fortification": int(_data.settlements[i].get("fortification_level", 0)), "is_city": false, "building": false}
	var ids: PackedStringArray = _live.get("id", PackedStringArray())
	var k: int = _live_index.get(id, -1)
	if k >= 0 and k < ids.size() and ids[k] != id:
		# L'ordre des colonies a changé : index refait.
		_live_index.clear()
		for n in ids.size():
			_live_index[ids[n]] = n
		k = _live_index.get(id, -1)
	if k >= 0 and k < ids.size():
		var buildings: Array = _live.get("buildings", [])
		var forts: PackedInt32Array = _live.get("fortification_level", PackedInt32Array())
		var cities: PackedByteArray = _live.get("is_city", PackedByteArray())
		state["buildings"] = Array(buildings[k]) if k < buildings.size() else []
		state["fortification"] = forts[k] if k < forts.size() else 0
		state["is_city"] = k < cities.size() and cities[k] != 0
		state["building"] = bool(state["is_city"]) and bool(province_live(str(_data.settlements[i]["province"])).get("constructing", false))
	elif _sim != null and _sim.has_method("settlement_detail"):
		var detail: Dictionary = _sim.call("settlement_detail", id)
		state["buildings"] = Array(detail.get("buildings", []))
		state["fortification"] = int(detail.get("fortification_level", state["fortification"]))
		state["is_city"] = bool(detail.get("is_city", false))
		state["building"] = not (detail.get("construction", {}) as Dictionary).is_empty()
	_states[id] = state
	return state


## Ressources de la province (statiques) : `get_province_city`, sinon `data/provinces/<id>.json`.
func _resources_of(province: String) -> Array:
	if _resources.has(province):
		return _resources[province]
	if _warm_task >= 0:
		_finish_warm()
		if _resources.has(province):
			return _resources[province]
	var out: Array = []
	if _sim != null and _sim.has_method("get_province_city"):
		out = Array((_sim.call("get_province_city", province) as Dictionary).get("resources", []))
	else:
		var path := data_dir().path_join("provinces").path_join(province + ".json")
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				out = Array((parsed as Dictionary).get("resources", []))
	_resources[province] = out
	return out


## Population de la province en 1337 (somme des classes de `data/provinces/<id>.json`), 0 si
## inconnue.
func baseline_population(province: String) -> float:
	if _baseline.has(province):
		return _baseline[province]
	var total := 0.0
	var path := data_dir().path_join("provinces").path_join(province + ".json")
	if FileAccess.file_exists(path):
		var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
		if parsed is Dictionary:
			var classes: Dictionary = ((parsed as Dictionary).get("population", {}) as Dictionary).get("classes", {})
			for name: String in classes:
				total += float((classes[name] as Dictionary).get("count", 0.0))
	_baseline[province] = total
	return total


## Population simulée de la province rapportée à celle de 1337 (1 si l'une des deux manque).
func population_ratio(province: String) -> float:
	if forced_population_ratio.has(province):
		return float(forced_population_ratio[province])
	var now := float(province_live(province).get("population", 0.0))
	var then := baseline_population(province)
	return now / then if now > 0.0 and then > 0.0 else 1.0


## Maquettes hors les murs de la colonie `id` pour l'état courant : `[{family, level, model…}]`,
## chantier compris (`family` = `worksite`).
func models_of(id: String) -> Array:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	return _models_of_index(i) if i >= 0 else []


## Croissance affichée de la colonie `id` : maquettes hors les murs, nombre de maisons de
## faubourg ajoutées et enceinte ajoutée ("" : aucune).
func growth_of(id: String) -> Dictionary:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	if i < 0:
		return {}
	var suburb := 0
	var enclosure := ""
	for inst: Dictionary in _growth_instances(i, _state_of(i)):
		if str(inst["family"]) == "suburb":
			suburb += 1
		elif str(inst.get("part", "")) == "wall":
			enclosure = TownGrowth.KINDS[int(inst["level"])]
	return {"outbuildings": _models_of_index(i), "suburb_houses": suburb, "enclosure": enclosure}


func _models_of_index(i: int) -> Array:
	var state := _state_of(i)
	var models := models_for(config, state["buildings"], _resources_of(str(_data.settlements[i]["province"])))
	if _maquette:
		# Carte généralisée : le décor générique (moulins GC5) porte déjà les bâtiments de 1337 ;
		# ces familles n'ont leur maquette que pour un bâtiment construit en cours de partie.
		var only_built: Array = (config["render"]["maquette"] as Dictionary).get("built_in_game_only", [])
		var initial: Array = _data.settlements[i].get("initial_buildings", [])
		var families: Dictionary = config.get("families", {})
		var kept: Array = []
		for model: Dictionary in models:
			var built_in_game := not only_built.has(str(model["family"]))
			if not built_in_game:
				# Bâtiments propres à la famille : premier groupe `require` de chaque niveau.
				for level: Dictionary in (families[str(model["family"])] as Dictionary)["levels"]:
					var groups: Array = (level["when"] as Dictionary).get("require", [])
					for id in (groups[0] as Dictionary).get("of", []) if not groups.is_empty() else []:
						if (state["buildings"] as Array).has(id) and not initial.has(id):
							built_in_game = true
			if built_in_game:
				kept.append(model)
		models = kept
	if bool(state["building"]) and manifest.has(WORKSITE_MODEL):
		models.append({"family": WORKSITE_FAMILY, "level": 1, "model": WORKSITE_MODEL, "site": "road", "slot": _family_order.size()})
	return models


# --- Mise à jour par image ---------------------------------------------------------------


## Réglage de rendu : celui du bloc `render.maquette` dans le style `maquette`, sinon `render`.
func _render(key: String, fallback: float) -> float:
	var render: Dictionary = config.get("render", {})
	if _maquette and (render.get("maquette", {}) as Dictionary).has(key):
		return float(render["maquette"][key])
	return float(render.get(key, fallback))


func _brighten() -> float:
	return _render("brighten", float(screen_config(config).get("brighten", 0.0)))


func view_range() -> float:
	return _render("view_range_units", 160.0)


func update_view(rig_distance: float) -> void:
	if not enabled or _data == null:
		visible = false
		return
	_view_distance = rig_distance
	_rig_distance = _render("size_distance", rig_distance) if _maquette else rig_distance
	var shown := force_active or rig_distance < view_range()
	if shown != visible:
		visible = shown
	if not shown:
		return
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera != null and not _maquette:
		_fov = camera.fov
	if _placed_distance < 0.0 or absf(_rig_distance - _placed_distance) > 0.04 * _placed_distance:
		var tw := Time.get_ticks_usec()
		_write_batches()
		PerfProbe.lap("out/rescale", tw)
	var opacity := 1.0 if force_active else clampf(inverse_lerp(view_range(), minf(_render("fade_from_units", view_range()), view_range() - 1e-3), rig_distance), 0.0, 1.0)
	if absf(opacity - _fade) > 0.02 or (opacity >= 1.0) != (_fade >= 1.0):
		_fade = opacity
		for mmi: MultiMeshInstance3D in _batches.values():
			mmi.transparency = 1.0 - _fade
	var shadows := rig_distance < _render("shadow_range_units", 12.0)
	if shadows != _shadows:
		_shadows = shadows
		for mmi: MultiMeshInstance3D in _batches.values():
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if MapData.vertical_scale() != _vertical_scale:
		_vertical_scale = MapData.vertical_scale()
		_refresh_aabbs()
	var here := _camera_ground()
	if _pending.is_empty() and (_dirty or not is_equal_approx(band_top(config, _rig_distance), _built_top) or here.distance_to(_center) > _loaded_radius * 0.3 or load_radius() > _loaded_radius * 1.3):
		_begin_rebuild(here)
	var ts := Time.get_ticks_usec()
	if not _pending.is_empty():
		_step(STEP_BUDGET_USEC if FrameBudget.in_frame() else 1 << 30)
		PerfProbe.lap("out/step", ts)
	elif _reground_in >= 0:
		_reground_in -= 1
		if _reground_in < 0:
			_reground()
			PerfProbe.lap("out/reground", ts)


## Rayon (unités) du voisinage chargé autour du point visé : `render.load_factor` × distance du
## rig, entre `render.load_min_units` et `render.load_max_units` (comme le chargement des villes
## 1:1 : de près, on ne pose que les maquettes des colonies proches).
func load_radius() -> float:
	return clampf(_render("load_factor", 3.0) * _view_distance, _render("load_min_units", 6.0), _render("load_max_units", view_range() * 1.5))


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


## Termine tout de suite la reconstruction autour de `center` (captures, tests).
func flush(center: Variant = null) -> void:
	if not enabled or _data == null:
		return
	if center is Vector2 or _dirty or _center.x == INF or not is_equal_approx(band_top(config, _rig_distance), _built_top):
		_begin_rebuild(center if center is Vector2 else _camera_ground())
	if not _pending.is_empty():
		_step(1 << 30)
	if _reground_in >= 0:
		_reground_in = -1
		_reground()


func _begin_rebuild(center: Vector2) -> void:
	_dirty = false
	_build_center = center
	_building = []
	_pending = []
	_build_top = band_top(config, _rig_distance)
	_discs = {}
	var radius := load_radius()
	_loaded_radius = radius
	var near: Array = []
	for i in _data.settlements.size():
		var px: Vector2 = _data.settlements[i]["px"]
		if absf(px.x - center.x) <= radius and absf(px.y - center.y) <= radius and px.distance_to(center) <= radius:
			# De loin, l'ordre fixe la priorité de la mise en place : les cités, puis les villes.
			var rank := int(KIND_RANK.get(str(_data.settlements[i].get("kind", "")), 2)) if _build_top > 0.0 else 0
			near.append([float(rank) * 1e6 + px.distance_to(center), i])
	near.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	for entry: Array in near:  # `pop_back` : les plus proches d'abord
		_pending.append(entry[1])
	stats["settlements"] = _pending.size()
	stats["build_start_usec"] = Time.get_ticks_usec()
	if _pending.is_empty():
		_finish_rebuild()


func _step(budget_usec: int) -> void:
	var t0 := Time.get_ticks_usec()
	while not _pending.is_empty():
		_building.append_array(_instances_of(_pending.pop_back()))
		if Time.get_ticks_usec() - t0 > budget_usec:
			break
	if _pending.is_empty():
		_finish_rebuild()


func _finish_rebuild() -> void:
	_instances = _building
	_building = []
	_built_top = _build_top
	_center = _build_center
	_reground_in = -1
	_write_batches()
	stats["build_ms"] = float(Time.get_ticks_usec() - int(stats.get("build_start_usec", Time.get_ticks_usec()))) / 1000.0


## Instances d'une colonie : maquettes hors les murs, chantier, puis croissance de la ville.
## Rien sous le brouillard de guerre.
func _instances_of(i: int) -> Array:
	var out: Array = []
	if _hidden_provinces().has(str(_data.settlements[i]["province"])):
		# Sous le brouillard, la ville garde son signe (elle reste connue), rien d'autre.
		if _build_top > 0.0:
			var hidden_sign := _sign_instance(i, _px_of(i), [])
			if not hidden_sign.is_empty():
				out.append(hidden_sign)
		return out
	var state := _state_of(i)
	var growth := _growth_instances(i, state)
	var center := _px_of(i)
	# De loin, seules les cités et les villes gardent leurs maquettes (la place manque).
	var minor := _build_top > float(screen_config(config).get("minor_until", INF)) and not KIND_RANK.has(str(_data.settlements[i].get("kind", "")))
	if _maquette:
		minor = not ((config["render"]["maquette"] as Dictionary).get("kinds", []) as Array).has(str(_data.settlements[i].get("kind", "")))
	for model: Dictionary in ([] if minor else _models_of_index(i)):
		var entry: Dictionary = manifest.get(str(model["model"]), {})
		if entry.is_empty():
			continue
		var family := str(model["family"])
		# L'ancrage vaut pour les trois niveaux : il réserve l'emprise du niveau 3.
		var reserve := float((manifest.get("%s_3" % family, entry) as Dictionary).get("radius", entry["radius"]))
		var anchor := {}
		if _maquette:
			# Autour d'une maquette GC, le site réel n'a pas de sens : seule compte la direction,
			# le secteur de la famille (tiré de l'identifiant de la colonie).
			var seed_value := MapInstancing.text_seed(str(_data.settlements[i]["id"]))
			var angle := float(seed_value % 3600) / 3600.0 * TAU + float(int(model["slot"])) * TAU / float(_family_order.size() + 1)
			anchor = {"px": center + Vector2(cos(angle), sin(angle)) * (_built_radius(i) + 0.1), "yaw": 0.0}
		else:
			anchor = _anchor(i, family, int(model["slot"]), str(model["site"]), reserve)
		var far_only := anchor.is_empty()
		if far_only:
			# Port sans grève au pied de la ville : de loin seulement, posé à la côte voisine.
			if _build_top <= 0.0 or str(model["site"]) != "shore":
				continue
			anchor = {"px": center + Vector2(0.0, 0.01), "yaw": 0.0}
		var px: Vector2 = anchor["px"]
		out.append({
			"key": "out:" + str(model["model"]),
			"settlement": i,
			"family": family,
			"level": int(model["level"]),
			"model": str(model["model"]),
			"site": str(model["site"]),
			"far_only": far_only,
			"px": px,
			"real_px": px,
			"c": center,
			"yaw": float(anchor["yaw"]),
			"base_m": _base_m(px),
			"scale": Vector3.ONE,
			"grow": "out",
			"tint": 0.5 + _brighten(),
			"width_m": maxf(float(entry.get("length", 30.0)), float(entry.get("depth", 30.0))),
			"radius_m": float(entry.get("radius", 30.0)),
			"top": float(entry.get("height", 20.0)),
			"reach": float(entry.get("radius", 30.0)),
		})
	if _build_top > 0.0:
		var sign := _sign_instance(i, center, growth)
		out = _layout(i, out, growth, _build_top, sign)
		if not sign.is_empty():
			out.append(sign)
			out.append_array(_quarter_signs(i, center, growth, sign))
		elif _maquette:
			# Maquette GC de la ville : les faubourgs ajoutés sont des groupes de maisons à son
			# bord ; son enceinte est déjà dans la maquette (rien n'est doublé).
			var built := _built_radius(i)
			for inst: Dictionary in growth:
				inst["held_edge"] = built
				inst["c"] = center
			out.append_array(_quarter_signs(i, center, growth, {"built": built}))
	out.append_array(growth)
	return out


## Signe de la colonie (ADR 0162) : de loin, la ville 1:1 ne se lit plus ; une maquette tenue à
## l'écran (`render.screen.signs`, par genre de colonie) la recouvre : toits serrés, église,
## enceinte si la ville est murée (plan de 1340, fortification de départ ou construite). Vide si
## le genre n'a pas de signe. Les faubourgs ajoutés partent alors du bord du signe.
func _sign_instance(i: int, center: Vector2, growth: Array) -> Dictionary:
	if _maquette:
		return {}
	var signs: Dictionary = screen_config(config).get("signs", {})
	var entry: Dictionary = _data.settlements[i]
	var rule: Dictionary = signs.get(str(entry.get("kind", "")), {})
	if rule.is_empty():
		return {}
	var model := str(rule.get("model", ""))
	if rule.has("walled_model"):
		var walled := int(entry.get("fortification_level", 0)) >= int(rule.get("walled_from", 1)) or str(_town_of(i).get("walls", "none")) != "none"
		for inst: Dictionary in growth:
			if str(inst.get("grow", "")) == "wall":
				walled = true
		if walled:
			model = str(rule["walled_model"])
	var shape: Dictionary = manifest.get(model, {})
	if shape.is_empty():
		return {}
	var width := maxf(float(shape.get("length", 60.0)), float(shape.get("depth", 60.0)))
	var sign := {
		"key": "out:" + model,
		"settlement": i,
		"family": "sign",
		"level": 0,
		"model": model,
		"px": center,
		"real_px": center,
		"c": center,
		"yaw": 0.0,
		"base_m": _base_m(center),
		"scale": Vector3.ONE,
		"grow": "sign",
		"tint": 0.5 + _brighten(),
		"width_m": width,
		"radius_m": float(shape.get("radius", 40.0)),
		"fraction": float(rule.get("fraction", 0.08)),
		"built": _built_radius(i),
		"top": float(shape.get("height", 30.0)),
		"reach": float(shape.get("radius", 40.0)),
	}
	# Part du rayon du signe dans la hauteur de terrain vue : rayon (unités) = part × `view_span`.
	var share := float(sign["fraction"]) * float(sign["radius_m"]) / width
	for inst: Dictionary in growth:
		inst["sign_share"] = share
		inst["sign_built"] = float(sign["built"])
		inst["c"] = center
	return sign


## Quartiers de faubourg ajoutés, de loin : un petit groupe de maisons (`screen.suburb_sign`) par
## quartier, au bord du signe de la ville, sur la route de sa porte ; les maisons du quartier
## elles-mêmes (trop petites, même grossies) sont alors masquées.
func _quarter_signs(i: int, center: Vector2, growth: Array, sign: Dictionary) -> Array:
	var rule: Dictionary = screen_config(config).get("suburb_sign", {})
	var shape: Dictionary = manifest.get(str(rule.get("model", "")), {})
	var out: Array = []
	if shape.is_empty():
		return out
	var seen := {}
	for inst: Dictionary in growth:
		if str(inst.get("grow", "")) != "house":
			continue
		inst["quarter_sign"] = true
		var q := int(inst["quarter"])
		if seen.has(q):
			continue
		seen[q] = true
		var dir: Vector2 = inst["quarter_dir"]
		var width := maxf(float(shape.get("length", 30.0)), float(shape.get("depth", 30.0)))
		out.append({
			"key": "out:" + str(rule["model"]),
			"settlement": i,
			"family": "suburb_sign",
			"level": q + 1,
			"model": str(rule["model"]),
			"px": center,
			"real_px": center,
			"c": center,
			"dir": dir,
			"yaw": atan2(dir.x, dir.y),
			"base_m": _base_m(center),
			"scale": Vector3.ONE,
			"grow": "quarter",
			"tint": 0.5 + _brighten(),
			"width_m": width,
			"radius_m": float(shape.get("radius", 20.0)),
			"fraction": float(rule.get("fraction", 0.04)),
			"sign_share": float(inst.get("sign_share", 0.0)),
			"sign_built": float(sign["built"]),
			"held_edge": float(sign["built"]) if _maquette else 0.0,
			"top": float(shape.get("height", 12.0)),
			"reach": float(shape.get("radius", 20.0)),
		})
	return out


## Rayon (unités) du signe d'une colonie à la distance `rig_distance` ; 0 s'il n'est pas affiché
## (ville réelle plus grande que lui : cités emblématiques vues d'assez près).
func _sign_radius(share: float, built: float, rig_distance: float) -> float:
	var radius := share * view_span(rig_distance, _fov)
	return radius if radius >= built * 1.25 else 0.0


## TB3, point 4 : faubourgs (population) et enceinte (fortification) de la ville 1:1.
func _growth_instances(i: int, state: Dictionary) -> Array:
	var town := _town_of(i)
	var growth: Dictionary = config.get("growth", {})
	if town.is_empty() or growth.is_empty():
		return []
	var entry: Dictionary = _data.settlements[i]
	var center := _layer.town_data().anchor_of(str(entry["id"]))
	var out := TownGrowth.suburb_instances(growth, town, i, center, _mpu, TownGrowth.quarter_count(growth, population_ratio(str(entry["province"]))), _base_m)
	var kind := TownGrowth.enclosure_to_add(growth, state["buildings"], entry.get("initial_buildings", []), str(town.get("walls", "none")))
	if kind != "":
		out.append_array(TownGrowth.enclosure_instances(growth, _layer.town_data().params.get("walls", {}), town, i, center, _mpu, kind, _base_m))
	return out


## Entrée de `towns_1340.json` de la colonie `i` (vide : ville emblématique ou colonie absente).
func _town_of(i: int) -> Dictionary:
	if _layer == null or _layer.town_data() == null:
		return {}
	return _layer.town_data().towns.get(str(_data.settlements[i]["id"]), {})


# --- Sites ---------------------------------------------------------------------------------


func _height_m(p: Vector2) -> float:
	if _terrain != null:
		return MapData.height_from_display(_terrain.surface_height_at(p.x, p.y), p.x, p.y)
	return _map.height_m_at(p.x, p.y) if _map != null else 0.0


## Altitude de pose (m), jamais sous la grève.
func _base_m(p: Vector2) -> float:
	return maxf(_height_m(p), 0.3)


func _is_water(p: Vector2) -> bool:
	return _height_m(p) <= 0.05 or (_map != null and _map.river_sd_at(p.x, p.y) < 0.0)


## Ancrage `{px, yaw}` de la famille `family` autour de la colonie `i`, calculé une fois ; vide
## si aucun site ne convient. `radius_m` : emprise à réserver.
func _anchor(i: int, family: String, slot: int, site: String, radius_m: float) -> Dictionary:
	var key := "%d|%s" % [i, family]
	if not _anchors.has(key):
		_anchors[key] = _find_anchor(i, slot, site, radius_m)
	return _anchors[key]


func _find_anchor(i: int, slot: int, site: String, radius_m: float) -> Dictionary:
	var render: Dictionary = config.get("render", {})
	var center := _px_of(i)
	var built := _built_radius(i)
	var reach := radius_m / _mpu
	var ring_min := built + float(render.get("ring_min_m", 90.0)) / _mpu + reach
	var ring_max := maxf(built + float(render.get("ring_max_m", 650.0)) / _mpu, ring_min + reach)
	var seed_value := MapInstancing.text_seed(str(_data.settlements[i]["id"]))
	var theta0 := float(seed_value % 3600) / 3600.0 * TAU
	if site == "shore" or site == "coast":
		var shore := _shore_anchor(i, center, ring_min - reach, built + float(render.get("shore_reach_m", 2500.0)) / _mpu, theta0)
		if not shore.is_empty() or site == "shore":
			return shore
	var slots := float(_family_order.size() + 1)
	var sector := TAU / slots
	var max_slope := float(render.get("max_slope", 0.22))
	var gates := PackedFloat32Array()
	for gate: Dictionary in _town_of(i).get("gates", []):
		gates.append(deg_to_rad(float(gate["bearing"])))
	var best := {}
	var best_score := INF
	for step in ANGLE_STEPS:
		var angle := theta0 + (float(slot) + float(step) / 3.0) * sector
		# Les routes des portes restent libres (faubourgs à venir).
		var on_road := false
		for gate_angle in gates:
			if absf(angle_difference(angle, gate_angle)) < GATE_CORRIDOR:
				on_road = true
		if on_road:
			continue
		var dir := Vector2(cos(angle), sin(angle))
		for t: float in [0.1, 0.45]:
			var p := center + dir * lerpf(ring_min, ring_max, t)
			var slope := _site_slope(i, p, reach, max_slope)
			if slope < 0.0:
				continue
			var score := float(absi(step)) * 10.0 + t
			match site:
				"slope":
					score -= slope * 4.0
				"water":
					score += clampf(_map.river_sd_at(p.x, p.y), 0.0, 8.0) if _map != null else 0.0
				"road":
					score += t * 2.0
				_:
					score += slope * 4.0
			if score < best_score:
				best_score = score
				# Façade (+Z de la maquette) tournée vers la colonie ; vers le fleuve s'il est
				# proche, pour un site de rive (roue du moulin côté eau).
				var facing := -dir
				if site == "water" and _map != null and _map.river_sd_at(p.x, p.y) < 3.0:
					var e := 0.25
					var slope_to_river := Vector2(_map.river_sd_at(p.x + e, p.y) - _map.river_sd_at(p.x - e, p.y), _map.river_sd_at(p.x, p.y + e) - _map.river_sd_at(p.x, p.y - e))
					if slope_to_river.length() > 0.05:
						facing = -slope_to_river.normalized()
				best = {"px": p, "yaw": atan2(facing.x, facing.y)}
		if not best.is_empty() and absi(step) >= 2:
			break
	return best


## Pente (m/m) du site `p` d'emprise `reach` (unités), ou -1 s'il ne convient pas : mer, lit de
## fleuve, emprise d'une autre colonie, pente trop forte.
func _site_slope(i: int, p: Vector2, reach: float, max_slope: float) -> float:
	if _map != null and (p.x < 1.0 or p.y < 1.0 or p.x > _map.size.x - 1.0 or p.y > _map.size.y - 1.0):
		return -1.0
	var h := _height_m(p)
	if h <= 0.5 or (_map != null and _map.river_sd_at(p.x, p.y) < 0.03):
		return -1.0
	if _on_other_settlement(i, p, reach):
		return -1.0
	if _layer != null and not _layer.is_landmark(i) and _layer.covered_by_landmark(p):
		return -1.0
	var steepest := 0.0
	for offset: Vector2 in [Vector2(reach, 0.0), Vector2(-reach, 0.0), Vector2(0.0, reach), Vector2(0.0, -reach)]:
		var other := _height_m(p + offset)
		if other <= 0.05:
			return -1.0
		steepest = maxf(steepest, absf(other - h) / (reach * _mpu))
	return steepest if steepest <= max_slope else -1.0


func _on_other_settlement(i: int, p: Vector2, reach: float) -> bool:
	var cell := Vector2i(floori(p.x / GRID_UNITS), floori(p.y / GRID_UNITS))
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			for j: int in _grid.get(cell + Vector2i(dx, dy), PackedInt32Array()):
				if j != i and p.distance_to(_px_of(j)) < _built_radius(j) + reach:
					return true
	return false


func _px_of(i: int) -> Vector2:
	return _layer.model_px(i) if _layer != null else (_data.settlements[i]["px"] as Vector2)


## Rayon bâti (unités) de la colonie : emprise réelle de la ville 1:1.
func _built_radius(i: int) -> float:
	return _layer.built_radius(i) if _layer != null else 150.0 / _mpu


## Site de rive : la grève la plus proche de la colonie (mer ou fleuve), façade vers l'eau.
func _shore_anchor(i: int, center: Vector2, from: float, to: float, theta0: float) -> Dictionary:
	var best := {}
	var best_r := INF
	var step := 60.0 / _mpu
	for k in SHORE_DIRECTIONS:
		var angle := theta0 + TAU * float(k) / float(SHORE_DIRECTIONS)
		var dir := Vector2(cos(angle), sin(angle))
		var r := from
		if _is_water(center + dir * r):
			continue
		while r < to and r < best_r:
			if _is_water(center + dir * (r + step)):
				var lo := r
				var hi := r + step
				for n in 4:
					var mid := (lo + hi) * 0.5
					if _is_water(center + dir * mid):
						hi = mid
					else:
						lo = mid
				var p := center + dir * maxf(lo - 10.0 / _mpu, from)
				if not _on_other_settlement(i, p, 30.0 / _mpu):
					best_r = lo
					best = {"px": p, "yaw": atan2(dir.x, dir.y)}
				break
			r += step
	return best


# --- Mise en place tenue à l'écran -----------------------------------------------------------
#
# De loin, chaque maquette garde une largeur d'écran : son emprise (unités) grandit comme la
# distance du rig. Une emprise est un disque {c, dir, edge, off, r} : centre `c + dir × (edge +
# off × q)` et rayon `r × q`, où q = distance / borne haute du palier (1 à la borne). `edge` est
# le bord de la ville (taille réelle, fixe) ; l'écart au bord et le rayon suivent la distance, si
# bien que deux emprises disjointes à la borne le restent dans tout le palier autour d'une même
# ville ; entre villes voisines, l'essai est refait au milieu et au bas du palier.


func _disc_center(disc: Dictionary, q: float) -> Vector2:
	return (disc["c"] as Vector2) + (disc["dir"] as Vector2) * (float(disc["edge"]) + float(disc["off"]) * q)


func _disc_cell(p: Vector2) -> Vector2i:
	return Vector2i(floori(p.x / DISC_CELL), floori(p.y / DISC_CELL))


func _disc_add(disc: Dictionary) -> void:
	var cell := _disc_cell(_disc_center(disc, 1.0))
	if not _discs.has(cell):
		_discs[cell] = []
	(_discs[cell] as Array).append(disc)


## Parts de la borne haute où une emprise est essayée : la borne, puis vers le bas du palier.
func _band_shares() -> Array[float]:
	var low := 1.0 / maxf(float(screen_config(config).get("band_ratio", 1.5)), 1.05)
	return [1.0, lerpf(1.0, low, 0.34), lerpf(1.0, low, 0.67), low]


## Vrai si l'emprise ne recouvre aucune emprise déjà posée : à la borne pour celles de la même
## colonie, et aussi plus bas dans le palier pour celles des colonies voisines.
func _disc_free(disc: Dictionary, shares: Array[float]) -> bool:
	var at := _disc_center(disc, 1.0)
	var cell := _disc_cell(at)
	var r := float(disc["r"])
	for dy in [-1, 0, 1]:
		for dx in [-1, 0, 1]:
			for other: Dictionary in _discs.get(cell + Vector2i(dx, dy), []):
				var reach := (r + float(other["r"])) * 1.04
				if at.distance_to(_disc_center(other, 1.0)) < reach:
					return false
				if (other["c"] as Vector2) == (disc["c"] as Vector2):
					continue
				for q in shares:
					if q < 1.0 and _disc_center(disc, q).distance_to(_disc_center(other, q)) < reach * q:
						return false
	return true


func _is_land(p: Vector2) -> bool:
	if _map == null:
		return true
	return _map.is_land_px(int(p.x), int(p.y)) and _map.height_m_at(p.x, p.y) > 0.5


## Routes principales : cases de `ROAD_CELL` traversées (lues une fois).
func _road_near(p: Vector2, reach: float) -> bool:
	if not _roads_ready:
		_roads_ready = true
		for road: Dictionary in _data.roads:
			if not bool(road.get("main", false)):
				continue
			var points: PackedVector2Array = road["points"]
			for k in range(1, points.size()):
				var steps := maxi(1, ceili(points[k - 1].distance_to(points[k]) / (ROAD_CELL * 0.5)))
				for n in steps + 1:
					var at := points[k - 1].lerp(points[k], float(n) / float(steps))
					_road_cells[Vector2i(floori(at.x / ROAD_CELL), floori(at.y / ROAD_CELL))] = true
	var span := ceili(reach / ROAD_CELL)
	var cell := Vector2i(floori(p.x / ROAD_CELL), floori(p.y / ROAD_CELL))
	for dy in range(-span, span + 1):
		for dx in range(-span, span + 1):
			if dx * dx + dy * dy <= span * span and _road_cells.has(cell + Vector2i(dx, dy)):
				return true
	return false


## Site de loin : 0 s'il convient, 1 s'il convient faute de mieux (route principale ou fleuve
## sous l'emprise), -1 s'il ne convient pas (mer, lit de fleuve, emprise d'une autre colonie).
func _far_site(i: int, p: Vector2, r: float, site: String) -> int:
	if _map != null and (p.x < 1.0 or p.y < 1.0 or p.x > _map.size.x - 1.0 or p.y > _map.size.y - 1.0):
		return -1
	if not _is_land(p):
		return -1
	var river := _map.river_sd_at(p.x, p.y) if _map != null else 8.0
	if river < minf(r * 0.25, 1.5):
		return -1
	if site != "shore" and site != "coast":
		for offset: Vector2 in [Vector2(r, 0.0), Vector2(-r, 0.0), Vector2(0.0, r), Vector2(0.0, -r)]:
			if not _is_land(p + offset * 0.7):
				return -1
	var cell := Vector2i(floori(p.x / GRID_UNITS), floori(p.y / GRID_UNITS))
	var span := ceili((r + 4.0) / GRID_UNITS)
	for dy in range(-span, span + 1):
		for dx in range(-span, span + 1):
			for j: int in _grid.get(cell + Vector2i(dx, dy), PackedInt32Array()):
				if j != i and p.distance_to(_px_of(j)) < _built_radius(j) + r * 1.05:
					return -1
	if river < minf(r * (0.45 if site == "water" else 0.65), 5.0) or _road_near(p, r * 0.5):
		return 1
	return 0


## Direction de l'eau (mer ou fleuve) au bord de l'emprise de rayon `r` posée en `p` ; nulle s'il
## n'y en a pas.
func _water_direction(p: Vector2, r: float) -> Vector2:
	var sum := Vector2.ZERO
	for k in 12:
		var dir := Vector2(cos(TAU * float(k) / 12.0), sin(TAU * float(k) / 12.0))
		var at := p + dir * r * 1.1
		if not _is_land(at) or (_map != null and _map.river_sd_at(at.x, at.y) < 0.0):
			sum += dir
	return sum.normalized() if sum.length() > 0.5 else Vector2.ZERO


## Port de loin : sur la grève la plus proche de la colonie (mer ou fleuve), à `shore_reach_far`
## unités au plus, façade vers l'eau ; l'emprise reste à la côte quand la distance change (elle
## recule vers la terre de 0,6 rayon). Vide : pas de grève libre à portée.
func _coast_spot(i: int, center: Vector2, from: float, r: float, shares: Array[float]) -> Dictionary:
	var reach := float(screen_config(config).get("shore_reach_far", 0.0))
	var best := {}
	var best_d := INF
	for k in 24:
		var dir := Vector2(cos(TAU * float(k) / 24.0), sin(TAU * float(k) / 24.0))
		var d := from + r * 0.5
		if not _is_land(center + dir * d):
			continue
		while d < minf(reach, best_d):
			d += 0.25
			var at := center + dir * d
			if not _is_land(at) or (_map != null and _map.river_sd_at(at.x, at.y) < 0.0):
				var disc := {"c": center, "dir": dir, "edge": d, "off": -0.6 * r, "r": r, "yaw": atan2(dir.x, dir.y)}
				var spot := _disc_center(disc, 1.0)
				if _is_land(spot) and _far_site(i, spot, r * 0.6, "shore") >= 0 and _disc_free(disc, shares):
					best = disc
					best_d = d
				break
	return best


## Emprise de loin d'une maquette autour de la colonie `i` : au plus près de la direction de son
## site réel, sur l'anneau au bord de la ville, puis les anneaux suivants. Vide : aucune place.
func _far_spot(i: int, center: Vector2, edge: float, start: float, inst: Dictionary, top: float, gates: PackedFloat32Array, shares: Array[float]) -> Dictionary:
	var screen := screen_config(config)
	var factor := maxf(1.0, held_width(config, int(inst["level"]), top, _fov) * _mpu / float(inst["width_m"]))
	var r := float(inst["radius_m"]) * factor / _mpu
	var preferred := ((inst["real_px"] as Vector2) - center).angle()
	var gap := float(screen.get("gap", 0.15)) * r
	var site := str(inst["site"])
	var fallback := {}
	if site == "shore":
		var coast := _coast_spot(i, center, edge + start, r, shares)
		if not coast.is_empty():
			return coast
	for ring in int(screen.get("rings", 2)):
		var radius := edge + start + gap + r * (1.0 + RING_STEP * float(ring))
		var delta := clampf(r / radius, 0.15, 0.7) * 0.75
		for m in int(PI / delta) + 1:
			for side: float in ([1.0] if m == 0 else [1.0, -1.0]):
				var angle := preferred + side * float(m) * delta
				var dir := Vector2(cos(angle), sin(angle))
				var disc := {"c": center, "dir": dir, "edge": edge, "off": radius - edge, "r": r}
				if not _disc_free(disc, shares):
					continue
				var fit := _far_site(i, center + dir * radius, r, site)
				if fit < 0:
					continue
				# Plus bas dans le palier, la maquette se rapproche de la ville : ni mer ni lit.
				for q in shares:
					var at := _disc_center(disc, q)
					if q < 1.0 and (not _is_land(at) or (_map != null and _map.river_sd_at(at.x, at.y) < minf(r * q * 0.25, 1.5))):
						fit = -1
				if fit < 0:
					continue
				for gate_angle in gates:  # routes des portes
					if absf(angle_difference(angle, gate_angle)) * radius < r * 0.6:
						fit = 1
				if site == "shore" or site == "coast":
					# Port et saline : au bord de l'eau, façade vers elle ; sinon faute de mieux.
					var water := _water_direction(center + dir * radius, r)
					if water == Vector2.ZERO:
						fit = 1
					else:
						disc["yaw"] = atan2(water.x, water.y)
				if fit == 0:
					return disc
				if fallback.is_empty():
					fallback = disc
		if not fallback.is_empty():
			return fallback
	return fallback


## Mise en place de loin de la colonie `i` au palier `top` : pose les emprises de sa croissance
## (quartiers de faubourg), puis celles de ses maquettes par niveau décroissant. Rend les
## maquettes placées (`dir`, `edge`, `off`, `top_d`) ; celles qui ne tiennent pas sont retirées.
func _layout(i: int, models: Array, growth: Array, top: float, sign: Dictionary = {}) -> Array:
	var center := _px_of(i)
	var edge := _built_radius(i)
	# Signe affiché jusqu'au bas du palier : l'anneau part de son bord, qui suit la distance.
	var low := top / maxf(float(screen_config(config).get("band_ratio", 1.5)), 1.05)
	var sign_top := 0.0
	if not sign.is_empty() and _sign_radius(float(sign["fraction"]) * float(sign["radius_m"]) / float(sign["width_m"]), edge, low) > 0.0:
		sign_top = float(sign["fraction"]) * float(sign["radius_m"]) / float(sign["width_m"]) * view_span(top, _fov)
		edge = 0.0
	var quarters := {}  # quartier → [origine, direction, longueur (m), demi-largeur (m)]
	for inst: Dictionary in growth:
		match str(inst.get("grow", "")):
			"wall":
				edge = maxf(edge, (float((inst["shape"] as Dictionary)["ring_m"]) + _wall_thickness_full(inst["shape"], top) * 1.6) / _mpu)
			"house":
				var q := int(inst["quarter"])
				if not quarters.has(q):
					quarters[q] = [inst["origin"], inst["quarter_dir"], 0.0, 0.0]
				if int(inst.get("rank", 0)) < int(screen_config(config).get("far_houses", 99)):
					quarters[q][2] = maxf(float(quarters[q][2]), float(inst["along_m"]))
				quarters[q][3] = maxf(float(quarters[q][3]), float(inst["half_m"]))
	var house := _house_factor_full(top)
	for q: int in quarters:
		var quarter: Array = quarters[q]
		var half := float(quarter[3]) * house / _mpu
		var length := float(quarter[2]) * house / _mpu
		var along := half
		while along < length + half:
			if sign_top > 0.0:
				var rule: Dictionary = screen_config(config).get("suburb_sign", {})
				var reach := float(rule.get("fraction", 0.04)) * view_span(top, _fov) * 0.75
				_disc_add({"c": center, "dir": quarter[1], "edge": 0.0, "off": sign_top + reach, "r": reach})
				break
			elif _maquette:
				var held: Dictionary = screen_config(config).get("suburb_sign", {})
				var cluster := float(held.get("fraction", 0.04)) * view_span(top, _fov) * 0.75
				_disc_add({"c": center, "dir": quarter[1], "edge": edge, "off": cluster, "r": cluster})
				break
			else:
				_disc_add({"c": quarter[0], "dir": quarter[1], "edge": 0.0, "off": along, "r": half})
			along += half * 1.6
	var names := PackedStringArray()
	for inst: Dictionary in models:
		names.append(str(inst["model"]))
	var sig := "%.2f|%.3f|%.3f|%d|%s" % [top, edge, sign_top, quarters.size(), ",".join(names)]
	var cached: Dictionary = _layouts.get(i, {})
	var shares := _band_shares()
	var spots: Dictionary = {}
	# Mise en place gardée tant que les maquettes ne changent pas, si elle ne recouvre pas celles
	# des colonies déjà posées.
	var reuse := str(cached.get("sig", "")) == sig
	if reuse:
		for family: String in cached["spots"]:
			if not _disc_free(cached["spots"][family], shares):
				reuse = false
				break
	if reuse:
		spots = cached["spots"]
		for family: String in spots:
			_disc_add(spots[family])
	else:
		var gates := PackedFloat32Array()
		for gate: Dictionary in _town_of(i).get("gates", []):
			gates.append(deg_to_rad(float(gate["bearing"])))
		var order := models.duplicate()
		order.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			if int(a["level"]) != int(b["level"]):
				return int(a["level"]) > int(b["level"])
			return str(a["family"]) < str(b["family"]))
		for inst: Dictionary in order:
			var disc := _far_spot(i, center, edge, sign_top, inst, top, gates, shares)
			if not disc.is_empty():
				spots[str(inst["family"])] = disc
				_disc_add(disc)
		_layouts[i] = {"sig": sig, "spots": spots}
	var out: Array = []
	for inst: Dictionary in models:
		var disc: Dictionary = spots.get(str(inst["family"]), {})
		if disc.is_empty():
			continue
		inst["dir"] = disc["dir"]
		inst["edge"] = disc["edge"]
		inst["off"] = disc["off"]
		# Façade vers l'eau, sans tourner le dos à la caméra de jeu (vue du sud) : ± 70° au plus.
		inst["far_yaw"] = clampf(wrapf(float(disc.get("yaw", 0.0)), -PI, PI), -1.22, 1.22)
		inst["top_d"] = top
		out.append(inst)
	return out


## Épaisseur (m) tenue à l'écran de l'enceinte ajoutée : `wall_fraction` de la hauteur d'écran,
## au plus `wall_max_share` du rayon de la ville, jamais moins que la vraie.
func _wall_thickness_full(shape: Dictionary, rig_distance: float) -> float:
	var screen := screen_config(config)
	var real := float(shape.get("thick_m", 2.0))
	var held := float(screen.get("wall_fraction", 0.0)) * view_span(rig_distance, _fov) * _mpu
	return clampf(held, real, maxf(real, float(screen.get("wall_max_share", 0.2)) * float(shape.get("ring_m", 100.0))))


## Grossissement tenu à l'écran des maisons de faubourg (`house_fraction` de la hauteur d'écran).
func _house_factor_full(rig_distance: float) -> float:
	var screen := screen_config(config)
	return maxf(1.0, float(screen.get("house_fraction", 0.0)) * view_span(rig_distance, _fov) * _mpu / maxf(float(screen.get("house_width_m", 9.0)), 0.1))


## Position, échelle et hauteur de base d'une instance à la distance courante du rig.
func _place(inst: Dictionary) -> void:
	var d := _rig_distance
	var px: Vector2 = inst.get("real_px", inst["px"])
	var blend := screen_blend(config, d)
	var screen := screen_config(config)
	var sign_r := _sign_radius(float(inst.get("sign_share", 0.0)), float(inst.get("sign_built", 0.0)), d) if blend > 0.0 else 0.0
	if float(inst.get("held_edge", 0.0)) > 0.0:  # style `maquette` : bord de la maquette GC
		sign_r = float(inst["held_edge"])
	match str(inst.get("grow", "")):
		"out":
			var factor := model_factor(config, float(inst["width_m"]), int(inst["level"]), d, _mpu, _fov)
			if inst.has("dir") and blend > 0.0:
				var far := (inst["c"] as Vector2) + (inst["dir"] as Vector2) * (float(inst["edge"]) + float(inst["off"]) * d / float(inst["top_d"]))
				px = px.lerp(far, blend)
				# De loin, la façade regarde la caméra de jeu (vue du sud), ou l'eau pour un port.
				inst["draw_yaw"] = lerp_angle(float(inst["yaw"]), float(inst.get("far_yaw", 0.0)), blend)
			else:
				inst["draw_yaw"] = float(inst["yaw"])
			inst["factor"] = factor
			# Volumes relevés de loin : les silhouettes se lisent en plongée.
			inst["draw_scale"] = Vector3(factor, factor * lerpf(1.0, float(screen.get("vertical", 1.0)), blend), factor)
			if bool(inst.get("far_only", false)):
				if inst.has("dir") and blend > 0.0:
					px = (inst["c"] as Vector2) + (inst["dir"] as Vector2) * (float(inst["edge"]) + float(inst["off"]) * d / float(inst["top_d"]))
					inst["draw_yaw"] = float(inst.get("far_yaw", 0.0))
					inst["draw_scale"] = (inst["draw_scale"] as Vector3) * blend
				else:
					inst["draw_scale"] = Vector3.ONE * 1e-3
		"sign":
			var radius := _sign_radius(float(inst["fraction"]) * float(inst["radius_m"]) / float(inst["width_m"]), float(inst["built"]), d)
			var factor := float(inst["fraction"]) * view_span(d, _fov) * _mpu / float(inst["width_m"]) * blend
			inst["factor"] = factor
			inst["draw_scale"] = Vector3(factor, factor * float(screen.get("sign_vertical", 1.0)), factor) if radius > 0.0 and blend > 0.0 else Vector3.ONE * 1e-3
		"quarter":
			var width := float(inst["fraction"]) * view_span(d, _fov)
			var factor := width * _mpu / float(inst["width_m"]) * blend
			px = (inst["c"] as Vector2) + (inst["dir"] as Vector2) * (sign_r + float(inst["radius_m"]) * factor / _mpu)
			inst["factor"] = factor
			inst["draw_yaw"] = float(inst["yaw"])
			inst["draw_scale"] = Vector3(factor, factor * float(screen.get("vertical", 1.0)), factor) if sign_r > 0.0 and blend > 0.0 else Vector3.ONE * 1e-3
		"house":
			var factor := lerpf(1.0, _house_factor_full(d), blend)
			var origin: Vector2 = inst["origin"]
			if sign_r > 0.0:  # le quartier part du bord du signe de la ville
				origin = origin.lerp((inst["c"] as Vector2) + (inst["quarter_dir"] as Vector2) * sign_r, blend)
			px = origin + (px - (inst["origin"] as Vector2)) * factor
			inst["factor"] = factor
			inst["draw_scale"] = (inst["scale"] as Vector3) * Vector3(factor, factor * lerpf(1.0, float(screen.get("vertical", 1.0)), blend), factor)
			# De loin, un quartier se résume à ses premières maisons, plus grosses.
			if blend >= 0.5 and (int(inst.get("rank", 0)) >= int(screen.get("far_houses", 99)) or (sign_r > 0.0 and bool(inst.get("quarter_sign", false)))):
				inst["draw_scale"] = Vector3.ONE * 1e-3
		"wall":
			var shape: Dictionary = inst["shape"]
			var thick := lerpf(float(shape["thick_m"]), _wall_thickness_full(shape, d), blend)
			inst["draw_scale"] = Vector3(float(inst["length_m"]) + thick, maxf(float(inst["height_m"]), thick * 1.5) + float(shape["sink_m"]), thick)
			if sign_r > 0.0 and blend >= 0.5:  # l'enceinte est portée par le signe muré
				inst["draw_scale"] = Vector3.ONE * 1e-3
		"tower":
			var shape: Dictionary = inst["shape"]
			var thick := lerpf(float(shape["thick_m"]), _wall_thickness_full(shape, d), blend)
			var radius := maxf(float(inst["radius_m"]), thick * 0.9)
			# Tours trop serrées une fois grossies : une sur `stride` reste.
			var stride := maxi(1, ceili(radius * 3.2 / maxf(float(inst["spacing_m"]), 1.0)))
			var height := maxf(float(inst["height_m"]), thick * 2.2) + float(shape["sink_m"])
			inst["draw_scale"] = Vector3(radius, height, radius) if int(inst["order"]) % stride == 0 else Vector3.ONE * 1e-3
			if sign_r > 0.0 and blend >= 0.5:
				inst["draw_scale"] = Vector3.ONE * 1e-3
		"gate":
			var shape: Dictionary = inst["shape"]
			var thick := lerpf(float(shape["thick_m"]), _wall_thickness_full(shape, d), blend)
			var real: Vector3 = inst["scale"]
			inst["draw_scale"] = Vector3(maxf(real.x, thick * 1.6), maxf(float(inst["height_m"]), thick * 1.95) + float(shape["sink_m"]), maxf(real.z, thick * 1.6))
			if sign_r > 0.0 and blend >= 0.5:
				inst["draw_scale"] = Vector3.ONE * 1e-3
		_:
			return
	if not px.is_equal_approx(inst["px"]):
		inst["px"] = px
		inst["base_m"] = _base_m(px)


# --- Rendu ---------------------------------------------------------------------------------


func _mesh_of(key: String) -> Mesh:
	if _meshes.has(key):
		return _meshes[key]
	if _warm_task >= 0:
		_finish_warm()
		if _meshes.has(key):
			return _meshes[key]
	var mesh: Mesh = null
	if key.begins_with("out:"):
		var model := key.trim_prefix("out:")
		mesh = with_pieces(_load_glb_mesh(MODEL_DIR + model + ".glb"), (manifest.get(model, {}) as Dictionary).get("pieces", []))
	elif key.begins_with("kit:"):
		mesh = TownBuilder.kit_mesh(key.trim_prefix("kit:"))
	elif key.begins_with("box:"):
		mesh = TownBuilder.box_mesh(key.trim_prefix("box:"))
	elif key == "tower":
		mesh = TownBuilder.tower_mesh()
	_meshes[key] = mesh
	return mesh


static func _load_glb_mesh(path: String) -> Mesh:
	if not ResourceLoader.exists(path):
		return null
	var scene := load(path) as PackedScene
	if scene == null:
		return null
	var node := scene.instantiate()
	var mesh: Mesh = null
	for child in node.find_children("*", "MeshInstance3D", true, false):
		mesh = (child as MeshInstance3D).mesh
		break
	node.free()
	return mesh


## Copie du maillage d'une maquette avec, par sommet, le centre xz (m, repère de la maquette) de
## sa pièce rigide dans `CUSTOM0.xy` (`pieces` du manifeste : centre x, z, puis boîte x0, z0, x1,
## z1) : `town_building.gdshader` (`drape`) pose chaque pièce sur le sol sous son propre centre.
## Un sommet appartient à la plus petite boîte qui le contient, sinon à la pièce la plus proche.
static func with_pieces(mesh: Mesh, pieces: Array) -> Mesh:
	if mesh == null or pieces.is_empty() or mesh.get_surface_count() == 0:
		return mesh
	var out := ArrayMesh.new()
	for surface in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(surface)
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var custom := PackedFloat32Array()
		custom.resize(vertices.size() * 4)
		for v in vertices.size():
			var p := vertices[v]
			var best := -1
			var best_area := INF
			var nearest := 0
			var nearest_d := INF
			for k in pieces.size():
				var piece: Array = pieces[k]
				var d := Vector2(p.x - float(piece[0]), p.z - float(piece[1])).length_squared()
				if d < nearest_d:
					nearest_d = d
					nearest = k
				if p.x >= float(piece[2]) - 0.05 and p.x <= float(piece[4]) + 0.05 and p.z >= float(piece[3]) - 0.05 and p.z <= float(piece[5]) + 0.05:
					var area := (float(piece[4]) - float(piece[2])) * (float(piece[5]) - float(piece[3]))
					if area < best_area:
						best_area = area
						best = k
			var chosen: Array = pieces[best if best >= 0 else nearest]
			custom[v * 4] = float(chosen[0])
			custom[v * 4 + 1] = float(chosen[1])
		arrays[Mesh.ARRAY_CUSTOM0] = custom
		out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT)
	return out


## Matériau des maquettes tenues à l'écran : celui des villes, avec la pose par pièce sur la
## heightmap du terrain (`drape`, suivi à chaque image de la part de tenue à l'écran).
func _drape_material() -> ShaderMaterial:
	if _drape_mat == null:
		_drape_mat = TownBuilder.material(0, false, 0.0, _mpu).duplicate() as ShaderMaterial
		var texture: Texture2D = _terrain.height_texture() if _terrain != null and _terrain.has_method("height_texture") else null
		if texture != null and _map != null and _terrain.height_texture_mode() == 1:
			_drape_mat.set_shader_parameter("drape_heightmap", texture)
			_drape_mat.set_shader_parameter("drape_info", Vector4(float(_map.size.x), float(_map.size.y), _map.height_min_m, _map.height_max_m - _map.height_min_m))
			_drape_ready = true
		# Signes de carte : teintes franches, peu d'usure.
		_drape_mat.set_shader_parameter("aging", float(screen_config(config).get("aging", 0.5)))
	return _drape_mat


## Tampon d'instances (`TownBuilder.pack_instances`) avec la suie de chaque instance dans la
## composante b des données d'instance (`INSTANCE_CUSTOM.b` de `town_building.gdshader`).
static func with_soot(buffer: PackedFloat32Array, soots: Array) -> PackedFloat32Array:
	for k in soots.size():
		buffer[k * 16 + 14] = float(soots[k])
	return buffer


## Suie (0-1) portée par les instances de la colonie `id`.
func soot_of(id: String) -> float:
	return float(_soot.get(int(_data.index_by_id.get(id, -1)), 0.0)) if _data != null else 0.0


## Réécrit les `MultiMesh` depuis `_instances` (repère en mètres centré sur `_center`).
func _write_batches() -> void:
	if _root == null or _center.x == INF:
		return
	var s := 1.0 / _mpu
	_root.transform = Transform3D(Basis().scaled(Vector3(s, s, s)), Vector3(_center.x, 0.0, _center.y))
	_placed_distance = _rig_distance
	if _drape_mat != null and _drape_ready:
		_drape_mat.set_shader_parameter("drape", screen_blend(config, _rig_distance))
	var groups := {}  # clé → [xforms, bases, tints, top, suies]
	for inst: Dictionary in _instances:
		_place(inst)
		var key := str(inst["key"])
		if not groups.has(key):
			groups[key] = [[], [], [], 0.0, []]
		var g: Array = groups[key]
		var px: Vector2 = inst["px"]
		var draw: Vector3 = inst.get("draw_scale", inst["scale"])
		var grow := float(inst.get("factor", 1.0))
		var basis := Basis(Vector3.UP, float(inst.get("draw_yaw", inst["yaw"]))) * Basis.from_scale(draw)
		(g[0] as Array).append(Transform3D(basis, Vector3((px.x - _center.x) * _mpu, float(inst.get("lift", 0.0)), (px.y - _center.y) * _mpu)))
		(g[1] as Array).append(float(inst["base_m"]))
		var tint := float(inst.get("tint", 0.5))
		if str(inst.get("grow", "")) == "house":  # faubourgs éclaircis de loin, comme les maquettes
			tint += _brighten() * screen_blend(config, _rig_distance)
		(g[2] as Array).append(tint)
		g[3] = maxf(float(g[3]), maxf((float(inst.get("top", 20.0)) + float(inst.get("reach", 30.0))) * grow, draw.y * 1.5 + maxf(draw.x, draw.z)))
		(g[4] as Array).append(float(_soot.get(int(inst["settlement"]), 0.0)))
	for key: String in _batches.keys():
		if not groups.has(key):
			(_batches[key] as Node).free()
			_batches.erase(key)
	var total := 0
	for key: String in groups:
		var mesh := _mesh_of(key)
		if mesh == null:
			continue
		var g: Array = groups[key]
		var packed := TownBuilder.pack_instances(g[0], g[1], g[2])
		var buffer := with_soot(packed["buffer"], g[4])
		var mmi: MultiMeshInstance3D = _batches.get(key)
		if mmi == null:
			mmi = MultiMeshInstance3D.new()
			mmi.name = key.replace(":", "_")
			mmi.multimesh = MapInstancing.make(mesh, 0, true)
			mmi.material_override = _drape_material() if key.begins_with("out:") else TownBuilder.material(0, not key.begins_with("kit:"), 0.0, _mpu)
			mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if _shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			mmi.transparency = 1.0 - _fade
			_root.add_child(mmi)
			_batches[key] = mmi
		mmi.multimesh.instance_count = int(packed["count"])
		mmi.multimesh.buffer = buffer
		mmi.set_meta("bounds", [float(packed["lo"]), float(packed["hi"]), float(g[3]), (packed["rect"] as Rect2).grow(float(g[3]))])
		total += int(packed["count"])
	_refresh_aabbs()
	stats["instances"] = total
	stats["nodes"] = _batches.size()


## Boîtes englobantes (mètres, repère du voisinage) : la base est ajoutée par le shader, Godot ne
## la voit pas (même calcul que `TownBuilder.refresh_aabbs`).
func _refresh_aabbs() -> void:
	var vertical := MapData.vertical_scale()
	var k := vertical * _mpu
	var up := 1.0 + MapData.relief_gain_for_scale(vertical)
	var down := 1.0 - MapData.relief_squash_max_for_scale(vertical)
	for mmi: MultiMeshInstance3D in _batches.values():
		var b: Array = mmi.get_meta("bounds", [])
		if b.size() < 4:
			continue
		var rect: Rect2 = b[3]
		# ± 600 m : les pièces d'une maquette tenue à l'écran suivent le sol sous elles.
		var y0 := (float(b[0]) - 600.0) * k * (down if float(b[0]) > 600.0 else 1.0) - 10.0
		var y1 := (float(b[1]) + 600.0) * k * (up if float(b[1]) > -600.0 else 1.0) + float(b[2])
		mmi.custom_aabb = AABB(Vector3(rect.position.x, y0, rect.position.y), Vector3(rect.size.x, y1 - y0, rect.size.y))


func _on_chunk_surface_changed(_index: int) -> void:
	if _terrain != null and _terrain.rescaling_vertical:
		return
	if visible and not _instances.is_empty():
		_reground_in = REGROUND_DELAY


## Hauteurs de base relues sur la surface affichée (pages plus fines arrivées).
func _reground() -> void:
	var changed := false
	for inst: Dictionary in _instances:
		var base := _base_m(inst["px"]) if not inst.has("base_px") else _base_m(inst["base_px"])
		if absf(base - float(inst["base_m"])) > 0.05:
			inst["base_m"] = base
			changed = true
	if changed:
		_write_batches()


# --- Lecture pour les tests et les mesures -------------------------------------------------


## Instances affichées (dictionnaires : `key`, `settlement`, `family`, `level`, `px`, `yaw`…).
func instances() -> Array:
	return _instances


## Instances affichées de la colonie `id` (toutes, ou d'une famille).
func instances_of(id: String, family: String = "") -> Array:
	var i: int = _data.index_by_id.get(id, -1) if _data != null else -1
	var out: Array = []
	for inst: Dictionary in _instances:
		if int(inst["settlement"]) == i and (family == "" or str(inst.get("family", "")) == family):
			out.append(inst)
	return out


func instance_count() -> int:
	return int(stats.get("instances", 0))


## Nombre de nœuds de rendu (un appel de dessin par nœud et par passe).
func node_count() -> int:
	return _batches.size()
