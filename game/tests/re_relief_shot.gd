extends SceneTree

## RE (relief exagéré façon Total War) : planche de comparaison, une ligne par lieu, une colonne
## par hauteur de caméra, pour un réglage du profil `ReliefExaggerationProfile` donné en argument.
## Usage : tools/godot_bg.sh --path game --resolution 640x400 --script res://tests/re_relief_shot.gd -- \
##   [--out=<fichier.png>] [--distances=120,400] [--points=lon,lat;…] [--lift=2.0] [--flatten=0.6]
##   [--power=0.0] [--knee=0] [--massif=2.0] [--smooth=1]
## CS2 : isolement des couches (pour trouver ce qui porte des taches au sol) :
##   --hide=Vegetation,GroundClutter,RockOutcrops,...  masque les nœuds enfants de la carte portant ces noms
##     (tout nœud 3D enfant direct : Vegetation arbres et bosquets, GroundClutter touffes et broussailles,
##     RockOutcrops rochers, FreshwaterLayer, CampaignLife foule et chariots, Settlements, Roads, Rivers, Coast,
##     Lakes, Armies, Atmosphere, ...) ; --list écrit la liste réelle des noms dans le journal ;
##   --ground-only  ne garde que Terrain, Sea, Sun, WorldEnvironment et la caméra ;
##   --terrain-off=hb,colormap,cloudshadow  éteint dans le shader du sol : parcellaire HB, carte de couleur,
##     ombre des cumulus.
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


const KEEP_GROUND := ["Terrain", "Sea", "Sun", "WorldEnvironment", "CameraRig", "Picker"]


func _isolate(map: Node) -> void:
	var names: Array[String] = []
	for child in map.get_children():
		if child is Node3D and not (child is Camera3D):
			names.append(str(child.name))
	if CmdArgs.has("--list"):
		print("re_relief_shot: couches : ", ", ".join(names))
	var hidden: Array[String] = []
	for part in CmdArgs.value("--hide", "").split(",", false):
		hidden.append(part.strip_edges())
	if CmdArgs.has("--ground-only"):
		for n in names:
			if not KEEP_GROUND.has(n):
				hidden.append(n)
	for n in hidden:
		var node := map.get_node_or_null(n)
		if node == null:
			print("re_relief_shot: couche inconnue ", n)
			continue
		node.set("visible", false)
		node.set_process(false)
		print("re_relief_shot: masquée ", n)
	var terrain: Variant = map.get("terrain")
	var material: ShaderMaterial = terrain.material if terrain != null else null
	if material == null:
		return
	for part in CmdArgs.value("--terrain-off", "").split(",", false):
		match part.strip_edges():
			"hb":
				material.set_shader_parameter("has_hb", false)
			"colormap":
				material.set_shader_parameter("has_colormap", false)
			"cloudshadow":
				material.set_shader_parameter("cloud_shadow_cumulus", 0.0)
				material.set_shader_parameter("cloud_shadow_amount", 0.0)
		print("re_relief_shot: sol sans ", part)


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
	_isolate(map)
	var shots: Array[Image] = []
	for point in points:
		var center := MapProjection.for_map(map_data).lonlat_to_px(point.x, point.y)
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
