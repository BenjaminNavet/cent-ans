extends SceneTree

## Captures de contrôle du lot A6-L9 (lisibilité de la carte de campagne) : une image 1280 x 720 par
## vue (été, hiver, parchemin, religion, plus une vue proche pour les rivières et les plaques).
## Fenêtre réelle (pas headless), écrit des JPEG dans le dossier donné ; ne lit rien :
##   godot --path game --resolution 1280x720 --script res://tests/a6_map_shots.gd -- --out=<dossier>
##   [--views=x,z,distance;…] (vue moyenne : Normandie / Paris par défaut) [--only=summer,winter,…]
##   [--map-weather=rain] (météo forcée, si le jeu l'accepte)
## Fichiers : a6_summer.jpg, a6_winter.jpg, a6_parchment.jpg, a6_religion.jpg, a6_near.jpg.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SIZE := Vector2i(1280, 720)
## [x, z] du point visé (px carte) : Île-de-France et Normandie.
const FOCUS := Vector2(2150.0, 3150.0)
const MEDIUM := 450.0
const PARCHMENT_DISTANCE := 1900.0
const NEAR := 120.0


func _init() -> void:
	var out_dir := "user://a6_shots"
	var focus := FOCUS
	var medium := MEDIUM
	var only: PackedStringArray = []
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			only = arg.trim_prefix("--only=").split(",", false)
		elif arg.begins_with("--views="):
			var parts := arg.trim_prefix("--views=").split(",")
			if parts.size() >= 3:
				focus = Vector2(float(parts[0]), float(parts[1]))
				medium = float(parts[2])
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
		push_error("A6 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.get("map_data")
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var life: CampaignLife = map.get("life")
	var modes: Node = map.get("map_modes")
	# [nom, saison, distance, mode de carte]
	var plan: Array = [
		["summer", "summer", medium, "political"],
		["winter", "winter", medium, "political"],
		["parchment", "summer", PARCHMENT_DISTANCE, "political"],
		["religion", "summer", medium, "religion"],
		["near", "summer", NEAR, "political"],
	]
	for entry: Array in plan:
		if not only.is_empty() and not only.has(str(entry[0])):
			continue
		if life != null:
			life.seasons.set_season(str(entry[1]), true)
		if modes != null:
			modes.call("set_mode", str(entry[3]))
		var point := Vector3(focus.x, data.surface_world_at(focus.x, focus.y), focus.y)
		rig.look_at_point(point, maxf(float(entry[2]), rig.min_distance_at(point)))
		rig.snap()
		await _settle(terrain, settlements)
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		image.convert(Image.FORMAT_RGB8)
		if image.get_size() != SIZE:
			image.resize(SIZE.x, SIZE.y, Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("a6_%s.jpg" % str(entry[0]))
		image.save_jpg(path, 0.9)
		print("A6 shot %s (rig %.0f)" % [path, rig.distance])
	map.queue_free()
	await process_frame
	quit(0)


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
