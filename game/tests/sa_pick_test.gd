extends SceneTree

## Test headless du lot SA (ADR 0160) sur la vraie simulation, campagne France :
##  1. loi d'échelle : croissante avec la distance, mais l'ost rétrécit à l'écran en dézoomant ;
##     exposant 1 = ancienne loi ; parchemin = pion de taille écran constante ;
##  2. l'ost en garnison se tient hors de l'emprise de sa ville, près comme loin ;
##  3. viser l'ost (centre de sa silhouette) désigne l'armée, pas la ville ; le clic la sélectionne ;
##  4. viser la ville désigne la ville ;
##  5. survol : l'objet visé est éclairé (armée, puis ville), un seul à la fois.
## Usage : godot --headless --path game --script res://tests/sa_pick_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("sa_pick_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("sa_pick_test: " + message)
	return condition


func _frames(count: int) -> void:
	for i in count:
		await process_frame


func _run() -> void:
	# 1. Loi d'échelle (statique).
	var previous := 0.0
	var previous_on_screen := INF
	for distance in [22.0, 60.0, 150.0, 400.0, 1000.0]:
		var value := ArmyMarkers.scale_for_distance(distance, 0.0, 0.6)
		_check(value > previous, "scale should grow with distance (%.0f → %.3f)" % [distance, value])
		_check(value / distance < previous_on_screen, "screen size should shrink when zooming out (%.0f)" % distance)
		previous = value
		previous_on_screen = value / distance
	_check(is_equal_approx(ArmyMarkers.scale_for_distance(400.0, 0.0, 1.0), 400.0 * ArmyMarkers.SCALE_PER_DISTANCE), "exponent 1 should give the former linear law")
	_check(is_equal_approx(ArmyMarkers.scale_for_distance(1500.0, 1.0, 0.6), ArmyMarkers.MAX_SCALE), "parchment should use the token scale")
	_check(ArmyMarkers.scale_for_distance(400.0, 0.0, 0.6) < 0.4 * 400.0 * ArmyMarkers.SCALE_PER_DISTANCE, "at 400 the army should be well under its former size")

	root.size = Vector2i(1280, 720)
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _frames(10)
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		return
	map.camera_rig.edge_pan_enabled = false
	# Première armée du joueur stationnée dans une colonie (l'ost du roi à Paris).
	var army_id := ""
	var settlement_id := ""
	for id in map.player_army_ids():
		var army: Dictionary = map.sim.call("get_army", id)
		if str(army.get("settlement", "")) != "":
			army_id = id
			settlement_id = str(army["settlement"])
			break
	if not _check(army_id != "", "no garrisoned player army"):
		return
	var marker: ArmyMarker = map.armies._markers[army_id]
	var city: Vector3 = map.settlement_layer.world_position_of(settlement_id)
	var radius: float = map.settlement_layer.model_radius_of(settlement_id)
	print("sa: army %s in %s (radius %.2f)" % [army_id, settlement_id, radius])

	for distance in [400.0, 60.0]:
		map.camera_rig.look_at_point(city, distance)
		map.camera_rig.snap()
		await _frames(6)
		# 2. Hors de l'emprise de la ville.
		var gap := Vector2(marker.position.x - city.x, marker.position.z - city.z).length()
		_check(gap >= radius + ArmyMarker.FOOTPRINT_RADIUS * marker.marker_scale - 0.01,
			"at %.0f the army should stand beside its town (gap %.2f, radius %.2f, scale %.2f)" % [distance, gap, radius, marker.marker_scale])
		# 3. Viser l'ost.
		var army_point: Vector2 = marker.screen_rect(map.camera).get_center()
		var target: Dictionary = map.pick_target(army_point)
		_check(str(target.get("kind", "")) == "army" and str(target.get("id", "")) == army_id,
			"at %.0f aiming at the army gives %s" % [distance, target])
		# 4. Viser la ville (centre de l'emprise).
		var city_point: Vector2 = map.camera.unproject_position(city)
		target = map.pick_target(city_point)
		_check(str(target.get("kind", "")) == "settlement" and str(target.get("id", "")) == settlement_id,
			"at %.0f aiming at the town gives %s" % [distance, target])

		# 5. Survol.
		map.hover_at(army_point)
		_check(str(map.hover_target.get("id", "")) == army_id and marker.is_hovered() and map.armies.hovered_army == army_id,
			"at %.0f hovering the army should light it (hover %s)" % [distance, map.hover_target])
		_check(map.settlement_layer.hovered_id == "", "only one target lit at a time")
		map.hover_at(city_point)
		_check(map.settlement_layer.hovered_id == settlement_id and not marker.is_hovered(),
			"at %.0f hovering the town should light it (hover %s)" % [distance, map.hover_target])

		# 3 bis. Le clic prend ce qui est visé.
		map.deselect_army()
		_check(map._try_select_army(army_point) and map.selected_army == army_id, "at %.0f clicking the army should select it" % distance)
		map.deselect_army()
		map._last_pick_position = Vector2(-1.0e6, -1.0e6)
		map.hover_at(Vector2.ZERO, false)
		_check(map.hover_target.is_empty() and not marker.is_hovered() and map.settlement_layer.hovered_id == "", "leaving the map should clear the hover")
	map.queue_free()
	await _frames(2)
