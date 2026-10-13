extends SceneTree

## GR (ADR 0352) : planche régionale des champs peints, une ligne par région, une colonne par
## hauteur de caméra, pour juger si la carte suit la géographie (openfield, bocage, vignobles,
## oliveraies, estives, prés de vallée).
## Usage : tools/godot_bg.sh --path game --resolution 640x400 --script res://tests/gr_regions_shot.gd -- \
##   [--out=<fichier.png>] [--distances=15,60] [--points=lon,lat;lon,lat…] [--season=summer|spring|autumn|winter]
## Régions par défaut, dans l'ordre : Beauce, Bretagne, Provence, Côte de Bourgogne, Flandre,
## Bordelais, Castille, Oxfordshire. Écrit hors dépôt (~/dev/cent-ans-raw/textures/planches/).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const DEFAULT_POINTS := "1.6,48.25;-3.2,48.1;5.9,43.55;4.88,47.12;3.0,50.85;-0.25,44.75;-5.0,42.0;-1.5,51.6"
const SEASONS := {"spring": Vector4(1, 0, 0, 0), "summer": Vector4(0, 1, 0, 0), "autumn": Vector4(0, 0, 1, 0), "winter": Vector4(0, 0, 0, 1)}


func _settle(frames: int) -> Image:
	for _i in frames:
		await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()


func _init() -> void:
	var out := CmdArgs.value("--out", OS.get_environment("HOME") + "/dev/cent-ans-raw/textures/planches/gr_regions.png")
	var distances: Array[float] = []
	for part in CmdArgs.value("--distances", "15,60").split(","):
		distances.append(float(part))
	var points: Array[Vector2] = []
	for part in CmdArgs.value("--points", DEFAULT_POINTS).split(";"):
		var lonlat := part.split(",")
		points.append(Vector2(float(lonlat[0]), float(lonlat[1])))
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
	RenderingServer.global_shader_parameter_set("campaign_season", SEASONS.get(CmdArgs.value("--season", "summer"), SEASONS["summer"]))
	var map_data: MapData = map.get("map_data")
	# Le brouillard de guerre (voile sépia) cache les régions que la France n'a pas vues (Bordelais,
	# Provence : « sol brun uni ») : coupé sauf avec --fog, pour juger l'habillage seul.
	if not CmdArgs.has("--fog"):
		for node in root.find_children("*", "MeshInstance3D", true, false):
			var material: Material = (node as MeshInstance3D).material_override
			if material is ShaderMaterial and (material as ShaderMaterial).get_shader_parameter("has_hb") != null:
				(material as ShaderMaterial).set_shader_parameter("fog_enabled", false)
	var shots: Array[Image] = []
	for point in points:
		var center := FaunaLayer.lonlat_to_px(point.x, point.y, map_data)
		for index in distances.size():
			rig.look_at_point(Vector3(center.x, map_data.surface_world_at(center.x, center.y), center.y), distances[index])
			rig.snap()
			shots.append(await _settle(300 if shots.is_empty() else 160))
			print("gr_regions_shot: %.2f,%.2f distance %.1f" % [point.x, point.y, distances[index]])
	var w := shots[0].get_width()
	var h := shots[0].get_height()
	var cols := distances.size()
	var board := Image.create(w * cols, h * points.size(), false, shots[0].get_format())
	for index in shots.size():
		board.blit_rect(shots[index], Rect2i(Vector2i.ZERO, Vector2i(w, h)), Vector2i((index % cols) * w, (index / cols) * h))
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	board.save_png(out)
	print("gr_regions_shot: planche -> ", out)
	map.queue_free()
	await process_frame
	quit(0)
