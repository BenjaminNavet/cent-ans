extends SceneTree

## Lot ZG8 (ADR 0036) : relief exagéré (hauteur affichée = s·(h + g·max(h − fond, 0))).
## godot --headless --path game --script res://tests/zg8_relief_test.gd
##  1. Formule sur un fond synthétique : plaine (h = fond) inchangée, sommet rehaussé, côte et mer
##     inchangées, monotone, inverse exacte, gain nul = ZG4.
##  2. Double GPU : la texture publiée (`ReliefFloor.texture_of`) relue comme le shader
##     (texelFetch + bilinéaire manuel, cellule / décalage de `campaign_relief_floor_info`) donne le
##     même fond que `MapData.relief_floor_at` ; le .gdshaderinc porte la même formule ; aucun shader
##     ne pose une hauteur en mètres hors de `campaign_display_height`.
##  3. Carte réelle : fond ≥ 0, plaine de Paris quasi inchangée, sommets pyrénéens rehaussés, gain
##     fonction de l'échelle (lointain → proche), profil désactivé = ZG4.
##  4. Objets posés : la surface du terrain (E0 et quadtree) vaut la hauteur affichée ; l'aller-retour
##     mètres → affiché → mètres des maquettes (`height_from_display`) retombe sur le sol affiché.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	_test_formula()
	_test_gpu_twin()
	_test_real_map()
	await _test_grounding()
	MapData.set_relief_floor({})
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	if _failures == 0:
		print("zg8_relief_test: OK")
	else:
		print("zg8_relief_test: %d failure(s)" % _failures)
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("zg8_relief_test: " + message)
	return condition


## Fond synthétique 4 × 4 cellules de 8 px : 100 m partout sauf une cuvette à 0 m au coin.
func _synthetic_floor() -> Dictionary:
	var data := PackedFloat32Array()
	for j in 4:
		for i in 4:
			data.append(0.0 if i == 0 and j == 0 else 100.0)
	return {"data": data, "side": Vector2i(4, 4), "cell": 8.0}


func _test_formula() -> void:
	MapData.set_relief_floor(_synthetic_floor())
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	var s := MapData.vertical_scale()
	var g := MapData.relief_gain()
	# HB6 (ADR 0143) : gain lointain négatif, puis RV-A (ADR 0167) : 0 (volume des collines rendu
	# au maillage) ; il doit seulement être celui du profil, et laisser 1 + g > 0.
	_check(absf(g - ReliefExaggerationProfile.load_default().gain_far) < 1e-6 and 1.0 + g > 0.0, "gain = gain_far with a published floor (%f)" % g)
	# Centre de la cellule (2, 2) : x = 2·8 + 3,5.
	var x := 19.5
	var z := 19.5
	_check(absf(MapData.relief_floor_at(x, z) - 100.0) < 1e-4, "floor at a cell centre")
	_check(absf(MapData.display_height(100.0, x, z) - 100.0 * s) < 1e-6, "plain (h = floor) unchanged")
	_check(absf(MapData.display_height(60.0, x, z) - 60.0 * s) < 1e-6, "below the floor: unchanged")
	var peak := MapData.display_height(1100.0, x, z)
	_check(absf(peak - s * (1100.0 + g * 1000.0)) < 1e-5, "peak raised by the local gain")
	_check(signf(peak - 1100.0 * s) == signf(g), "peak moved by the sign of the gain vs ZG4")
	_check(MapData.display_height(0.0, 3.5, 3.5) == 0.0, "coast stays at sea level")
	_check(absf(MapData.display_height(-50.0, 3.5, 3.5) + 50.0 * s) < 1e-6, "sea floor unchanged")
	_check(MapData.display_height(0.0, x, z) == 0.0, "coast stays at sea level under a raised floor")
	var previous := -INF
	for k in 60:
		var h := -100.0 + k * 40.0
		var y := MapData.display_height(h, x + 3.0, z - 5.0)
		_check(y >= previous, "display height must grow with altitude (%f)" % h)
		previous = y
		_check(absf(MapData.height_from_display(y, x + 3.0, z - 5.0) - h) < 1e-3, "inverse at %f" % h)
	# Gain en fonction de l'échelle : lointain → proche, monotone.
	var profile := ReliefExaggerationProfile.load_default()
	var near := MapData.near_vertical_scale()
	_check(absf(MapData.relief_gain_for_scale(MapData.HEIGHT_SCALE) - profile.gain_far) < 1e-6, "far gain")
	_check(absf(MapData.relief_gain_for_scale(near) - profile.gain_near) < 1e-6, "near gain")
	_check(absf(near / MapData.HEIGHT_SCALE * 4.31 - profile.near_exaggeration) < 0.02, "near floor = near_exaggeration")
	MapData.set_vertical_scale(near)
	_check(absf(MapData.relief_gain() - profile.gain_near) < 1e-6, "gain published with the scale")
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	# Interrupteur : profil désactivé = ZG4 (gain nul, plancher ×1,5).
	var off := profile.duplicate() as ReliefExaggerationProfile
	off.enabled = false
	ReliefExaggerationProfile.set_default(off)
	_check(MapData.relief_gain_for_scale(MapData.HEIGHT_SCALE) == 0.0, "disabled profile: no gain")
	var camera := CloseCameraProfile.load_default()
	_check(absf(camera.near_exaggeration() - camera.exaggeration_near) < 1e-6, "disabled profile: ZG4 near floor")
	ReliefExaggerationProfile.set_default(null)
	# Sans fond publié : gain nul.
	MapData.set_relief_floor({})
	_check(MapData.relief_gain() == 0.0 and absf(MapData.display_height(1000.0, x, z) - 1000.0 * s) < 1e-6, "no floor: ZG4")


## Relecture de la texture comme le fait `campaign_floor_m_grad` (campaign_relief.gdshaderinc).
func _shader_floor(image: Image, info: Vector4, x: float, z: float) -> float:
	var sz := Vector2i(image.get_width(), image.get_height())
	var cell := maxf(info.x, 1.0)
	var f := Vector2(clampf((x - info.y) / cell, 0.0, sz.x - 1.0), clampf((z - info.y) / cell, 0.0, sz.y - 1.0))
	var i := Vector2i(mini(int(f.x), sz.x - 2), mini(int(f.y), sz.y - 2))
	var t := f - Vector2(i)
	var a := image.get_pixel(i.x, i.y).r
	var b := image.get_pixel(i.x + 1, i.y).r
	var c := image.get_pixel(i.x, i.y + 1).r
	var d := image.get_pixel(i.x + 1, i.y + 1).r
	return lerpf(lerpf(a, b, t.x), lerpf(c, d, t.x), t.y)


func _test_gpu_twin() -> void:
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var grid := ReliefFloor.compute(map_data, ReliefExaggerationProfile.load_default())
	print("zg8_relief_test: floor %s cells of %d px in %.1f ms" % [grid["side"], int(grid["cell"]), float(grid["ms"])])
	MapData.set_relief_floor(grid)
	var image := ReliefFloor.texture_of(grid).get_image()
	var cell := float(grid["cell"])
	var info := Vector4(cell, 0.5 * (cell - 1.0), 0.0, 0.0)
	var worst := 0.0
	var rng := RandomNumberGenerator.new()
	rng.seed = 8
	for k in 400:
		var x := rng.randf_range(-10.0, 4100.0)
		var z := rng.randf_range(-10.0, 4100.0)
		worst = maxf(worst, absf(_shader_floor(image, info, x, z) - MapData.relief_floor_at(x, z)))
	_check(worst < 1e-3, "GPU floor sampling differs from MapData.relief_floor_at by %f m" % worst)
	# Formule du .gdshaderinc identique au double GDScript.
	var inc := FileAccess.get_file_as_string("res://shaders/campaign_relief.gdshaderinc")
	# SZ1 : formule complète (écrasement des montagnes) vérifiée par `sz1_mountain_test.gd`.
	_check(inc.contains("+ campaign_relief_gain * (1.0 - k) * max(h_m - fields.x, 0.0));"), "shader formula changed: update MapData.display_height too")
	_check(inc.contains("vec2(campaign_relief_floor_info.y)) / cell"), "shader floor sampling changed: update MapData.relief_floor_at too")
	# Aucun shader de la carte ne pose des mètres sans la fonction commune.
	for file in DirAccess.get_files_at("res://shaders"):
		if not (file.ends_with(".gdshader") or file.ends_with(".gdshaderinc")) or file == "campaign_relief.gdshaderinc":
			continue
		var code := FileAccess.get_file_as_string("res://shaders".path_join(file))
		_check(not code.contains("global uniform float campaign_vertical_scale"), "%s redeclares campaign_vertical_scale (include campaign_relief.gdshaderinc)" % file)
		_check(not code.contains("* campaign_vertical_scale;") or file == "town_building.gdshader", "%s scales meters without campaign_display_height" % file)


func _test_real_map() -> void:
	if not MapData.has_relief_floor():
		return
	MapData.set_vertical_scale(MapData.HEIGHT_SCALE)
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	var lowest := INF
	for k in 2000:
		var x := float((k * 7919) % 4096)
		var z := float((k * 104729) % 4096)
		lowest = minf(lowest, MapData.relief_floor_at(x, z))
	_check(lowest >= 0.0, "floor must never be below sea level (%f)" % lowest)
	# Plaine de Paris : relief local faible, hauteur affichée presque celle de ZG4.
	var s := MapData.vertical_scale()
	var paris := Vector2(2214.0, 3204.0)
	var h_paris := map_data.height_m_at(paris.x, paris.y)
	var lift_paris := MapData.display_height(h_paris, paris.x, paris.y) / s - h_paris
	print("zg8_relief_test: Paris h=%.0f m floor=%.0f m lift=%.1f m" % [h_paris, MapData.relief_floor_at(paris.x, paris.y), lift_paris])
	_check(lift_paris < 25.0, "Paris plain should stay flat (lift %.1f m)" % lift_paris)
	# Pyrénées (Gavarnie) : le point le plus haut alentour est nettement rehaussé.
	var best := Vector2.ZERO
	var best_h := -INF
	for j in 41:
		for i in 41:
			var p := Vector2(1835.5 + i, 4094.5 + j)  # centres des pixels (SZ2b, ADR 0086) ; +1280 en y (OM)
			var h := map_data.height_m_at(p.x, p.y)
			if h > best_h:
				best_h = h
				best = p
	var lift_peak := MapData.display_height(best_h, best.x, best.y) / s - best_h
	print("zg8_relief_test: Pyrenees peak h=%.0f m floor=%.0f m lift=%.0f m (gain %.2f)" % [best_h, MapData.relief_floor_at(best.x, best.y), lift_peak, MapData.relief_gain()])
	# ZG7c : relief local plafonné (`local_relief_cap_m`) : rehaussé, mais sans doubler la montagne.
	var cap := ReliefExaggerationProfile.load_default().local_relief_cap_m
	# HB6 (ADR 0143) : gain lointain abaissé (0,42 → 0,12) et montagnes écrasées dès la vue
	# stratégique (`mountain_squash_far`) : le sommet peut être abaissé, jamais écrasé (≥ 50 %).
	_check(best_h > 2000.0 and best_h + lift_peak > 0.5 * best_h, "Pyrenean peak crushed (%.0f m)" % lift_peak)
	_check(cap <= 0.0 or best_h - MapData.relief_floor_at(best.x, best.y) < 3.0 * cap, "local relief of the peak should be capped (%.0f m above the floor)" % (best_h - MapData.relief_floor_at(best.x, best.y)))
	_check(map_data.height_world_at(best.x, best.y) == MapData.display_height(map_data.height_m_at(best.x, best.y), best.x, best.y), "height_world_at = display height")


func _test_grounding() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	var world := Node3D.new()
	root.add_child(world)
	# Repli E0 (sans pyramide) : sommets cuits à la hauteur affichée (gain lointain).
	var terrain := TerrainBuilder.new()
	terrain.pyramid_enabled = false
	world.add_child(terrain)
	terrain.build(map_data)
	_check(MapData.has_relief_floor(), "terrain build publishes the valley floor")
	var index := terrain.chunk_index_at(1855.0, 4106.0)
	var grid := terrain.surface_grid(index)
	var heights: PackedFloat32Array = grid["heights"]
	var side: int = grid["side"]
	var unit: float = grid["unit"]
	var origin := Vector2((index % terrain.chunks_x) * terrain.chunk_px, (index / terrain.chunks_x) * terrain.chunk_px)
	var worst := 0.0
	for k in [0, side + 1, side * 5 + 7, side * side / 2 + 3]:
		var p := origin + Vector2((k % side) * unit, (k / side) * unit)
		# Hauteur cuite (m) retrouvée par l'inverse, reposée : même sommet.
		var y := heights[k]
		var h_m := MapData.height_from_display(y, p.x, p.y)
		worst = maxf(worst, absf(MapData.display_height(h_m, p.x, p.y) - y))
		_check(absf(terrain.surface_height_at(p.x, p.y) - maxf(y, 0.0)) < 1e-4, "object on a vertex = mesh height")
	_check(worst < 1e-4, "baked E0 vertices round-trip through the display height (%f)" % worst)
	# Maquette (LandmarkModel) : mètres = inverse de la surface, reposés par le shader à la même hauteur.
	for p in [Vector2(2214.0, 3204.0), Vector2(1860.0, 4110.0), Vector2(2652.0, 3684.0)]:
		var ground := terrain.surface_height_at(p.x, p.y)
		var meters := MapData.height_from_display(ground, p.x, p.y)
		_check(absf(MapData.display_height(meters, p.x, p.y) - ground) < 1e-4, "landmark base at %s lands on the displayed ground" % p)
	world.queue_free()
	await process_frame
