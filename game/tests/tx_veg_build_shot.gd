extends SceneTree

## TX T3/T4/eau : captures de contrôle de la campagne (villages par région, sol à hauteur de plantes,
## mer méditerranéenne, rivière), sans interface, en PNG sous --out (hors dépôt).
## Usage : tools/godot_bg.sh --path game --resolution 1280x720 \
##   --script res://tests/tx_veg_build_shot.gd -- --out=<dossier> [--only=amiens]
## Positions en coordonnées carte (px = unités monde, x vers l'est, z vers le sud).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

# nom, x, z, distance caméra (< 0 : minimum + 5 %, au ras du sol)
const VIEWS := [
	["build_norwich", 2175.1, 2611.5, 4.0],
	["build_gand", 2380.6, 2878.2, 4.0],
	["build_siena", 3142.9, 4098.1, 4.0],
	["build_alger", 2124.2, 5068.7, 4.0],
	["veg_ground_beauce", 2300.0, 3150.0, 1.0],
	["veg_ground_steppe", 4800.0, 3600.0, 1.0],
	["water_mediterranean", 3050.0, 4650.0, 60.0],
	["water_seine", 2150.0, 3050.0, 40.0],
]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tx_veg_build")
	var only := CmdArgs.value("--only", "")
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
	for _i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	# Zoom au ras du sol : lève les planchers de la caméra (captures seulement).
	rig.min_distance = 0.4
	rig.close_min_distance = 0.4
	rig.floor_distance = 0.0
	rig.floor_zones = PackedVector3Array()
	rig.floor_zones_set = true
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var map_data: MapData = map.get("map_data")
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var x: float = view[1]
		var z: float = view[2]
		var focus := Vector3(x, map_data.surface_world_at(x, z), z)
		var distance := float(view[3])
		if distance < 0.0:
			distance = rig.min_distance_at(focus) * 1.05
		rig.look_at_point(focus, distance)
		rig.snap()
		for _i in 150:
			await process_frame
		await RenderingServer.frame_post_draw
		var path := out.path_join("%s.png" % view[0])
		root.get_texture().get_image().save_png(path)
		print("tx_veg_build_shot: ", path, " dist=", rig.distance, " regional=", BuildingMaterials.regional_ready())
	map.queue_free()
	await process_frame
	quit(0)
