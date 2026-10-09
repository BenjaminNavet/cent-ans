extends TestCase

## Test headless du lot M5a (vision par rayon) sur la vraie simulation et les vraies données :
##  1. brouillard par case actif : texture de vue (R8, une case par cellule du cœur) posée sur le terrain et la minicarte,
##     une partie seulement de la carte vue ;
##  2. une armée étrangère placée hors de vue n'a ni marqueur ni point sur la minicarte ;
##  3. une armée du joueur placée à 15 km la révèle.
## Usage : godot --headless --path game --script res://tests/m5a_vision_ui_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_vision"),
			"CampaignSim.get_vision missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "map/fog_of_war", true, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var ctl: Node = map.minimap_ctl
	var player := str(map.player_faction)

	# 1. Brouillard par case.
	map.refresh_all()
	check(ctl.fog_active and ctl.fog_by_cell, "per-cell fog should be active")
	var texture: ImageTexture = ctl.fog_texture
	if check(texture != null, "fog texture missing"):
		var image := texture.get_image()
		check(image.get_width() >= 256 and image.get_height() >= 256, "fog texture should cover the map grid, got %s" % [image.get_size()])
		check(image.get_format() == Image.FORMAT_R8, "fog texture should be R8")
	check(ctl.seen_share > 0.0 and ctl.seen_share < 0.6, "part of the map only should be seen (%.3f)" % ctl.seen_share)
	var terrain_material: ShaderMaterial = map.terrain.material
	check(terrain_material.get_shader_parameter("fog_enabled") == true and terrain_material.get_shader_parameter("fog_by_cell") == true,
		"terrain shader should use the per-cell fog")
	check(terrain_material.get_shader_parameter("fog_cells") is Texture2D, "terrain shader should sample the fog texture")
	var minimap_material: ShaderMaterial = ctl.minimap.get("_material")
	check(minimap_material.get_shader_parameter("fog_by_cell") == true, "minimap should use the per-cell fog")
	check(ctl.minimap.visible_province_count() > 0 and ctl.minimap.visible_province_count() < map.map_data.province_count,
		"visible provinces should be a part of the map (%d)" % ctl.minimap.visible_province_count())

	# 2. Armée étrangère hors de vue.
	var foreign := ""
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if str(army.get("faction", "")) == "fac_england":
			foreign = str(army_id)
			break
	if not check(foreign != "", "no English army"):
		map.queue_free()
		return
	var rules: Dictionary = sim.call("get_movement_rules")
	var px_per_km := float(rules.get("px_per_km", 1.39))
	var hidden_point := Vector2(-1, -1)
	var size: Vector2 = Vector2(map.map_data.size)
	for gy in range(8, 56):
		for gx in range(8, 56):
			var point := Vector2(size.x * gx / 64.0, size.y * gy / 64.0)
			if not map.map_data.is_land_px(int(point.x), int(point.y)):
				continue
			if sim.call("is_point_visible", player, point):
				continue
			if sim.call("is_point_visible", player, point + Vector2(15.0 * px_per_km, 0.0)):
				continue
			hidden_point = point
			break
		if hidden_point.x >= 0.0:
			break
	if not check(hidden_point.x >= 0.0, "no unseen land point found"):
		map.queue_free()
		return
	check(sim.call("debug_place_army", foreign, hidden_point.x, hidden_point.y), "debug_place_army failed")
	map.refresh_all()
	check(not ctl.visible_armies.has(foreign), "the English army out of sight should not be visible")
	check(not map.armies.has_army(foreign), "the English army out of sight should have no marker")
	var dots_hidden: int = ctl.minimap.army_dot_count()

	# 3. Une armée du joueur à 15 km la révèle.
	var own: String = map.player_army_ids()[0]
	var watch := hidden_point + Vector2(15.0 * px_per_km, 0.0)
	check(sim.call("debug_place_army", own, watch.x, watch.y), "debug_place_army (own) failed")
	map.refresh_all()
	check(ctl.visible_armies.has(foreign), "the English army 15 km from a French army should be visible")
	check(map.armies.has_army(foreign), "the seen English army should have a marker")
	check(ctl.minimap.army_dot_count() == dots_hidden + 1, "the seen English army should get a minimap dot (%d → %d)" % [dots_hidden, ctl.minimap.army_dot_count()])
	print("m5a_vision_ui_test: seen share %.3f, %d visible provinces" % [ctl.seen_share, ctl.minimap.visible_province_count()])
	map.queue_free()
	await process_frame
