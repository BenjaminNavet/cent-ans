extends TestCase
## Relief fin à l'Est et au Sud (lot OMR R7, ADR 0121) : pyramide dans le cadre monde.
##
## 1. Manifeste réel : `root_origin_tiles` [0, 0], étages E1-E2 listés au Bosphore, à Moscou,
##    au Caire et à Tunis ; Rouen garde ses étages fins (E4) dans le cadre monde.
## 2. Si le cache est là : page E2 du Bosphore décodée, eau du détroit sous 0 m et collines
##    de Beykoz au-dessus de 150 m (Copernicus à 90 m, pas l'ETOPO lissé d'E0).
##
## godot --headless --path game --script res://tests/omr_r7_east_relief_test.gd


func _init() -> void:
	_run()
	finish()


## Hauteurs (m) extrêmes des 3 × 3 pixels autour d'un point monde, page PNG 16 bits de `level`
## (x = min, y = max).
func _range_at(pyramid: ReliefPyramid, level: int, x: float, y: float) -> Vector2:
	var t := ReliefPyramid.tile_at(level, x, y)
	var store: Object = ClassDB.instantiate("GameDataStore")
	var data: PackedByteArray = store.call("load_heightmap_u16", pyramid.tile_path(level, t.x, t.y))
	if data.is_empty():
		return Vector2(NAN, NAN)
	var decoded := {"width": ReliefPyramid.TILE_PX, "height": ReliefPyramid.TILE_PX}
	var origin := ReliefPyramid.tile_origin(level, t.x, t.y)
	var px := int((x - origin.x) / ReliefPyramid.pixel_units(level))
	var py := int((y - origin.y) / ReliefPyramid.pixel_units(level))
	var width := int(decoded["width"])
	var lo := INF
	var hi := -INF
	for dy in range(-1, 2):
		for dx in range(-1, 2):
			var cx := clampi(px + dx, 0, width - 1)
			var cy := clampi(py + dy, 0, int(decoded["height"]) - 1)
			var o := (cy * width + cx) * 2
			var h := pyramid.height_min_m + float(int(data[o]) | (int(data[o + 1]) << 8)) / 65535.0 * pyramid.height_range_m
			lo = minf(lo, h)
			hi = maxf(hi, h)
	return Vector2(lo, hi)


func _run() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	var composed := map_data.crs == MapProjection.CRS_COMPOSED
	# CC : la carte composée a sa propre pyramide (`pyramid/relief_pyramid.json`, E1-E3 seulement :
	# E3 sur l'Europe de l'Ouest, E1 à l'Est et au Sud), et ses lieux sont placés par (lon, lat).
	var manifest_path := map_dir.path_join("pyramid/relief_pyramid.json" if composed else "relief_pyramid.json")
	var manifest: Variant = JSON.parse_string(FileAccess.get_file_as_string(manifest_path))
	if not check(manifest is Dictionary, "manifeste illisible"):
		return
	check(str((manifest as Dictionary).get("root_origin_tiles", [])) == str([0.0, 0.0]), "root_origin_tiles " + str((manifest as Dictionary).get("root_origin_tiles")))
	var pyramid := ReliefPyramid.new()
	# Tuiles listées (sans le contrôle de présence sur le disque) : `tiles_override` vide = manifeste.
	if not (pyramid.load_manifest(map_dir, manifest_path) if composed else pyramid.load_manifest(map_dir)):
		print("omr_r7_east_relief_test: pas de cache de relief, contrôles réels sautés")
		return
	# Points en (lon, lat) réels, projetés dans le CRS de la carte chargée.
	var projection := MapProjection.for_map(map_data)
	var places := {
		"Bosphore": projection.lonlat_to_px(29.06, 41.10),
		"Moscou": projection.lonlat_to_px(37.6, 55.75),
		"Le Caire": projection.lonlat_to_px(31.24, 30.04),
		"Tunis": projection.lonlat_to_px(10.18, 36.80),
	}
	# Carte réelle : E2 à l'Est et au Sud ; carte composée : E1 (pas de tuile plus fine hors Occident).
	var east_level := 1 if composed else 2
	for name in places:
		var p: Vector2 = places[name]
		var level := pyramid.finest_level_at(p.x, p.y)
		check(level >= east_level, "%s : étage le plus fin %d (E%d attendu)" % [name, level, east_level])
	var rouen_px := projection.lonlat_to_px(1.09, 49.44)
	var rouen := pyramid.finest_level_at(rouen_px.x, rouen_px.y)
	var rouen_min := 3 if composed else 4
	check(rouen >= rouen_min, "Rouen : étage le plus fin %d (E%d+ attendu)" % [rouen, rouen_min])
	if composed:
		# CC : pas de page E2 au Bosphore (E1 à 180 m/px ne résout ni le détroit ni Beykoz).
		return
	# Page E2 réelle du Bosphore, profil à 41,10° N : détroit (Rumeli Hisarı, ≈ 29,057° E)
	# et collines de Beykoz (≈ 29,12° E, ≈ 320 m).
	var strait := _range_at(pyramid, 2, 5201.9, 4160.6)
	var hills := _range_at(pyramid, 2, 5209.2, 4158.7)
	if is_nan(strait.x) or is_nan(hills.y):
		failures += 1
		push_error("omr_r7_east_relief_test: page E2 du Bosphore illisible")
		return
	print("omr_r7_east_relief_test: Bosphore %.1f m, collines de Beykoz %.1f m" % [strait.x, hills.y])
	check(strait.x <= 0.0, "détroit à %.1f m" % strait.x)
	check(hills.y > 150.0, "collines de Beykoz à %.1f m" % hills.y)
