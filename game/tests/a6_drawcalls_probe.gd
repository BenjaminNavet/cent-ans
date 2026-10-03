extends SceneTree

## Sonde du lot A6-L10 (P2) : appels de dessin, primitives et objets de la carte de campagne à
## plusieurs distances de caméra autour de Paris, puis ventilation par couche à la distance la plus
## proche (une couche masquée à la fois). Fenêtre réelle obligatoire (le rendu headless renvoie 0) :
##   godot --path game --resolution 1920x1080 --script res://tests/a6_drawcalls_probe.gd --
##     [--out=/chemin/sortie.json] [--distances=300,150,60,0] [--at=2213,3204] [--quality=high]
##     [--no-ablation] [--prop=<champ de MapPropScale>=<valeur>]…
## Distance 0 = au plus près permis par la caméra (`min_distance_at`). Les appels de dessin et
## primitives sont déterministes (à météo et heure figées), pas les temps d'image.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out_path := "user://a6_drawcalls.json"
	var distances: Array = [300.0, 150.0, 60.0, 0.0]
	var at := Vector2(2213.0, 3204.0)
	var ablation := true
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_path = arg.trim_prefix("--out=")
		elif arg.begins_with("--distances="):
			distances.clear()
			for item in arg.trim_prefix("--distances=").split(",", false):
				distances.append(float(item))
		elif arg.begins_with("--at="):
			var xz := arg.trim_prefix("--at=").split(",")
			at = Vector2(float(xz[0]), float(xz[1]))
		elif arg == "--no-ablation":
			ablation = false
		elif arg.begins_with("--prop="):
			var kv := arg.trim_prefix("--prop=").split("=")
			MapPropScale.shared().set(kv[0], float(kv[1]))
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
	if not map.get("load_ok"):
		push_error("A6 probe: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.get("map_data")
	var terrain: TerrainBuilder = map.get("terrain")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var report := {"quality": RenderQuality.current(), "window": [root.size.x, root.size.y], "at": [at.x, at.y], "views": [], "ablation": {}}
	var focus := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
	var closest := 0.0
	for d: float in distances:
		var dist := maxf(d, rig.min_distance_at(focus))
		rig.look_at_point(focus, dist)
		rig.snap()
		await _settle(terrain, settlements, vegetation)
		var info := await _read()
		info["distance"] = dist
		info["requested"] = d
		report["views"].append(info)
		print("A6 view d=%.1f %s" % [dist, JSON.stringify(info)])
		closest = dist
	if ablation:
		var base := await _read()
		report["ablation"]["_base"] = base
		for target: Node3D in _ablation_targets(map):
			var path := str(map.get_path_to(target))
			var was := target.visible
			target.visible = false
			var info := await _read()
			target.visible = was
			info["delta_draw_calls"] = int(base["draw_calls"]) - int(info["draw_calls"])
			info["delta_primitives"] = int(base["primitives"]) - int(info["primitives"])
			info["delta_objects"] = int(base["objects"]) - int(info["objects"])
			info["nodes"] = _count_visuals(target)
			report["ablation"][path] = info
			print("A6 ablation %s -%d calls -%d prims" % [path, info["delta_draw_calls"], info["delta_primitives"]])
		report["ablation_distance"] = closest
	var file := FileAccess.open(out_path, FileAccess.WRITE)
	file.store_string(JSON.stringify(report, "  "))
	file.close()
	print("A6 written %s" % out_path)
	map.queue_free()
	await process_frame
	quit(0)


## Couches masquables : enfants Node3D de la carte, puis enfants des conteneurs lourds.
func _ablation_targets(map: Node3D) -> Array:
	var out: Array = []
	for child in map.get_children():
		if not (child is Node3D):
			continue
		var node := child as Node3D
		out.append(node)
		if node.name in ["Settlements", "CampaignLife", "Vegetation"] or (node.get_child_count() > 1 and _count_visuals(node) > 300):
			for sub in node.get_children():
				if sub is Node3D and _count_visuals(sub) > 0:
					out.append(sub)
	return out


func _count_visuals(node: Node) -> int:
	var n := 1 if node is VisualInstance3D else 0
	for child in node.get_children():
		n += _count_visuals(child)
	return n


func _read() -> Dictionary:
	for i in 3:
		await process_frame
	await RenderingServer.frame_post_draw
	return {
		"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		"objects": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_OBJECTS_IN_FRAME),
	}


func _settle(terrain: TerrainBuilder, settlements: SettlementLayer, vegetation: Vegetation) -> void:
	for i in 30:
		await process_frame
	var guard := 0
	while guard < 2000:
		guard += 1
		var busy := false
		if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
			busy = true
		if ReliefLandcover.pending():
			busy = true
		if vegetation != null and (vegetation.pending_jobs() > 0 or vegetation.pending_regrounds() > 0):
			busy = true
		if not busy:
			break
		await process_frame
	if settlements != null:
		settlements.flush()
	for i in 60:
		await process_frame
