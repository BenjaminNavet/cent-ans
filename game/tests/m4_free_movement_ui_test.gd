extends TestCase

## Test headless du lot M4 (mouvement libre sur la carte) sur la vraie simulation et les
## vraies données `data/` :
##  1. armée du joueur sélectionnée → bulle atteignable affichée (masque non vide), aucun
##     anneau C5 ni masque de provinces ;
##  2. survol d'un point au-delà de la bulle → chemin en deux couleurs (ce tour, tours
##     suivants) ;
##  3. clic droit au sol dans la bulle → déplacement immédiat : position libre mise à jour,
##     points de mouvement dépensés, marqueur animé puis posé à la position ;
##  4. survol d'une armée ennemie → cercle de zone de contrôle ;
##  5. clic droit sur une armée ennemie à portée → attaque (bataille résolue ou en attente).
## Usage : godot --headless --path game --script res://tests/m4_free_movement_ui_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_reachable_area"),
			"CampaignSim.get_reachable_area missing (run core/build.sh)"):
		return
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
	await process_frame
	await process_frame
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var ctl: Node = map.movement_ctl
	if not check(ctl != null and ctl.available(), "ArmyMovementController missing or inactive"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var army_id := ""
	for id in map.player_army_ids():
		army_id = id
		break
	if not check(army_id != "", "France has no army"):
		map.queue_free()
		return

	# 1. Sélection → bulle.
	map.select_army(army_id)
	var army: Dictionary = sim.call("get_army", army_id)
	check(army.has("position") and army.has("settlement") and army.has("movement_left") and army.has("planned_path"),
		"get_army should expose position, settlement, movement_left and planned_path: %s" % [army.keys()])
	check(ctl.active(), "the controller should be active for a player army")
	check(ctl.bubble.visible and ctl.bubble.cell_count() > 100, "the reachable bubble should be shown (%d cells)" % ctl.bubble.cell_count())
	check(ctl.bubble.material_override.get_shader_parameter("mask") is Texture2D, "the bubble should sample a mask texture")
	check(map.settlements_ctl.markers.marker_count() == 0, "the C5 rings are replaced by the bubble")
	var start: Vector2 = army["position"]
	var left_before := int(army["movement_left"])
	var rect: Rect2 = ctl.bubble.covered_rect()
	check(rect.has_point(start), "the bubble should cover the army (%s, %s)" % [rect, start])
	var radius := maxf(rect.size.x, rect.size.y) * 0.5
	print("m4: army %s at %s, %d points, bubble %d cells, rect %s" % [army_id, start, left_before, ctl.bubble.cell_count(), rect])

	# 2. Aperçu au-delà de la bulle : deux couleurs.
	var far := Vector2(-1, -1)
	var near := Vector2(-1, -1)
	for i in 24:
		var direction := Vector2.RIGHT.rotated(TAU * float(i) / 24.0)
		var far_point := start + direction * radius * 1.5
		var plan: Dictionary = sim.call("find_path_points", army_id, far_point.x, far_point.y)
		if far.x < 0.0 and plan.get("ok", false) and int(plan["turns"]) >= 2:
			far = far_point
		var near_point := start + direction * radius * 0.35
		var near_plan: Dictionary = sim.call("find_path_points", army_id, near_point.x, near_point.y)
		if near.x < 0.0 and near_plan.get("ok", false) and bool(near_plan["reachable_this_turn"]):
			near = near_point
	if check(far.x >= 0.0, "no multi-turn target around %s" % start):
		ctl.preview_target({"kind": "ground", "id": "", "point": far})
		check(ctl.path_line.visible and ctl.path_line.has_now_part() and ctl.path_line.has_later_part(),
			"a multi-turn preview should show this turn and later turns")
		var shown: PackedVector2Array = ctl.path_line.shown_points()
		check(shown.size() >= 2 and shown[0].distance_to(start) < 0.01, "the preview should start at the army")
	if not check(near.x >= 0.0, "no target inside the bubble around %s" % start):
		map.queue_free()
		return

	# 3. Clic droit au sol → déplacement immédiat.
	var world := Vector3(near.x, map.map_data.surface_world_at(near.x, near.y), near.y)
	map.camera_rig.look_at_point(world, 200.0)
	map.camera_rig.snap()
	for _i in 3:
		await process_frame
	var screen: Vector2 = map.camera.unproject_position(world)
	var hit: Dictionary = map.picker.pick_ray_screen(screen)
	var report: Dictionary = {}
	var target := near
	if not hit.is_empty() and Vector2(hit["x"], hit["z"]).distance_to(near) < 6.0 and map.armies.pick_screen(screen) == "" \
			and map.settlement_layer.pick_screen(screen) == "":
		target = Vector2(hit["x"], hit["z"])
		check(ctl.try_right_click(screen), "a ground right click should be handled")
		print("m4: ground right click at %s → %s" % [screen, target])
	else:
		print("m4: screen picking unusable here (headless viewport), ordering directly")
		report = ctl.order_target(army_id, {"kind": "ground", "id": "", "point": near})
		check(report.get("ok", false), "move order refused: %s" % report.get("error", ""))
	var moved: Dictionary = sim.call("get_army", army_id)
	var position: Vector2 = moved["position"]
	check(position.distance_to(target) < 2.5, "the army should stand at %s, got %s" % [target, position])
	check(str(moved["settlement"]) == "" or position.distance_to(start) > 1.0, "the army should have left its place")
	check(int(moved["movement_left"]) < left_before, "movement points should be spent (%d → %d)" % [left_before, moved["movement_left"]])
	check(ctl.is_animating(army_id), "the march should be animated")
	ctl.finish_animations()
	var marker_world: Vector3 = map.armies.world_position_of(army_id)
	check(Vector2(marker_world.x, marker_world.z).distance_to(position) < 0.5, "the marker should stand at the army's free position")
	check(ctl.bubble.visible, "the bubble should be redrawn after the move")

	# 4-5. Armée ennemie à portée : cercle de zone de contrôle, attaque.
	var enemy := ""
	var summary: Dictionary = sim.call("get_faction_summary", "fac_france")
	var at_war: PackedStringArray = summary.get("at_war_with", PackedStringArray())
	for id in sim.call("get_army_ids"):
		var other: Dictionary = sim.call("get_army", id)
		if at_war.has(str(other.get("faction", ""))):
			enemy = id
			break
	if not check(enemy != "", "no enemy army"):
		map.queue_free()
		return
	var engage_px := float(sim.call("get_movement_rules").get("engage_radius_px", 7.0))
	var spot := _free_spot_near(sim, army_id, position, engage_px * 3.0)
	check(sim.call("debug_place_army", enemy, spot.x, spot.y), "debug_place_army failed")
	map.refresh_all()
	ctl.show_zoc(enemy)
	check(ctl.zoc_visible(), "the zone of control ring should show over an enemy army")
	var ring_center := Vector2(ctl.zoc_ring.position.x, ctl.zoc_ring.position.z)
	check(ring_center.distance_to(spot) < 0.5, "the ring should be centred on the enemy army")
	ctl.show_zoc("")
	var pending_before: int = (sim.call("get_pending_battles") as Array).size() if sim.has_method("get_pending_battles") else 0
	var enemy_before: Dictionary = sim.call("get_army", enemy)
	var attack: Dictionary = ctl.order_target(army_id, {"kind": "army", "id": enemy, "point": spot, "faction": str(enemy_before["faction"])})
	if check(attack.get("ok", false), "attack refused: %s" % attack.get("error", "")):
		check(str(attack["stop"]) == "engaged", "the attack should engage, got %s" % attack["stop"])
		var pending_after: int = (sim.call("get_pending_battles") as Array).size() if sim.has_method("get_pending_battles") else 0
		var battle_event := false
		for event in attack.get("events", []):
			battle_event = battle_event or str(event.get("kind", "")) == "battle"
		check(battle_event or pending_after > pending_before, "the attack should fight at once or offer the battle")
		check(int(sim.call("get_army", army_id).get("movement_left", 1)) == 0, "no movement after a battle")
		print("m4: attack on %s: stop=%s, events=%d, pending %d → %d" % [enemy, attack["stop"], (attack.get("events", []) as Array).size(), pending_before, pending_after])
	map.queue_free()
	await process_frame


## Point à `distance` pixels de `from` où l'armée peut aller (terre ferme, chemin trouvé).
func _free_spot_near(sim: Object, army_id: String, from: Vector2, distance: float) -> Vector2:
	for i in 16:
		var point := from + Vector2.RIGHT.rotated(TAU * float(i) / 16.0) * distance
		var plan: Dictionary = sim.call("find_path_points", army_id, point.x, point.y)
		if plan.get("ok", false):
			return point
	return from + Vector2(distance, 0.0)
