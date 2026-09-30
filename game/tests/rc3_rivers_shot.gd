extends SceneTree

## Captures du chantier RC (fleuves lisibles, noms des cours d'eau) aux mêmes cadrages avant/après.
## Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/rc3_rivers_shot.gd -- --out=<dossier> [--only=<nom>]
## JPEG 640 px de large. Imprime aussi le nombre d'étiquettes de fleuves posées.

## [nom, point carte (px), distance caméra (unités)]
const VIEWS := [
	["france", Vector2(2100, 3380), 900.0],
	["loire", Vector2(2150, 3350), 220.0],
	["orleans", Vector2(2152, 3346), 60.0],
]


func _init() -> void:
	var out_dir := "user://rc3"
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
		push_error("rc3 shots: campaign map failed to load")
		quit(1)
		return
	var rivers: RiversRenderer = map.get("rivers")
	print("RC3 river labels: %d" % (rivers.labels.count() if rivers != null and rivers.labels != null else 0))
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	for view in VIEWS:
		if only != "" and view[0] != only:
			continue
		var point: Vector2 = view[1]
		rig.look_at_point(Vector3(point.x, data.surface_world_at(point.x, point.y), point.y), view[2])
		rig.snap()
		for i in 30:
			await process_frame
		var guard := 0
		while guard < 900 and terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
			guard += 1
			await process_frame
		for i in 30:
			await process_frame
		if rivers != null:
			var cam := root.get_viewport().get_camera_3d()
			var lab: Label3D = rivers.labels.get_child(0) if rivers.labels != null and rivers.labels.get_child_count() > 0 else null
			print("RC3 diag %s: rivers vis=%s tree=%s aabb=%s cam=%s lab=%s" % [view[0], rivers.visible, rivers.is_visible_in_tree(), rivers.get_aabb(), cam.global_position if cam else null, ("%s d=%.0f end=%.0f vis=%s" % [lab.text, cam.global_position.distance_to(lab.global_position), lab.visibility_range_end, lab.is_visible_in_tree()]) if lab != null and cam != null else "none"])
		for layer in root.find_children("*", "CanvasLayer", true, false):
			(layer as CanvasLayer).visible = false
		for i in 3:
			await process_frame
		var image := root.get_viewport().get_texture().get_image()
		if rivers != null and OS.get_cmdline_user_args().has("--probe"):
			var cam := root.get_viewport().get_camera_3d()
			# Pixel au centre de la Loire à Orléans, et à 3 px de là (terre), puis à une étiquette.
			for probe in [Vector2(2140.0, 3348.0), Vector2(2170.0, 3342.0)]:
				var nearest := Vector2.ZERO
				var best := 1e9
				for r in rivers.rivers:
					if str(r["name"]) != "Loire":
						continue
					for q: Vector2 in r["points"]:
						if q.distance_to(probe) < best:
							best = q.distance_to(probe)
							nearest = q
				var w3 := Vector3(nearest.x, data.surface_world_at(nearest.x, nearest.y), nearest.y)
				var sp := cam.unproject_position(w3)
				var c := image.get_pixelv(Vector2i(clampi(int(sp.x), 0, image.get_width() - 1), clampi(int(sp.y), 0, image.get_height() - 1)))
				var c2 := image.get_pixelv(Vector2i(clampi(int(sp.x) + 12, 0, image.get_width() - 1), clampi(int(sp.y), 0, image.get_height() - 1)))
				print("RC3 probe loire %s -> screen %s behind=%s color %s / +12px %s" % [nearest, sp, cam.is_position_behind(w3), c, c2])
			var best_lab: Label3D = null
			var best_d := 1e9
			for lab: Label3D in rivers.labels.get_children():
				var d := cam.global_position.distance_to(lab.global_position)
				if d < lab.visibility_range_end and d < best_d and not cam.is_position_behind(lab.global_position):
					if Rect2(Vector2(40, 20), Vector2(image.get_size()) - Vector2(80, 40)).has_point(cam.unproject_position(lab.global_position)):
						best_d = d
						best_lab = lab
			if best_lab != null:
				var sp2 := cam.unproject_position(best_lab.global_position)
				var ink := 0
				for dy in range(-8, 9):
					for dx in range(-40, 41):
						var px := image.get_pixelv(Vector2i(int(sp2.x) + dx, int(sp2.y) + dy))
						if px.b > px.r + 0.12 and px.b > 0.3:
							ink += 1
				print("RC3 probe label %s at %s d=%.0f blue-ink pixels %d / 1377" % [best_lab.text, sp2, best_d, ink])
		var w := 640
		image.resize(w, int(float(image.get_height()) * float(w) / float(image.get_width())), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("rc3_%s.jpg" % view[0])
		image.save_jpg(path, 0.85)
		print("RC3 shot: %s" % path)
	quit(0)
