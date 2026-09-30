extends SceneTree

## Captures de contrôle du lot VT2 (moulins, panaches de cheminée et figurants FK à l'échelle 1:1,
## ADR 0138 addendum) : Paris à d = 15 (ville 1:1, moulins de sa couronne, fumées), puis un moulin
## de Paris à d = 3 (sous `figure_max_distance` : gens, bêtes, charrettes) et d = 1 (moulin, maisons
## et figurants côte à côte).
## Fenêtre réelle (pas headless) :
##   godot --path game --resolution 640x400 --script res://tests/vt2_shots.gd -- --out=<dossier>
##   --hide-armies --map-weather=clear [--only=a,b] [--full=<dossier>]
## JPEG `vt2_<vue>.jpg` ≤ 640 px ; une ligne `VT2 shot` par capture (moulins et panaches affichés,
## figurines posées, taille médiane à l'écran).

const PARIS := Vector2(2212.9, 3204.5)
## [nom, point (null : un moulin de Paris), distance du rig (unités), cap (degrés)]
const SHOTS := [
	["paris_d15", PARIS, 15.0, 0.0],
	["paris_mill_d3", null, 3.0, 0.0],
	["paris_mill_d1", null, 1.0, 0.0],
]


func _init() -> void:
	var out_dir := "user://vt2_shots"
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
		push_error("vt2 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var life: CampaignLife = map.get("life")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var mill := _paris_mill(settlements, life)
	print("VT2 Paris windmill at %s" % mill)
	for shot: Array in SHOTS:
		if only != "" and not (shot[0] as String) in only.split(","):
			continue
		var point: Vector2 = shot[1] if shot[1] != null else mill
		var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
		rig.look_at_point(focus, maxf(float(shot[2]), rig.min_distance_at(focus)))
		rig.target_yaw = deg_to_rad(float(shot[3]))
		rig.snap()
		await _settle(terrain, settlements, life)
		for i in 4:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		if full_dir != "":
			image.save_png(full_dir.path_join("vt2_%s_full.png" % shot[0]))
		if image.get_width() > 640:
			image.resize(640, roundi(image.get_height() * 640.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("vt2_%s.jpg" % shot[0])
		image.save_jpg(path, 0.85)
		var effects: LifeEffects = life.effects if life != null else null
		var folk: FolkPool = life.folk if life != null else null
		var cam := root.get_viewport().get_camera_3d()
		print("VT2 shot %s d=%.2f mills_shown=%s chimneys_shown=%s effects=%s folk=%s" % [
			path, rig.target_distance,
			effects.get_node("WindmillBodies").visible if effects != null else "-",
			effects.get_node("Chimneys").visible if effects != null else "-",
			JSON.stringify(effects.stats) if effects != null else "-",
			JSON.stringify(folk.view_report(cam)) if folk != null else "-"])
	map.queue_free()
	await process_frame
	quit(0)


## Un moulin de la couronne de Paris (repli : Paris).
func _paris_mill(settlements: SettlementLayer, life: CampaignLife) -> Vector2:
	if settlements == null or life == null or life.effects == null:
		return PARIS
	var index := -1
	for i in settlements.data.settlements.size():
		if str(settlements.data.settlements[i]["id"]) == "set_paris":
			index = i
			break
	for point: Array in life.effects.get("_windmill_points"):
		if int(point[5]) == index:
			return point[0]
	return PARIS


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer, life: CampaignLife) -> void:
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
		if life != null and life.folk != null and not bool(life.folk.get("_settled")):
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 40:
		await process_frame
