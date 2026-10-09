extends TestCase

## Test headless du lot FK5a (incidents sur la carte, spec carte vivante § 3.4) sur la vraie
## simulation et les vraies données, vérifié par l'état des nœuds :
##  1. une décision `map` (evt_crue en Touraine) a son sceau au-dessus de la province, avec ses
##     tours restants et sa bulle ; une décision `dialog` (evt_sluys) n'en a pas ;
##  2. clic sur le sceau → la fenêtre de la chronique sur cette décision ;
##  3. fin de tour : la fenêtre de début de tour ne s'ouvre pas pour l'incident (repli sur la
##     fenêtre quand les sceaux sont coupés) ;
##  4. à l'expiration, l'avis dit ce que le conseil a tranché, et le sceau disparaît.
## Usage : godot --headless --path game --script res://tests/fk5_incidents_test.gd

const PROVINCE := "prov_touraine"


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("debug_offer_decision"),
			"CampaignSim.debug_offer_decision missing (run core/build.sh)"):
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
	await _test_incidents(map, map.sim)
	map.queue_free()
	await process_frame


func _pending_ids(chronicle: ChronicleController, modal: bool) -> Array:
	var decisions: Array = chronicle.modal_pending() if modal else chronicle.pending()
	return decisions.map(func(d: Dictionary) -> int: return int(d.get("id", -1)))


func _test_incidents(map: Node, sim: Object) -> void:
	sim.call("set_chronicle_enabled", false)  # pas d'autre décision que les nôtres
	var chronicle: ChronicleController = map.chronicle
	chronicle.window.hide()
	var incidents: IncidentMarkers = map.life.incidents if map.life != null else null
	if not check(incidents != null, "CampaignLife should set up the incident markers"):
		return
	var map_id := int(sim.call("debug_offer_decision", "evt_crue", PROVINCE))
	var dialog_id := int(sim.call("debug_offer_decision", "evt_sluys", ""))
	check(int(sim.call("debug_offer_decision", "evt_crue", "prov_nowhere")) == -1, "an unknown province should be refused")
	if not check(map_id > 0 and dialog_id > 0, "staging refused (%d, %d)" % [map_id, dialog_id]):
		return
	var by_id := {}
	for decision: Dictionary in sim.call("get_pending_decisions"):
		by_id[int(decision["id"])] = decision
	check(str(by_id.get(map_id, {}).get("presentation", "")) == "map", "evt_crue should be a map incident: %s" % [by_id.get(map_id)])
	check(str(by_id.get(dialog_id, {}).get("presentation", "")) == "dialog", "evt_sluys should be a dialog: %s" % [by_id.get(dialog_id)])
	map.refresh_all()
	await process_frame
	await process_frame

	# 1. Sceau de l'incident, aucun pour la fenêtre.
	var seal := incidents.marker(map_id)
	check(incidents.marker(dialog_id) == null, "a dialog decision should have no seal")
	check(incidents.marker_count() == 1, "one seal expected, got %d" % incidents.marker_count())
	if not check(seal != null, "the map incident should have a seal"):
		return
	var expires := int(by_id[map_id].get("expires_in", 0))
	check(seal.call("turns_left") == maxi(1, expires), "the seal should show %d turns left" % expires)
	check(seal.tooltip_text.contains(str(by_id[map_id].get("title", "?"))), "the tooltip should name the incident: %s" % seal.tooltip_text)
	check(seal.tooltip_text.contains("Touraine"), "the tooltip should name the province: %s" % seal.tooltip_text)
	var map_data: MapData = map.map_data
	var centroid := map_data.centroid_of_id(PROVINCE)
	check(seal.get("world").distance_to(Vector3(centroid.x, seal.get("world").y, centroid.y)) < 0.01, "the seal should hang over the province")
	# Visible à tous les zooms : contrôle d'écran, placé au loin comme au plus près.
	# TB2 : le sceau n'apparaît plus que dans la couche « Signes » (ou au dernier tour).
	MapReadability.signs_layer_on = true
	for distance in [map.camera_rig.max_distance, 45.0]:
		map.camera_rig.look_at_point(seal.get("world"), distance)
		map.camera_rig.snap()
		await process_frame
		await process_frame
		var screen := map.get_viewport().get_visible_rect()
		check(seal.visible and screen.grow(8.0).has_point(seal.position + seal.size * 0.5),
			"the seal should be on screen at distance %.0f (%s)" % [distance, seal.position])

	# 2. Clic sur le sceau → fenêtre de la chronique sur cette décision.
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	seal.call("_gui_input", click)
	await process_frame
	check(chronicle.window.visible and chronicle.window.current_decision() == map_id,
		"clicking the seal should open the decision window on it (%d)" % chronicle.window.current_decision())
	chronicle.window.hide()

	# 2b. LR-02 : clic sur la scène de l'incident (pas seulement le sceau).
	var staged_here := false
	if map.life.folk_scenes != null:
		for scene: Dictionary in map.life.folk_scenes.staged:
			staged_here = staged_here or str(scene.get("province", "")) == PROVINCE
	var hit := incidents.scene_hit(map_id)
	check(staged_here == (hit != null), "a scene in the province should give a clickable area (scene %s, area %s)" % [staged_here, hit])
	if hit != null:
		map.camera_rig.look_at_point(seal.get("world"), 45.0)
		map.camera_rig.snap()
		await process_frame
		await process_frame
		check(hit.visible and hit.size.x >= 2.0 * IncidentMarkers.SCENE_HIT_MIN - 0.01, "the scene area should be shown and sized (%s)" % hit.size)
		check(hit.tooltip_text.contains(str(by_id[map_id].get("title", "?"))), "the scene area should carry the incident tooltip")
		hit.call("_gui_input", click)
		await process_frame
		check(chronicle.window.visible and chronicle.window.current_decision() == map_id,
			"clicking the scene should open the decision window on it (%d)" % chronicle.window.current_decision())
		chronicle.window.hide()

	# 3. Pas de fenêtre de début de tour pour l'incident ; repli quand les sceaux sont coupés.
	check(_pending_ids(chronicle, true) == [dialog_id], "only the dialog should open by itself: %s" % [_pending_ids(chronicle, true)])
	map.life.incidents = null
	check(_pending_ids(chronicle, true).has(map_id), "without seals, the incident should fall back to the window")
	map.life.incidents = incidents
	sim.call("choose_event_option", dialog_id, 0)
	await map._on_end_turn()
	for _i in 4:
		await process_frame
	if _pending_ids(chronicle, false).has(map_id):
		check(chronicle.window.current_decision() != map_id, "the turn start should not open the incident window")
		check(incidents.marker(map_id) != null, "the incident seal should stay while pending")

	# 4. Expiration : l'avis dit ce que le conseil a tranché ; le sceau part.
	var notified := PackedStringArray()
	for _turn in 6:
		if not _pending_ids(chronicle, false).has(map_id):
			break
		var events: Array = sim.call("end_turn")
		notified.append_array(chronicle.notify_expired(events))
		map.refresh_all()
	check(not _pending_ids(chronicle, false).has(map_id), "the incident should expire")
	var title := str(by_id[map_id].get("title", "?"))
	var said := notified.size() == 1 and notified[0].begins_with("Délai écoulé") and notified[0].contains(title)
	check(said, "one expiry notice naming the incident expected: %s" % [notified])
	await process_frame
	check(incidents.marker(map_id) == null, "the seal should go once the incident expired")
	print("fk5: notice %s" % [notified])
