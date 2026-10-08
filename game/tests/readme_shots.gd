extends SceneTree

## Captures de la carte de campagne pour le README (villes et vues d'ensemble), fenêtré :
##   godot --path game --resolution 1920x1080 --script res://tests/readme_shots.gd -- --out=<dossier>
## Options : `--only=<nom>[,<nom>]`, `--ui` (garde l'interface), `--width=<px>` (1600 par défaut).
## Brouillard de guerre coupé, armées masquées sauf avec `--ui`. JPEG `<nom>.jpg`.

## [nom, point carte (px 4096), distance caméra (unités, < 0 = minimum + 5 %), lacet (degrés)]
const SHOTS := [
	["paris", Vector2(2213.2, 3203.9), 7.0, -15.0],
	["londres", Vector2(2018.0, 2767.5), 4.5, 0.0],
	["londres_site", Vector2(2018.0, 2767.5), -1.0, 0.0],
	["rouen", Vector2(2097.0, 3099.4), -1.0, 0.0],
	["rouen_est", Vector2(2097.0, 3099.4), -1.0, 140.0],
	["dijon", Vector2(2471.2, 3464.2), -1.0, 30.0],
	["reims", Vector2(2388.8, 3157.9), 3.0, 200.0],
	["carcassonne", Vector2(2127.3, 4071.7), -1.0, 300.0],
	["york", Vector2(1984.7, 2377.4), 3.0, 60.0],
	["orleans", Vector2(2152.0, 3346.5), -1.0, 0.0],
	["orleans_vallee", Vector2(2152.0, 3346.5), 5.0, 0.0],
]


func _init() -> void:
	var out_dir := "user://readme"
	var only := ""
	var keep_ui := false
	var width := 1600
	out_dir = CmdArgs.value("--out", out_dir)
	only = CmdArgs.value("--only", only)
	keep_ui = CmdArgs.has("--ui")
	width = int(CmdArgs.number("--width", width))
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("readme shots: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	rig.edge_pan_enabled = false
	if not keep_ui:
		(map.get("armies") as Node3D).visible = false
	for shot: Array in SHOTS:
		if only != "" and not (shot[0] as String) in only.split(","):
			continue
		var point: Vector2 = shot[1]
		var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
		var distance: float = shot[2]
		if distance < 0.0:
			distance = rig.min_distance_at(focus) * 1.05
		rig.target_yaw = deg_to_rad(float(shot[3]))
		rig.look_at_point(focus, distance)
		rig.snap()
		for i in 30:
			await process_frame
		var guard := 0
		while guard < 1500:
			guard += 1
			var busy := false
			if vegetation != null and vegetation.has_method("pending_jobs") and vegetation.pending_jobs() > 0:
				busy = true
			if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
				busy = true
			if ReliefLandcover.pending():
				busy = true
			if not busy:
				break
			await process_frame
		for i in 120:
			await process_frame
		# Masquée juste avant chaque rendu : l'interface de campagne se réaffiche seule selon le zoom.
		var hide_ui := func() -> void:
			for layer in root.find_children("*", "CanvasLayer", true, false):
				(layer as CanvasLayer).visible = keep_ui
		RenderingServer.frame_pre_draw.connect(hide_ui)
		if terrain != null and terrain.material != null:
			terrain.material.set_shader_parameter("fog_enabled", false)
		for i in 3:
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_viewport().get_texture().get_image()
		RenderingServer.frame_pre_draw.disconnect(hide_ui)
		if image.get_width() > width:
			image.resize(width, roundi(image.get_height() * float(width) / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("%s.jpg" % shot[0])
		image.save_jpg(path, 0.86)
		print("readme shot %s d=%.2f waited %d%s" % [path, distance, guard, " (NOT SETTLED)" if guard >= 1500 else ""])
	map.queue_free()
	await process_frame
	quit(0)
