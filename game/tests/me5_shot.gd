extends SceneTree

## ME5 : captures de l'atmosphère de la carte (cumulus éclairés, cirrus, rideaux de pluie, brouillard
## de fleuve, aurore). Écrit des PNG hors dépôt. Usage (fenêtre en arrière-plan) :
##   tools/godot_bg.sh --path game --resolution 1280x720 --script res://tests/me5_shot.gd -- \
##     --out=<dossier> --shots=name:height:season:kind:fx,... (fx : on|off ; season : spring..winter)

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out := CmdArgs.value("--out", "user://me5")
	var shots := CmdArgs.value("--shots", "mid:600:summer:rain:on").split(",")
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
	var view: CampaignWeatherView = map.weather_view
	var life: Node = map.get("life")
	DirAccess.make_dir_recursive_absolute(out)
	var thumbs: Array[Image] = []
	for shot in shots:
		var parts := shot.split(":")
		var weather := {}
		for index in range(1, map_data.province_count + 1):
			var id := str(map_data.get_province(index).get("id", ""))
			weather[id] = {"kind": parts[3] if index % 3 == 0 else "clear", "intensity": 0.8, "label": ""}
		view.weather = weather
		view._upload_mask()
		view._fog_clock = 0.0
		view.atmosphere.refresh(weather)
		view.atmosphere.visible = parts[4] == "on"
		if life != null:
			life.set("forced_season", parts[2])
			life.seasons.set_season(parts[2], true)
		var focus := Vector3(float(parts[5]) if parts.size() > 5 else 1500.0, 0.0, float(parts[6]) if parts.size() > 6 else 2400.0)
		focus.y = map_data.surface_world_at(focus.x, focus.z)
		rig.look_at_point(focus, float(parts[1]))
		rig.snap()
		for _i in 90:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		image.save_png(out.path_join(parts[0] + ".png"))
		image.resize(640, 360, Image.INTERPOLATE_BILINEAR)
		thumbs.append(image)
		var states := []
		for child in view.atmosphere.get_children():
			states.append("%s=%s" % [child.name, child.visible])
		var aurora := view.atmosphere.get_node_or_null("Aurora") as MeshInstance3D
		print("me5_shot: ", parts[0], " ", states, " map=", map_data.size, " aurora_strength=",
				(aurora.material_override as ShaderMaterial).get_shader_parameter("strength") if aurora != null else -1)
	if thumbs.size() > 1:  # planche 2 colonnes, pour une seule lecture d'image
		var sheet := Image.create(1280, 360 * ((thumbs.size() + 1) / 2), false, Image.FORMAT_RGB8)
		for index in thumbs.size():
			sheet.blit_rect(thumbs[index], Rect2i(0, 0, 640, 360), Vector2i((index % 2) * 640, (index / 2) * 360))
		sheet.save_png(out.path_join("sheet.png"))
	map.queue_free()
	await process_frame
	quit(0)
