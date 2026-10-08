extends SceneTree

## FL3 (ADR 0192) : compare la météo cuite (champ basse résolution) à la météo recalculée à chaque
## pixel, sur une mosaïque de météos par province. Écrit deux captures et l'écart moyen.
## Usage (avec affichage) : tools/godot_bg.sh --path game --resolution 1280x720 \
##   --script res://tests/fl_weather_field_shot.gd -- --out=<dossier> [--height=300] [--natural]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out := "user://fl_weather"
	var height := 300.0
	var exaggerate := true
	out = CmdArgs.value("--out", out)
	height = CmdArgs.number("--height", height)
	if CmdArgs.has("--natural"):
		exaggerate = false
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
	# Sans orage (éclairs) ni vent : deux captures de la même météo sont comparables.
	var kinds := ["rain", "fog", "snow", "clear"]
	var weather := {}
	for index in range(1, map_data.province_count + 1):
		var id := str(map_data.get_province(index).get("id", ""))
		weather[id] = {"kind": kinds[index % kinds.size()], "intensity": 0.9, "label": ""}
	view.weather = weather
	view._upload_mask()
	var terrain: TerrainBuilder = map.get("terrain")
	var ground_material := terrain.material as ShaderMaterial
	ground_material.set_shader_parameter("weather_wind", Vector2.ZERO)
	# Effets forcés (pluie sombre, brume opaque) : une frange décalée ou retournée saute aux yeux.
	if exaggerate:
		ground_material.set_shader_parameter("weather_wet_dim", 0.7)
		ground_material.set_shader_parameter("weather_mist_max", 1.0)
	view._clouds.visible = false
	view.set_process(false)
	var focus := Vector3(2213.0, map_data.surface_world_at(2213.0, 3204.0), 3204.0)
	rig.look_at_point(focus, height)
	rig.snap()
	for _i in 60:
		await process_frame
	var images: Array[Image] = []
	for use_field in [true, false, false]:
		view.use_field = use_field
		for _i in 30:
			await process_frame
		await RenderingServer.frame_post_draw
		images.append(root.get_texture().get_image())
	DirAccess.make_dir_recursive_absolute(out)
	images[0].save_png(out.path_join("field.png"))
	images[1].save_png(out.path_join("direct.png"))
	# Écart cuit / recalculé, et bruit de fond (deux captures recalculées : animations restantes).
	print("fl_weather_field_shot: field_vs_direct=%s noise=%s (%s)" % [_diff(images[0], images[1]), _diff(images[1], images[2]), out])
	map.queue_free()
	await process_frame
	quit(0)


## Écart moyen et maximal (0..1, moyenne RVB) sur un pixel sur quatre.
func _diff(first: Image, second: Image) -> String:
	var total := 0.0
	var worst := 0.0
	var size := first.get_size()
	for y in range(0, size.y, 4):
		for x in range(0, size.x, 4):
			var a := first.get_pixel(x, y)
			var b := second.get_pixel(x, y)
			var d := (absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b)) / 3.0
			total += d
			worst = maxf(worst, d)
	return "%.4f/%.3f" % [total / float((size.x / 4) * (size.y / 4)), worst]
