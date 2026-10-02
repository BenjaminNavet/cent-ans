extends SceneTree

## GC2 (ADR 0158) : captures des villes 1:1 ; ajouter `--town-style=real` après `--` (le style par
## défaut est `maquette`, voir `gc_shots.gd`).
## Captures et mesure du lot VH7 (Orléans vers 1340-1429 à l'échelle 1:1, ADR 0078) : vue
## stratégique (colonie ordinaire), palier vallée, palier site, pont et Tourelles, Sainte-Croix,
## enceinte ; mesure d'images par seconde au-dessus d'Orléans et de Rouen à la même distance.
## Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/vh7_shots.gd -- --out=<dossier> --map-weather=clear
##   [--only=a,b] [--no-fps] [--year=1429]
## JPEG ≤ 960 px, `orleans_<vue>.jpg`.

const ORLEANS := Vector2(2152.671, 3346.053)  # Sainte-Croix (origine du fichier v2)
const ROUEN := Vector2(2096.54, 3099.88)
## [nom, point, distance (unités), cap (degrés)]
const SHOTS := [
	["strategique", ORLEANS, 60.0, 0.0],
	["transition", ORLEANS, 8.0, 0.0],
	["vallee", ORLEANS, 4.0, 0.0],
	["site", ORLEANS + Vector2(-0.2, 0.2), 1.6, 0.0],
	["pont", Vector2(2152.15, 3346.78), 0.6, 90.0],
	["tourelles", Vector2(2152.13, 3347.03), 0.32, 120.0],
	["sainte_croix", Vector2(2152.76, 3346.06), 0.3, 300.0],
	["enceinte", Vector2(2152.03, 3345.93), 0.45, 200.0],
	["loire", Vector2(2152.4, 3346.6), 0.9, 0.0],
]


func _init() -> void:
	var out_dir := "user://vh7"
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
		push_error("vh7 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var lc: LandmarkCityLayer = settlements.landmark_cities if settlements != null else null
	var suffix := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--year=") and lc != null:
			lc.set_year(int(arg.substr(7)))
			suffix = "_" + arg.substr(7)
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
		var path := out_dir.path_join("orleans_%s%s.jpg" % [shot[0], suffix])
		image.save_jpg(path, 0.85)
		print("VH7 shot %s d=%.2f (min %.2f) shown %s city %s" % [path, rig.target_distance, rig.min_distance_at(focus),
			str(lc.is_shown("set_orleans")) if lc != null else "-", JSON.stringify(lc.stats) if lc != null else "-"])
	if fps:
		for place: Array in [["orleans", ORLEANS + Vector2(-0.2, 0.2)], ["rouen", ROUEN]]:
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
				print("VH7 fps %s d=%.1f : %.1f i/s (pire image %.1f ms)" % [place[0], d, frames / seconds, worst / 1000.0])
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
