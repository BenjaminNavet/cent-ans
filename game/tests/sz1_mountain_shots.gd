extends SceneTree

## Captures de recette du lot SZ1 (défaut S1 de ZG7c : haute montagne au palier vallée), inspirées
## de `zg7c_recette_shots.gd` : montagnes (Pyrénées, Alpes, pays de Galles) et témoins de
## non-régression (Crécy, falaises normandes, Massif central) aux paliers vallée (d = 6) et site
## (distance minimale + 5 %). Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/sz1_mountain_shots.gd -- --out=<dossier> [--prefix=avant_]
## Options : `--only=<nom>[,<nom>]`, `--tiers=vallee,site`, `--no-crest` (caméra sans la garde des
## crêtes voisines), et celles de la carte (`--map-weather=clear`, `--no-relief-exaggeration`…).
## Brouillard de guerre et interface coupés. Imprime par capture la hauteur de la caméra, le sol
## sous elle et le plus haut sol affiché autour d'elle (rayon ≤ distance).

## [nom, point carte (px 4096)]
const PLACES := [
	["pyrenees", Vector2(1862.0, 4098.0)],
	["alpes", Vector2(2652.0, 3684.0)],
	["galles", Vector2(1694.0, 2468.0)],
	["massif_central", Vector2(2233.0, 3685.0)],
	["crecy", Vector2(2191.5, 2986.0)],
	["falaises_normandes", Vector2(2018.0, 3052.0)],
]
## Distances des paliers (unités) ; « site » = distance minimale au point (+ 5 %).
const TIERS := {"strat": 60.0, "vallee": 6.0, "site": -1.0}


func _init() -> void:
	var out_dir := "user://sz1"
	var only := ""
	var prefix := ""
	var no_crest := false
	var tiers := PackedStringArray(["vallee", "site"])
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
		elif arg.begins_with("--tiers="):
			tiers = arg.substr(8).split(",")
		elif arg == "--no-crest":
			no_crest = true
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("sz1 shots: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	# Interface créée en différé : on la laisse apparaître avant de la masquer à chaque capture.
	for i in 120:
		await process_frame
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--crest-radius=") and rig.profile != null:
			rig.profile = rig.profile.duplicate()
			rig.profile.crest_radius_factor = float(arg.substr(15))
	if no_crest and rig.profile != null:
		rig.profile = rig.profile.duplicate()
		rig.profile.crest_samples = 0
	for place in PLACES:
		if only != "" and not (place[0] as String) in only.split(","):
			continue
		var point: Vector2 = place[1]
		for tier: String in tiers:
			var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
			var distance: float = TIERS[tier]
			if distance < 0.0:
				distance = rig.min_distance_at(focus) * 1.05
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
			for i in 60:
				await process_frame
			for layer in root.find_children("*", "CanvasLayer", true, false):
				(layer as CanvasLayer).visible = false
			if terrain != null and terrain.material != null:
				terrain.material.set_shader_parameter("fog_enabled", false)
			for i in 4:
				await process_frame
			var image := root.get_viewport().get_texture().get_image()
			if image.get_width() > 960:
				image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
			var path := out_dir.path_join("%s%s_%s.jpg" % [prefix, place[0], tier])
			image.save_jpg(path, 0.82)
			var level := terrain.quadtree.chunk_top(terrain.chunk_index_at(point.x, point.y)) if terrain != null and terrain.quadtree != null else -1
			var eye := rig.camera.global_position
			var active := root.get_viewport().get_camera_3d()
			if active != rig.camera or not active.global_position.is_equal_approx(eye):
				print("SZ1 note: active camera %s at %s, rig camera at %s" % [active.get_path(), active.global_position, eye])
			var highest := -INF
			for k in 16:
				var angle := TAU * k / 16.0
				for r in [0.25, 0.5, 1.0]:
					highest = maxf(highest, terrain.surface_height_at(eye.x + distance * r * cos(angle), eye.z + distance * r * sin(angle)))
			print("SZ1 shot %s d=%.2f scale ×%.2f gain %.2f squash %.2f E%d eye %.2f ground %.2f focus %.2f highest %.2f pitch %.1f%s" % [
				path.get_file(), distance, MapData.vertical_exaggeration(), MapData.relief_gain(), MapData.relief_squash(), level,
				eye.y, terrain.surface_height_at(eye.x, eye.z), rig.focus.y, highest, rig.pitch_deg(),
				" (NOT SETTLED)" if guard >= 1500 else ""])
	map.queue_free()
	await process_frame
	quit(0)
