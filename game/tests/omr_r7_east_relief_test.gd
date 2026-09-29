extends SceneTree
## Relief fin à l'Est et au Sud (lot OMR R7, ADR 0119) : pyramide dans le cadre monde.
##
## 1. Manifeste réel : `root_origin_tiles` [0, 0], étages E1-E2 listés au Bosphore, à Moscou,
##    au Caire et à Tunis ; Rouen garde ses étages fins (E4) dans le cadre monde.
## 2. Si le cache est là : page E2 du Bosphore décodée, eau du détroit sous 0 m et collines
##    de Beykoz au-dessus de 150 m (Copernicus à 90 m, pas l'ETOPO lissé d'E0).
##
## godot --headless --path game --script res://tests/omr_r7_east_relief_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	_run()
	print("omr_r7_east_relief_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("omr_r7_east_relief_test: " + message)
	return condition


## Hauteurs (m) extrêmes des 3 × 3 pixels autour d'un point monde, page PNG 16 bits de `level`
## (x = min, y = max).
func _range_at(pyramid: ReliefPyramid, level: int, x: float, y: float) -> Vector2:
	var t := ReliefPyramid.tile_at(level, x, y)
	var decoded := Png16.load_gray16(pyramid.tile_path(level, t.x, t.y))
	if decoded.is_empty():
		return Vector2(NAN, NAN)
	var origin := ReliefPyramid.tile_origin(level, t.x, t.y)
	var px := int((x - origin.x) / ReliefPyramid.pixel_units(level))
	var py := int((y - origin.y) / ReliefPyramid.pixel_units(level))
	var data: PackedByteArray = decoded["data"]
	var width := int(decoded["width"])
	var lo := INF
	var hi := -INF
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var cx := clampi(px + dx, 0, width - 1)
			var cy := clampi(py + dy, 0, int(decoded["height"]) - 1)
			var o := (cy * width + cx) * 2
			var h := pyramid.height_min_m + float((int(data[o]) << 8) | int(data[o + 1])) / 65535.0 * pyramid.height_range_m
			lo = minf(lo, h)
			hi = maxf(hi, h)
	return Vector2(lo, hi)


func _run() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var manifest: Variant = JSON.parse_string(FileAccess.get_file_as_string(map_dir.path_join("relief_pyramid.json")))
	if not _check(manifest is Dictionary, "manifeste illisible"):
		return
	_check((manifest as Dictionary).get("root_origin_tiles", []) == [0, 0], "root_origin_tiles %s" % (manifest as Dictionary).get("root_origin_tiles"))
	var pyramid := ReliefPyramid.new()
	# Tuiles listées (sans le contrôle de présence sur le disque) : `tiles_override` vide = manifeste.
	if not pyramid.load_manifest(map_dir):
		print("omr_r7_east_relief_test: pas de cache de relief, contrôles réels sautés")
		return
	# Points en unités monde (EPSG:3035, 719 m).
	var places := {
		"Bosphore": Vector2(5199.0, 4166.1),
		"Moscou": Vector2(5337.3, 1718.9),
		"Le Caire": Vector2(5860.3, 5740.7),
		"Tunis": Vector2(3013.7, 5100.8),
	}
	for name in places:
		var p: Vector2 = places[name]
		var level := pyramid.finest_level_at(p.x, p.y)
		_check(level >= 2, "%s : étage le plus fin %d (E2 attendu)" % [name, level])
	var rouen := pyramid.finest_level_at(2097.0, 3099.0)
	_check(rouen >= 4, "Rouen : étage le plus fin %d (E4+ attendu)" % rouen)
	# Page E2 réelle du Bosphore, profil à 41,10° N : détroit (Rumeli Hisarı, ≈ 29,057° E)
	# et collines de Beykoz (≈ 29,12° E, ≈ 320 m).
	var strait := _range_at(pyramid, 2, 5201.9, 4160.6)
	var hills := _range_at(pyramid, 2, 5209.2, 4158.7)
	if is_nan(strait.x) or is_nan(hills.y):
		_failures += 1
		push_error("omr_r7_east_relief_test: page E2 du Bosphore illisible")
		return
	print("omr_r7_east_relief_test: Bosphore %.1f m, collines de Beykoz %.1f m" % [strait.x, hills.y])
	_check(strait.x <= 0.0, "détroit à %.1f m" % strait.x)
	_check(hills.y > 150.0, "collines de Beykoz à %.1f m" % hills.y)
