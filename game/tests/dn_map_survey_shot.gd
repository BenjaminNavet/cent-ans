extends SceneTree

## DN : relevé de la carte de campagne (revue direction artistique). Six vues régionales à zoom
## moyen-proche (sous le seuil parchemin), sans interface, écrites en PNG.
## Usage : tools/godot_bg.sh --path game --resolution 1280x720 \
##   --script res://tests/dn_map_survey_shot.gd -- --out=<dossier> [--distance=220] [--only=paris]
## Positions en coordonnées carte (px = unités monde, x vers l'est, z vers le sud).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

# nom, x, z
const VIEWS := [
	["1_camargue", 2440.0, 4035.0],
	["2_bretagne_atlantique", 1490.0, 3300.0],
	["3_alpes", 2690.0, 3675.0],
	["4_paris", 2213.0, 3204.0],
	["5_marais_poitevin", 1880.0, 3590.0],
	["6_castille", 1450.0, 4160.0],
]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://dn_survey")
	var distance := CmdArgs.number("--distance", 220.0)
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
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var map_data: MapData = map.get("map_data")
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var x: float = view[1]
		var z: float = view[2]
		rig.look_at_point(Vector3(x, map_data.surface_world_at(x, z), z), distance)
		rig.snap()
		for _i in 90:
			await process_frame
		await RenderingServer.frame_post_draw
		var path := out.path_join("%s.png" % view[0])
		root.get_texture().get_image().save_png(path)
		print("dn_map_survey_shot: ", path, " dist=", rig.distance)
	map.queue_free()
	await process_frame
	quit(0)
