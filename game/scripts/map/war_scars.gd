class_name WarScars
extends Node3D

## Lot TB4 (`docs/design/2026-10-02-campagne-tob.md` § 3) : conséquences visibles de la guerre et
## des fléaux sur la carte de campagne. Rendu seulement : tout l'état vient du pont, les seuils et
## durées de `data/ui/war_scars.json` (schéma `war_scars_ui`). Aucun pictogramme d'interface.
##
## - **Peste** (`set_plague_sites`, scènes `plague` de `get_map_scenes` résolues par `FolkScenes`) :
##   fosses communes groupées au bord de la colonie et charrette des morts sur la route, grossies
##   selon la distance de la caméra pour rester lisibles aux distances de jeu
##   (`plague.scale_per_distance`, sous `plague.max_distance`) ; portes marquées d'une croix sur
##   les maisons de la ville 1:1 ordinaire (`plan_provider`), à l'échelle réelle.
## - **Champ de bataille** (`refresh`, événements `battle` de `get_events` et
##   `get_pending_events`) : tertre, débris, corbeaux à l'endroit de la bataille (position de
##   l'armée de l'événement, à défaut centre de la province) pendant `battlefield.turns` tours ;
##   corbeaux puis débris partent plus tôt. Taille des figurines d'armée (même loi d'échelle).
##   Mémoire du rendu : le cœur ne garde pas l'historique des batailles, une partie rechargée ne
##   retrouve que celles du dernier tour.
## - **Siège** (`refresh`, `get_assault_odds(armée).engines`) : les engins bâtis sur place (N7,
##   ADR 0128) près du camp des assiégeants : charpente et tas de bois, puis maquette sous
##   échafaudage, puis engin prêt. Enfants des figurines de l'armée (échelle et visibilité suivies).
## Branché par `CampaignLife` (`--life-off=scars` le coupe).

const DATA_PATH := "ui/war_scars.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const DEFAULT_METERS_PER_UNIT := 719.0
## Recalage au sol et recherche des plans de ville : au plus une fois par intervalle (s).
const REGROUND_INTERVAL := 0.25
const DOOR_RETRY_INTERVAL := 0.5
## Écart (unités monde) entre l'armée et le tertre (pas sous les figurines du vainqueur).
const FIELD_OFFSET := 0.7
## Hauteur d'un homme dans le repère des figurines d'armée (`ArmyFigures.FIGURE_SCALE` 2,3 ×
## 1,88 m de maillage) : taille des débris et des corbeaux.
const FIGURE_UNIT := 4.3
const ENGINES_NODE := "SiegeEngines"

static var _settings: Dictionary = {}
static var _loaded := false

var stats: Dictionary = {}
## Plan de la ville 1:1 d'une colonie : `func(id) -> {plan, anchor: Vector2, meters_per_unit}`
## (`{}` tant qu'elle n'est pas chargée). Sans lui, pas de portes marquées.
var plan_provider: Callable = Callable()
## Captures (`tb4_shot.gd`) : engins imposés par armée (`[{kind, ready, turns_left}]`), à la place
## de ceux du pont.
var forced_engines: Dictionary = {}

var _map_data: MapData = null
var _terrain: TerrainBuilder = null
## `ArmyMarkers` (ou objet de test) : `marker_ids()`, `marker_of(id)`, `world_position_of(id)`,
## `hidden_provinces`.
var _armies: Object = null
## colonie → {key, node, site, intensity, seed, doors_done, factor, stale}
var _plague: Dictionary = {}
## clé (province ou « army:<id> ») → {at, province, turn, texts, node, stage, seed}
var _fields: Dictionary = {}
## armée → {marker, signature, node}
var _engines: Dictionary = {}
var _turn := -1
var _siege_key := ""
var _time := 0.0
var _ground_dirty := false
var _reground_timer := 0.0
var _door_timer := 0.0


# --- Réglages --------------------------------------------------------------------------


static func settings() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := _data_dir().path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_settings = parsed
			else:
				push_warning("WarScars: %s invalid" % path)
	return _settings


## Oublie le fichier lu (tests : autre dossier de données).
static func reload() -> void:
	_loaded = false
	_settings = {}


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


static func _block(key: String) -> Dictionary:
	var block: Variant = settings().get(key, {})
	return block if block is Dictionary else {}


## Tours pendant lesquels un champ de bataille reste marqué (0 sans fichier : rien n'est posé).
static func battlefield_turns() -> int:
	return int(_block("battlefield").get("turns", 0))


static func _count_for(range_value: Variant, intensity: float) -> int:
	if not (range_value is Array) or (range_value as Array).size() < 2:
		return 0
	return maxi(roundi(lerpf(float(range_value[0]), float(range_value[1]), clampf(intensity, 0.0, 1.0))), 0)


static func _h(a: int, b: int) -> float:
	return float(absi(hash(Vector2i(a, b))) % 10007) / 10007.0


# --- Branchement -----------------------------------------------------------------------


func setup(map_data: MapData, terrain: TerrainBuilder, armies: Object) -> void:
	name = "WarScars"
	_map_data = map_data
	_terrain = terrain
	_armies = armies
	if _terrain != null:
		# Exagération du relief ou pages plus fines : les éléments posés au sol sont recalés.
		_terrain.vertical_scale_changed.connect(func(_old: float, _new: float) -> void: _ground_dirty = true)
		_terrain.chunk_surface_changed.connect(func(_index: int) -> void: _ground_dirty = true)


## Partie chargée : champs de bataille de l'ancienne partie oubliés, tout est relu.
func reset() -> void:
	for key in _fields.keys():
		_drop_field(key)
	_turn = -1
	_siege_key = ""


func _meters_per_unit() -> float:
	return _map_data.meters_per_px if _map_data != null else DEFAULT_METERS_PER_UNIT


func _ground(xz: Vector2) -> float:
	if _terrain != null:
		return _terrain.surface_height_at(xz.x, xz.y)
	if _map_data != null:
		return _map_data.surface_world_at(xz.x, xz.y)
	return 0.0


## Après tout changement d'état : batailles du tour (et de la main du joueur), âge des champs
## marqués, engins des sièges en cours.
func refresh(sim: Object) -> void:
	if sim == null:
		return
	var turn := int(sim.call("get_turn")) if sim.has_method("get_turn") else 0
	if turn < _turn:
		reset()
	var new_turn := turn != _turn
	_turn = turn
	if new_turn and sim.has_method("get_events"):
		_read_battles(sim, sim.call("get_events"))
	if sim.has_method("get_pending_events"):
		_read_battles(sim, sim.call("get_pending_events"))
	_age_fields()
	_refresh_sieges(sim, new_turn)
	_update_stats()


func _update_stats() -> void:
	var doors := 0
	var pits := 0
	for id in _plague:
		var node: Node3D = _plague[id]["node"]
		for child in node.get_children():
			if str(child.name).begins_with("Door_"):
				doors += 1
			elif str(child.name).begins_with("Pit_"):
				pits += 1
	# Même dictionnaire mis à jour (référencé par `CampaignLife.stats`).
	stats.merge({"plague_sites": _plague.size(), "pits": pits, "doors": doors, "battlefields": _fields.size(), "sieges": _engines.size()}, true)


# --- Peste -----------------------------------------------------------------------------


## Colonies pestiférées du tour : `[{settlement, center, out, footprint, intensity, seed, road}]`
## (`FolkScenes.edge_frame`), `center` et `footprint` en unités monde, `out` unitaire, `road` :
## tracé d'une route qui sort de la colonie (facultatif).
func set_plague_sites(sites: Array) -> void:
	var block := _block("plague")
	var seen := {}
	for entry in sites:
		if not (entry is Dictionary) or block.is_empty():
			continue
		var site: Dictionary = entry
		var id := str(site.get("settlement", ""))
		var intensity := clampf(float(site.get("intensity", 1.0)), 0.0, 1.0)
		if id == "" or intensity < float(block.get("min_intensity", 0.0)):
			continue
		seen[id] = true
		var key := "%s|%.2f|%s" % [id, intensity, site.get("center", Vector2.ZERO)]
		var existing: Variant = _plague.get(id)
		if existing != null and str(existing["key"]) == key:
			continue
		if existing != null:
			(existing["node"] as Node).free()
		_plague[id] = _build_plague(id, site, intensity, block, key)
	for id in _plague.keys():
		if not seen.has(id):
			(_plague[id]["node"] as Node).free()
			_plague.erase(id)
	_update_stats()


func _build_plague(id: String, site: Dictionary, intensity: float, block: Dictionary, key: String) -> Dictionary:
	var root := Node3D.new()
	root.name = "Plague_%s" % id
	root.visible = false
	add_child(root)
	var seed_value := int(site.get("seed", id.hash()))
	var pits := _count_for(block.get("pits", []), intensity)
	for k in pits:
		# Deux rangs en quinconce, groupés au bord de la colonie (mètres avant grossissement).
		var pit := _add_prop(root, "Pit_%d" % k, WarScarMeshes.plague_pit())
		pit.set_meta("offset", Vector2(10.0 + 7.5 * float(k % 2), (float(k) - float(pits - 1) * 0.5) * 8.5))
		pit.set_meta("turn", (_h(seed_value, 40 + k) - 0.5) * 0.5)
	if bool(block.get("cart", false)) and pits > 0:
		var cart := _add_prop(root, "DeadCart", FolkModels.prop_mesh("dead_cart"))
		cart.set_meta("cart", float(pits))
	return {"key": key, "node": root, "site": site, "intensity": intensity, "seed": seed_value, "doors_done": false, "factor": -1.0}


func _add_prop(parent: Node3D, node_name: String, mesh: Mesh) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	parent.add_child(instance)
	return instance


## Grossissement des fosses et de la charrette par rapport à l'échelle réelle : proportionnel à
## la distance de la caméra (taille à l'écran constante, comme les armées), jamais sous 1.
static func plague_factor(camera_distance: float) -> float:
	var block := _block("plague")
	return clampf(float(block.get("scale_per_distance", 0.0)) * camera_distance, 1.0, maxf(float(block.get("max_scale", 1.0)), 1.0))


## Place fosses et charrette d'une colonie au grossissement `factor` : fosses groupées au bord,
## charrette sur la route qui sort de la colonie (`site.road`), à défaut près des fosses.
func _layout_plague(entry: Dictionary, factor: float) -> void:
	var site: Dictionary = entry["site"]
	var unit := factor / _meters_per_unit()
	var center: Vector2 = site.get("center", Vector2.ZERO)
	var out: Vector2 = site.get("out", Vector2.RIGHT)
	var along := Vector2(-out.y, out.x)
	var footprint := float(site.get("footprint", 0.0))
	var edge := center + out * footprint
	for child in (entry["node"] as Node3D).get_children():
		var node := child as Node3D
		if node.has_meta("offset"):
			var offset: Vector2 = node.get_meta("offset")
			var at := edge + (out * offset.x + along * offset.y) * unit
			var dir := along.rotated(float(node.get_meta("turn", 0.0)))
			node.transform = Transform3D(Basis(Vector3.UP, atan2(-dir.y, dir.x)).scaled(Vector3.ONE * unit), Vector3(at.x, _ground(at), at.y))
		elif node.has_meta("cart"):
			var cart_unit := unit * maxf(float(_block("plague").get("cart_scale", 1.0)), 1.0)
			var at := edge + (out * 2.0 - along * (float(node.get_meta("cart")) * 4.3 + 6.0)) * unit
			var dir := along
			var road: Variant = site.get("road")
			if road is PackedVector2Array and (road as PackedVector2Array).size() >= 2:
				var spot := road_spot(road, center, footprint + 6.0 * cart_unit)
				at = spot[0]
				dir = spot[1]
			node.transform = Transform3D(Basis(Vector3.UP, atan2(dir.x, dir.y)).scaled(Vector3.ONE * cart_unit), Vector3(at.x, _ground(at), at.y))
		elif node.has_meta("xz"):  # portes : échelle réelle, seul le sol change
			var xz: Vector2 = node.get_meta("xz")
			node.position.y = _ground(xz) + float(node.get_meta("lift", 0.0))


## Point d'un tracé (orienté depuis la colonie) à `distance` du centre, et direction du tracé en
## ce point : `[Vector2, Vector2]` ; fin du tracé s'il est plus court.
static func road_spot(road: PackedVector2Array, center: Vector2, distance: float) -> Array:
	for i in road.size() - 1:
		var a := road[i]
		var b := road[i + 1]
		var da := a.distance_to(center)
		var db := b.distance_to(center)
		if db >= distance and b != a:
			var t := clampf((distance - da) / maxf(db - da, 1e-6), 0.0, 1.0)
			return [a.lerp(b, t), (b - a).normalized()]
	var last := road.size() - 1
	var dir := road[last] - road[last - 1]
	return [road[last], dir.normalized() if dir.length() > 0.0 else Vector2.RIGHT]


## Pose un maillage au sol : méta `xz` (point carte) et `lift` pour les recalages.
func _place(parent: Node3D, node_name: String, mesh: Mesh, xz: Vector2, yaw: float, unit: float, lift: float, basis: Variant = null) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	instance.set_meta("xz", xz)
	instance.set_meta("lift", lift)
	var b: Basis = basis if basis is Basis else Basis(Vector3.UP, yaw)
	instance.transform = Transform3D(b.scaled(Vector3.ONE * unit), Vector3(xz.x, _ground(xz) + lift, xz.y))
	parent.add_child(instance)
	return instance


## Portes marquées d'une croix sur les maisons de la ville 1:1, dès que son plan est chargé.
## Vrai une fois posées (ou s'il n'y a rien à poser).
func _try_doors(id: String) -> bool:
	var entry: Dictionary = _plague[id]
	var block := _block("plague")
	var count := _count_for(block.get("doors", []), float(entry["intensity"]))
	if count <= 0:
		return true
	if not plan_provider.is_valid():
		return false
	var info: Variant = plan_provider.call(id)
	if info is Dictionary and bool((info as Dictionary).get("none", false)):
		entry["doors_done"] = true  # ville sans portes marquées (croix illisibles : voir `CampaignLife`)
		return true
	if not (info is Dictionary) or not ((info as Dictionary).get("plan") is Dictionary):
		return false
	var houses: Variant = ((info as Dictionary)["plan"] as Dictionary).get("houses")
	if not (houses is Dictionary) or not (houses as Dictionary).has("x"):
		return false
	var xs: PackedFloat32Array = houses["x"]
	var ys: PackedFloat32Array = houses["y"]
	if xs.is_empty():
		return false
	var anchor: Vector2 = info.get("anchor", Vector2.ZERO)
	var meters_per_unit := float(info.get("meters_per_unit", _meters_per_unit()))
	var search := float(block.get("door_search_m", 0.0))
	var seed_value := int(entry["seed"])
	var order: Array = []
	for i in xs.size():
		if search <= 0.0 or Vector2(xs[i], ys[i]).length() <= search:
			order.append([_h(seed_value, 1000 + i), i])
	if order.is_empty():
		for i in xs.size():
			order.append([_h(seed_value, 1000 + i), i])
	order.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0])
	var root: Node3D = entry["node"]
	var unit := 1.0 / meters_per_unit
	for k in mini(count, order.size()):
		var i: int = order[k][1]
		var yaw: float = houses["yaw"][i]
		# `yaw` : axe X de la maison (le long de la façade) ; la façade regarde +Z du modèle,
		# vers la rue (`TownBuilder.basis_x`).
		var d := Vector2(cos(yaw), sin(yaw))
		var facing := Vector2(-d.y, d.x)
		var local := Vector2(xs[i], ys[i]) + facing * (float(houses["depth"][i]) * 0.5 + 0.15)
		var basis := Basis(Vector3(d.x, 0.0, d.y), Vector3.UP, Vector3(facing.x, 0.0, facing.y))
		_place(root, "Door_%d" % k, WarScarMeshes.marked_door(), anchor + local * unit, 0.0, unit, 0.0, basis)
	entry["doors_done"] = true
	_update_stats()
	return true


# --- Champs de bataille ----------------------------------------------------------------


func _read_battles(sim: Object, events: Variant) -> void:
	if battlefield_turns() <= 0 or not (events is Array):
		return
	for entry in events:
		if not (entry is Dictionary) or str((entry as Dictionary).get("kind", "")) != "battle":
			continue
		var event: Dictionary = entry
		var province := str(event.get("province", ""))
		var army := str(event.get("army", ""))
		var key := province if province != "" else ("army:" + army if army != "" else "")
		if key == "":
			continue
		var text := str(event.get("text_fr", ""))
		var existing: Variant = _fields.get(key)
		if existing != null and (existing["texts"] as Dictionary).has(text):
			continue  # déjà lu (événement de la main du joueur repris au journal du tour suivant)
		var at := _battle_position(sim, army, province)
		if existing != null:
			existing["turn"] = _turn
			existing["texts"][text] = true
			continue
		if at.x < 0.0:
			continue
		add_battlefield(key, at, _turn, province)
		_fields[key]["texts"][text] = true


## Endroit de la bataille : position de l'armée de l'événement (marqueur, sinon état du pont), à
## défaut centre de la province ; (-1, -1) si inconnu.
func _battle_position(sim: Object, army: String, province: String) -> Vector2:
	var at := Vector2(-1.0, -1.0)
	if army != "":
		if _armies != null and _armies.has_method("world_position_of"):
			var world: Vector3 = _armies.call("world_position_of", army)
			if world != Vector3.ZERO:
				at = Vector2(world.x, world.z)
		if at.x < 0.0 and sim != null and sim.has_method("get_army"):
			var state: Dictionary = sim.call("get_army", army)
			if state.get("position") is Vector2:
				at = state["position"]
	if at.x < 0.0 and province != "" and _map_data != null and _map_data.index_of_id(province) > 0:
		at = _map_data.centroid_of_id(province)
	if at.x < 0.0:
		return at
	var angle := _h(hash(army + province), 7) * TAU
	return at + Vector2.from_angle(angle) * FIELD_OFFSET


## Marque un champ de bataille en `at` (point carte) au tour `turn` (aussi : captures, tests).
func add_battlefield(key: String, at: Vector2, turn: int, province: String = "") -> void:
	if _fields.has(key):
		_drop_field(key)
	_fields[key] = {"at": at, "province": province, "turn": turn, "texts": {}, "node": null, "stage": "", "seed": absi(key.hash())}
	_age_fields()
	_update_stats()


func _drop_field(key: String) -> void:
	var node: Variant = _fields[key]["node"]
	if node != null and is_instance_valid(node):
		(node as Node).free()
	_fields.erase(key)


## Retire les champs trop anciens, reconstruit ceux dont l'état change (corbeaux partis, débris
## ramassés).
func _age_fields() -> void:
	var block := _block("battlefield")
	var turns := int(block.get("turns", 0))
	for key in _fields.keys():
		var field: Dictionary = _fields[key]
		var age := _turn - int(field["turn"])
		if age >= turns or age < 0:
			_drop_field(key)
			continue
		var crows := age < int(block.get("crow_turns", 0))
		var debris := age < int(block.get("debris_turns", 0))
		var stage := "%d%d" % [int(crows), int(debris)]
		if stage != str(field["stage"]) or field["node"] == null:
			if field["node"] != null and is_instance_valid(field["node"]):
				(field["node"] as Node).free()
			field["stage"] = stage
			field["node"] = _build_field(str(key), field, block, crows, debris)
			field.erase("scale")  # nouvelle mise en place à la prochaine vue


func _build_field(key: String, field: Dictionary, block: Dictionary, crows: bool, debris: bool) -> Node3D:
	var root := Node3D.new()
	root.name = "Battlefield_%s" % key.replace(":", "_")
	root.visible = false
	add_child(root)
	var radius := float(block.get("radius", 2.0))
	var seed_value := int(field["seed"])
	var mound := MeshInstance3D.new()
	mound.name = "Mound"
	mound.mesh = WarScarMeshes.mound(radius * 0.4, radius * 0.17)
	mound.set_meta("local", Vector3.ZERO)
	mound.set_meta("yaw", _h(seed_value, 1) * TAU)
	root.add_child(mound)
	if debris:
		for k in int(block.get("debris", 0)):
			var angle := _h(seed_value, 10 + k) * TAU
			var reach := radius * (0.5 + 0.5 * sqrt(_h(seed_value, 60 + k)))
			var piece := MeshInstance3D.new()
			piece.name = "Debris_%d" % k
			piece.mesh = WarScarMeshes.debris(int(_h(seed_value, 110 + k) * 5.0), FIGURE_UNIT)
			piece.set_meta("local", Vector3(cos(angle) * reach, 0.0, sin(angle) * reach))
			piece.set_meta("yaw", _h(seed_value, 160 + k) * TAU)
			root.add_child(piece)
	if crows:
		for k in int(block.get("crows", 0)):
			var crow := MeshInstance3D.new()
			crow.name = "Crow_%d" % k
			crow.mesh = WarScarMeshes.crow(FIGURE_UNIT * 0.55)
			crow.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			# Orbite : rayon, hauteur, vitesse angulaire (sens tiré), phase.
			crow.set_meta("orbit", Vector4(radius * (0.35 + 0.5 * _h(seed_value, 210 + k)), radius * (0.55 + 0.5 * _h(seed_value, 260 + k)),
				(0.35 + 0.3 * _h(seed_value, 310 + k)) * (1.0 if _h(seed_value, 360 + k) < 0.5 else -1.0), _h(seed_value, 410 + k) * TAU))
			root.add_child(crow)
	return root


## Place les éléments d'un champ à l'échelle `scale` (unités monde par unité du repère des
## figurines) : chacun sur le sol de son propre point (les champs en pente ne flottent pas).
func _layout_field(field: Dictionary, scale_value: float) -> void:
	var root: Node3D = field["node"]
	var at: Vector2 = field["at"]
	field["ground"] = _ground(at)
	for child in root.get_children():
		var node := child as Node3D
		if not node.has_meta("local"):
			continue
		var local: Vector3 = node.get_meta("local")
		var xz := at + Vector2(local.x, local.z) * scale_value
		node.transform = Transform3D(Basis(Vector3.UP, float(node.get_meta("yaw", 0.0))).scaled(Vector3.ONE * scale_value),
			Vector3(xz.x, _ground(xz) + local.y * scale_value, xz.y))


func _fly_crows(field: Dictionary, scale_value: float) -> void:
	var root: Node3D = field["node"]
	var at: Vector2 = field["at"]
	var ground: float = field.get("ground", 0.0)
	for child in root.get_children():
		var node := child as Node3D
		if not node.has_meta("orbit"):
			continue
		var orbit: Vector4 = node.get_meta("orbit")
		var angle := orbit.w + _time * orbit.z
		var bob := sin(_time * 1.7 + orbit.w * 3.0) * 0.08
		var local := Vector3(cos(angle) * orbit.x, orbit.y * (1.0 + bob), sin(angle) * orbit.x)
		# Nez (+Z) le long de la tangente de l'orbite, léger roulis vers l'intérieur.
		var basis := Basis(Vector3.UP, -angle + (0.0 if orbit.z > 0.0 else PI)) * Basis(Vector3.BACK, 0.3 * signf(orbit.z))
		node.transform = Transform3D(basis.scaled(Vector3.ONE * scale_value), Vector3(at.x, ground, at.y) + local * scale_value)


# --- Sièges ----------------------------------------------------------------------------


## Engins à montrer pour la liste `engines` du pont (`[{kind, ready, turns_left}]`, dans l'ordre
## de construction) : les prêts, puis celui en chantier (`frame`, ou `almost` à
## `almost_ready_turns` tours ou moins) ; les suivants n'ont pas commencé.
static func engine_stages(engines: Variant, almost_ready_turns: int) -> Array:
	var out: Array = []
	if not (engines is Array):
		return out
	for entry in engines:
		if not (entry is Dictionary):
			continue
		var kind := str((entry as Dictionary).get("kind", ""))
		if bool((entry as Dictionary).get("ready", false)):
			out.append({"kind": kind, "stage": "ready"})
		else:
			out.append({"kind": kind, "stage": "almost" if int((entry as Dictionary).get("turns_left", 0)) <= almost_ready_turns else "frame"})
			break
	return out


func _refresh_sieges(sim: Object, new_turn: bool) -> void:
	var block := _block("siege")
	if _armies == null or block.is_empty() or not _armies.has_method("marker_ids") or not sim.has_method("get_assault_odds"):
		return
	var besiegers := {}
	for id in _armies.call("marker_ids"):
		var marker: Variant = _armies.call("marker_of", id)
		if marker != null and str((marker as Object).get("status")) == "siege":
			besiegers[str(id)] = marker
	var ids := besiegers.keys()
	ids.sort()
	var key := ",".join(PackedStringArray(ids))
	var stale := new_turn or key != _siege_key
	_siege_key = key
	for id in _engines.keys():
		if not besiegers.has(id):
			var gone: Variant = _engines[id]["node"]
			if gone != null and is_instance_valid(gone):
				(gone as Node).free()
			_engines.erase(id)
	for id in ids:
		var marker: Object = besiegers[id]
		var entry: Variant = _engines.get(id)
		var attached: bool = entry != null and entry["marker"] == marker and entry["node"] != null and is_instance_valid(entry["node"])
		if attached and not stale:
			continue
		var engines: Variant = forced_engines[id] if forced_engines.has(id) else (sim.call("get_assault_odds", id) as Dictionary).get("engines", [])
		var stages := engine_stages(engines, int(block.get("almost_ready_turns", 0)))
		var signature := str(stages)
		if attached and str(entry["signature"]) == signature:
			continue
		if entry != null and entry["node"] != null and is_instance_valid(entry["node"]):
			(entry["node"] as Node).free()
		_engines[id] = {"marker": marker, "signature": signature, "node": _build_engines(marker, stages, block)}


## Engins posés près du camp : enfants des figurines de l'armée (repère, échelle et visibilité du
## camp), à défaut du marqueur.
func _build_engines(marker: Object, stages: Array, block: Dictionary) -> Node3D:
	var host := marker.get("figures") as Node3D
	if host == null:
		host = marker as Node3D
	if host == null:
		return null
	var previous := host.get_node_or_null(ENGINES_NODE)
	if previous != null:
		previous.free()
	var root := Node3D.new()
	root.name = ENGINES_NODE
	host.add_child(root)
	var configs: Dictionary = block.get("engines", {})
	for stage in stages:
		var kind := str(stage["kind"])
		if not (configs.get(kind) is Dictionary):
			continue
		var node := _engine_node(kind, str(stage["stage"]), configs[kind])
		var slot: Array = configs[kind].get("slot", [0.0, 0.0])
		node.position = Vector3(float(slot[0]), 0.0, float(slot[1]))
		root.add_child(node)
	for geometry in root.find_children("*", "GeometryInstance3D", true, false):
		(geometry as GeometryInstance3D).layers = 2  # comme le camp (`ArmyFigures._build_camp`)
	return root


## Un engin à son stade : `frame` (charpente et tas de bois), `almost` (maquette sous
## échafaudage), `ready` (maquette seule). Échelles : procédurales à tous les stades.
func _engine_node(kind: String, stage: String, config: Dictionary) -> Node3D:
	var node := Node3D.new()
	node.name = "Engine_%s_%s" % [kind, stage]
	var size := float(config.get("size", 1.0))
	if kind == "ladders":
		var rack := Vector3(1.0, 0.25, 0.6) * size
		if stage != "ready":
			_add_mesh(node, "Timber", WarScarMeshes.timber_pile(Vector3(rack.x * 0.8, rack.y * 0.6, rack.z * 0.5)))
		if stage != "frame":
			var racks := _add_mesh(node, "Ladders", WarScarMeshes.ladders(3 if stage == "ready" else 1))
			racks.scale = Vector3.ONE * size
			racks.position.z = 0.0 if stage == "ready" else rack.z * 0.6
		return node
	var model := _engine_model(str(config.get("model", "")), size) if stage != "frame" else null
	var dims := Vector3(0.4, 1.0, 0.4) * size if kind == "tower" else Vector3(1.0, 0.45, 0.4) * size
	if model != null:
		dims = model.get_meta("dims")
		node.add_child(model)
	elif stage != "frame":
		_add_mesh(node, "Model", WarScarMeshes.engine_fallback(kind, dims))
	if stage == "frame":
		_add_mesh(node, "Frame", WarScarMeshes.engine_frame(Vector3(dims.x, dims.y * 0.6, dims.z)))
	elif stage == "almost":
		_add_mesh(node, "Scaffold", WarScarMeshes.scaffold(dims))
	return node


func _add_mesh(parent: Node3D, node_name: String, mesh: Mesh) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	parent.add_child(instance)
	return instance


## Maquette d'un engin (kit N7 / SG2) ramenée à `size` pour sa plus grande dimension, posée au
## sol et centrée ; méta `dims` = emprise. Null si absente.
static func _engine_model(path: String, size: float) -> Node3D:
	if path == "" or not ResourceLoader.exists(path):
		return null
	var scene := load(path) as PackedScene
	var model := scene.instantiate() as Node3D if scene != null else null
	if model == null:
		return null
	model.name = "Model"
	var box := AABB()
	var first := true
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var instance := child as MeshInstance3D
		if instance.mesh == null:
			continue
		var xform := Transform3D.IDENTITY
		var walk: Node = instance
		while walk != null and walk != model:
			if walk is Node3D:
				xform = (walk as Node3D).transform * xform
			walk = walk.get_parent()
		var part: AABB = xform * instance.mesh.get_aabb()
		box = part if first else box.merge(part)
		first = false
	var longest := maxf(box.size.x, maxf(box.size.y, box.size.z))
	if first or longest <= 0.0:
		model.free()
		return null
	var factor := size / longest
	model.scale = Vector3.ONE * factor
	model.position = Vector3(-box.get_center().x, -box.position.y, -box.get_center().z) * factor
	model.set_meta("dims", box.size * factor)
	return model


# --- Vue -------------------------------------------------------------------------------


## À chaque image : visibilité par palier, taille des champs de bataille (loi d'échelle des
## armées), vol des corbeaux, recalage au sol différé. `normal_weight` : 1 en vue normale, 0 sur
## le parchemin.
func update_view(camera_distance: float, normal_weight: float) -> void:
	var delta := get_process_delta_time()
	_time += delta
	_reground_timer -= delta
	_door_timer -= delta
	var normal := normal_weight > 0.02
	var plague_block := _block("plague")
	var plague_on := normal and camera_distance <= float(plague_block.get("max_distance", 0.0))
	var plague_scale := plague_factor(camera_distance)
	if _ground_dirty and _reground_timer <= 0.0:
		# Sol changé (exagération du relief, pages plus fines) : chaque élément est recalé à sa
		# prochaine vue, pas seulement ceux visibles à cet instant.
		for id in _plague:
			_plague[id]["stale"] = true
		for key in _fields:
			_fields[key].erase("scale")
		_ground_dirty = false
		_reground_timer = REGROUND_INTERVAL
	for id in _plague:
		var entry: Dictionary = _plague[id]
		var root: Node3D = entry["node"]
		root.visible = plague_on
		if not plague_on:
			continue
		if not bool(entry["doors_done"]) and _door_timer <= 0.0:
			_try_doors(str(id))
		# Fosses et charrette : taille à l'écran tenue (grossissement selon la distance) ; mise en
		# place à la première vue, quand il change de plus de 2 %, ou après un recalage du sol.
		var laid := float(entry.get("factor", -1.0))
		if laid < 0.0 or bool(entry.get("stale", false)) or absf(plague_scale - laid) > laid * 0.02:
			entry["stale"] = false
			entry["factor"] = plague_scale
			_layout_plague(entry, plague_scale)
	if _door_timer <= 0.0:
		_door_timer = DOOR_RETRY_INTERVAL
	var field_block := _block("battlefield")
	var fields_on := normal and camera_distance <= float(field_block.get("max_distance", 0.0))
	var scale_now := ArmyMarkers.scale_for_distance(camera_distance) * maxf(normal_weight, 0.02)
	var hidden: Variant = _armies.get("hidden_provinces") if _armies != null else null
	for key in _fields:
		var field: Dictionary = _fields[key]
		var root := field["node"] as Node3D
		if root == null:
			continue
		root.visible = fields_on and not (hidden is Dictionary and (hidden as Dictionary).has(field["province"]))
		if not root.visible:
			continue
		# Mise en place par champ : à la première vue, quand l'échelle des armées change de plus
		# de 2 %, ou après un recalage du sol (aussi pour un champ resté caché entre-temps).
		var laid := float(field.get("scale", -1.0))
		if laid < 0.0 or absf(scale_now - laid) > laid * 0.02:
			_layout_field(field, scale_now)
			field["scale"] = scale_now
		_fly_crows(field, float(field["scale"]))
	var siege_on := camera_distance <= float(_block("siege").get("max_distance", 0.0))
	for id in _engines:
		var node: Variant = _engines[id]["node"]
		if node != null and is_instance_valid(node):
			(node as Node3D).visible = siege_on


# --- Lecture (tests, captures) ---------------------------------------------------------


func plague_node(settlement: String) -> Node3D:
	return (_plague.get(settlement, {}) as Dictionary).get("node") as Node3D


## Nombre d'éléments d'une colonie pestiférée dont le nom commence par `prefix` (`Pit_`, `Door_`,
## `DeadCart`).
func plague_count(settlement: String, prefix: String) -> int:
	var node := plague_node(settlement)
	if node == null:
		return 0
	var count := 0
	for child in node.get_children():
		if str(child.name).begins_with(prefix):
			count += 1
	return count


func battlefield_keys() -> Array:
	return _fields.keys()


func battlefield_node(key: String) -> Node3D:
	return (_fields.get(key, {}) as Dictionary).get("node") as Node3D


func battlefield_at(key: String) -> Vector2:
	return (_fields.get(key, {}) as Dictionary).get("at", Vector2(-1.0, -1.0))


## Noms des engins montrés pour l'armée `army_id` (`Engine_<kind>_<stage>`).
func engine_names(army_id: String) -> PackedStringArray:
	var names := PackedStringArray()
	var node: Variant = (_engines.get(army_id, {}) as Dictionary).get("node")
	if node != null and is_instance_valid(node):
		for child in (node as Node).get_children():
			names.append(str(child.name))
	return names
