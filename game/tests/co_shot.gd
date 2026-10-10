extends SceneTree

## Captures de contrôle du chantier CO (fenêtré, via tools/godot_bg.sh) : une cité de province pleine
## (5 colonies) sélectionnée — pastilles des colonies de la province + onglet « Bâtiments » — puis un village.
## Usage : tools/godot_bg.sh --path game --script res://tests/co_shot.gd -- --out=/chemin/absolu


const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _frames(count: int) -> void:
	for _i in count:
		await process_frame


func _shot(path: String) -> void:
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(path)
	print("co_shot: -> %s" % path)


func _init() -> void:
	var out := CmdArgs.value("--out", "user://co_shot")
	DirAccess.make_dir_recursive_absolute(out)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _frames(10)
	var layer: SettlementLayer = map.settlement_layer
	var city := ""
	var village := ""
	for entry: Dictionary in layer.data.settlements:
		if city == "" and entry["kind"] == "city" and layer.data.province_mates(entry["id"]).size() >= 4:
			city = entry["id"]
		if village == "" and entry["kind"] == "village":
			village = entry["id"]
	for pick in [[city, 260.0, "city"], [village, 40.0, "village"]]:
		var id: String = pick[0]
		var entry: Dictionary = layer.data.settlements[layer.data.index_by_id[id]]
		var px: Vector2 = entry["px"]
		map.camera_rig.look_at_point(Vector3(px.x, map.map_data.surface_world_at(px.x, px.y), px.y), float(pick[1]))
		map.camera_rig.snap()
		layer.select(id)
		await _frames(150)
		var panel: Control = map.settlements_ctl.panel
		panel.tabs.current_tab = panel.TAB_BUILDINGS
		await _frames(10)
		var scroller: Node = panel.get_parent()
		while scroller != null and not scroller is ScrollContainer:
			scroller = scroller.get_parent()
		if scroller != null:
			(scroller as ScrollContainer).ensure_control_visible(panel.buildings_view)
		await _frames(30)
		await _shot(out.path_join("co_%s.png" % pick[2]))
	map.queue_free()
	await process_frame
	quit(0)
