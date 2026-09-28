extends SceneTree

## Captures et mesure du lot VH6 (Londres vers 1340 à l'échelle 1:1, ADR 0078) : vue
## stratégique (maquette sous loupe), fondu, paliers vallée et site (Tour, London Bridge, Old
## St Paul's, Westminster), puis mesure d'images par seconde au-dessus de Londres et de Rouen
## (VH4) à la même distance.
## Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/vh6_shots.gd -- --out=<dossier> --map-weather=clear
##   [--only=a,b] [--no-fps] [--no-landmarks-1to1]
## JPEG ≤ 960 px, `london_<vue>.jpg`.

const LONDON := Vector2(2019.888, 2766.127)
const ROUEN := Vector2(2096.54, 3099.88)
## Repère : 1 unité ≈ 719 m ; +x à l'est, +y au sud.
const TOWER := LONDON + Vector2(3.13, 1.13)
const BRIDGE := LONDON + Vector2(2.13, 1.03)
const PAULS := LONDON + Vector2(1.14, -0.04)
const WESTMINSTER := LONDON + Vector2(-1.8, 1.77)
const CITY := LONDON + Vector2(1.8, 0.1)
## [nom, point, distance (unités), décalage de cap (degrés)]
const SHOTS := [
	["strategique", CITY, 60.0, 0.0],
	["transition", CITY, 8.0, 0.0],
	["vallee", CITY, 4.0, 0.0],
	["site", CITY, 1.8, 0.0],
	["tour", TOWER, 0.6, 20.0],
	["pont", BRIDGE, 0.6, -60.0],
	["pont_bas", BRIDGE + Vector2(0.15, 0.05), 0.35, -90.0],
	["st_pauls", PAULS, 0.6, 30.0],
	["westminster", WESTMINSTER, 0.8, 200.0],
	["toits", CITY + Vector2(-0.2, -0.1), 0.45, 20.0],
]


func _init() -> void:
	var out_dir := "user://vh6"
	var only := ""
	var fps := true
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg == "--no-fps":
			fps = false
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("vh6 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	for shot: Array in SHOTS:
		if only != "" and not (shot[0] as String) in only.split(","):
			continue
		var point: Vector2 = shot[1]
		var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
		var distance: float = shot[2]
		rig.look_at_point(focus, maxf(distance, rig.min_distance_at(focus)))
		rig.target_yaw = deg_to_rad(float(shot[3]))
		rig.snap()
		await _settle(terrain, settlements)
		for layer in root.find_children("*", "CanvasLayer", true, false):
			(layer as CanvasLayer).visible = false
		if terrain != null and terrain.material != null:
			terrain.material.set_shader_parameter("fog_enabled", false)
		for i in 4:
			await process_frame
		var image := root.get_viewport().get_texture().get_image()
		if image.get_width() > 960:
			image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("london_%s.jpg" % shot[0])
		image.save_jpg(path, 0.85)
		var lc: LandmarkCityLayer = settlements.landmark_cities if settlements != null else null
		print("VH6 shot %s d=%.2f (min %.2f) fade %.2f city %s" % [path, rig.target_distance, rig.min_distance_at(focus),
			lc.fade("set_londres") if lc != null else -1.0, JSON.stringify(lc.stats) if lc != null else "-"])
	if fps:
		for place: Array in [["london", CITY], ["london_pont", BRIDGE], ["rouen", ROUEN]]:
			for d: float in [1.6, 0.6]:
				var p: Vector2 = place[1]
				rig.look_at_point(Vector3(p.x, data.surface_world_at(p.x, p.y), p.y), d)
				rig.target_yaw = 0.0
				rig.snap()
				await _settle(terrain, settlements)
				var frames := 0
				var t0 := Time.get_ticks_usec()
				var worst := 0
				var last := t0
				while Time.get_ticks_usec() - t0 < 4_000_000:
					await process_frame
					var now := Time.get_ticks_usec()
					worst = maxi(worst, now - last)
					last = now
					frames += 1
				var seconds := (Time.get_ticks_usec() - t0) / 1e6
				print("VH6 fps %s d=%.1f : %.1f i/s (pire image %.1f ms)" % [place[0], d, frames / seconds, worst / 1000.0])
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
