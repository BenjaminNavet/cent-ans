class_name FolkRoutine
extends RefCounted

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 3.1) : vie ordinaire de
## la vue rapprochée. Rendu seulement : aucune règle de jeu ; tout vient du pont (population,
## dévastation, saison) via `CampaignLife`, qui renseigne `province_states`, `season`, `cities`
## et `terroir` avant `refresh`.
##
## - Routes (tracés de `SettlementData.roads`, ceux des rubans de `RoadRenderer`) : piétons,
##   cavaliers isolés et charrettes de paysans ; densité proportionnelle à la population de la
##   province traversée, réduite par la dévastation.
## - Champs (canal R du masque de terroirs) : labour au printemps, fauche en été, vendange en
##   automne dans les vignes (canal G), presque personne l'hiver ; rien sur le brûlis (canal B).
## - Pâtures (canal A) : troupeaux (moutons ou vaches) et un berger.
## - Forêts (canal B de `splat`, couverture forestière) en hiver : bûcherons.
## - Pèlerins : petits groupes marchant vers les cités (niveau 3 de `SettlementGrowth`).
## Tirages déterministes par lieu (`VegetationFields.hash01`) : un nouveau placement retrouve les
## mêmes gens au même endroit.

## Taille des cases de l'index spatial des tronçons de route (unités monde).
const CELL := 32.0
## Longueur maximale d'un trajet rectiligne (les tronçons plus longs sont coupés).
const PIECE := 4.0
## Pas des grilles de semis (champs, pâtures, forêts).
const FIELD_STEP := 3.5
const PASTURE_STEP := 5.0
const FOREST_STEP := 5.0
## Longueur (unités monde) d'un sillon ou d'un trajet de porteur aux champs.
const FURROW := 2.0
## Densités par défaut (surchargées par `map_scenes.json`, voir `FolkPool.settings`).
const ROAD_FOLK_PER_UNIT := 0.12
const FIELD_WORK_PROBABILITY := 0.35
const HERD_PROBABILITY := 0.12
const WOODCUTTER_PROBABILITY := 0.06
const PILGRIM_PROBABILITY := 0.1
const PILGRIM_RANGE := 20.0
## Parts du plafond restant : routes, champs, pâtures, forêts et pèlerins.
const SHARES := {"roads": 0.45, "fields": 0.3, "pastures": 0.15, "other": 0.1}
const FOREST_COVER := 0.45

## province_id → {devastation, population} (`CampaignLife.province_states`).
var province_states: Dictionary = {}
## "spring" | "summer" | "autumn" | "winter".
var season: String = "summer"
## Positions (carte) des cités, destinations des pèlerins.
var cities: PackedVector2Array = PackedVector2Array()
## Masque des terroirs (`CampaignLife.terroir`) ; sans masque, champs et pâtures sont sautés.
var terroir: TerroirMask = null
var off: Dictionary = {}
var stats: Dictionary = {}

var _map_data: MapData = null
var _seg_a := PackedVector2Array()
var _seg_b := PackedVector2Array()
var _seg_main := PackedByteArray()
## Case → PackedInt32Array des tronçons (milieu dans la case).
var _cells: Dictionary = {}
var _pool: FolkPool = null
## Facteur de densité par province (index raster), vidé à chaque placement.
var _factor_cache: Dictionary = {}


func setup(map_data: MapData, settlement_data: SettlementData) -> void:
	_map_data = map_data
	var t0 := Time.get_ticks_msec()
	var buckets: Dictionary = {}
	if settlement_data != null:
		for road in settlement_data.roads:
			var points: PackedVector2Array = road["points"]
			var main := 1 if bool(road["main"]) else 0
			for i in points.size() - 1:
				var index := _seg_a.size()
				_seg_a.append(points[i])
				_seg_b.append(points[i + 1])
				_seg_main.append(main)
				var key := _cell_key((points[i] + points[i + 1]) * 0.5)
				if not buckets.has(key):
					buckets[key] = PackedInt32Array()
				var list: PackedInt32Array = buckets[key]
				list.append(index)
				buckets[key] = list
	_cells = buckets
	stats = {"road_segments": _seg_a.size(), "index_ms": Time.get_ticks_msec() - t0}


static func _cell_key(p: Vector2) -> Vector2i:
	return Vector2i(int(floorf(p.x / CELL)), int(floorf(p.y / CELL)))


func refresh(_sim: Object) -> void:
	pass


func populate(pool: FolkPool, focus: Vector2, radius: float) -> void:
	_pool = pool
	_factor_cache.clear()
	var budget := pool.remaining()
	if budget <= 0:
		return
	var counts := {}
	var ms := {}
	var t := Time.get_ticks_usec()
	counts["roads"] = 0 if off.has("roads") else _roads(focus, radius, int(budget * SHARES["roads"]))
	ms["roads"] = Time.get_ticks_usec() - t
	t = Time.get_ticks_usec()
	counts["fields"] = _fields(focus, radius, int(budget * SHARES["fields"]))
	ms["fields"] = Time.get_ticks_usec() - t
	t = Time.get_ticks_usec()
	counts["pastures"] = _pastures(focus, radius, int(budget * SHARES["pastures"]))
	ms["pastures"] = Time.get_ticks_usec() - t
	t = Time.get_ticks_usec()
	var other := int(budget * SHARES["other"])
	counts["pilgrims"] = _pilgrims(focus, radius, other / 2)
	counts["woodcutters"] = _woodcutters(focus, radius, other - int(counts["pilgrims"]))
	ms["other"] = Time.get_ticks_usec() - t
	for key in ms:
		ms[key] = snappedf(float(ms[key]) / 1000.0, 0.01)
	counts["ms"] = ms
	stats.merge(counts, true)


func _setting(key: String, fallback: float) -> float:
	return float(_pool.settings.get(key, fallback)) if _pool != null else fallback


static func _h(a: int, b: int, c: int = 0) -> float:
	return VegetationFields.hash01(a * 73856093 ^ b * 19349663 ^ c * 83492791)


## Tronçons de route dont le milieu est dans le disque, plus proches d'abord.
func _segments_near(focus: Vector2, radius: float) -> Array:
	var found: Array = []
	var lo := _cell_key(focus - Vector2(radius, radius))
	var hi := _cell_key(focus + Vector2(radius, radius))
	var r2 := radius * radius
	for cy in range(lo.y, hi.y + 1):
		for cx in range(lo.x, hi.x + 1):
			var list: Variant = _cells.get(Vector2i(cx, cy))
			if list == null:
				continue
			for index in list:
				var mid := (_seg_a[index] + _seg_b[index]) * 0.5
				var d2 := mid.distance_squared_to(focus)
				if d2 <= r2:
					found.append([d2, index])
	found.sort_custom(func(x: Array, y: Array) -> bool: return x[0] < y[0])
	return found


func _province_state(p: Vector2) -> Dictionary:
	if _map_data == null:
		return {}
	var province := _map_data.get_province(_map_data.province_index_at(p.x, p.y))
	return province_states.get(str(province.get("id", "")), {})


## Facteur de densité d'une province : population (relative à la référence du masque des
## terroirs), réduit par la dévastation.
func _people_factor(p: Vector2) -> float:
	var index := _map_data.province_index_at(p.x, p.y) if _map_data != null else 0
	if _factor_cache.has(index):
		return _factor_cache[index]
	var state := _province_state(p)
	var population := float(state.get("population", TerroirMask.REFERENCE_POPULATION))
	var devastation := float(state.get("devastation", 0.0))
	var factor := clampf(population / TerroirMask.REFERENCE_POPULATION, 0.2, 2.5) * clampf(1.0 - devastation / 100.0, 0.0, 1.0)
	_factor_cache[index] = factor
	return factor


# --- Routes -------------------------------------------------------------------------


func _roads(focus: Vector2, radius: float, budget: int) -> int:
	if _seg_a.is_empty():
		_pool.warn_once("routine_roads", "FolkRoutine: no road segments, road folk skipped")
		return 0
	var per_unit := _setting("road_folk_per_unit", ROAD_FOLK_PER_UNIT)
	var placed := 0
	for entry in _segments_near(focus, radius):
		if placed >= budget:
			break
		var index: int = entry[1]
		var a := _seg_a[index]
		var b := _seg_b[index]
		var length := a.distance_to(b)
		if length < 0.2:
			continue
		var density := per_unit * _people_factor((a + b) * 0.5) * (1.5 if _seg_main[index] == 1 else 1.0)
		var pieces := ceili(length / PIECE)
		for piece in pieces:
			var pa := a.lerp(b, float(piece) / pieces)
			var pb := a.lerp(b, float(piece + 1) / pieces)
			var expected := pa.distance_to(pb) * density
			var n := int(expected) + (1 if _h(index, piece, 1) < fposmod(expected, 1.0) else 0)
			for k in n:
				if placed >= budget:
					break
				placed += _road_traveller(index * 64 + piece, k, pa, pb)
	return placed


## Un voyageur (ou un petit groupe) sur un trajet ; renvoie le nombre de figurines posées.
func _road_traveller(seed_id: int, k: int, a: Vector2, b: Vector2) -> int:
	var forward := _h(seed_id, k, 2) < 0.5
	var from := a if forward else b
	var to := b if forward else a
	var phase := _h(seed_id, k, 3)
	# Chacun tient sa droite.
	var side := 0.5 + 0.3 * _h(seed_id, k, 4)
	var kind := _h(seed_id, k, 5)
	if kind < 0.15:
		return 1 if _pool.add("rider", "ride", from, to, phase, side) else 0
	if kind < 0.27:
		var count := 0
		if _pool.add("peasant_cart", "roll", from, to, phase, side):
			# Charretier à sa place (manifeste FK2 : `carter`).
			var carter := FolkModels.slot("peasant_cart", "carter", Vector2(1.1, 0.8))
			count += 1 if _pool.add("peasant", "walk", from, to, phase, side + carter.x, -carter.y) else 0
		return count
	var pick := _h(seed_id, k, 6)
	# Porteurs (sac à l'épaule : leur marche est `carry`) parmi les piétons.
	var role := "porter" if pick < 0.25 else ("peasant" if pick < 0.7 else "peasant_b")
	return 1 if _pool.add(role, "walk", from, to, phase, side) else 0


# --- Champs, pâtures, forêts ----------------------------------------------------------


## Points d'une grille ancrée sur la carte (stables d'un placement à l'autre) retenus par
## `pick(p, i, j)` (résultat non nul), plus proches d'abord : [distance², position jittée, i, j,
## résultat]. Le tri ne porte que sur les points retenus.
static func _grid(focus: Vector2, radius: float, step: float, salt: int, pick: Callable) -> Array:
	var points: Array = []
	var i0 := int(floorf((focus.x - radius) / step))
	var i1 := int(ceilf((focus.x + radius) / step))
	var j0 := int(floorf((focus.y - radius) / step))
	var j1 := int(ceilf((focus.y + radius) / step))
	var r2 := radius * radius
	for j in range(j0, j1 + 1):
		for i in range(i0, i1 + 1):
			var p := Vector2((i + _h(i, j, salt)) * step, (j + _h(i, j, salt + 1)) * step)
			var d2 := p.distance_squared_to(focus)
			if d2 > r2:
				continue
			var picked: Variant = pick.call(p, i, j)
			if picked != null:
				points.append([d2, p, i, j, picked])
	points.sort_custom(func(x: Array, y: Array) -> bool: return x[0] < y[0])
	return points


func _fields(focus: Vector2, radius: float, budget: int) -> int:
	if off.has("fields"):
		return 0
	if terroir == null or terroir.image == null:
		if _pool != null:
			_pool.warn_once("routine_terroir", "FolkRoutine: no terroir mask, field work skipped")
		return 0
	var probability := _setting("field_work_probability", FIELD_WORK_PROBABILITY)
	var placed := 0
	for entry in _grid(focus, radius, FIELD_STEP, 11, _field_plan.bind(probability)):
		if placed >= budget:
			break
		var p: Vector2 = entry[1]
		var i: int = entry[2]
		var j: int = entry[3]
		var plan: Array = entry[4]
		var activity: String = plan[0]
		var group: int = plan[1]
		var yaw := _h(i, j, 22) * TAU
		var dir := Vector2(sin(yaw), cos(yaw))
		if activity == "plough":
			placed += _plough_team(p, dir, _h(i, j, 23))
			continue
		for k in group:
			if placed >= budget:
				break
			# Les membres d'un groupe à 2-3 m les uns des autres (mètres → monde).
			var offset := Vector2(cos(yaw + k * 2.1), sin(yaw + k * 2.1)) * (2.5 * k) * _pool.current_scale()
			if activity == "harvest":
				# Porteurs (vendange, gerbes) : allers et venues courtes.
				if _pool.add("porter", "harvest", p + offset, p + offset + dir.rotated(k * 0.7) * FURROW, _h(i, j, 30 + k)):
					placed += 1
				continue
			var role := "reaper" if activity == "scythe" else ("peasant" if (i + j + k) % 3 != 0 else "peasant_b")
			if _pool.add_static(role, activity, p + offset, yaw + (_h(i, j, 30 + k) - 0.5) * 0.8):
				placed += 1
	return placed


## Attelage de labour (FK2 `plough` : charrue et bœufs, origine aux pieds du laboureur) qui
## remonte un sillon ; laboureur à `ploughman`, bouvier à `drover`. Renvoie les figurines posées.
func _plough_team(p: Vector2, dir: Vector2, phase: float) -> int:
	var to := p + dir * FURROW
	if not _pool.add("plough", "", p, to, phase):
		return 0
	var placed := 0
	var ploughman := FolkModels.slot("plough", "ploughman", Vector2.ZERO)
	var drover := FolkModels.slot("plough", "drover", Vector2(1.0, 5.4))
	placed += 1 if _pool.add("peasant", "plough", p, to, phase, ploughman.x, -ploughman.y) else 0
	placed += 1 if _pool.add("peasant_b", "plough", p, to, phase, drover.x, -drover.y) else 0
	return placed


## Travail d'un point de champ selon la saison : [activité, taille du groupe], ou null.
func _field_plan(p: Vector2, i: int, j: int, probability: float) -> Variant:
	var mask := terroir.sample(p)
	var fields := mask.r * (1.0 - mask.b)
	if fields < 0.25:
		return null
	var activity := "idle"
	var chance := fields * probability * 0.12
	var group := 1
	match season:
		"spring":
			activity = "plough"
			chance = fields * probability
		"summer":
			activity = "scythe"
			chance = fields * probability
			group = 2 + int(_h(i, j, 21) * 2.0)
		"autumn":
			activity = "harvest"
			if mask.g > 0.25:
				chance = mask.g * probability * 1.4
				group = 2 + int(_h(i, j, 21) * 3.0)
			else:
				chance = fields * probability * 0.4
	var roll := _h(i, j, 20)
	if roll >= chance * 1.5:
		return null  # rejet avant la lecture (coûteuse) de la province
	if roll >= chance * clampf(_people_factor(p), 0.3, 1.5):
		return null
	return [activity, group]


func _pastures(focus: Vector2, radius: float, budget: int) -> int:
	if off.has("pastures") or terroir == null or terroir.image == null:
		return 0
	var probability := _setting("herd_probability", HERD_PROBABILITY) * (0.4 if season == "winter" else 1.0)
	var placed := 0
	var pick := func(p: Vector2, i: int, j: int) -> Variant:
		var mask := terroir.sample(p)
		var pasture := mask.a * (1.0 - mask.b)
		return null if pasture < 0.3 or _h(i, j, 42) >= pasture * probability else true
	for entry in _grid(focus, radius, PASTURE_STEP, 41, pick):
		if placed >= budget:
			break
		var p: Vector2 = entry[1]
		var i: int = entry[2]
		var j: int = entry[3]
		var cows := _h(i, j, 43) < 0.35
		var beasts := (2 + int(_h(i, j, 44) * 3.0)) if cows else (4 + int(_h(i, j, 44) * 4.0))
		var spread := (4.0 if cows else 3.0) * _pool.current_scale()
		for k in beasts:
			var angle := _h(i, j, 50 + k) * TAU
			var at := p + Vector2(cos(angle), sin(angle)) * spread * sqrt(_h(i, j, 60 + k)) * 2.0
			_pool.add_static("cow" if cows else "sheep", "", at, _h(i, j, 70 + k) * TAU)
		if _pool.add_static("peasant_b", "herd", p + Vector2(spread * 3.0, 0.0), _h(i, j, 45) * TAU):
			placed += 1
	return placed


func _woodcutters(focus: Vector2, radius: float, budget: int) -> int:
	if off.has("forest") or season != "winter" or budget <= 0:
		return 0
	var splat := _map_data.splat_image if _map_data != null else null
	if splat == null:
		if _pool != null:
			_pool.warn_once("routine_forest", "FolkRoutine: no forest cover (splat), woodcutters skipped")
		return 0
	var sx := float(splat.get_width()) / maxf(float(_map_data.size.x), 1.0)
	var sy := float(splat.get_height()) / maxf(float(_map_data.size.y), 1.0)
	var probability := _setting("woodcutter_probability", WOODCUTTER_PROBABILITY)
	var placed := 0
	var pick := func(p: Vector2, i: int, j: int) -> Variant:
		if _h(i, j, 82) >= probability * 2.5:
			return null
		var x := clampi(int(p.x * sx), 0, splat.get_width() - 1)
		var y := clampi(int(p.y * sy), 0, splat.get_height() - 1)
		if splat.get_pixel(x, y).b < FOREST_COVER or _h(i, j, 82) >= probability * _people_factor(p):
			return null
		return true
	for entry in _grid(focus, radius, FOREST_STEP, 81, pick):
		if placed >= budget:
			break
		var p: Vector2 = entry[1]
		var i: int = entry[2]
		var j: int = entry[3]
		for k in 2:
			if placed < budget and _pool.add_static("peasant", "chop", p + Vector2(k * 1.2, k * 0.6) * _pool.current_scale(), _h(i, j, 83 + k) * TAU):
				placed += 1
	return placed


func _pilgrims(focus: Vector2, radius: float, budget: int) -> int:
	if off.has("pilgrims") or cities.is_empty() or budget <= 0:
		return 0
	var near_cities := PackedVector2Array()
	for city in cities:
		if city.distance_to(focus) <= radius + PILGRIM_RANGE:
			near_cities.append(city)
	if near_cities.is_empty():
		return 0
	var probability := _setting("pilgrim_probability", PILGRIM_PROBABILITY)
	var placed := 0
	for entry in _segments_near(focus, radius):
		if placed >= budget:
			break
		var index: int = entry[1]
		var a := _seg_a[index]
		var b := _seg_b[index]
		var target := Vector2.ZERO
		var best := INF
		for city in near_cities:
			var d := city.distance_to((a + b) * 0.5)
			if d < best:
				best = d
				target = city
		if best > PILGRIM_RANGE or best < 1.5 or _h(index, 0, 90) >= probability:
			continue
		# Vers la cité : départ au bout le plus éloigné.
		var from := a if a.distance_to(target) > b.distance_to(target) else b
		var to := b if from == a else a
		if from.distance_to(to) > PIECE:
			from = to + (from - to).normalized() * PIECE
		var phase := _h(index, 0, 91)
		for k in 3:
			if placed < budget and _pool.add("pilgrim", "walk", from, to, phase, 0.5 + 0.5 * (k % 2), 1.3 * k):
				placed += 1
	return placed
