extends SceneTree

## Test headless du lot ZG4 (caméra rapprochée, exagération verticale dynamique, ADR 0036), sans
## dépendre du cache réel de la pyramide :
##  1. `CloseCameraProfile` : courbe d'exagération (×4,31 en vue stratégique, ×1,5 au plus près,
##     monotone), échelle quantifiée par paliers géométriques avec hystérésis ;
##  2. distance minimale par étage (table, champ adouci continu autour d'une zone fine) ;
##  3. `CampaignCamera` : distance minimale selon l'étage sous le point visé, point visé posé au
##     sol, caméra jamais sous le relief, tangage rasant de près, plans `near` / `far` ;
##  4. `ZoomTiers` étendus (vallée, site) ;
##  5. `TerrainBuilder.set_vertical_scale` sur une pyramide factice : paramètre global, surface
##     proportionnelle, recalages étalés puis terminés, `rescaling_vertical` pendant l'émission ;
##     repli sans pyramide : échelle fixe.
## Usage : godot --headless --path game --script res://tests/zg4_camera_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TEST_DIR := "user://zg4_test"

var _failures := 0


func _init() -> void:
	await process_frame
	_test_profile()
	_test_min_distance()
	await _test_camera()
	_test_tiers()
	await _test_rescale()
	print("zg4_camera_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("zg4_camera_test: " + message)
	return condition


func _test_profile() -> void:
	var profile := CloseCameraProfile.load_default()
	_check(absf(profile.exaggeration_at(1500.0) - profile.exaggeration_far) < 1e-4, "far exaggeration")
	_check(absf(profile.exaggeration_at(0.3) - profile.exaggeration_near) < 1e-4, "near exaggeration")
	_check(absf(profile.vertical_scale_at(200.0) - MapData.HEIGHT_SCALE) < 1e-9, "strategic scale = HEIGHT_SCALE")
	var previous := 0.0
	var curve := []
	for k in 60:
		var d := 0.2 * pow(1.15, k)
		var e := profile.exaggeration_at(d)
		_check(e >= previous - 1e-6, "exaggeration must grow with distance (%f at %f)" % [e, d])
		previous = e
		if k % 8 == 0:
			curve.append([snappedf(d, 0.01), snappedf(e, 0.01)])
	print("zg4_camera_test: exaggeration curve %s" % JSON.stringify(curve))
	# Paliers : rapport constant, hystérésis (pas d'aller-retour autour d'un seuil).
	var steps := {}
	var current := -1.0
	for k in 400:
		var d := 0.25 * pow(1.02, k)
		current = profile.quantized_scale(d, current)
		steps[snappedf(current, 1e-7)] = true
	var values: Array = steps.keys()
	values.sort()
	_check(values.size() >= 20 and values.size() <= 40, "expected 20-40 quantized steps, got %d" % values.size())
	for i in range(1, values.size()):
		var ratio: float = values[i] / values[i - 1]
		_check(absf(ratio - 1.04) < 0.002, "step ratio %f" % ratio)
	_check(absf(float(values[values.size() - 1]) - MapData.HEIGHT_SCALE) < 1e-9, "top step = HEIGHT_SCALE")
	var s := profile.quantized_scale(3.0)
	var flips := 0
	for k in 50:
		var d := 3.0 * (1.0 + 0.004 * sin(k))
		var next := profile.quantized_scale(d, s)
		if not is_equal_approx(next, s):
			flips += 1
		s = next
	_check(flips == 0, "hysteresis should prevent flicker, %d flips" % flips)


func _test_min_distance() -> void:
	var profile := CloseCameraProfile.load_default()
	_check(is_equal_approx(profile.min_distance_for_level(-1), 22.0), "E0 / sea min distance")
	_check(is_equal_approx(profile.min_distance_for_level(2), 5.0), "E2 min distance")
	_check(is_equal_approx(profile.min_distance_for_level(4), 1.5), "E4 min distance")
	_check(is_equal_approx(profile.min_distance_for_level(7), 0.3), "E7 min distance")
	# Zone E7 carrée de 10 unités dans une région E2.
	var level_at := FakeRelief.new(Vector2(100.0, 100.0))
	_check(is_equal_approx(profile.soft_min_distance(Vector2(100.0, 100.0), level_at), 0.3), "inside the E7 zone")
	_check(is_equal_approx(profile.soft_min_distance(Vector2(200.0, 100.0), level_at), 5.0), "far from the zone")
	var last := profile.soft_min_distance(Vector2(100.0, 100.0), level_at)
	var worst_jump := 0.0
	for k in 200:
		var x := 100.0 + k * 0.1
		var value := profile.soft_min_distance(Vector2(x, 100.0), level_at)
		worst_jump = maxf(worst_jump, value - last)
		last = value
	# Pas de 0,1 unité : saut ≤ pente × écart entre anneaux (échantillonnage discret).
	_check(worst_jump < 0.5, "soft min distance must rise gradually, jump %f" % worst_jump)
	print("zg4_camera_test: soft min at +1/+3/+8 units from the zone edge: %.2f %.2f %.2f" % [
		profile.soft_min_distance(Vector2(106.0, 100.0), level_at),
		profile.soft_min_distance(Vector2(108.0, 100.0), level_at),
		profile.soft_min_distance(Vector2(113.0, 100.0), level_at)])


## Pyramide factice : E2 partout, E3-E7 sur un carré de 10 unités centré sur `center`.
class FakeRelief:
	extends RefCounted

	var max_level := 7
	var center := Vector2.ZERO

	func _init(zone_center: Vector2) -> void:
		center = zone_center

	func finest_level_at(x: float, y: float) -> int:
		return 7 if absf(x - center.x) < 5.0 and absf(y - center.y) < 5.0 else 2

	func has_tile(level: int, col: int, row: int) -> bool:
		if level <= 2:
			return true
		var origin := ReliefPyramid.tile_origin(level, col, row)
		var t := ReliefPyramid.tile_units(level)
		var tile := Rect2(origin, Vector2(t, t))
		return Rect2(center - Vector2(5.0, 5.0), Vector2(10.0, 10.0)).intersects(tile)


func _test_camera() -> void:
	var rig := CampaignCamera.new()
	var camera := Camera3D.new()
	camera.name = "Camera3D"
	rig.add_child(camera)
	root.add_child(rig)
	rig.edge_pan_enabled = false
	rig.setup(Rect2(0, 0, 4096, 4096), 500.0)
	# Sans pyramide : comportement historique.
	_check(is_equal_approx(rig.min_distance_at(Vector3(100.0, 0.0, 100.0)), 22.0), "legacy min distance")
	rig.relief = FakeRelief.new(Vector2(1000.0, 1000.0))
	# Colline : sol à 1 unité, pente vers l'est.
	rig.ground_height = func(x: float, _y: float) -> float:
		return 1.0 + maxf(x - 1000.0, 0.0) * 0.5
	_check(is_equal_approx(rig.min_distance_at(Vector3(1000.0, 0.0, 1000.0)), 0.3), "E7 zone min distance")
	_check(is_equal_approx(rig.min_distance_at(Vector3(3000.0, 0.0, 1000.0)), 5.0), "E2 min distance")
	# ZG4b : plancher provisoire au-dessus d'une ville emblématique (cercle de 3 unités au centre de
	# la zone E7), adouci au-dehors, désactivable (VH4).
	var floor_m := rig.profile.landmark_min_distance
	_check(floor_m >= 2.2, "landmark floor above the site tier (%f)" % floor_m)
	rig.close_zones = PackedVector3Array([Vector3(1000.0, 1000.0, 3.0)])
	_check(is_equal_approx(rig.min_distance_at(Vector3(1000.0, 0.0, 1000.0)), floor_m), "landmark floor inside the zone")
	var outside := rig.min_distance_at(Vector3(1005.0, 0.0, 1000.0))
	_check(outside < floor_m and outside > 0.3, "landmark floor softened outside the zone (%f)" % outside)
	_check(is_equal_approx(rig.min_distance_at(Vector3(1000.0, 0.0, 1030.0)), rig.profile.soft_min_distance(Vector2(1000.0, 1030.0), rig.relief)), "no landmark floor far from the zone")
	rig.look_at_point(Vector3(1000.0, 0.0, 1000.0), 0.1)
	_check(is_equal_approx(rig.target_distance, floor_m), "zoom clamped to the landmark floor, got %f" % rig.target_distance)
	rig.profile = rig.profile.duplicate()
	rig.profile.landmark_min_distance = 0.0
	rig._soft_min_key = Vector3(INF, INF, INF)
	_check(is_equal_approx(rig.min_distance_at(Vector3(1000.0, 0.0, 1000.0)), 0.3), "landmark floor disabled (VH4)")
	rig.profile = CloseCameraProfile.load_default()
	rig.close_zones = PackedVector3Array()
	rig.look_at_point(Vector3(1000.0, 0.0, 1000.0), 0.1)
	rig.snap()
	_check(is_equal_approx(rig.target_distance, 0.3), "zoom clamped to the E7 min distance, got %f" % rig.target_distance)
	_check(is_equal_approx(rig.focus.y, 1.0), "focus should rest on the ground, y = %f" % rig.focus.y)
	_check(rig.pitch_deg() < 15.0, "grazing pitch near the ground, got %f" % rig.pitch_deg())
	_check(camera.near < 0.02 and camera.near > 0.0, "near plane follows distance, got %f" % camera.near)
	# Caméra tournée vers la pente montante : jamais sous le sol.
	rig.target_yaw = PI * 0.5
	rig.snap()
	var eye := camera.global_position
	_check(eye.y >= float(rig.ground_height.call(eye.x, eye.z)) + 0.003, "camera above ground: %f vs %f" % [eye.y, float(rig.ground_height.call(eye.x, eye.z))])
	# Crête entre la caméra (au sud) et le point visé : la visée passe au-dessus.
	rig.ground_height = func(_x: float, z: float) -> float:
		return 1.2 if z > 1000.12 and z < 1000.2 else 1.0
	rig.target_yaw = 0.0
	rig.snap()
	eye = camera.global_position
	var ridge_t := (1000.16 - rig.focus.z) / (eye.z - rig.focus.z)
	var sight_y := rig.focus.y + ridge_t * (eye.y - rig.focus.y)
	_check(ridge_t > 0.0 and ridge_t < 1.0 and sight_y > 1.2, "line of sight should clear the ridge (t %f, y %f)" % [ridge_t, sight_y])
	rig.ground_height = func(x: float, _y: float) -> float:
		return 1.0 + maxf(x - 1000.0, 0.0) * 0.5
	# En sortant de la zone, la distance remonte sans saut brutal.
	var previous := rig.target_distance
	var worst := 0.0
	for k in 60:
		rig.target_focus = Vector3(1004.0 + k * 0.1, 0.0, 1000.0)
		rig._process(1.0 / 60.0)
		worst = maxf(worst, rig.target_distance - previous)
		previous = rig.target_distance
	_check(worst < 0.3, "min distance should not push the camera abruptly (%f)" % worst)
	_check(rig.target_distance > 0.3, "leaving the zone raises the camera")
	# Vue stratégique : tangage historique, plans lointains.
	rig.relief = null
	rig.ground_height = Callable()
	rig.look_at_point(Vector3(2048.0, 0.0, 2048.0), 900.0)
	rig.snap()
	_check(rig.pitch_deg() > 30.0 and camera.far >= 4000.0, "strategic pitch and far plane")
	rig.queue_free()
	await process_frame


func _test_tiers() -> void:
	var tiers := ZoomTiers.load_default()
	_check(tiers.tier_at(900.0) == ZoomTiers.Tier.FAR, "far tier")
	_check(tiers.tier_at(60.0) == ZoomTiers.Tier.NEAR, "county tier")
	_check(tiers.tier_at(3.0) == ZoomTiers.Tier.VALLEY, "valley tier")
	_check(tiers.tier_at(0.5) == ZoomTiers.Tier.SITE, "site tier")
	_check(tiers.near_weight(3.0) > 0.99, "near weight stays 1 in the valley view")
	_check(tiers.valley_weight(0.5) > 0.99 and tiers.valley_weight(60.0) < 0.01, "valley weight")
	_check(tiers.site_weight(0.4) > 0.99 and tiers.site_weight(5.0) < 0.01, "site weight")


## Pyramide factice : tuiles E1 au-dessus de Paris (contenu de la tuile E0 versionnée).
func _write_fixture(map_dir: String) -> String:
	var dir := ProjectSettings.globalize_path(TEST_DIR)
	DirAccess.make_dir_recursive_absolute(dir.path_join("pyramid/E1"))
	var source := map_dir.path_join("height/h_8_7.png")
	for row in range(14, 16):
		for col in range(16, 18):
			DirAccess.copy_absolute(source, dir.path_join("pyramid/E1/%d_%d.png" % [col, row]))
	var manifest := {
		"version": 1, "tile_px": 512, "root_tile_units": 256, "height_min_m": -200.0,
		"height_range_m": 5000.0, "dir": "pyramid", "pattern": "E{level}/{col}_{row}.png",
		"levels": [{"level": 1, "meters_per_px": 179.744, "tier": 1, "source": "test", "tiles_rle": [
			{"row": 14, "runs": [[16, 2]]}, {"row": 15, "runs": [[16, 2]]}]}],
	}
	var path := dir.path_join("relief_pyramid.json")
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(JSON.stringify(manifest))
	file.close()
	return path


func _test_rescale() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	terrain.pyramid_manifest_path = _write_fixture(map_dir)
	world.add_child(terrain)
	terrain.build(map_data)
	if not _check(terrain.quadtree != null, "quadtree should be active with the fixture pyramid"):
		return
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.current = true
	var paris := Vector2(2213.2, 1923.9)
	var focus := Vector3(paris.x, map_data.surface_world_at(paris.x, paris.y), paris.y)
	camera.look_at_from_position(focus + Vector3(0.0, 8.0, 8.0), focus, Vector3.UP)
	for _i in 30:
		terrain.update_lod(camera.global_position, 11.0, focus, 170.0)
		terrain.wait_fine_jobs()
		await process_frame
	var before := terrain.surface_height_at(paris.x, paris.y)
	var signals := [0, 0]
	var rescaling := [0]
	terrain.vertical_scale_changed.connect(func(_old: float, _new: float) -> void: signals[0] += 1)
	terrain.chunk_surface_changed.connect(func(_index: int) -> void:
		signals[1] += 1
		if terrain.rescaling_vertical:
			rescaling[0] += 1)
	var target := MapData.HEIGHT_SCALE * 1.5 / 4.31
	_check(terrain.set_vertical_scale(target), "scale change accepted")
	_check(signals[0] == 1, "vertical_scale_changed emitted once")
	var global_value: Variant = RenderingServer.global_shader_parameter_get("campaign_vertical_scale")
	if global_value != null:  # serveur factice (--headless) : pas de paramètres globaux lisibles
		_check(is_equal_approx(float(global_value), target), "global shader parameter updated")
	_check(absf(terrain.surface_height_at(paris.x, paris.y) - before * target / MapData.HEIGHT_SCALE) < 1e-4, "surface follows the scale")
	_check(terrain.pending_rescales() == 256, "all chunks queued, got %d" % terrain.pending_rescales())
	terrain.update_lod(camera.global_position, 11.0, focus, 170.0)
	_check(terrain.pending_rescales() == 256, "rescale waits for the scale to settle")
	terrain.rescale_settle_ms = 0
	terrain.rescale_budget_ms = 0.0  # un morceau par image au plus
	terrain.update_lod(camera.global_position, 11.0, focus, 170.0)
	_check(terrain.pending_rescales() == 255, "rescale spread over frames (%d left)" % terrain.pending_rescales())
	terrain.wait_fine_jobs()
	_check(terrain.pending_rescales() == 0, "flush completes the rescale")
	_check(rescaling[0] >= 256, "rescale emits flagged (%d)" % rescaling[0])
	_check(not terrain.set_vertical_scale(target), "same scale: no-op")
	terrain.reset_vertical_scale()
	_check(is_equal_approx(MapData.vertical_scale(), MapData.HEIGHT_SCALE), "reset to strategic scale")
	# Repli sans pyramide : échelle fixe.
	var fallback := TerrainBuilder.new()
	fallback.pyramid_enabled = false
	world.add_child(fallback)
	fallback.build(map_data)
	_check(not fallback.set_vertical_scale(target) and is_equal_approx(MapData.vertical_scale(), MapData.HEIGHT_SCALE), "no dynamic scale without pyramid")
	world.queue_free()
	await process_frame
