class_name FolkSceneLayers
extends RefCounted

## SC MC6 : mise en scène des scènes de province par couches. Chaque type de scène
## (`data/rules/folk_scene_layouts.json`, schéma `folk_scene_layouts.schema.json`) est une liste
## de couches de 4 archétypes paramétrés :
## - `procession` : cortège en file sur un trajet (croix, bannières, marcheurs deux par deux) ou
##   groupes de fuyards sur des trajets de fuite (`path: "escape"`) ;
## - `convoy` : véhicule en marche avec des figurines posées sur ses emplacements ;
## - `scatter` : figurines disposées en grille, en disque ou en patrouille, immobiles ou
##   allant et venant ;
## - `site` : accessoires posés (étals, échafaudage, bûcher, nappes de crue…), figurines attachées
##   à leurs emplacements, navettes de marcheurs.
## Positions des données : [devant, le long] en mètres depuis `frame.anchor` (devant = vers
## l'extérieur de la colonie). Rendu seulement : `FolkScenes` résout les lieux et l'emprise.

static var _lookup := JsonLookup.new("rules/folk_scene_layouts.json", {"scenes": {}})

var _host: FolkScenes
var _pool: FolkPool


static func layouts() -> Dictionary:
	return _lookup.data().get("scenes", {})


## Types de scène connus, dans l'ordre du fichier (l'ordre entre dans la graine des scènes).
static func kinds() -> Array:
	return layouts().keys()


## Effets hors réservoir d'un type (voir la description du fichier de données).
static func effects_of(kind: String) -> Dictionary:
	return (layouts().get(kind, {}) as Dictionary).get("effects", {})


func _init(host: FolkScenes) -> void:
	_host = host


## Pose les couches de `kind` pour la scène `frame` (`anchor`, `out`, `along`, `seed`, `center`,
## `index`) avec `figures` figurants.
func stage(pool: FolkPool, frame: Dictionary, kind: String, figures: int) -> void:
	_pool = pool
	var scene: Dictionary = layouts().get(kind, {})
	for layer: Dictionary in scene.get("layers", []):
		match str(layer["type"]):
			"procession":
				_procession(frame, layer, figures)
			"convoy":
				_convoy(frame, layer, figures)
			"scatter":
				_scatter(frame, layer, figures)
			"site":
				_site(frame, layer, figures, float(effects_of(kind).get("river_cache", 0.0)))


# --- Aides -----------------------------------------------------------------------------


## floor(base + per_n × figurants), borné.
static func count_of(spec: Dictionary, figures: int) -> int:
	if spec.has("fixed"):
		return int(spec["fixed"])
	var value := floori(float(spec.get("base", 0.0)) + float(spec.get("per_n", 0.0)) * float(figures))
	return clampi(value, int(spec.get("min", 0)), int(spec.get("max", 1 << 30)))


func _m(meters: float) -> float:
	return meters * _pool.current_scale()


func _at(frame: Dictionary, point: Array, extra_out_m: float = 0.0, extra_along_m: float = 0.0) -> Vector2:
	return (frame["anchor"] as Vector2) + (frame["out"] as Vector2) * _m(float(point[0]) + extra_out_m) \
			+ (frame["along"] as Vector2) * _m(float(point[1]) + extra_along_m)


static func _yaw(dir: Vector2) -> float:
	return atan2(dir.x, dir.y)


static func _right(dir: Vector2) -> Vector2:
	return Vector2(-dir.y, dir.x)


## Direction d'un jeton ("along", "-out*row"…) ; `row` = ±1 pour le suffixe `*row`. ZERO si
## "random".
static func _dir_of(frame: Dictionary, token: String, row: float = 1.0) -> Vector2:
	var sign_value := 1.0
	if token.begins_with("-"):
		sign_value = -1.0
		token = token.substr(1)
	if token.ends_with("*row"):
		sign_value *= row
		token = token.trim_suffix("*row")
	match token:
		"along":
			return (frame["along"] as Vector2) * sign_value
		"out":
			return (frame["out"] as Vector2) * sign_value
	return Vector2.ZERO


func _yaw_of(frame: Dictionary, token: String, row: float, salt: int) -> float:
	var dir := _dir_of(frame, token, row)
	if dir == Vector2.ZERO:
		return Hash.h01(int(frame["seed"]), salt) * TAU
	return _yaw(dir)


func _pick(roles: Array, index: int) -> String:
	return str(roles[index % roles.size()])


# --- procession ------------------------------------------------------------------------


func _procession(frame: Dictionary, layer: Dictionary, figures: int) -> void:
	if layer.get("path", "") == "escape":
		_escape_groups(frame, layer, figures)
		return
	var seed_value: int = frame["seed"]
	var from := _at(frame, layer["from"])
	var to := _at(frame, layer["to"])
	var phase := Hash.h01(seed_value, int(layer["phase_salt"]))
	var banners := int(layer.get("banners", 0))
	if bool(layer.get("cross", false)) and _pool.add("monk", "procession", from, to, phase, 0.0, 0.0):
		_pool.add("procession_cross", "", from, to, phase, 0.35, 0.0)
	for b in banners:
		var behind := 1.6 * (b + 1)
		if _pool.add("monk", "procession", from, to, phase, 0.0, behind):
			_pool.add("procession_banner", "", from, to, phase, 0.35, behind)
	var start := 1.6 * (banners + 1) + 0.8
	var roles: Array = layer["roles"]
	for k in count_of(layer["walkers"], figures):
		if not _pool.add(_pick(roles, k), "procession", from, to, phase, -0.5 + float(k % 2), start + 1.2 * float(k / 2)):
			break


## Dévastation : fuyards avec baluchons qui s'éloignent de la colonie par petits groupes, sur un
## trajet de fuite (route voisine, sinon arc qui s'écarte de l'eau ; LR-18).
func _escape_groups(frame: Dictionary, layer: Dictionary, figures: int) -> void:
	var seed_value: int = frame["seed"]
	var center: Vector2 = frame["center"]
	var groups := count_of(layer["groups"], figures)
	var size_range: Array = layer["group_size"]
	var roles: Array = layer["roles"]
	var placed := 0
	var inner := (frame["anchor"] as Vector2).distance_to(center) - _m(FolkScenes.EDGE_M) * 0.5
	for g in groups:
		var angle := Hash.h01(seed_value, 5) * TAU + (float(g) + Hash.h01(seed_value, 10 + g) * 0.5) * TAU / float(groups)
		var dir := Vector2(cos(angle), sin(angle))
		var path := _host.escape_path_for(int(frame["index"]), g, center + dir * inner, dir, seed_value + g)
		var piece := g % (path.size() - 1)
		var phase := Hash.h01(seed_value, 30 + g)
		var span := int(size_range[1]) - int(size_range[0]) + 1
		var size := int(size_range[0]) + int(Hash.h01(seed_value, 50 + g) * float(span))
		for k in size:
			if placed >= figures:
				return
			if not _pool.add(_pick(roles, k), "procession", path[piece], path[piece + 1], phase, 0.6 * float(k % 2), 1.2 * float(k)):
				return
			placed += 1


# --- convoy ----------------------------------------------------------------------------


func _convoy(frame: Dictionary, layer: Dictionary, figures: int) -> void:
	var seed_value: int = frame["seed"]
	var vehicle := str(layer["vehicle"])
	var from_step: Array = layer.get("from_step", [0, 0])
	var to_step: Array = layer.get("to_step", [0, 0])
	for c in count_of(layer.get("count", {"fixed": 1}), figures):
		var from := _at(frame, layer["from"], float(from_step[0]) * c, float(from_step[1]) * c)
		var to := _at(frame, layer["to"], float(to_step[0]) * c, float(to_step[1]) * c)
		var phase := Hash.h01(seed_value, int(layer["phase_salt"]) + c)
		if not _pool.add(vehicle, str(layer.get("activity", "")), from, to, phase):
			continue
		for part: Dictionary in layer.get("attached", []):
			var slot := FolkModels.slot(vehicle, str(part["slot"]), _vec(part["default"]))
			_pool.add(str(part["role"]), str(part["activity"]), from, to, phase, slot.x, -slot.y)


static func _vec(point: Array) -> Vector2:
	return Vector2(float(point[0]), float(point[1]))


# --- scatter ---------------------------------------------------------------------------


func _scatter(frame: Dictionary, layer: Dictionary, figures: int) -> void:
	var count := count_of(layer["count"], figures)
	match str(layer["pattern"]):
		"grid":
			_grid(frame, layer, count)
		"disc":
			_disc(frame, layer, count)
		"patrol":
			var from := _at(frame, layer["origin"])
			var to := from + (frame["along"] as Vector2) * _m(float(layer["length_m"]))
			for k in count:
				_pool.add(_pick(layer["roles"], k), str(layer["activity"]), from, to, Hash.h01(int(frame["seed"]), int(layer["salt"]) + k))


## Yaw d'une figurine posée en `at` selon `facing` (in : vers la colonie, out, center, random).
func _facing_yaw(frame: Dictionary, layer: Dictionary, at: Vector2, k: int) -> float:
	var seed_value: int = frame["seed"]
	var salt := int(layer["salt"])
	var jitter := (Hash.h01(seed_value, salt + 80 + k) - 0.5) * float(layer.get("yaw_jitter", 0.0))
	match str(layer["facing"]):
		"in":
			return _yaw(-(frame["out"] as Vector2)) + jitter
		"out":
			return _yaw(frame["out"]) + jitter
		"center":
			return _yaw(((frame["center"] as Vector2) - at).normalized()) + jitter
	return Hash.h01(seed_value, salt + 80 + k) * TAU


func _grid(frame: Dictionary, layer: Dictionary, count: int) -> void:
	var seed_value: int = frame["seed"]
	var origin: Array = layer["origin"]
	var per_row: int = clampi(int(ceil(sqrt(float(count) * 1.5))), 2, 8) if str(layer["per_row"]) == "auto" else int(layer["per_row"])
	var columns := mini(per_row, count)
	var along_axis := str(layer["i_axis"]) == "along"
	var roles: Array = layer["roles"]
	for k in count:
		var i := float(k % per_row)
		var j := float(k / per_row)
		if bool(layer.get("center_rows", false)):
			i -= float(columns - 1) * 0.5
		i *= float(layer["i_step_m"])
		j *= float(layer["j_step_m"])
		var jitter := (Hash.h01(seed_value, int(layer["salt"]) + 30 + k) - 0.5) * float(layer.get("jitter_along_m", 0.0))
		var at := _at(frame, origin, j if along_axis else i, i + jitter if along_axis else j + jitter)
		if not _pool.add_static(_pick(roles, k), str(layer["activity"]), at, _facing_yaw(frame, layer, at, k)):
			return


func _disc(frame: Dictionary, layer: Dictionary, count: int) -> void:
	var seed_value: int = frame["seed"]
	var salt := int(layer["salt"])
	var angles: Array = layer["angle"]
	var radii := Vector2(0.0, float(layer["radius_n_scale"]) * sqrt(float(count))) if layer.has("radius_n_scale") else _vec(layer["radius_m"])
	var walk: Dictionary = layer.get("walk", {})
	var walkers := floori(float(walk.get("share", 0.0)) * float(count))
	var roles: Array = layer["roles"]
	for k in count:
		var angle := lerpf(float(angles[0]), float(angles[1]), Hash.h01(seed_value, salt + k))
		var radius := lerpf(radii.x, radii.y, pow(Hash.h01(seed_value, salt + 30 + k), float(layer["radius_pow"])))
		var at := _at(frame, layer["origin"], cos(angle) * radius, sin(angle) * radius * float(layer.get("along_stretch", 1.0)))
		if k < walkers:
			var dir: Vector2
			if str(walk["direction"]) == "along_alternate":
				dir = (frame["along"] as Vector2) * (1.0 if k % 2 == 0 else -1.0)
			else:
				dir = Vector2.from_angle(Hash.h01(seed_value, salt + 60 + k) * TAU)
			if not _pool.add(_pick(roles, k), str(walk["activity"]), at, at + dir * _m(float(walk["distance_m"])), Hash.h01(seed_value, salt + 90 + k)):
				return
			continue
		if not _pool.add_static(_pick(roles, k), str(layer["activity"]), at, _facing_yaw(frame, layer, at, k)):
			return


# --- site ------------------------------------------------------------------------------


func _site(frame: Dictionary, layer: Dictionary, figures: int, river_range: float) -> void:
	var seed_value: int = frame["seed"]
	for prop: Dictionary in layer.get("props", []):
		if str(prop.get("place", "")) == "river":
			_river_props(frame, prop, figures, river_range)
		elif prop.has("repeat"):
			_repeated_props(frame, prop, figures)
		elif prop.has("count"):
			var salt := int(prop.get("salt", 0))
			var scatter: Array = prop.get("scatter_m", [0, 0])
			for k in count_of(prop["count"], figures):
				var extra_out := (Hash.h01(seed_value, salt + k) - 0.5) * float(scatter[0])
				var extra_along := (Hash.h01(seed_value, salt + 10 + k) - 0.5) * float(scatter[1])
				_pool.add_static(str(prop["model"]), "", _at(frame, prop["at"], extra_out, extra_along), _yaw_of(frame, str(prop["yaw"]), 1.0, salt + 20 + k))
		else:
			var at := _at(frame, prop["at"])
			_pool.add_static(str(prop["model"]), "", at, _yaw_of(frame, str(prop["yaw"]), 1.0, int(prop.get("salt", 0))))
			_attach(frame, prop, at, 1.0, 0)
	if layer.has("shuttle"):
		_shuttle(frame, layer["shuttle"], figures)


## Figurines posées sur les emplacements d'un accessoire en `at` (orienté par son jeton de yaw).
func _attach(frame: Dictionary, prop: Dictionary, at: Vector2, row: float, index: int) -> void:
	var facing := _dir_of(frame, str(prop["yaw"]), row)
	for part: Dictionary in prop.get("attached", []):
		var slot := FolkModels.slot(str(prop["model"]), str(part["slot"]), _vec(part["default"]))
		var place := at + (facing * slot.y + _right(facing) * slot.x) * _pool.current_scale()
		var role := str(part["role"]) if part.has("role") else _pick(part["roles"], index)
		_pool.add_static(role, str(part["activity"]), place, _yaw_of(frame, str(part.get("yaw", prop["yaw"])), row, 0))


## Rangées d'accessoires de part et d'autre d'une allée (étals), un vendeur et un chaland par étal.
func _repeated_props(frame: Dictionary, prop: Dictionary, figures: int) -> void:
	var repeat: Dictionary = prop["repeat"]
	var count := count_of(repeat["count"], figures)
	var step := float(repeat["step_m"])
	var half := float(count / 2) * step * 0.5
	for s in count:
		var row := 1.0 if s % 2 == 0 else -1.0
		var at := _at(frame, [row * float(repeat["row_out_m"]), -half + step * float(s / 2)])
		_pool.add_static(str(prop["model"]), "", at, _yaw_of(frame, str(prop["yaw"]), row, 0))
		_attach(frame, prop, at, row, s)


## Nappes d'eau le long du fleuve voisin (sinon devant la colonie). Les points de fleuve sont
## ceux mis en cache par `FolkScenes.resolve` (effet `river_cache`).
func _river_props(frame: Dictionary, prop: Dictionary, figures: int, river_range: float) -> void:
	var seed_value: int = frame["seed"]
	var salt := int(prop.get("salt", 0))
	var index := int(frame["index"])
	var points := _host.river_points(index, frame["center"], river_range + _host.footprint(index), count_of(prop["count"], figures), _m(float(prop["spacing_m"])))
	if points.is_empty():
		_pool.warn_once("scenes:flood_river", "FolkScenes: no river near a flooded settlement, water laid in front of it")
		var fallback: Dictionary = prop["fallback"]
		var step: Array = fallback["step_m"]
		var scatter: Array = fallback["scatter_m"]
		for k in int(fallback["count"]):
			points.append(_at(frame, fallback["at"], float(step[0]) * k + (Hash.h01(seed_value, 5 + k) - 0.5) * float(scatter[0]),
					float(step[1]) * k + (Hash.h01(seed_value, 5 + k) - 0.5) * float(scatter[1])))
	for k in points.size():
		_pool.add_static(str(prop["model"]), "", points[k], Hash.h01(seed_value, salt + k) * TAU)


## Marcheurs qui font la navette entre deux points (maçons du tas à l'échelle, chalands de l'allée).
func _shuttle(frame: Dictionary, shuttle: Dictionary, figures: int) -> void:
	var seed_value: int = frame["seed"]
	var salt := int(shuttle["salt"])
	var ends: Array = shuttle["between"]
	var first := _at(frame, ends[0])
	var second := _at(frame, ends[1])
	var roles: Array = shuttle["roles"]
	for k in count_of(shuttle["count"], figures):
		var forward := k % 2 == 0
		var lateral := (Hash.h01(seed_value, salt + 30 + k) - 0.5) * float(shuttle.get("lateral_m", 0.0)) * 2.0
		var behind := float(shuttle.get("behind_every_m", 0.0)) * float(k % 3)
		if not _pool.add(_pick(roles, k), str(shuttle["activity"]), first if forward else second, second if forward else first, Hash.h01(seed_value, salt + k), lateral, behind):
			return
