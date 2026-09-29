extends SceneTree
## Relief fin à l'Est et au Sud (lot OMR R7, ADR 0119) : pyramide dans le cadre monde.
##
## 1. Manifeste réel : `root_origin_tiles` [0, 0], étages E1-E2 listés au Bosphore, à Moscou,
##    au Caire et à Tunis ; Rouen garde ses étages fins (E4) dans le cadre monde.
## 2. Si le cache est là : page E2 du Bosphore décodée, eau du détroit sous 0 m et collines
##    d'Üsküdar / Çamlıca au-dessus de 40 m (Copernicus, pas l'ETOPO lissé d'E0).
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


## Hauteur (m) d'un point monde sur une page PNG 16 bits de l'étage `level`.
func _height_at(pyramid: ReliefPyramid, level: int, x: float, y: float) -> float:
	var t := ReliefPyramid.tile_at(level, x, y)
	var decoded := Png16.load_gray16(pyramid.tile_path(level, t.x, t.y))
	if decoded.is_empty():
		return NAN
	var origin := ReliefPyramid.tile_origin(level, t.x, t.y)
	var px := int((x - origin.x) / ReliefPyramid.pixel_units(level))
	var py := int((y - origin.y) / ReliefPyramid.pixel_units(level))
	var data: PackedByteArray = decoded["data"]
	var o := (py * int(decoded["width"]) + px) * 2
	var v := (int(data[o]) << 8) | int(data[o + 1])
	return pyramid.height_min_m + float(v) / 65535.0 * pyramid.height_range_m


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
	# Page E2 réelle du Bosphore.
	var strait := _height_at(pyramid, 2, 5199.0, 4166.1)
	var hills := _height_at(pyramid, 2, 5207.2, 4170.4)
	if is_nan(strait) or is_nan(hills):
		_failures += 1
		push_error("omr_r7_east_relief_test: page E2 du Bosphore illisible")
		return
	print("omr_r7_east_relief_test: Bosphore %.1f m, collines d'Üsküdar %.1f m" % [strait, hills])
	_check(strait <= 0.0, "détroit à %.1f m" % strait)
	_check(hills > 40.0, "collines d'Üsküdar à %.1f m" % hills)
