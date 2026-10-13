extends SceneTree

## RE (relief exagéré façon Total War) : planche de comparaison, une ligne par lieu, une colonne
## par hauteur de caméra, pour un réglage du profil `ReliefExaggerationProfile` donné en argument.
## Usage : tools/godot_bg.sh --path game --resolution 640x400 --script res://tests/re_relief_shot.gd -- \
##   [--out=<fichier.png>] [--distances=120,400] [--points=lon,lat;…] [--lift=2.0] [--flatten=0.6]
##   [--power=0.0] [--knee=0] [--massif=2.0] [--smooth=1]
## Sans --lift : profil de la ressource (référence). Avec --lift : écrasement SZ1 coupé (--knee=0
## par défaut), relevé et aplanissement RE appliqués. Lieux par défaut : Alpes (Valais), Pyrénées
## centrales, Massif central, Beauce. Écrit hors dépôt (~/dev/cent-ans-raw/relief/planches/).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const DEFAULT_POINTS := "7.4,46.2;0.6,42.75;2.9,45.3;1.7,48.3"


func _settle(frames: int) -> Image:
	for _i in frames:
		await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()


func _profile() -> void:
	var lift := CmdArgs.value("--lift", "")
	if lift == "":
		return
	var profile := (load("res://resources/relief_exaggeration.tres") as ReliefExaggerationProfile).duplicate() as ReliefExaggerationProfile
	profile.mountain_knee_m = float(CmdArgs.value("--knee", "0"))
	profile.mountain_lift = float(lift)
	profile.mountain_lift_power = float(CmdArgs.value("--power", "0"))
	profile.plain_flatten = float(CmdArgs.value("--flatten", "0.6"))
	profile.massif_lift = float(CmdArgs.value("--massif", "0"))
	profile.massif_smooth_radius = int(CmdArgs.value("--smooth", "1"))
	ReliefExaggerationProfile.set_default(profile)
	print("re_relief_shot: lift %s massif %.2f flatten %.2f power %.2f knee %.0f" % [lift, profile.massif_lift, profile.plain_flatten, profile.mountain_lift_power, profile.mountain_knee_m])


func _init() -> void:
	var out := CmdArgs.value("--out", OS.get_environment("HOME") + "/dev/cent-ans-raw/relief/planches/re_relief.png")
	var distances: Array[float] = []
	for part in CmdArgs.value("--distances", "120,400").split(","):
		distances.append(float(part))
	var points: Array[Vector2] = []
	for part in CmdArgs.value("--points", DEFAULT_POINTS).split(";"):
		var lonlat := part.split(",")
		points.append(Vector2(float(lonlat[0]), float(lonlat[1])))
	_profile()
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
	var shots: Array[Image] = []
	for point in points:
		var center := FaunaLayer.lonlat_to_px(point.x, point.y, map_data)
		for index in distances.size():
			rig.look_at_point(Vector3(center.x, map_data.surface_world_at(center.x, center.y), center.y), distances[index])
			rig.snap()
			shots.append(await _settle(300 if shots.is_empty() else 160))
			print("re_relief_shot: %.2f,%.2f distance %.1f" % [point.x, point.y, distances[index]])
	var w := shots[0].get_width()
	var h := shots[0].get_height()
	var cols := distances.size()
	var board := Image.create(w * cols, h * points.size(), false, shots[0].get_format())
	for index in shots.size():
		board.blit_rect(shots[index], Rect2i(Vector2i.ZERO, Vector2i(w, h)), Vector2i((index % cols) * w, (index / cols) * h))
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	board.save_png(out)
	print("re_relief_shot: planche -> ", out)
	map.queue_free()
	await process_frame
	quit(0)
