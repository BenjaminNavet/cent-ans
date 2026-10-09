extends TestCase

## Lot SZ1 (défaut S1 de ZG7c) : écrasement des montagnes dans la hauteur affichée
##     y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0)), K = c(s)·k.
## godot --headless --path game --script res://tests/sz1_mountain_test.gd
##  1. Formule sur des champs synthétiques : sous la base inchangée, écrasement proportionnel à
##     c·k, monotone, inverse exacte ; c nul en vue stratégique, 1 au palier vallée ; k nul sous le
##     genou.
##  2. Double GPU : la texture RGBF relue comme le shader rend les trois champs de
##     `MapData.relief_fields_at` ; le .gdshaderinc porte la même expression.
##  3. Carte réelle au palier vallée (d = 6) : amplitude affichée des Pyrénées, des Alpes, du pays
##     de Galles et des coteaux de Rouen (ville 1:1 VH4) nettement réduite ; collines (Crécy),
##     falaises normandes, Loire et plaine de Paris inchangées (écart ≤ 2 % de l'amplitude affichée).


## [nom, point carte, écrasé ?]
const PLACES := [
	["pyrenees", Vector2(1862.0, 4098.0), true],
	["alpes", Vector2(2652.0, 3684.0), true],
	["galles", Vector2(1694.0, 2468.0), true],
	["massif_central", Vector2(2233.0, 3685.0), false],
	["crecy", Vector2(2191.5, 2986.0), false],
	["falaises_normandes", Vector2(2018.0, 3052.0), false],
	["paris", Vector2(2213.2, 3203.9), true],  # ville 1:1 (VH5) : relief à l'échelle vraie
	["rouen_seine", Vector2(2097.0, 3099.4), true],  # ville 1:1 (VH4) : coteaux à l'échelle vraie
	["val_de_loire", Vector2(2052.0, 3407.3), false],
]


func _init() -> void:
	_test_profile()
	_test_formula()
	_test_real_map()
	MapData.set_relief_floor({})
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	if failures == 0:
		print("sz1_mountain_test: OK")
	finish()


func _test_profile() -> void:
	var profile := ReliefExaggerationProfile.load_default()
	check(profile.mountain_knee_m > 0.0, "mountain squash enabled by default")
	check(profile.mountain_squash_of(profile.mountain_knee_m * 0.9) == 0.0, "no squash under the knee")
	var k2 := profile.mountain_squash_of(2000.0)
	var shown := 2000.0 * (1.0 - k2)
	check(k2 > 0.0 and shown < 2000.0 and shown >= profile.mountain_knee_m, "2000 m squashed above the knee (%f)" % shown)
	check(profile.mountain_squash_of(1e6) <= profile.mountain_squash_max, "squash bounded")


## Champs synthétiques 4 × 4 cellules de 8 px : fond 1 000 m, base 400 m, k 0,5.
func _synthetic() -> Dictionary:
	var data := PackedFloat32Array()
	var base := PackedFloat32Array()
	var squash := PackedFloat32Array()
	for i in 16:
		data.append(1000.0)
		base.append(400.0)
		squash.append(0.5)
	return {"data": data, "base": base, "squash": squash, "side": Vector2i(4, 4), "cell": 8.0}


func _test_formula() -> void:
	MapData.set_relief_floor(_synthetic())
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	var profile := ReliefExaggerationProfile.load_default()
	check(absf(MapData.relief_squash() - profile.mountain_squash_far) < 1e-6, "strategic squash weight")
	MapData.set_vertical_scale(MapData.squash_full_vertical_scale())
	check(absf(MapData.relief_squash() - 1.0) < 1e-6, "full squash at the valley tier (%f)" % MapData.relief_squash())
	MapData.set_vertical_scale(MapData.near_vertical_scale())
	check(absf(MapData.relief_squash() - 1.0) < 1e-6, "full squash at the closest zoom")
	var s := MapData.vertical_scale()
	var g := MapData.relief_gain()
	var x := 19.5
	var z := 19.5
	check(MapData.relief_fields_at(x, z).distance_to(Vector3(1000.0, 400.0, 0.5)) < 1e-4, "fields at a cell centre")
	check(absf(MapData.display_height(300.0, x, z) - 300.0 * s) < 1e-6, "below the base: unchanged")
	check(absf(MapData.display_height(800.0, x, z) - s * (800.0 - 0.5 * 400.0)) < 1e-6, "between base and floor: squashed")
	check(absf(MapData.display_height(2000.0, x, z) - s * (2000.0 - 0.5 * 1600.0 + g * 0.5 * 1000.0)) < 1e-6, "above the floor: squashed and raised")
	check(MapData.display_height(0.0, x, z) == 0.0, "coast stays at sea level")
	var previous := -INF
	for k in 80:
		var h := -100.0 + k * 50.0
		var y := MapData.display_height(h, x + 3.0, z - 5.0)
		check(y > previous, "display height must increase with altitude (%f)" % h)
		previous = y
		check(absf(MapData.height_from_display(y, x + 3.0, z - 5.0) - h) < 1e-3, "inverse at %f" % h)
	# GPU : texture RGBF relue comme `campaign_relief_fields_grad`.
	var grid := _synthetic()
	var image := ReliefFloor.texture_of(grid).get_image()
	check(image.get_format() == Image.FORMAT_RGBF, "RGBF relief texture")
	var c := image.get_pixel(2, 2)
	check(Vector3(c.r, c.g, c.b).distance_to(Vector3(1000.0, 400.0, 0.5)) < 1e-3, "texture channels = floor, base, squash")
	var inc := FileAccess.get_file_as_string("res://shaders/campaign_relief.gdshaderinc")
	check(inc.contains("return campaign_vertical_scale * (h_m - k * max(h_m - fields.y, 0.0)"), "shader formula changed: update MapData.display_height_fields and the Rust twin")
	check(inc.contains("+ campaign_relief_gain * (1.0 - k) * max(h_m - fields.x, 0.0));"), "shader gain term changed: update the twins")
	var map_data_code := FileAccess.get_file_as_string("res://scripts/map/map_data.gd")
	check(map_data_code.contains("return scale * (h_m - k * maxf(h_m - fields.y, 0.0) + gain * (1.0 - k) * maxf(h_m - fields.x, 0.0))"), "GDScript twin changed")
	var rust := FileAccess.get_file_as_string(ProjectSettings.globalize_path("res://").path_join("../core/crates/vegetation/src/lib.rs"))
	if check(rust != "", "Rust twin readable"):
		check(rust.contains("* (h_m - squash * (h_m - base).max(0.0)") and rust.contains("self.relief_gain * (1.0 - squash) * (h_m - floor).max(0.0)"), "Rust twin (vegetation) changed")


## Amplitude affichée (unités monde) autour d'un point : max − min de la hauteur affichée sur un
## disque de `radius` pixels.
func _displayed_span(map_data: MapData, point: Vector2, radius: float) -> Vector2:
	var lo := INF
	var hi := -INF
	var lo_m := INF
	var hi_m := -INF
	var steps := 24
	for j in range(-steps, steps + 1):
		for i in range(-steps, steps + 1):
			var p := point + Vector2(i, j) * (radius / steps)
			if p.distance_to(point) > radius:
				continue
			var h := map_data.height_m_at(p.x, p.y)
			var y := MapData.display_height(h, p.x, p.y)
			lo = minf(lo, y)
			hi = maxf(hi, y)
			lo_m = minf(lo_m, h)
			hi_m = maxf(hi_m, h)
	return Vector2(hi - lo, hi_m - lo_m)


func _test_real_map() -> void:
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var profile := ReliefExaggerationProfile.load_default()
	var grid := ReliefFloor.compute(map_data, profile)
	MapData.set_relief_floor(grid)
	# Palier vallée : échelle de la caméra à d = 6 (profil ZG4 courant).
	var camera := CloseCameraProfile.load_default()
	var valley_scale := camera.quantized_scale(6.0)
	MapData.set_vertical_scale(valley_scale)
	var with := {}
	for place in PLACES:
		var p: Vector2 = place[1]
		with[place[0]] = _displayed_span(map_data, p, 8.0)
	var squash_on := MapData.relief_squash()
	# Même échelle, écrasement coupé (profil dupliqué, genou et villes 1:1 nuls).
	var off := profile.duplicate() as ReliefExaggerationProfile
	off.mountain_knee_m = 0.0
	off.true_scale_squash = 0.0
	ReliefExaggerationProfile.set_default(off)
	MapData.set_relief_floor(ReliefFloor.compute(map_data, off))
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	MapData.set_vertical_scale(valley_scale)
	check(MapData.relief_squash() == 0.0, "knee 0 disables the squash")
	for place in PLACES:
		var p: Vector2 = place[1]
		var before := _displayed_span(map_data, p, 8.0)
		var after: Vector2 = with[place[0]]
		var fields := MapData.relief_fields_at(p.x, p.y)
		var ratio := after.x / maxf(before.x, 1e-6)
		print("sz1_mountain_test: %s true span %.0f m, displayed %.2f → %.2f units (×%.2f), squash c=%.2f k=%.2f, regional amplitude %.0f m" % [
			place[0], after.y, before.x, after.x, ratio, squash_on, _cell_at(grid, "squash", p), _cell_at(grid, "amplitude", p)])
		if place[2]:
			check(ratio < 0.8, "%s should be squashed (×%.2f)" % [place[0], ratio])
		elif place[0] != "massif_central":
			check(ratio > 0.98, "%s should keep its relief (×%.2f)" % [place[0], ratio])
		check(fields.y <= fields.x + 1e-3, "base ≤ floor at %s" % place[0])
	ReliefExaggerationProfile.set_default(null)


func _cell_at(grid: Dictionary, key: String, p: Vector2) -> float:
	var cell := float(grid["cell"])
	var side: Vector2i = grid["side"]
	var i := clampi(int(p.x / cell), 0, side.x - 1)
	var j := clampi(int(p.y / cell), 0, side.y - 1)
	return (grid[key] as PackedFloat32Array)[j * side.x + i]
