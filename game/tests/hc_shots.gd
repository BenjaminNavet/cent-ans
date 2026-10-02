extends SceneTree

## Planche de contrôle du lot HC1 (ADR 0161, arbres généralisés) : quatre vues de la carte de
## campagne assemblées en UNE planche JPEG 2 × 2 (1280 × 800 au plus, 640 × 400 par vue).
## Fenêtre réelle (pas headless), armées masquées, météo claire :
##   godot --path game --resolution 1280x800 --script res://tests/hc_shots.gd -- --out=<dossier>
##   --hide-armies --map-weather=clear [--name=hc_board] [--views=x,z,d;x,z,d;…]
##   [--tree-style=real|generalised] [--prop=<champ de MapPropScale>=<valeur>]…
##   [--bench] (avec `--disable-vsync` avant `--`)
## Vues par défaut : Orléans (2180, 3333) rig 300, Paris (2213, 3204) rig 150, Paris rig 60,
## Orléans rig 30. Une ligne `HC view` par vue (tuiles et instances d'arbres soumises au rendu) ;
## `--bench` : temps d'image moyen avec puis sans la couche d'arbres (240 images chacun), sans
## écrire de planche.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const ORLEANS := Vector2(2180.0, 3333.0)
const PARIS := Vector2(2213.0, 3204.0)
const VIEW_SIZE := Vector2i(640, 400)
const BENCH_FRAMES := 240


func _init() -> void:
	var out_dir := "user://hc_shots"
	var board_name := "hc_board"
	var bench := false
	## [x, z, distance du rig]
	var views: Array = [[ORLEANS.x, ORLEANS.y, 300.0], [PARIS.x, PARIS.y, 150.0], [PARIS.x, PARIS.y, 60.0], [ORLEANS.x, ORLEANS.y, 30.0]]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--name="):
			board_name = arg.trim_prefix("--name=")
		elif arg == "--bench":
			bench = true
		elif arg.begins_with("--prop="):
			# Réglage de `MapPropScale` remplacé pour cette exécution (essais sans toucher au .tres).
			var kv := arg.trim_prefix("--prop=").split("=")
			MapPropScale.shared().set(kv[0], float(kv[1]))
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
	facade.pending_faction = "fac_france"  # Paris et Orléans vus (pas de brouillard de guerre)
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("HC shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.get("map_data")
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var board := Image.create(VIEW_SIZE.x * 2, VIEW_SIZE.y * 2, false, Image.FORMAT_RGB8)
	board.fill(Color.BLACK)
	for v in mini(views.size(), 4):
		var view: Array = views[v]
		var focus := Vector3(view[0], data.surface_world_at(view[0], view[1]), view[1])
		rig.look_at_point(focus, maxf(float(view[2]), rig.min_distance_at(focus)))
		rig.snap()
		await _settle(terrain, settlements, vegetation)
		var census := vegetation.visible_census() if vegetation != null else {}
		print("HC view %d (%.0f, %.0f) rig %.0f style %s census %s stats %s" % [v, view[0], view[1], rig.distance, MapPropScale.tree_style(),
			JSON.stringify(census), JSON.stringify(vegetation.stats) if vegetation != null else "-"])
		if bench:
			Engine.max_fps = 0
			var with_trees := await _bench()
			vegetation.enabled = false
			for i in 20:
				await process_frame
			var without := await _bench()
			vegetation.enabled = true
			print("HC bench (%.0f, %.0f) rig %.0f : %.2f ms with trees, %.2f ms without, +%.2f ms (%d instances)" % [view[0], view[1], rig.distance,
				with_trees, without, with_trees - without, int(census.get("instances", 0))])
			continue
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		image.convert(Image.FORMAT_RGB8)
		image.resize(VIEW_SIZE.x, VIEW_SIZE.y, Image.INTERPOLATE_LANCZOS)
		board.blit_rect(image, Rect2i(Vector2i.ZERO, VIEW_SIZE), Vector2i((v % 2) * VIEW_SIZE.x, (v / 2) * VIEW_SIZE.y))
	if not bench:
		var path := out_dir.path_join("%s.jpg" % board_name)
		board.save_jpg(path, 0.88)
		print("HC board %s" % path)
	map.queue_free()
	await process_frame
	quit(0)


func _bench() -> float:
	var t0 := Time.get_ticks_usec()
	for i in BENCH_FRAMES:
		await process_frame
	return (Time.get_ticks_usec() - t0) / (BENCH_FRAMES * 1000.0)


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer, vegetation: Vegetation) -> void:
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
		if vegetation != null and (vegetation.pending_jobs() > 0 or vegetation.pending_regrounds() > 0):
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 60:
		await process_frame
