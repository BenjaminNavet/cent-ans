extends SceneTree

## Sonde du clignotement des arbres de la carte quand la caméra bouge. Fenêtre réelle :
##   godot --path game --resolution 640x400 --script res://tests/tree_flicker_probe.gd --
##   --hide-armies --map-weather=clear [--views=x,z,d;…] [--step=0.15] [--frames=120] [--yaw=0]
## Par vue : panoramique lent (`step` unités par image le long de x, ou rotation de `yaw` degrés
## par image) ; chaque image, changements de palier (LOD), d'ombres, de visibilité des parties,
## recalages et tuiles installées ; « blink » = part des pixels qui changent puis reviennent
## (image t différente de t−1 et t+1 alors que t−1 ≈ t+1).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var views: Array = [[2180.0, 3333.0, 60.0], [2180.0, 3333.0, 150.0], [2180.0, 3333.0, 300.0]]
	var step := 0.15
	var frames := 120
	var yaw := 0.0
	var keys := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--step="):
			step = float(arg.trim_prefix("--step="))
		elif arg.begins_with("--frames="):
			frames = int(arg.trim_prefix("--frames="))
		elif arg == "--keys":
			keys = true
		elif arg.begins_with("--yaw="):
			yaw = float(arg.trim_prefix("--yaw="))
		elif arg.begins_with("--views="):
			views.clear()
			for item in arg.trim_prefix("--views=").split(";", false):
				var parts := item.split(",")
				views.append([float(parts[0]), float(parts[1]), float(parts[2])])
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
	for i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.get("map_data")
	var terrain: TerrainBuilder = map.get("terrain")
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	print("FLICKER style %s" % MapPropScale.tree_style())
	for pass_index in 2 * views.size():
		var view: Array = views[pass_index / 2]
		var trees_on := pass_index % 2 == 0
		vegetation.enabled = trees_on
		if vegetation.forest_detail != null:
			vegetation.forest_detail.visible = trees_on
		var focus := Vector3(view[0], data.surface_world_at(view[0], view[1]), view[1])
		rig.look_at_point(focus, maxf(float(view[2]), rig.min_distance_at(focus)))
		rig.snap()
		for i in 240:
			await process_frame
		var prev_state := _state(vegetation)
		var prev_levels := _levels(terrain)
		var prev_regrounds := int(vegetation.stats["regrounds"])
		var images: Array[Image] = []
		var known := {}
		for index in (vegetation.get("_tiles") as Dictionary):
			known[index] = true
		var totals := {"lod": 0, "shadow": 0, "vis": 0, "levels": 0, "regrounds": 0}
		for f in frames:
			if keys:
				Input.action_press("map_pan_right")
			elif yaw != 0.0:
				rig.set("yaw", float(rig.get("yaw")) + deg_to_rad(yaw))
			else:
				focus.x += step
				focus.y = data.surface_world_at(focus.x, focus.z)
				rig.look_at_point(focus, rig.distance)
			if not keys:
				rig.snap()
			await process_frame
			await RenderingServer.frame_post_draw
			var image := root.get_viewport().get_texture().get_image()
			image.convert(Image.FORMAT_L8)
			images.append(image)
			var state := _state(vegetation)
			var levels := _levels(terrain)
			var diff := {"lod": 0, "shadow": 0, "vis": 0}
			for key in state:
				if prev_state.has(key):
					var a: Array = prev_state[key]
					var b: Array = state[key]
					diff["lod"] += int(a[0] != b[0])
					# FL1 : seulement les parties visibles aux deux images (une partie qui apparaît au bord du
					# fondu reçoit son ombre en même temps, arbres de taille nulle).
					diff["shadow"] += int(a[1] != b[1] and a[2] and b[2])
					diff["vis"] += int(a[2] != b[2])
			var level_changes := 0
			for i in levels.size():
				level_changes += int(i < prev_levels.size() and levels[i] != prev_levels[i])
			var regrounds := int(vegetation.stats["regrounds"]) - prev_regrounds
			prev_regrounds += regrounds
			for k in diff:
				totals[k] += diff[k]
			totals["levels"] += level_changes
			totals["regrounds"] += regrounds
			var err := _visible_ground_error(vegetation, terrain)
			var mat2 := vegetation.foliage_material()
			if keys:
				print("  f%03d rig %.2f focus (%.1f, %.2f, %.1f) fade_end %.1f density %.3f origin %s" % [f, rig.distance, rig.focus.x, rig.focus.y, rig.focus.z,
					mat2.get_shader_parameter("fade_end"), mat2.get_shader_parameter("density"), mat2.get_shader_parameter("view_origin")])
			var tiles: Dictionary = vegetation.get("_tiles")
			for index in tiles:
				if not known.has(index):
					known[index] = true
					var node := tiles[index]["node"] as Node3D
					if node.visible:
						var shown := 0
						for mmi in node.find_children("*", "MultiMeshInstance3D", true, false):
							if (mmi as Node3D).is_visible_in_tree():
								shown += (mmi as MultiMeshInstance3D).multimesh.visible_instance_count
						var mat := mat2
						var origin: Vector3 = mat.get_shader_parameter("view_origin")
						var fade_start: float = mat.get_shader_parameter("fade_start")
						var fade_end: float = mat.get_shader_parameter("fade_end")
						var px: int = vegetation.get("chunk_px")
						var cx: int = vegetation.get("chunks_x")
						var rect := Rect2((index % cx) * px, (index / cx) * px, px, px)
						var p := Vector2(origin.x, origin.z)
						var near := Vector2(clampf(p.x, rect.position.x, rect.end.x), clampf(p.y, rect.position.y, rect.end.y)).distance_to(p)
						var far := 0.0
						for corner in [rect.position, rect.end, Vector2(rect.position.x, rect.end.y), Vector2(rect.end.x, rect.position.y)]:
							far = maxf(far, (corner as Vector2).distance_to(p))
						print("  f%03d POP tile %d visible on install, %d instances drawn, tile d %.0f-%.0f, fade %.0f-%.0f, prefetch %.0f, jobs %d" % [f, index, shown, near, far, fade_start, fade_end,
							vegetation.get("tile_prefetch_distance"), vegetation.pending_jobs()])
			if diff["lod"] + diff["shadow"] + diff["vis"] + level_changes + regrounds > 0 or err > 0.1:
				print("  f%03d lod %d shadow %d vis %d levels %d regrounds %d ground_err %.2f" % [f, diff["lod"], diff["shadow"], diff["vis"], level_changes, regrounds, err])
			prev_state = state
			prev_levels = levels
		var blinks: Array = []
		for f in range(1, images.size() - 1):
			blinks.append(_blink(images[f - 1], images[f], images[f + 1]))
		var worst := 0.0
		var mean := 0.0
		for b: float in blinks:
			worst = maxf(worst, b)
			mean += b / blinks.size()
		print("FLICKER trees %s view (%.0f, %.0f) rig %.0f : %s ground_err %.2f blink mean %.4f max %.4f" % [trees_on, view[0], view[1], rig.distance, JSON.stringify(totals),
			vegetation.max_ground_error(13), mean, worst])
		var line := ""
		for b: float in blinks:
			line += "%.3f " % b
		print("  blink per frame: %s" % line)
	map.queue_free()
	await process_frame
	quit(0)


## Palier, ombre et visibilité par partie (clé = chemin du nœud).
func _state(vegetation: Vegetation) -> Dictionary:
	var out := {}
	for part in vegetation.find_children("Part_*", "Node3D", true, false):
		var shadow := 0
		for child in part.get_children():
			if child is MultiMeshInstance3D and (child as MultiMeshInstance3D).cast_shadow != GeometryInstance3D.SHADOW_CASTING_SETTING_OFF:
				shadow += 1
		var mesh_id := 0
		for child in part.get_children():
			if child is MultiMeshInstance3D:
				mesh_id = (child as MultiMeshInstance3D).multimesh.mesh.get_instance_id()
				break
		out[str(part.get_path())] = [mesh_id, shadow, (part as Node3D).is_visible_in_tree()]
	return out


## Écart pied/sol maximal (unités) des tuiles d'arbres visibles.
func _visible_ground_error(vegetation: Vegetation, terrain: TerrainBuilder) -> float:
	var worst := 0.0
	var tiles: Dictionary = vegetation.get("_tiles")
	for index in tiles:
		var entry: Dictionary = tiles[index]
		if not (entry["node"] as Node3D).visible:
			continue
		for buffer: PackedFloat32Array in entry["buffers"]:
			var k := 0
			while k < buffer.size():
				var height := Vector3(buffer[k + 1], buffer[k + 5], buffer[k + 9]).length()
				var foot := buffer[k + 7] + VegetationTileJob.GROUND_SINK * height
				worst = maxf(worst, absf(foot - terrain.surface_height_at(buffer[k + 3], buffer[k + 11])) / maxf(height, 1e-4))
				k += VegetationTileJob.FLOATS_PER_INSTANCE * 2
	return worst


func _levels(terrain: TerrainBuilder) -> PackedInt32Array:
	var out := PackedInt32Array()
	if terrain == null:
		return out
	var count: int = terrain.get("chunks_x") * terrain.get("chunks_y") if terrain.get("chunks_x") != null else 0
	for i in count:
		out.append(terrain.chunk_level(i))
	return out


## Part des pixels qui clignotent : t s'écarte de t−1 et t+1 de plus de 24/255 dans le même sens
## alors que t−1 et t+1 restent proches.
func _blink(a: Image, b: Image, c: Image) -> float:
	var da := a.get_data()
	var db := b.get_data()
	var dc := c.get_data()
	var count := 0
	for i in range(0, db.size(), 3):
		var x := int(da[i])
		var y := int(db[i])
		var z := int(dc[i])
		if absi(x - z) < 8 and ((y - x > 24 and y - z > 24) or (x - y > 24 and z - y > 24)):
			count += 1
	return count * 3.0 / db.size()
