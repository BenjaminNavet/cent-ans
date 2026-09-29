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
## - `forced` : scènes forcées (`--scene=<province>:<kind>`, tests, captures), intensité 1.
## Aucune scène n'est signalée (pas d'icône) : on les découvre en zoomant (spec § 3.3).

const KINDS := ["plague", "famine", "revolt", "devastation", "siege", "construction", "fair", "celebration", "flood", "muster"]
const FIGURES_MIN := 6
const FIGURES_MAX := 24
## Rayon (unités monde) d'une maquette de colonie sans `SettlementLayer` (tests).
const DEFAULT_MODEL_RADIUS := 2.0
## Marge (m) entre le bord de la maquette et le cœur de la scène.
const EDGE_M := 6.0
## Distance (unités monde) au centre de la colonie en deçà de laquelle un fleuve déborde (crue).
const FLOOD_RANGE := 18.0

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
var _warned: Dictionary = {}


func setup(map_data: MapData, settlement_data: SettlementData, layer: SettlementLayer = null) -> void:
	_map_data = map_data
	_data = settlement_data
	_layer = layer
	_seat.clear()
	if _data == null:
		return
	for i in _data.settlements.size():
		var province := str(_data.settlements[i]["province"])
		if not _seat.has(province):
			_seat[province] = i


## `--scene=<province>:<kind>[,…]` → entrées de `forced` (valeurs invalides ignorées).
static func parse_forced(spec: String) -> Array:
	var out: Array = []
	for pair in spec.split(",", false):
		var parts := pair.split(":")
		if parts.size() == 2 and KINDS.has(parts[1]):
			out.append({"province": parts[0], "kind": parts[1], "settlement": "", "intensity": 1.0})
	return out


## Une fois par tour : scènes du pont (conservées si `sim` est nul) et forcées, lieux résolus.
func refresh(sim: Object) -> void:
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
		if not KINDS.has(kind):
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
			"center": entry["px"], "seed": absi(id.hash()) + KINDS.find(kind) * 7919,
			"intensity": intensity, "since_turn": int(scene.get("since_turn", 0)),
		}
	staged = by_key.values()
	quiet_settlements.clear()
	idle_provinces.clear()
	fire_points = PackedVector2Array()
	for scene in staged:
		match str(scene["kind"]):
			"plague":
				quiet_settlements[scene["settlement"]] = true
				fire_points.append(_side_point(scene, 1.6))
			"revolt":
				fire_points.append(_side_point(scene, 1.3))
			"famine":
				idle_provinces[scene["province"]] = true
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


static func _h(a: int, b: int) -> float:
	return VegetationFields.hash01(a * 73856093 ^ b * 19349663)


## Rayon (unités monde) de la maquette de la colonie à l'échelle courante.
func _model_radius(index: int) -> float:
	if _layer == null:
		return DEFAULT_MODEL_RADIUS
	return _layer.model_radius(index) * _layer.model_scale(index)


## Direction (carte) du côté de la colonie où se tient la scène (stable par colonie et type).
static func _side(scene: Dictionary) -> Vector2:
	var angle := _h(int(scene["seed"]), 1) * TAU
	return Vector2(cos(angle), sin(angle))


## Point au bord de la maquette (`factor` × rayon), du côté de la scène (fumées).
func _side_point(scene: Dictionary, factor: float) -> Vector2:
	return (scene["center"] as Vector2) + _side(scene) * (_model_radius(int(scene["index"])) * factor + 0.5)


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
		if d <= radius + _model_radius(int(scene["index"])):
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


## Mètres → unités monde (échelle des figurines du placement en cours).
func _m(meters: float) -> float:
	return meters * _pool.current_scale()


static func _yaw(dir: Vector2) -> float:
	return atan2(dir.x, dir.y)


static func _right(dir: Vector2) -> Vector2:
	return Vector2(-dir.y, dir.x)


## Point à `out_m` vers l'extérieur et `along_m` le long du bord, depuis `anchor`.
func _at(frame: Dictionary, out_m: float, along_m: float) -> Vector2:
	return (frame["anchor"] as Vector2) + (frame["out"] as Vector2) * _m(out_m) + (frame["along"] as Vector2) * _m(along_m)


## Point d'un emplacement `slot` (latéral, avance : `FolkModels.slot`) d'un accessoire posé en
## `at` et tourné vers `dir`.
func _slot_at(at: Vector2, dir: Vector2, slot: Vector2) -> Vector2:
	return at + (dir * slot.y + _right(dir) * slot.x) * _pool.current_scale()


func _stage(scene: Dictionary, figures: int) -> void:
	var out := _side(scene)
	var frame := {
		"anchor": (scene["center"] as Vector2) + out * (_model_radius(int(scene["index"])) + _m(EDGE_M)),
		"out": out,
		"along": _right(out),
		"seed": int(scene["seed"]),
		"center": scene["center"],
		"index": scene["index"],
	}
	match str(scene["kind"]):
		"plague":
			_plague(frame, figures)
		"famine":
			_famine(frame, figures)
		"revolt":
			_revolt(frame, figures)
		"devastation":
			_devastation(frame, figures)
		"siege":
			_siege(frame, figures)
		"construction":
			_construction(frame, figures)
		"fair":
			_fair(frame, figures)
		"celebration":
			_celebration(frame, figures)
		"flood":
			_flood(frame, figures)
		"muster":
			_muster(frame, figures)


## Cortège en file sur un trajet : croix en tête (et bannières), puis les marcheurs deux par
## deux ; tous à la même phase, décalés par `behind_m`. Renvoie les figurines posées.
func _procession(from: Vector2, to: Vector2, phase: float, walkers: int, roles: Array, banners: int) -> int:
	var placed := 0
	if _pool.add("monk", "procession", from, to, phase, 0.0, 0.0):
		placed += 1
		_pool.add("procession_cross", "", from, to, phase, 0.35, 0.0)
	for b in banners:
		var behind := 1.6 * (b + 1)
		if _pool.add("monk", "procession", from, to, phase, 0.0, behind):
			placed += 1
			_pool.add("procession_banner", "", from, to, phase, 0.35, behind)
	var start := 1.6 * (banners + 1) + 0.8
	for k in walkers:
		var role: String = roles[k % roles.size()]
		if not _pool.add(role, "procession", from, to, phase, -0.5 + float(k % 2), start + 1.2 * float(k / 2)):
			break
		placed += 1
	return placed


## Peste : convoi des morts (charrette et deux porteurs), flagellants en procession derrière la
## croix, bûcher (sa fumée : `fire_points`).
func _plague(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var from := _at(f, 0.0, -12.0)
	var to := _at(f, 0.0, 12.0)
	var phase := _h(seed_value, 2)
	if _pool.add("dead_cart", "", from, to, phase):
		for slot_name in ["porter_l", "porter_r"]:
			var slot := FolkModels.slot("dead_cart", slot_name, Vector2(0.25 if slot_name == "porter_r" else -0.25, 1.95))
			_pool.add("porter", "procession", from, to, phase, slot.x, -slot.y)
	_procession(_at(f, 9.0, 14.0), _at(f, 9.0, -14.0), _h(seed_value, 3), maxi(n - 3, 1), ["pilgrim", "monk"], 0)
	_pool.add_static("pyre", "", _at(f, 5.0, 7.0), _h(seed_value, 4) * TAU)


## Disette : file de gens devant l'église (vers la colonie), immobiles.
func _famine(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var face := -(f["out"] as Vector2)
	var roles := ["peasant", "peasant_b", "pilgrim", "porter"]
	for k in n:
		var at := _at(f, -2.0 + 0.9 * k, (_h(seed_value, 10 + k) - 0.5) * 0.6)
		if not _pool.add_static(roles[k % roles.size()], "idle", at, _yaw(face) + (_h(seed_value, 40 + k) - 0.5) * 0.5):
			return


## Révolte : attroupement d'émeutiers (fourches et torches des `villager_2`) tourné vers la
## colonie, quelques agités qui vont et viennent ; fumée (`fire_points`).
func _revolt(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var center: Vector2 = f["center"]
	var spread := sqrt(float(n)) * 0.9
	var pacing := n / 5
	for k in n:
		var angle := _h(seed_value, 10 + k) * TAU
		var r := spread * sqrt(_h(seed_value, 40 + k))
		var at := _at(f, cos(angle) * r, sin(angle) * r)
		if k < pacing:
			var dir := (f["along"] as Vector2) * (1.0 if k % 2 == 0 else -1.0)
			if not _pool.add("rioter", "walk", at, at + dir * _m(4.0), _h(seed_value, 70 + k)):
				return
			continue
		var face := (center - at).normalized()
		if not _pool.add_static("rioter", "idle", at, _yaw(face) + (_h(seed_value, 90 + k) - 0.5) * 0.7):
			return


## Dévastation : fuyards avec baluchons qui s'éloignent de la colonie par petits groupes (les
## ruines existent déjà, CV1).
func _devastation(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var center: Vector2 = f["center"]
	var groups := maxi(n / 3, 1)
	var placed := 0
	var inner := (f["anchor"] as Vector2).distance_to(center) - _m(EDGE_M) + _m(2.0)
	for g in groups:
		var angle := _h(seed_value, 5) * TAU + (float(g) + _h(seed_value, 10 + g) * 0.5) * TAU / float(groups)
		var dir := Vector2(cos(angle), sin(angle))
		var from := center + dir * inner
		var to := from + dir * _m(30.0)
		var phase := _h(seed_value, 30 + g)
		var size := 2 + int(_h(seed_value, 50 + g) * 2.0)
		for k in size:
			if placed >= n:
				return
			var role := "porter" if k % 2 == 0 else "peasant_b"
			if not _pool.add(role, "procession", from, to, phase, 0.6 * float(k % 2), 1.2 * float(k)):
				return
			placed += 1


## Siège : charrettes de ravitaillement vers le camp (le camp existe déjà, CV2) et fourrageurs
## dans les champs voisins.
func _siege(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var carts := 1 + int(n >= 12)
	for c in carts:
		var from := _at(f, 40.0, -6.0 + 12.0 * c)
		var to := _at(f, 8.0, -3.0 + 6.0 * c)
		var phase := _h(seed_value, 2 + c)
		if _pool.add("peasant_cart", "roll", from, to, phase):
			var carter := FolkModels.slot("peasant_cart", "carter", Vector2(0.75, 3.3))
			_pool.add("peasant", "walk", from, to, phase, carter.x, -carter.y)
	for k in maxi(n - carts, 0):
		var angle := _h(seed_value, 10 + k) * PI - PI * 0.5
		var r := 14.0 + 18.0 * _h(seed_value, 40 + k)
		var at := _at(f, cos(angle) * r, sin(angle) * r * 1.5)
		var dir := Vector2.from_angle(_h(seed_value, 70 + k) * TAU)
		if not _pool.add("porter", "harvest", at, at + dir * _m(6.0), _h(seed_value, 90 + k)):
			return


## Chantier : échafaudage le long de la colonie, maçons qui portent du tas à l'échelle,
## charrette de pierres et son charretier.
func _construction(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var dir: Vector2 = f["along"]
	var at := _at(f, -2.0, 0.0)
	_pool.add_static("scaffold", "", at, _yaw(dir))
	var pile := _slot_at(at, dir, FolkModels.slot("scaffold", "pile", Vector2(0.35, 2.5)))
	var ladder := _slot_at(at, dir, FolkModels.slot("scaffold", "ladder_foot", Vector2(-1.6, 1.85)))
	var cart_at := _at(f, 3.0, 8.0)
	_pool.add_static("stone_cart", "", cart_at, _yaw(-dir))
	var carter := FolkModels.slot("stone_cart", "carter", Vector2(0.75, 3.3))
	_pool.add_static("peasant", "idle", _slot_at(cart_at, -dir, carter), _yaw(-dir))
	var masons := mini(n - 1, 10)
	for k in masons:
		var forward := k % 2 == 0
		if not _pool.add("porter", "walk", pile if forward else ladder, ladder if forward else pile, float(k) / float(maxi(masons, 1)) + _h(seed_value, 10 + k) * 0.1, 0.4 * float(k % 3)):
			return


## Foire : deux rangées d'étals face à l'allée, vendeurs et chalands, foule qui déambule,
## bétail à l'entrée, étendards aux deux bouts.
func _fair(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var out: Vector2 = f["out"]
	var stalls := clampi(n / 5, 2, 6)
	var placed := 0
	var half := float(stalls / 2) * 4.5 * 0.5
	for s in stalls:
		var row := 1.0 if s % 2 == 0 else -1.0
		var along := -half + 4.5 * float(s / 2)
		var at := _at(f, row * 3.5, along)
		# Étal (long axe = avance du modèle) couché le long de l'allée, chalands (`buyer`, côté
		# gauche du modèle) côté allée : la droite du modèle pointe vers l'extérieur de la rangée.
		var facing := (f["along"] as Vector2) * -row
		_pool.add_static("market_stall", "", at, _yaw(facing))
		var seller := FolkModels.slot("market_stall", "seller", Vector2(0.9, 0.0))
		var buyer := FolkModels.slot("market_stall", "buyer", Vector2(-1.2, 0.0))
		if _pool.add_static("merchant", "idle", _slot_at(at, facing, seller), _yaw(-out * row)):
			placed += 1
		if placed < n and _pool.add_static("peasant_b" if s % 3 else "peasant", "idle", _slot_at(at, facing, buyer), _yaw(out * row)):
			placed += 1
	var lane_from := _at(f, 0.0, -half - 3.0)
	var lane_to := _at(f, 0.0, half + 3.0)
	var k := 0
	while placed < n:
		var forward := k % 2 == 0
		if not _pool.add("peasant" if k % 3 else "porter", "walk", lane_from if forward else lane_to, lane_to if forward else lane_from, _h(seed_value, 10 + k), (_h(seed_value, 40 + k) - 0.5) * 2.0):
			break
		placed += 1
		k += 1
	for b in 4:
		_pool.add_static("sheep", "", _at(f, (_h(seed_value, 60 + b) - 0.5) * 3.0, half + 6.0 + _h(seed_value, 70 + b) * 3.0), _h(seed_value, 80 + b) * TAU)
	_pool.add_static("cow", "", _at(f, 2.0, half + 8.0), _h(seed_value, 90) * TAU)
	for end in [-1.0, 1.0]:
		_pool.add_static("procession_banner", "", _at(f, 0.0, end * (half + 2.0)), _yaw(f["along"]))


## Fête (sacre, paix, mariage) : procession derrière la croix et deux bannières, le long du bord
## de la colonie.
func _celebration(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	_procession(_at(f, 2.0, -18.0), _at(f, 2.0, 18.0), _h(seed_value, 2), maxi(n - 3, 1), ["peasant", "peasant_b", "monk", "merchant"], 2)


## Crue : nappes d'eau boueuse le long du fleuve voisin (sinon devant la colonie) et habitants
## réfugiés au sec, près des maisons (pas de toits : le réservoir pose au sol).
func _flood(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var center: Vector2 = f["center"]
	var sheets := 3 + int(float(n) / 6.0)
	var points := _river_points(center, FLOOD_RANGE + _model_radius(int(f["index"])),sheets, _m(18.0))
	if points.is_empty():
		_pool.warn_once("scenes:flood_river", "FolkScenes: no river near a flooded settlement, water laid in front of it")
		for k in 2:
			points.append(_at(f, 8.0 + 10.0 * k, (_h(seed_value, 5 + k) - 0.5) * 12.0))
	for k in points.size():
		_pool.add_static("flood_water", "", points[k], _h(seed_value, 20 + k) * TAU)
	var face := -(f["out"] as Vector2)
	for k in n:
		var at := _at(f, -4.0 + 1.5 * float(k % 4), -3.0 + 1.5 * float(k / 4))
		var role := "porter" if k % 3 == 0 else "peasant"
		if not _pool.add_static(role, "idle", at, _yaw(face) + (_h(seed_value, 40 + k) - 0.5) * 1.2):
			return


## Points des fleuves à moins de `range_world` de `center`, espacés d'au moins `spacing`, les
## plus proches d'abord, au plus `count`.
func _river_points(center: Vector2, range_world: float, count: int, spacing: float) -> PackedVector2Array:
	var found: Array = []
	if _map_data == null:
		return PackedVector2Array()
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
	var out := PackedVector2Array()
	for entry in found:
		var p: Vector2 = entry[1]
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


## Recrutement : carré de recrues à l'exercice près de la ville, un sergent qui longe le front.
func _muster(f: Dictionary, n: int) -> void:
	var seed_value: int = f["seed"]
	var along: Vector2 = f["along"]
	var columns := clampi(int(ceil(sqrt(float(n) * 1.5))), 2, 8)
	var face := f["out"] as Vector2
	for k in maxi(n - 1, 1):
		var row := k / columns
		var col := k % columns
		var at := _at(f, 6.0 + 1.3 * float(row), (float(col) - float(columns - 1) * 0.5) * 1.2)
		if not _pool.add_static("recruit", "drill", at, _yaw(-face)):
			return
	var front_from := _at(f, 4.0, -float(columns) * 0.7)
	_pool.add("guard", "guard_walk", front_from, front_from + along * _m(float(columns) * 1.4), _h(seed_value, 3))
