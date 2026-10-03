extends SceneTree

## Planche de contrôle du lot RV-F (vie visible en vue régionale) : quatre vues de la carte de
## campagne en UNE planche JPEG 2 × 2 (640 × 400 par vue). Fenêtre réelle (pas headless) :
##   godot --path game --resolution 1280x800 --script res://tests/rv_life_shot.gd -- --out=<dossier>
##   --hide-armies --map-weather=clear [--season=winter] [--name=rv_board] [--views=x,z,d;…]
##   [--bench] (avec `--disable-vsync` avant `--`) [--off] (effets RV-F coupés : A/B visuel)
## Une ligne `RV view` par vue (panaches, effets de vie) ; `--bench` : temps d'image moyen avec puis
## sans les effets RV-F (panaches régionaux, paillettes, vent des forêts), sans planche.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW_SIZE := Vector2i(640, 400)
const BENCH_FRAMES := 240


func _init() -> void:
	var out_dir := "user://rv_shots"
	var board_name := "rv_board"
	var bench := false
	var off := false
	## [x, z, distance du rig] : côte normande (régional), Caen (moyen), Gironde et Landes, Paris.
	var views: Array = [[2050.0, 3060.0, 600.0], [1944.0, 3110.0, 300.0], [1831.0, 3782.0, 420.0], [2213.0, 3204.0, 220.0]]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--name="):
			board_name = arg.trim_prefix("--name=")
		elif arg == "--bench":
			bench = true
		elif arg == "--off":
			off = true
		elif arg.begins_with("--views="):
			views.clear()
			for item in arg.trim_prefix("--views=").split(";", false):
				var parts := item.split(",")
				if parts.size() >= 3:
					views.append([float(parts[0]), float(parts[1]), float(parts[2])])
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("RV shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.get("map_data")
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var life: CampaignLife = map.get("life")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	if off:
		_set_effects(map, false)
	var board := Image.create(VIEW_SIZE.x * 2, VIEW_SIZE.y * 2, false, Image.FORMAT_RGB8)
	board.fill(Color.BLACK)
	for v in mini(views.size(), 4):
		var view: Array = views[v]
		var focus := Vector3(view[0], data.surface_world_at(view[0], view[1]), view[1])
		rig.look_at_point(focus, maxf(float(view[2]), rig.min_distance_at(focus)))
		rig.snap()
		await _settle(terrain, settlements)
		var plumes := map.find_child("RegionalPlumes", true, false) as MultiMeshInstance3D
		print("RV view %d (%.0f, %.0f) rig %.0f stats %s plumes visible %s" % [v, view[0], view[1], rig.distance,
			JSON.stringify(life.effects.stats) if life != null and life.effects != null else "-", plumes.visible if plumes != null else "-"])
		if bench:
			Engine.max_fps = 0
			var with_fx := await _bench()
			_set_effects(map, false)
			for i in 20:
				await process_frame
			var without := await _bench()
			_set_effects(map, true)
			print("RV bench (%.0f, %.0f) rig %.0f : %.2f ms with RV-F, %.2f ms without, %+.2f ms" % [view[0], view[1], rig.distance,
				with_fx, without, with_fx - without])
			continue
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		image.convert(Image.FORMAT_RGB8)
		image.resize(VIEW_SIZE.x, VIEW_SIZE.y, Image.INTERPOLATE_LANCZOS)
		board.blit_rect(image, Rect2i(Vector2i.ZERO, VIEW_SIZE), Vector2i((v % 2) * VIEW_SIZE.x, (v / 2) * VIEW_SIZE.y))
	if not bench:
		var path := out_dir.path_join("%s.jpg" % board_name)
		board.save_jpg(path, 0.88)
		print("RV board %s" % path)
	map.queue_free()
	await process_frame
	quit(0)


## Effets RV-F allumés ou coupés (A/B) : panaches régionaux, paillettes de la mer, vent des forêts.
func _set_effects(map: Node3D, on: bool) -> void:
	var life: CampaignLife = map.get("life")
	if life != null and life.effects != null:
		life.effects.regional_plumes_enabled = on
	var sea := map.get("sea") as GeometryInstance3D
	if sea != null and sea.material_override is ShaderMaterial:
		(sea.material_override as ShaderMaterial).set_shader_parameter("glint_amount", 0.55 if on else 0.0)
	var terrain: TerrainBuilder = map.get("terrain")
	if terrain != null and terrain.material != null:
		terrain.material.set_shader_parameter("forest_wind_amount", 0.11 if on else 0.0)


func _bench() -> float:
	var t0 := Time.get_ticks_usec()
	for i in BENCH_FRAMES:
		await process_frame
	return (Time.get_ticks_usec() - t0) / (BENCH_FRAMES * 1000.0)


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer) -> void:
	for i in 30:
		await process_frame
	var guard := 0
	while guard < 2000:
		guard += 1
		var busy := false
		if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
			busy = true
		if ReliefLandcover.pending():
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 60:
		await process_frame
