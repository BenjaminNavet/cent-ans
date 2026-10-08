extends SceneTree

## DN-ME2 : captures des troupeaux (Camargue, Alpes, Castille) à trois distances, sans interface.
## Usage : tools/godot_bg.sh --path game --resolution 640x400 \
##   --script res://tests/me2_fauna_shot.gd -- --out=<dossier> [--season=summer] [--distances=2,10,40]
## Les PNG sont écrits hors dépôt (jamais commités).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
# nom, lon, lat
const VIEWS := [["camargue", 4.55, 43.52], ["alpes", 7.2, 45.9], ["castille", -4.2, 41.4]]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://me2_shots")
	var distances := CmdArgs.value("--distances", "2,10,40").split(",")
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
	rig.floor_distance = 0.0  # captures : zoom plus près que le plancher du jeu (animaux à taille réelle)
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var data: MapData = map.get("map_data")
	var fauna: FaunaLayer = map.life.fauna
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var px := FaunaLayer.lonlat_to_px(float(view[1]), float(view[2]), data)
		# Se place sur le troupeau le plus proche du point (visée au centre d'un troupeau).
		var best := px
		var best_d := 1e9
		for cy in range(floori(px.y / 96.0) - 1, floori(px.y / 96.0) + 2):
			for cx in range(floori(px.x / 96.0) - 1, floori(px.x / 96.0) + 2):
				for herd: Dictionary in fauna.cell_herds(Vector2i(cx, cy)):
					var d := (herd["center"] as Vector2).distance_to(px)
					if d < best_d:
						best_d = d
						best = herd["center"]
		for dist_text in distances:
			var distance := float(dist_text)
			rig.look_at_point(Vector3(best.x, data.surface_world_at(best.x, best.y), best.y), distance)
			rig.snap()
			for _i in 120:
				await process_frame
			await RenderingServer.frame_post_draw
			var path := out.path_join("%s_d%s.png" % [view[0], dist_text])
			var image := root.get_texture().get_image()
			image.save_png(path)
			_probe(image, fauna, data, best, root.get_camera_3d() if root.get_camera_3d() != null else rig.get_viewport().get_camera_3d())
			print("me2_fauna_shot: ", path, " px=", best, " dist=", rig.distance, " stats=", fauna.stats)
	map.queue_free()
	await process_frame
	quit(0)


## Sonde numérique (sans lire l'image) : projette les bêtes du troupeau visé à l'écran et imprime
## le contraste local (luminance min/max dans une fenêtre 9 × 9) autour de chacune.
func _probe(image: Image, fauna: FaunaLayer, data: MapData, at: Vector2, camera: Camera3D) -> void:
	if camera == null:
		print("probe: no camera")
		return
	var herds := fauna.cell_herds(Vector2i(floori(at.x / 96.0), floori(at.y / 96.0)))
	var shown := 0
	for herd: Dictionary in herds:
		if (herd["center"] as Vector2).distance_to(at) > 0.5:
			continue
		for off: Vector2 in herd["members"]:
			var world := Vector3(at.x + off.x, data.surface_world_at(at.x + off.x, at.y + off.y), at.y + off.y)
			if camera.is_position_behind(world):
				continue
			var screen := camera.unproject_position(world)
			var ix := int(screen.x)
			var iy := int(screen.y)
			if ix < 5 or iy < 5 or ix >= image.get_width() - 5 or iy >= image.get_height() - 5:
				continue
			var low := 9.0
			var high := -1.0
			for dy in range(-4, 5):
				for dx in range(-4, 5):
					var l := image.get_pixel(ix + dx, iy + dy).get_luminance()
					low = minf(low, l)
					high = maxf(high, l)
			print("probe: member screen=", screen, " lum min=", snappedf(low, 0.01), " max=", snappedf(high, 0.01))
			shown += 1
			if shown >= 6:
				return
