class_name FolkScenes
extends RefCounted

## Chantier FK, lot FK4 (`docs/design/2026-09-29-carte-vivante-folk.md` § 3.3) : mise en scène
## des scènes de province (`get_map_scenes` du pont : province, colonie, type, tour de début,
## intensité) sur la vue rapprochée. Rendu seulement : aucune règle de jeu.
##
## - Fournisseur du `FolkPool` (enregistré avant les marchands : servi d'abord au plafond) :
##   `refresh(sim)` une fois par tour (résolution des lieux), `populate(pool, focus, radius)` au
##   placement.
## - Lieu : au bord de la maquette de la colonie touchée, d'un côté tiré par colonie et par type ;
##   colonie absente ou inconnue → chef-lieu (colonie la plus importante de la province) ; sans
##   colonie dans la province → scène ignorée, avertissement unique (spec § 6).
## - Intensité → figurants entre `scene_figures_min` et `scene_figures_max`
##   (`data/rules/map_scenes.json`).
## - Effets hors réservoir, relus par `CampaignLife` après `refresh` : cheminées coupées dans les
##   colonies pestiférées (`quiet_settlements` → `LifeEffects`), fumées de bûcher et d'émeute
##   (`fire_points` → `LifeEffects`), champs sans travailleurs en disette (`idle_provinces` →
##   `FolkRoutine`).
## - SC MC6 : la mise en scène (10 types → 4 archétypes paramétrés) est dans `FolkSceneLayers`
##   (`data/rules/folk_scene_layouts.json`) ; ici : lieux, emprises, trajets et effets.
## - `forced` : scènes forcées (`--scene=<province>:<kind>`, tests, captures), intensité 1.
## Aucune scène n'est signalée (pas d'icône) : on les découvre en zoomant (spec § 3.3).

const FIGURES_MIN := 6
const FIGURES_MAX := 24
## Rayon (unités monde) d'une maquette de colonie sans `SettlementLayer` (tests).
const DEFAULT_MODEL_RADIUS := 0.5
## Marge (unités monde, ≈ 60 m réels) entre le bord de la colonie et une scène de fumée.
const SIDE_MARGIN := 0.08
## Marge (m) entre le bord de la maquette et le cœur de la scène.
const EDGE_M := 6.0

## Scènes forcées : [{province, kind, settlement?, intensity?}].
var forced: Array = []
## Scènes lues au dernier `refresh` (pont), avant résolution.
var live: Array = []
## Scènes résolues : {kind, province, settlement, index, center, seed, intensity}.
var staged: Array = []
## id de colonie → vrai : cheminées éteintes (peste).
var quiet_settlements: Dictionary = {}
## id de province → vrai : champs sans travailleurs (disette).
var idle_provinces: Dictionary = {}
## Fumées de bûcher (peste) et d'émeute (révolte), en unités monde.
var fire_points: PackedVector2Array = PackedVector2Array()
var stats: Dictionary = {}

var _map_data: MapData = null
var _data: SettlementData = null
var _layer: SettlementLayer = null
## province → indice de sa colonie la plus importante (les colonies sont triées par importance).
var _seat: Dictionary = {}
var _pool: FolkPool = null
var _layers := FolkSceneLayers.new(self)
var _warned: Dictionary = {}
## FK6 : cercles (x, z, rayon) des villes emblématiques (maquette L1, ville 1:1), relus au
## premier `refresh` ; une scène se tient hors de ces emprises.
var disks: PackedVector3Array = PackedVector3Array()
## Emprise (rayon) par indice de colonie, calculée une fois par tour (`resolve`).
var _footprints: Dictionary = {}
## Points de fleuve proches par indice de colonie (crue), calculés une fois par tour.
var _river_cache: Dictionary = {}
## LR-18 : trajets de fuite par (colonie, groupe), calculés une fois par tour.
var _escape_cache: Dictionary = {}
## LR-18 : index des points de route par case (Vector2i → [route, rang, …] à plat), bâti au `setup`.
var _road_cells: Dictionary = {}
const ROAD_CELL := 2.0


func setup(map_data: MapData, settlement_data: SettlementData, layer: SettlementLayer = null) -> void:
	_map_data = map_data
	_data = settlement_data
	_layer = layer
	_seat.clear()
	_road_cells.clear()
	if _data == null:
		return
	for r in _data.roads.size():
		var road_points: PackedVector2Array = _data.roads[r]["points"]
		for i in road_points.size():
			var key := Vector2i(int(floorf(road_points[i].x / ROAD_CELL)), int(floorf(road_points[i].y / ROAD_CELL)))
			var list: PackedInt32Array = _road_cells.get(key, PackedInt32Array())
			list.append(r)
			list.append(i)
			_road_cells[key] = list
	for i in _data.settlements.size():
		var province := str(_data.settlements[i]["province"])
		if not _seat.has(province):
			_seat[province] = i


## Types de scène (SC MC6 : `data/rules/folk_scene_layouts.json`).
static func kinds() -> Array:
	return FolkSceneLayers.kinds()


## `--scene=<province>:<kind>[,…]` → entrées de `forced` (valeurs invalides ignorées).
static func parse_forced(spec: String) -> Array:
	var out: Array = []
	for pair in spec.split(",", false):
		var parts := pair.split(":")
		if parts.size() == 2 and kinds().has(parts[1]):
			out.append({"province": parts[0], "kind": parts[1], "settlement": "", "intensity": 1.0})
	return out


## Une fois par tour : scènes du pont (conservées si `sim` est nul) et forcées, lieux résolus.
func refresh(sim: Object) -> void:
	if disks.is_empty() and _layer != null:
		disks = FolkPool.landmark_disks(_layer)
	if sim != null and sim.has_method("get_map_scenes"):
		live = Array(sim.call("get_map_scenes"))
	resolve(live + forced)


## Résout les lieux (colonie, repli chef-lieu), fusionne les doublons (même lieu, même type :
## intensité la plus forte) et calcule les effets hors réservoir.
func resolve(scenes: Array) -> void:
	var by_key: Dictionary = {}
	var skipped := 0
	for scene in scenes:
		if not (scene is Dictionary):
			continue
		var kind := str(scene.get("kind", ""))
		var province := str(scene.get("province", ""))
		if not kinds().has(kind):
			_warn("kind:" + kind, "FolkScenes: unknown scene kind %s" % kind)
			skipped += 1
			continue
		var index := _site_of(province, str(scene.get("settlement", "")))
		if index < 0:
			_warn("site:" + province, "FolkScenes: no settlement in province %s, %s scene skipped" % [province, kind])
			skipped += 1
			continue
		var entry: Dictionary = _data.settlements[index]
		var id := str(entry["id"])
		var key := "%s|%s" % [id, kind]
		var intensity := clampf(float(scene.get("intensity", 1.0)), 0.0, 1.0)
		if by_key.has(key) and float(by_key[key]["intensity"]) >= intensity:
			continue
		by_key[key] = {
			"kind": kind, "province": province, "settlement": id, "index": index,
			"center": entry["px"], "seed": absi(id.hash()) + kinds().find(kind) * 7919,
			"intensity": intensity, "since_turn": int(scene.get("since_turn", 0)),
		}
	staged = by_key.values()
	_footprints.clear()
	_river_cache.clear()
	_escape_cache.clear()
	quiet_settlements.clear()
	idle_provinces.clear()
	fire_points = PackedVector2Array()
	for scene in staged:
		var effects := FolkSceneLayers.effects_of(str(scene["kind"]))
		if effects.get("quiet_settlement", false):
			quiet_settlements[scene["settlement"]] = true
		if effects.has("fire_side_factor"):
			fire_points.append(_side_point(scene, float(effects["fire_side_factor"])))
		if effects.get("idle_province", false):
			idle_provinces[scene["province"]] = true
		if effects.has("river_cache"):
			# Parcours des fleuves une fois par tour, pas au placement (≈ 25 ms).
			var index := int(scene["index"])
			if not _river_cache.has(index):
				_river_cache[index] = _near_river_points(scene["center"], float(effects["river_cache"]) + _footprint(index))
	stats = {"staged": staged.size(), "skipped": skipped}


## Colonie de la scène : celle nommée si elle existe, sinon le chef-lieu ; -1 sinon.
func _site_of(province: String, settlement: String) -> int:
	if _data == null:
		return -1
	if settlement != "" and _data.index_by_id.has(settlement):
		return int(_data.index_by_id[settlement])
	return int(_seat.get(province, -1))


func _warn(key: String, message: String) -> void:
	if _pool != null:
		_pool.warn_once("scenes:" + key, message)
	elif not _warned.has(key):
		_warned[key] = true
		push_warning(message)


## Rayon réel (unités monde) de la colonie (VT : villes 1:1, plus de maquette grossie).
func _model_radius(index: int) -> float:
	if _layer == null:
		return DEFAULT_MODEL_RADIUS
	return _layer.model_radius(index)


## FK6 : rayon (unités monde) de l'emprise de la colonie `index` autour de son centre : maquette
## à l'échelle courante, élargie à toute ville emblématique (L1 ou 1:1) qui couvre le centre (le
## rayon de la maquette générique d'une ville 1:1 ne dit rien de son enceinte). Mis en cache par
## tour (les emprises des villes emblématiques ne dépendent pas de l'échelle).
func _footprint(index: int) -> float:
	var landmark: float = _footprints.get(index, -1.0)
	if landmark < 0.0:
		landmark = 0.0
		var center: Vector2 = _data.settlements[index]["px"] if _data != null and index >= 0 and index < _data.settlements.size() else Vector2.INF
		for disk in disks:
			var d := center.distance_to(Vector2(disk.x, disk.y))
			if d < disk.z:
				landmark = maxf(landmark, d + disk.z)
		_footprints[index] = landmark
	return maxf(_model_radius(index), landmark)


## Direction (carte) du côté de la colonie où se tient la scène (stable par colonie et type).
static func _side(scene: Dictionary) -> Vector2:
	var angle := Hash.h01(int(scene["seed"]), 1) * TAU
	return Vector2(cos(angle), sin(angle))


## Point au bord de la colonie (`factor` × rayon + `SIDE_MARGIN`), du côté de la scène (fumées).
func _side_point(scene: Dictionary, factor: float) -> Vector2:
	return (scene["center"] as Vector2) + _side(scene) * (_footprint(int(scene["index"])) * factor + SIDE_MARGIN)


## TB4 : bord de la colonie d'une scène résolue (`staged`), pour les éléments posés hors
## réservoir (`WarScars` : fosses de la peste) : centre et emprise en unités monde, `out` unitaire
## vers l'extérieur, du côté où se tient la scène.
func edge_frame(scene: Dictionary) -> Dictionary:
	return {
		"settlement": scene["settlement"], "center": scene["center"], "out": _side(scene),
		"footprint": _footprint(int(scene["index"])), "intensity": scene["intensity"], "seed": scene["seed"],
	}


func _figures_for(intensity: float) -> int:
	var lo := int(_pool.settings.get("scene_figures_min", FIGURES_MIN))
	var hi := int(_pool.settings.get("scene_figures_max", FIGURES_MAX))
	return maxi(roundi(lerpf(float(lo), float(hi), clampf(intensity, 0.0, 1.0))), 1)


func populate(pool: FolkPool, focus: Vector2, radius: float) -> void:
	_pool = pool
	var by_kind: Dictionary = {}
	var shown := 0
	var order: Array = []
	for scene in staged:
		var d := (scene["center"] as Vector2).distance_to(focus)
		if d <= radius + _footprint(int(scene["index"])):
			order.append([d, scene])
	order.sort_custom(func(x: Array, y: Array) -> bool: return x[0] < y[0])
	for pair in order:
		if pool.remaining() <= 0:
			break
		var scene: Dictionary = pair[1]
		var before := pool.figure_count()
		var props_before := pool.prop_count()
		_stage(scene, _figures_for(float(scene["intensity"])))
		var placed := pool.figure_count() - before
		var kind := str(scene["kind"])
		var entry: Dictionary = by_kind.get(kind, {"figures": 0, "props": 0})
		entry["figures"] = int(entry["figures"]) + placed
		entry["props"] = int(entry["props"]) + pool.prop_count() - props_before
		by_kind[kind] = entry
		shown += 1
	stats["shown"] = shown
	stats["by_kind"] = by_kind



# --- Mise en scène -------------------------------------------------------------------
# Repère d'une scène : `anchor` au bord de la maquette, `out` vers l'extérieur (loin de la
# colonie), `along` le long du bord (à droite de `out`, convention de `FolkPool.add`) ; les
# distances sont en mètres, converties par `_m`.


func _stage(scene: Dictionary, figures: int) -> void:
	var out := _side(scene)
	var frame := {
		"anchor": (scene["center"] as Vector2) + out * (_footprint(int(scene["index"])) + _pool.current_scale() * EDGE_M),
		"out": out,
		"along": Vector2(-out.y, out.x),
		"seed": int(scene["seed"]),
		"center": scene["center"],
		"index": scene["index"],
	}
	_layers.stage(_pool, frame, str(scene["kind"]), figures)


## Mètres → unités monde (échelle des figurines du placement en cours).
func _m(meters: float) -> float:
	return meters * _pool.current_scale()


## Emprise de la colonie `index` (SC MC6 : accès pour `FolkSceneLayers`).
func footprint(index: int) -> float:
	return _footprint(index)


## Trajet de fuite du groupe `group` de la colonie `index`, calculé une fois par tour (LR-18).
func escape_path_for(index: int, group: int, from: Vector2, dir: Vector2, salt: int) -> PackedVector2Array:
	var cache_key := index * 64 + group
	if not _escape_cache.has(cache_key):
		_escape_cache[cache_key] = _escape_path(index, from, dir, salt)
	return _escape_cache[cache_key]


## Trajet de fuite (3 tronçons chaînés, 4 points) depuis `from` : la route la plus proche (suivie
## en s'éloignant de la colonie, sur ≈ 70 m), sinon un arc dont la courbure fuit le fleuve voisin.
func _escape_path(index: int, from: Vector2, dir: Vector2, salt: int) -> PackedVector2Array:
	var road := _road_from(from)
	if road.size() >= 2:
		return _resample(road, 3)
	var to := from + dir * _m(70.0)
	var side := Vector2(-dir.y, dir.x)
	var mid := (from + to) * 0.5
	var bend := _m(18.0)
	var pos := mid + side * bend
	var neg := mid - side * bend
	var sign_value := 1.0 if Hash.h01(salt, 1) < 0.5 else -1.0
	var rivers: Array = _river_cache.get(index, [])
	if not rivers.is_empty():
		var near_pos := INF
		var near_neg := INF
		for entry in rivers:
			var p: Vector2 = entry[1]
			near_pos = minf(near_pos, p.distance_squared_to(pos))
			near_neg = minf(near_neg, p.distance_squared_to(neg))
		sign_value = 1.0 if near_pos >= near_neg else -1.0
	var control := mid + side * bend * 2.0 * sign_value
	var out := PackedVector2Array()
	for i in 4:
		var t := float(i) / 3.0
		out.append(from.lerp(control, t).lerp(control.lerp(to, t), t))
	return out


## Route la plus proche de `from` (à moins de 60 m) : points suivis dans le sens qui s'éloigne de
## `from`, sur ≈ 70 m. Vide si aucune route.
func _road_from(from: Vector2) -> PackedVector2Array:
	if _data == null:
		return PackedVector2Array()
	var best := _m(60.0) * _m(60.0)
	var best_points := PackedVector2Array()
	var best_i := -1
	var cx := int(floorf(from.x / ROAD_CELL))
	var cy := int(floorf(from.y / ROAD_CELL))
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var list: PackedInt32Array = _road_cells.get(Vector2i(cx + dx, cy + dy), PackedInt32Array())
			for k in range(0, list.size(), 2):
				var points: PackedVector2Array = _data.roads[list[k]]["points"]
				var d2 := points[list[k + 1]].distance_squared_to(from)
				if d2 < best:
					best = d2
					best_points = points
					best_i = list[k + 1]
	if best_i < 0 or best_points.size() < 2:
		return PackedVector2Array()
	var step := 1
	if best_i + 1 >= best_points.size() or (best_i > 0 and best_points[best_i - 1].distance_squared_to(from) > best_points[best_i + 1].distance_squared_to(from)):
		step = -1
	var out := PackedVector2Array([from])
	var length := 0.0
	var i := best_i
	while i >= 0 and i < best_points.size() and length < _m(70.0):
		length += out[out.size() - 1].distance_to(best_points[i])
		out.append(best_points[i])
		i += step
	return out


## `pieces` tronçons de même longueur le long de la ligne brisée (pieces + 1 points).
static func _resample(line: PackedVector2Array, pieces: int) -> PackedVector2Array:
	var total := 0.0
	for i in line.size() - 1:
		total += line[i].distance_to(line[i + 1])
	var out := PackedVector2Array([line[0]])
	var target := total / float(pieces)
	var walked := 0.0
	var next := target
	for i in line.size() - 1:
		var seg := line[i].distance_to(line[i + 1])
		while seg > 0.0 and next <= walked + seg + 1e-6 and out.size() < pieces:
			out.append(line[i].lerp(line[i + 1], (next - walked) / seg))
			next += target
		walked += seg
	out.append(line[line.size() - 1])
	while out.size() < pieces + 1:
		out.append(line[line.size() - 1])
	return out


## Points des fleuves à moins de `range_world` de `center`, espacés d'au moins `spacing`, les
## plus proches d'abord, au plus `count`, hors des villes emblématiques. Les points proches de la
## colonie `index` sont mis en cache par tour (parcours des fleuves une fois, pas à chaque
## placement).
func river_points(index: int, center: Vector2, range_world: float, count: int, spacing: float) -> PackedVector2Array:
	if _map_data == null:
		return PackedVector2Array()
	if not _river_cache.has(index):
		_river_cache[index] = _near_river_points(center, range_world)
	var out := PackedVector2Array()
	for entry in _river_cache[index]:
		var p: Vector2 = entry[1]
		if _pool != null and _pool.excluded(p):
			continue
		var ok := true
		for q in out:
			if q.distance_to(p) < spacing:
				ok = false
				break
		if ok:
			out.append(p)
			if out.size() >= count:
				break
	return out


## Points (et milieux de tronçons) des fleuves à moins de `range_world` de `center`, triés par
## distance : [[d², point], …].
func _near_river_points(center: Vector2, range_world: float) -> Array:
	var found: Array = []
	var r2 := range_world * range_world
	for river in _map_data.rivers:
		var line: Variant = river.get("points")
		if not (line is PackedVector2Array):
			continue
		var pts: PackedVector2Array = line
		for i in pts.size():
			var d2 := pts[i].distance_squared_to(center)
			if d2 <= r2:
				found.append([d2, pts[i]])
			# Points intermédiaires sur les longs tronçons.
			if i + 1 < pts.size():
				var mid := (pts[i] + pts[i + 1]) * 0.5
				var dm := mid.distance_squared_to(center)
				if dm <= r2:
					found.append([dm, mid])
	found.sort_custom(func(x: Array, y: Array) -> bool: return x[0] < y[0])
	return found
