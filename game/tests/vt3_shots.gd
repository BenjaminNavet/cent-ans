extends SceneTree

## Captures de contrôle du chantier VT3 (arbres de la carte à l'échelle 1:1, ADR 0138 addendum) :
## forêt d'Orléans à d = 300 (canopée du terrain seule), 40 (juste au-delà de la portée des arbres)
## et 5 (arbres 1:1, forêt dense), puis Paris à d = 15 (arbres et maisons 1:1 côte à côte).
## Fenêtre réelle (pas headless) :
##   godot --path game --resolution 640x400 --script res://tests/vt3_shots.gd -- --out=<dossier>
##   --hide-armies --map-weather=clear [--only=a,b]
## JPEG `vt3_<vue>.jpg` ≤ 640 px ; une ligne `VT3 shot` par capture (arbres affichés).

const ORLEANS_FOREST := Vector2(2180.0, 3333.0)
const PARIS := Vector2(2212.9, 3204.5)
## [nom, point, distance du rig (unités), cap (degrés)]
const SHOTS := [
	["orleans_d300", ORLEANS_FOREST, 300.0, 0.0],
	["orleans_d40", ORLEANS_FOREST, 40.0, 0.0],
	["orleans_d5", ORLEANS_FOREST, 5.0, 0.0],
	["paris_d15", PARIS, 15.0, 0.0],
]


func _init() -> void:
	var out_dir := "user://vt3_shots"
	var only := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("vt3 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
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
		await _settle(terrain, settlements, vegetation)
		if vegetation != null and vegetation.forest_detail != null and vegetation.visible:
			vegetation.forest_detail.flush(point, rig.target_distance)
		for i in 30:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		if image.get_width() > 640:
			image.resize(640, roundi(image.get_height() * 640.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("vt3_%s.jpg" % shot[0])
		image.save_jpg(path, 0.85)
		var detail := vegetation.forest_detail if vegetation != null else null
		print("VT3 shot %s d=%.2f trees_shown=%s tiles=%d dense=%s" % [path, rig.target_distance,
			vegetation.visible if vegetation != null else "-", vegetation.tile_count() if vegetation != null else 0,
			JSON.stringify({"visible": detail.visible_count(), "radius": detail.stats.get("radius"), "gain": detail.stats.get("gain"), "fraction": detail.stats.get("fraction")}) if detail != null else "-"])
	map.queue_free()
	await process_frame
	quit(0)


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer, vegetation: Vegetation) -> void:
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
		if vegetation != null and vegetation.pending_jobs() > 0:
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 40:
		await process_frame
