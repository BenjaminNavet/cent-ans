extends "res://tests/smoke_base.gd"

## Sections « carte » : scène de carte sur les fixtures, minicarte et brouillard, routes maritimes.
## Découpage de `smoke.gd` (SC GT7) : les sections se chaînent par héritage et partagent l'état.
## Ne se lance pas seul : point d'entrée `res://tests/smoke.gd`.

func _run_campaign_map() -> void:
	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	if scene == null:
		_fail("cannot load campaign_map.tscn")
		return
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame

	if not _check(map.load_ok, "campaign map failed to load: %s" % (map.map_data.load_error if map.map_data else "no data")):
		return
	var data: MapData = map.map_data
	_check(data.height_bpp == 2, "heightmap should be decoded as 16-bit, got bpp %d" % data.height_bpp)
	print("smoke map: heightmap decoder = %s" % data.height_decoder)
	_check(data.size == Vector2i(512, 512), "fixture size should be 512x512, got %s" % data.size)
	_check(data.province_count == 6, "expected 6 provinces, got %d" % data.province_count)
	_check(data.index_of_id("prov_synth_3") == 3, "index_of_id(prov_synth_3) should be 3")
	var terrain: TerrainBuilder = map.terrain
	_check(terrain.chunk_count() == terrain.chunks_x * terrain.chunks_y and terrain.chunks_x * terrain.chunk_px >= terrain.map_data.size.x and terrain.chunks_y * terrain.chunk_px >= terrain.map_data.size.y,
		"expected %d × %d terrain chunks covering the map, got %d" % [terrain.chunks_x, terrain.chunks_y, terrain.chunk_count()])
	for chunk in terrain.get_children():
		if chunk is MeshInstance3D and (chunk.mesh == null or chunk.mesh.get_surface_count() == 0):
			_fail("terrain chunk %s has no mesh surface" % chunk.name)
			break
	_check(data.rivers.size() == 2, "expected 2 rivers, got %d" % data.rivers.size())
	_check(not data.coastlines.is_empty(), "coastline missing")
	_check(map.get_node_or_null("Cities") == null, "DV: province name markers (CityMarkers) should be gone")

	# Picking : centroïde de la province 3, en coordonnées monde puis via l'écran.
	var province: Dictionary = data.get_province(PICK_PROVINCE_INDEX)
	if not _check(not province.is_empty(), "province %d missing from provinces.geojson" % PICK_PROVINCE_INDEX):
		return
	var centroid: Vector2 = province["centroid"]
	var picker: ProvincePicker = map.picker
	var direct := picker.province_at_world(centroid.x, centroid.y)
	_check(direct == PICK_PROVINCE_INDEX, "province_at_world at centroid returned %d, expected %d" % [direct, PICK_PROVINCE_INDEX])

	var world := Vector3(centroid.x, data.surface_world_at(centroid.x, centroid.y), centroid.y)
	var rig: CampaignCamera = map.camera_rig
	rig.look_at_point(world, 120.0)
	rig.snap()
	await process_frame
	var camera: Camera3D = map.camera
	var screen := camera.unproject_position(world)
	var picked := picker.pick_screen(screen)
	_check(picked == PICK_PROVINCE_INDEX, "pick_screen at province %d centroid (%s) returned %d" % [PICK_PROVINCE_INDEX, screen, picked])
	var hit := picker.pick_ray_screen(screen)
	if _check(not hit.is_empty(), "pick_ray_screen returned no hit"):
		var error := Vector2(hit["x"], hit["z"]).distance_to(centroid)
		_check(error < 1.5, "ray refinement error %.2f px too large" % error)

	# Sélection de province → panneau visible avec le nom.
	picker.select_index(PICK_PROVINCE_INDEX)
	await process_frame
	_check(map.ui.province_panel.visible, "province panel should be visible after selection")
	_check(map.ui.province_panel.name_label.text == province["name"], "province panel name mismatch: '%s'" % map.ui.province_panel.name_label.text)

	print("smoke map: %s" % JSON.stringify(map.startup_stats))
	if failures == 0:
		print("smoke OK: map loaded, %d chunks, picked province %d" % [terrain.chunk_count(), picked])
	map.queue_free()
	await process_frame


func _run_minimap_fog() -> void:
	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "minimap: campaign scene failed to start"):
		map.queue_free()
		return
	var ctl: Node = map.minimap_ctl  # non typé : compilé après les autoloads
	if not _check(ctl != null and ctl.minimap != null and ctl.minimap.is_inside_tree(), "minimap: controller or minimap missing"):
		map.queue_free()
		return
	var minimap: Control = ctl.minimap
	map.ui.layout_hud()
	await process_frame
	_check(minimap.visible and minimap.size.x > 100.0 and minimap.size.y > 80.0, "minimap should be visible with a sensible size, got %s" % minimap.size)
	# PO1 : minicarte dans la zone `MINIMAP` de `UiLayout` (bas droite), cloche à sa gauche, lettres
	# dans la zone `SIDE_PANEL`.
	var mini_rect := minimap.get_global_rect()
	var bell_rect: Rect2 = map.ui.end_turn_cluster.get_global_rect()
	_check(not mini_rect.intersects(bell_rect), "minimap %s overlaps the end-turn cluster %s" % [mini_rect, bell_rect])
	var layout: Node = root.get_node("/root/UiLayout")
	_check(minimap.get_parent() == layout.zone_node(layout.Zone.MINIMAP), "minimap should sit in the MINIMAP zone")
	_check(layout.occupants(layout.Zone.SIDE_PANEL).has(map.ui.news_letters), "news letters should sit in the SIDE_PANEL zone")
	_check(minimap.army_dot_count() > 0, "minimap should show the player's armies")
	# Clic : la caméra vise le point cliqué (coordonnées carte).
	var local: Vector2 = minimap.map_rect().size * Vector2(0.25, 0.7)
	var expected: Vector2 = minimap.view_to_map(local)
	minimap.click_at(local)
	var focus: Vector3 = map.camera_rig.target_focus
	_check(Vector2(focus.x, focus.z).distance_to(expected) < 1.0, "minimap click should recenter the camera on %s, got %s" % [expected, focus])
	await process_frame
	_check(ctl.view_frame().size() == 4, "camera frame should have 4 corners")
	if map.sim.has_method("get_visible_provinces"):
		_check(ctl.fog_active, "fog of war should be active by default with the real simulation")
		var hidden := ""
		for index in range(1, map.map_data.province_count + 1):
			var id := str(map.map_data.get_province(index).get("id", ""))
			if id != "" and not ctl.is_province_visible(id):
				hidden = id
				break
		_check(hidden != "", "at least one province should be hidden by the fog")
		_check(minimap.visible_province_count() > 0 and minimap.visible_province_count() < map.map_data.province_count,
			"minimap fog mask should cover part of the map (%d visible)" % minimap.visible_province_count())
		_check(map.terrain.material.get_shader_parameter("fog_enabled") == true, "terrain shader fog should be enabled")
		_check(ctl.fog_by_cell and map.terrain.material.get_shader_parameter("fog_by_cell") == true and ctl.fog_texture != null,
			"terrain fog should come from the per-cell vision texture (M5a)")
		for army_id in map.sim.call("get_army_ids"):
			var army: Dictionary = map.sim.call("get_army", army_id)
			# M5a : vue par case ; une armée étrangère dont le point n'est pas vu n'a pas de marqueur.
			if not ctl.is_army_visible(str(army_id), army):
				_check(not map.armies.has_army(army_id), "foreign army %s out of sight should have no marker" % army_id)
		var settings: Node = root.get_node_or_null("/root/Settings")
		if settings != null:
			settings.call("set_value", "map/fog_of_war", false, false)
			_check(not ctl.fog_active and ctl.is_province_visible(hidden), "fog setting off should reveal %s" % hidden)
			settings.call("set_value", "map/fog_of_war", true, false)
			_check(ctl.fog_active, "fog setting back on")
	if failures == 0:
		print("smoke OK: minimap (%d dots, %d visible provinces), fog %s" % [minimap.army_dot_count(), minimap.visible_province_count(), "on" if ctl.fog_active else "off"])
	map.queue_free()
	await process_frame


## SL1 (ADR 0139) : routes maritimes (pont `get_sea_lanes` / `is_sea_link`, géométrie de la
## couche, infobulle). Vraie simulation uniquement.
func _run_sea_lanes() -> void:
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_sea_lanes")):
		print("smoke sea lanes: skipped, CampaignSim has no get_sea_lanes (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), "fac_england", 1337), "sea lanes: new_campaign failed"):
		return
	var lanes: Array = sim.call("get_sea_lanes")
	_check(lanes.size() >= 20, "get_sea_lanes should list >= 20 lanes, got %d" % lanes.size())
	var wine: Dictionary = {}
	for lane_variant in lanes:
		if str((lane_variant as Dictionary).get("id", "")) == "lane_southampton_bordeaux":
			wine = lane_variant
	_check(not wine.is_empty(), "lane_southampton_bordeaux expected")
	_check(bool(sim.call("is_sea_link", "set_southampton", "set_bordeaux")), "Southampton-Bordeaux should be a sea link")
	_check(not bool(sim.call("is_sea_link", "set_southampton", "set_paris")), "Southampton-Paris is no sea link")
	var map_data := MapData.new()
	map_data.map_dir = _project_root().path_join("data/map")
	var layer := SeaLaneLayer.new()
	layer.setup(map_data)
	_check(layer.has_lanes(), "sea_lanes_px.json should load")
	_check(layer.lane_points("set_bordeaux", "set_southampton").size() > 2, "lane geometry should reverse")
	var tip := SeaLaneLayer.tooltip(wine)
	_check(tip.contains("Gascogne"), "sea lane tooltip should name the sea: %s" % tip)
	layer.free()
	_run_sea_voyage(sim)
	if failures == 0:
		print("smoke OK: sea lanes, %d lanes" % lanes.size())


## EM (ADR 0167) : voyage de plusieurs traversées depuis un port, clic sur la mer.
func _run_sea_voyage(sim: Object) -> void:
	if not sim.has_method("sea_voyage"):
		print("smoke sea voyage: skipped, CampaignSim has no sea_voyage (run core/build.sh)")
		return
	var army_id := ""
	for id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", str(id))
		if str(army.get("faction", "")) == "fac_england":
			army_id = str(id)
			break
	if not _check(army_id != "", "sea voyage: an English army expected"):
		return
	_check(bool(sim.call("debug_place_army", army_id, 0.0, 0.0)), "sea voyage: debug_place_army failed")
	_check((sim.call("sea_voyage", army_id, "set_bordeaux") as PackedStringArray).is_empty(), "an army in the field has no voyage")
	_check(str(sim.call("sea_port_near", army_id, 0.0, 0.0)) == "", "an army in the field has no port to sail to")
	print("smoke OK: sea voyage bridge (field army refused)")
