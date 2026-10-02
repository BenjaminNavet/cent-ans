extends SceneTree

## GC2 (ADR 0158) : captures des villes 1:1 ; ajouter `--town-style=real` après `--` (le style par
## défaut est `maquette`, voir `gc_shots.gd`).
## Captures de contrôle du lot VT-I (villes 1:1 à toutes les hauteurs, ADR 0138) : Paris à
## d = 1100 (F2), 300 (fondu F1/F2), 60 (F1), 15 (raccord ville 1:1 / lointain, bande
## 0,86-1,1 × `block_range`) et Amiens (ville ordinaire) à d = 150.
## Fenêtre réelle (pas headless) :
##   godot --path game --resolution 640x400 --script res://tests/vt_shots.gd -- --out=<dossier> --hide-armies
##   --map-weather=clear [--only=a,b] [--no-town-far] [--full=<dossier>]
## `--full` : copie à pleine résolution en plus (recadrages).
## JPEG `vt_<vue>.jpg` ≤ 640 px ; une ligne `VT shot` par capture (statistiques
## du lointain).

const PARIS := Vector2(2212.9, 3204.5)
const AMIENS := Vector2(2224.0, 3043.6)
## [nom, point, distance du rig (unités), cap (degrés)]
const SHOTS := [
	["paris_d1100", PARIS, 1100.0, 0.0],
	["paris_d300", PARIS, 300.0, 0.0],
	["paris_d60", PARIS, 60.0, 0.0],
	["paris_d35", PARIS, 35.0, 0.0],
	["paris_d22", PARIS, 22.0, 0.0],
	["paris_d15", PARIS, 15.0, 0.0],
	["amiens_d150", AMIENS, 150.0, 0.0],
]


func _init() -> void:
	var out_dir := "user://vt_shots"
	var only := ""
	var full_dir := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg.begins_with("--full="):
			full_dir = arg.substr(7)
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("vt shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	for shot: Array in SHOTS:
		if only != "" and not (shot[0] as String) in only.split(","):
			continue
		var point: Vector2 = shot[1]
		var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
		rig.look_at_point(focus, maxf(float(shot[2]), rig.min_distance_at(focus)))
		rig.target_yaw = deg_to_rad(float(shot[3]))
		rig.snap()
		await _settle(terrain, settlements)
		for i in 4:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		if full_dir != "":
			image.save_png(full_dir.path_join("vt_%s_full.png" % shot[0]))
		if image.get_width() > 640:
			image.resize(640, roundi(image.get_height() * 640.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("vt_%s.jpg" % shot[0])
		image.save_jpg(path, 0.85)
		var far: TownFarLayer = settlements.town_far if settlements != null else null
		print("VT shot %s d=%.1f far %s" % [path, rig.target_distance, JSON.stringify(far.stats) if far != null else "-"])
		var cam := root.get_viewport().get_camera_3d()
		var lc: LandmarkCityLayer = settlements.landmark_cities
		print("VT probe cam=%s focus=%s cam_to_focus=%.2f sink=%.2f lc_built=%s towns_built=%d lc_stats=%s" % [
			cam.global_position, focus, cam.global_position.distance_to(focus), far.sink_distance() if far != null else -1.0,
			lc.built_ids() if lc != null else [], settlements.towns.built_ids().size() if settlements.towns != null else -1,
			JSON.stringify(lc.stats) if lc != null else "-"])
	map.queue_free()
	await process_frame
	quit(0)


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer) -> void:
	for i in 30:
		await process_frame
	var guard := 0
	while guard < 1500:
		guard += 1
		var busy := false
		if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
			busy = true
		if ReliefLandcover.pending():
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 40:
		await process_frame
