extends SceneTree

## Captures du lot R1 (relief fin, forêts, zones humides) aux mêmes cadrages avant/après.
## Fenêtre réelle (pas headless) : godot --path game --script res://tests/r1_relief_shots.gd -- --out=<dossier> [--prefix=after]
## Cadrages : France entière, Île-de-France/forêt d'Orléans (zoom moyen), forêt d'Orléans (proche),
## Normandie, Dombes/Alpes, Fens.

const VIEWS := [
	["france", Vector2(2000, 1900), 1400.0],
	["idf_orleans", Vector2(2195, 1990), 330.0],
	["orleans_near", Vector2(2175, 2050), 130.0],
	["normandie", Vector2(2040, 1850), 300.0],
	["dombes_alpes", Vector2(2530, 2400), 380.0],
	["fens", Vector2(2056, 1337), 260.0],
]
## Vues rapprochées (après seulement, `--near`) : étangs de la Dombes, marais des Fens, Seine normande.
const NEAR_VIEWS := [
	["dombes_near", Vector2(2459, 2390), 70.0],
	["fens_near", Vector2(2050, 1345), 70.0],
	["seine_near", Vector2(2120, 1880), 90.0],
]


func _init() -> void:
	var out_dir := "user://r1"
	var prefix := "shot"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("r1 shots: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	print("R1 height file: %s" % data.height_file)
	var views: Array = VIEWS.duplicate()
	if OS.get_cmdline_user_args().has("--near"):
		views.append_array(NEAR_VIEWS)
	for view in views:
		var point: Vector2 = view[1]
		rig.look_at_point(Vector3(point.x, data.surface_world_at(point.x, point.y), point.y), view[2])
		rig.snap()
		for i in 20:
			await process_frame
		var guard := 0
		while guard < 900:
			guard += 1
			var busy := false
			if vegetation != null and vegetation.has_method("pending_jobs") and vegetation.pending_jobs() > 0:
				busy = true
			if terrain != null and not terrain.fine_ready():
				busy = true
			if not busy:
				break
			await process_frame
		for i in 30:
			await process_frame
		var image := root.get_viewport().get_texture().get_image()
		var path := out_dir.path_join("%s_%s.png" % [prefix, view[0]])
		image.save_png(path)
		print("R1 shot %s" % path)
	map.queue_free()
	await process_frame
	quit(0)
