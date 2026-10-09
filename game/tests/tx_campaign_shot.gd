extends SceneTree

## TX T2b : captures du sol de campagne (fond par biome, fondu aux frontières, micro-détail).
## Usage : tools/godot_bg.sh --path game --resolution 960x600 --script res://tests/tx_campaign_shot.gd -- \
##   --out=<dossier> [--only=beauce] [--distances=40,180] [--legacy-textures] [--parcels-source=tx]
## Les PNG sont écrits hors dépôt (jamais commités) ; imprime aussi ms/image et appels de dessin.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
# nom, lon, lat (frontières de biomes : Pyrénées-Meseta, Sahel du Sahara...)
const VIEWS := [
	["beauce", 1.6, 48.3],
	["bretagne", -3.0, 48.2],
	["provence", 5.4, 43.7],
	["steppe", 38.0, 47.5],
	["desert", 9.0, 30.5],
	["taiga", 36.0, 62.0],
	["alpes", 7.0, 45.9],
	["frontiere", 2.0, 44.2],
]


func _measure(frames: int) -> Dictionary:
	var start := Time.get_ticks_usec()
	var draws := 0.0
	for _i in frames:
		await process_frame
		draws += Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	return {"ms": (Time.get_ticks_usec() - start) / 1000.0 / frames, "draws": draws / frames}


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tx_campaign")
	var distances := CmdArgs.value("--distances", "40,180").split(",")
	var only := CmdArgs.value("--only", "")
	var prefix := CmdArgs.value("--prefix", "campaign_")
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
	var data: MapData = map.get("map_data")
	RenderingServer.global_shader_parameter_set("campaign_season", Vector4(0.0, 1.0, 0.0, 0.0))
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var px := FaunaLayer.lonlat_to_px(float(view[1]), float(view[2]), data)
		for dist_text in distances:
			rig.look_at_point(Vector3(px.x, data.surface_world_at(px.x, px.y), px.y), float(dist_text))
			rig.snap()
			for _i in 120:
				await process_frame
			await RenderingServer.frame_post_draw
			var stats: Dictionary = await _measure(60)
			var path := out.path_join("%s%s_d%s.png" % [prefix, view[0], dist_text])
			root.get_texture().get_image().save_png(path)
			print("tx_campaign_shot: %s d=%s %.2f ms/frame %.0f draws -> %s" % [view[0], dist_text, stats["ms"], stats["draws"], path])
	map.queue_free()
	await process_frame
	quit(0)
