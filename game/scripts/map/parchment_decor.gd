class_name ParchmentDecor
extends RefCounted

## Lot CM2 : emplacements des ornements de la mer des portulans (roses des vents, navires,
## monstres marins), choisis au démarrage à partir de la distance à la côte (aucune donnée
## codée en dur) : roses et monstres au large, navires près des côtes. Purement visuel.

## Pas de la grille d'échantillonnage (px carte).
const GRID := 24
## Distance à la côte au-delà de laquelle la mer compte comme « large » (px carte).
const OPEN_SEA_PX := 45.0

## Lot FA6 : ornements réels (Atlas catalan de 1375, domaine public) découpés par
## `tools/cent_ans_tools/parchment_ornaments.py` ; catalogue `data/map/parchment_ornaments.json`.
const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const ORNAMENTS_FILE := "map/parchment_ornaments.json"
const ORNAMENTS_DIR := "res://assets/textures/parchment/"
## Shader de la mer : la rose peinte y est posée comme texture par défaut de `pm_rose_tex`
## (`parchment_sea.gdshaderinc`), sans toucher au matériau de `Sea`.
const SEA_SHADER_PATH := "res://shaders/water.gdshader"
const ROSE_UNIFORM := "pm_rose_tex"

## Navires peints : distance minimale à la côte et eau libre exigée autour (px carte).
const PAINTED_SHIP_COAST := 30.0
const PAINTED_SHIP_CLEARANCE := 60.0
## Pas de la grille des emplacements candidats des navires peints (multiple de `GRID`).
const PAINTED_SHIP_GRID := GRID * 4
## Navires peints : poids de l'éloignement du centre de la carte face à l'écart aux autres
## ornements (0 = répartition uniforme, bords de carte compris).
const PAINTED_SHIP_CENTER_BIAS := 0.25
## Écart aux autres ornements au-delà duquel un emplacement n'est plus mieux noté (px carte).
const PAINTED_SHIP_SPACING := 700.0
const PAINTED_MONSTER_CLEARANCE := 120.0
const COAST_MARGIN_PX := 8.0

var roses: Array[Vector3] = []  # x, y, rayon (px carte)
var ships: Array[Vector3] = []  # x, y, cap (radians, 0 = vers l'est)
var monsters: Array[Vector3] = []  # x, y, variante
## FA6 : ornements peints par genre, {texture, anchor (0-1), height (unités du dessin), faces_left}.
## Vide = dessin par code d'origine (`use` du catalogue à `drawn`, `--no-fa-parchment`, catalogue
## ou textures absents).
var ship_ornaments: Array[Dictionary] = []
var monster_ornaments: Array[Dictionary] = []
var rose_texture: Texture2D


## `coast_dist` : raster L8 signé (valeur × 255 − 128) / `scale` px, < 0 en mer.
static func build(map: MapData, coast_scale: float = 2.0) -> ParchmentDecor:
	var decor := ParchmentDecor.new()
	decor._load_ornaments()
	var img := map.coast_dist_image
	if img == null or img.is_empty():
		return decor
	# FA6 : les ornements peints sont plus grands à l'écran que les dessins : ils demandent de
	# l'eau libre autour d'eux (sinon ils chevauchent la côte et les noms).
	var painted_ships := not decor.ship_ornaments.is_empty()
	var painted_monsters := not decor.monster_ornaments.is_empty()
	var sea_at := func(q: Vector2) -> float:
		if q.x < 0.0 or q.y < 0.0 or q.x >= map.size.x or q.y >= map.size.y:
			return 0.0
		return (img.get_pixel(int(q.x * sx_of(img, map)), int(q.y * sy_of(img, map))).r * 255.0 - 128.0) / coast_scale
	var sx := float(img.get_width()) / float(map.size.x)
	var sy := float(img.get_height()) / float(map.size.y)
	var open_sea: Array = []  # [score, Vector2]
	var near_sea: Array = []
	var margin := GRID * 4
	for y in range(margin, map.size.y - margin, GRID):
		for x in range(margin, map.size.x - margin, GRID):
			var c := (img.get_pixel(int(x * sx), int(y * sy)).r * 255.0 - 128.0) / coast_scale
			var jitter := _hash(x, y)
			if c <= -OPEN_SEA_PX:
				open_sea.append([-c + jitter * 8.0, Vector2(x, y)])
			if painted_ships and c < -PAINTED_SHIP_COAST and x % PAINTED_SHIP_GRID == 0 and y % PAINTED_SHIP_GRID == 0:
				near_sea.append([jitter, Vector2(x, y)])
			elif not painted_ships and c < -10.0 and c > -30.0:
				near_sea.append([jitter, Vector2(x, y)])
	open_sea.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	near_sea.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	# Roses : les points les plus au large, espacés ; privilégie le cœur de la carte.
	var center := Vector2(map.size) * 0.5
	var rose_candidates := open_sea.duplicate()
	rose_candidates.sort_custom(func(a: Array, b: Array) -> bool:
		return a[0] - a[1].distance_to(center) * 0.02 > b[0] - b[1].distance_to(center) * 0.02)
	var taken: Array[Vector2] = []
	for entry in rose_candidates:
		var p: Vector2 = entry[1]
		if _far_from(p, taken, 850.0):
			taken.append(p)
			decor.roses.append(Vector3(p.x, p.y, 62.0))
			if decor.roses.size() >= 4:
				break
	# Monstres marins : au large, loin des roses.
	var monster_taken: Array[Vector2] = taken.duplicate()
	for entry in open_sea:
		var p: Vector2 = entry[1]
		if painted_monsters and not _clear_water(sea_at, p, PAINTED_MONSTER_CLEARANCE):
			continue
		if _far_from(p, monster_taken, 520.0):
			monster_taken.append(p)
			decor.monsters.append(Vector3(p.x, p.y, float(decor.monsters.size() % 2)))
			if decor.monsters.size() >= 4:
				break
	# Navires : près des côtes, espacés, cap tiré au sort.
	var ship_taken: Array[Vector2] = monster_taken.duplicate()
	if painted_ships:
		# Peu nombreux et grands : répartis au plus loin les uns des autres (et des roses et des
		# monstres), le cœur de la carte d'abord.
		var spots: Array[Vector2] = []
		for entry in near_sea:
			if _clear_water(sea_at, entry[1], PAINTED_SHIP_CLEARANCE):
				spots.append(entry[1])
		while decor.ships.size() < 9 and not spots.is_empty():
			var best := -1
			var best_score := -INF
			for i in spots.size():
				var nearest := INF
				for q in ship_taken:
					nearest = minf(nearest, spots[i].distance_squared_to(q))
				var score := minf(sqrt(nearest), PAINTED_SHIP_SPACING) - spots[i].distance_to(center) * PAINTED_SHIP_CENTER_BIAS
				if score > best_score:
					best_score = score
					best = i
			var p := spots[best]
			spots.remove_at(best)
			ship_taken.append(p)
			decor.ships.append(Vector3(p.x, p.y, (1.0 if _hash(int(p.x), int(p.y) + 7) > 0.5 else -1.0)))
		return decor
	for entry in near_sea:
		var p: Vector2 = entry[1]
		if _far_from(p, ship_taken, 330.0):
			ship_taken.append(p)
			decor.ships.append(Vector3(p.x, p.y, (1.0 if _hash(int(p.x), int(p.y) + 7) > 0.5 else -1.0)))
			if decor.ships.size() >= 9:
				break
	return decor


static func sx_of(img: Image, map: MapData) -> float:
	return float(img.get_width()) / float(map.size.x)


static func sy_of(img: Image, map: MapData) -> float:
	return float(img.get_height()) / float(map.size.y)


## Vrai si la mer est libre (à plus de `COAST_MARGIN_PX` de la côte) sur un cercle de rayon
## `radius` autour de `p` et à mi-rayon.
static func _clear_water(sea_at: Callable, p: Vector2, radius: float) -> bool:
	for k in 8:
		var dir := Vector2.from_angle(TAU * k / 8.0)
		if sea_at.call(p + dir * radius) > -COAST_MARGIN_PX or sea_at.call(p + dir * radius * 0.5) > -COAST_MARGIN_PX:
			return false
	return true


## `--no-fa-parchment` après `--` : ornements dessinés par code d'avant FA6 (captures A/B).
static func painted_enabled() -> bool:
	return not OS.get_cmdline_user_args().has("--no-fa-parchment")


## Lit le catalogue des ornements peints et pose la rose sur le shader de la mer.
func _load_ornaments() -> void:
	var sea_shader := load(SEA_SHADER_PATH) as Shader if ResourceLoader.exists(SEA_SHADER_PATH) else null
	if sea_shader != null:
		sea_shader.set_default_texture_parameter(ROSE_UNIFORM, null)
	if not painted_enabled():
		return
	var path := str(MAP_PATHS.default_data_dir()).path_join(ORNAMENTS_FILE)
	if not FileAccess.file_exists(path):
		return
	var catalogue: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not catalogue is Dictionary:
		push_warning("ParchmentDecor: unreadable %s" % path)
		return
	# `use` : par genre, `real` (découpes du catalogue) ou `drawn` (dessin par code d'origine).
	var use: Dictionary = (catalogue as Dictionary).get("use", {})
	for entry: Dictionary in (catalogue as Dictionary).get("ornaments", []):
		if str(use.get(str(entry.get("kind", "")), "real")) != "real":
			continue
		var texture_path := ORNAMENTS_DIR + str(entry.get("file", ""))
		if not ResourceLoader.exists(texture_path):
			continue
		var texture := load(texture_path) as Texture2D
		var display: Dictionary = entry.get("display", {})
		var anchor: Array = display.get("anchor", [0.5, 0.5])
		var ornament := {
			"texture": texture,
			"anchor": Vector2(float(anchor[0]), float(anchor[1])),
			"height": float(display.get("height", 3.0)),
			"faces_left": str(display.get("faces", "right")) == "left",
		}
		match str(entry.get("kind", "")):
			"rose":
				rose_texture = texture
			"ship":
				ship_ornaments.append(ornament)
			"monster":
				monster_ornaments.append(ornament)
	if sea_shader != null and rose_texture != null:
		sea_shader.set_default_texture_parameter(ROSE_UNIFORM, rose_texture)


static func _far_from(p: Vector2, points: Array[Vector2], spacing: float) -> bool:
	for q in points:
		if p.distance_to(q) < spacing:
			return false
	return true


static func _hash(x: int, y: int) -> float:
	var h := (x * 73856093) ^ (y * 19349663)
	h = (h ^ (h >> 13)) * 1274126177
	return float(absi(h) % 10007) / 10007.0
