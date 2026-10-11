extends TestCase

## Test headless du lot RG11 (ADR 0345) : panneau Bâtiments avec arbre de construction.
## 1. Maquette : cases d'emplacements, file d'attente, emplacement `VocationSlot` (RG10), états des nœuds
##    (construit, chantier, en file, constructible, verrouillé avec raison en clair), clic = `build_requested`.
## 2. Vraie simulation (France, graine 1337) : `get_building_tree` (getter du cœur, via `SimFacade`),
##    cohérence avec `settlement_buildable`, le panneau montre l'arbre, un clic sur un nœud constructible
##    lance la construction par l'ordre existant, et le nœud passe en chantier.
## Usage : godot --headless --path game --script res://tests/rg11_buildings_tree_test.gd

var VIEW_SCRIPT: Variant = null


func _init() -> void:
	await process_frame
	VIEW_SCRIPT = load("res://scripts/ui/settlement_buildings_view.gd")
	_check_mock_view()
	await _run_real()
	finish()


func _wait(frames: int) -> void:
	for i in frames:
		await process_frame


func _node(id: String, state: String, extra: Dictionary = {}) -> Dictionary:
	var node := {"building": id, "name": id, "category": "commerce", "tier": 1, "parent": "", "root": id, "depth": 1,
		"state": state, "current": state == "built", "lock": "", "reason": "", "blocked_now": "", "can_build": state == "available",
		"cost": 300, "turns": 2, "upkeep": 5, "takes_slot": true, "turns_left": 0, "total_turns": 0, "queue_index": -1,
		"allowed_here": true, "requirements": []}
	node.merge(extra, true)
	return node


func _check_mock_view() -> void:
	var view = VIEW_SCRIPT.new()
	root.add_child(view)
	check(view.vocation_slot != null and view.vocation_slot.name == "VocationSlot", "mock: VocationSlot header exists for RG10")
	var nodes := [
		_node("bld_market", "built", {"name": "Marché"}),
		_node("bld_fair", "building", {"name": "Foire", "parent": "bld_market", "root": "bld_market", "depth": 2, "turns_left": 2, "total_turns": 4, "can_build": false}),
		_node("bld_forge", "queued", {"name": "Forge", "category": "military", "turns_left": 3, "total_turns": 3, "queue_index": 1, "can_build": false}),
		_node("bld_stables", "available", {"name": "Écuries", "category": "military"}),
		_node("bld_university", "locked", {"name": "Université", "category": "religious", "lock": "technology",
			"reason": "Requiert la technologie Universités", "can_build": false,
			"requirements": [{"kind": "technology", "id": "tech_universities", "text": "Requiert la technologie Universités", "met": false}]}),
		_node("bld_castle", "locked", {"name": "Château", "category": "fortification", "lock": "rank", "reason": "Requiert le rang Château", "allowed_here": false, "can_build": false}),
	]
	var tree := {"settlement": "set_x", "kind": "town", "kind_label": "Ville", "slots": {"used": 3, "max": 4}, "queue_size": 3,
		"queue": [{"building": "bld_fair", "name": "Foire", "turns_left": 2, "total_turns": 4}, {"building": "bld_forge", "name": "Forge", "turns_left": 3, "total_turns": 3}],
		"nodes": nodes}
	var rows := [{"root": "bld_market", "built": "bld_market", "level": 1, "max_level": 2, "state": "upgradable", "next": [{"building": "bld_fair"}]}]
	var detail := {"kind": "town", "construction": {"building": "bld_fair", "name": "Foire", "turns_left": 2}, "build_queue": [{"building": "bld_forge", "name": "Forge", "turns_left": 3}]}
	view.set_data("set_x", rows, {"used": 3, "max": 4}, 2, detail, {}, true, tree)
	check(view.card_count() == 4, "mock: one case per slot (4), got %d" % view.card_count())
	check(view.find_child("CardProgress", true, false) != null, "mock: the work in progress has a progress bar")
	check(view.queue_strip.find_child("QueueCell2", true, false) != null, "mock: 3 queue places")
	for index in 3:
		check(view.queue_strip.find_child("QueueCell%d" % index, true, false) != null, "mock: queue cell %d" % index)
	check(view.find_child("Node_bld_market", true, false).get_meta("node_state") == "built", "mock: built node")
	check(view.find_child("Node_bld_fair", true, false).get_meta("node_state") == "building", "mock: building node")
	check(view.find_child("Node_bld_forge", true, false).get_meta("node_state") == "queued", "mock: queued node")
	var locked: Button = view.find_child("Node_bld_university", true, false)
	check(locked.disabled and locked.text.contains("Requiert la technologie Universités"), "mock: locked node shows its reason: %s" % locked.text)
	check(view.find_child("Node_bld_castle", true, false) == null, "mock: other ranks hidden by default")
	view.show_all_check.button_pressed = true
	check(view.find_child("Node_bld_castle", true, false) != null, "mock: other ranks shown on demand")
	var asked: Array = []
	view.build_requested.connect(func(settlement: String, building: String) -> void: asked.append([settlement, building]))
	var open: Button = view.find_child("Node_bld_stables", true, false)
	check(not open.disabled, "mock: available node clickable")
	open.pressed.emit()
	check(asked == [["set_x", "bld_stables"]], "mock: click requests the build, got %s" % str(asked))
	check(view.find_child("Chain_bld_market", true, false).find_child("ChainArrow", true, false) != null, "mock: chain column links levels")
	var live: Dictionary = VIEW_SCRIPT.node_live(nodes[4])
	check(not bool(live["available"]) and str(live["reason"]) == "Requiert la technologie Universités", "mock: tooltip carries the lock reason")
	view.queue_free()


func _run_real() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_building_tree"),
			"CampaignSim.get_building_tree missing (run core/build.sh)"):
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
	root.size = Vector2i(1280, 720)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var id := ""
	for entry in sim.call("settlements"):
		if str(entry["kind"]) in ["city", "town"] and str(entry["owner"]) == "fac_france" and str(entry["controller"]) == "fac_france":
			var candidate := str(entry["id"])
			var detail: Dictionary = sim.call("settlement_detail", candidate)
			if not detail.has("construction") and (detail.get("build_queue", []) as Array).is_empty():
				id = candidate
				break
	if not check(id != "", "a French city or town with no work under way"):
		map.queue_free()
		return
	var tree: Dictionary = facade.building_tree(id)
	check(not tree.is_empty() and str(tree.get("settlement", "")) == id, "facade.building_tree returns the tree")
	check(facade.building_tree("set_nowhere").is_empty(), "unknown settlement: empty tree")
	var nodes: Array = tree.get("nodes", [])
	check(nodes.size() > 20, "tree lists the buildings (%d)" % nodes.size())
	check(int(tree.get("queue_size", 0)) == 3, "queue of 3")
	var options := {}
	for option in sim.call("settlement_buildable", id):
		options[str(option["building"])] = option
	var wanted := ""
	var states := {}
	for node in nodes:
		var entry: Dictionary = node
		var state := str(entry["state"])
		states[state] = int(states.get(state, 0)) + 1
		if state == "locked":
			check(str(entry["reason"]).begins_with("Requiert"), "%s: locked reason in plain French: %s" % [entry["building"], entry["reason"]])
			if options.has(entry["building"]):
				check(not bool(options[entry["building"]]["available"]), "%s locked in the tree but buildable in the core" % entry["building"])
		if state == "available" and options.has(entry["building"]):
			check(bool(entry["can_build"]) == bool(options[entry["building"]]["available"]), "%s: can_build agrees with settlement_buildable" % entry["building"])
		if bool(entry["can_build"]) and wanted == "" and int(entry["cost"]) > 0:
			wanted = str(entry["building"])
	print("rg11: settlement %s, %d nodes, states %s, will build %s" % [id, nodes.size(), str(states), wanted])
	check(states.has("available") and states.has("locked"), "both available and locked nodes exist")
	if not check(wanted != "", "a node can be built now"):
		map.queue_free()
		return
	map.settlement_layer.select(id)
	await _wait(5)
	var panel: Control = map.settlements_ctl.panel
	var view = panel.buildings_view
	var button: Button = view.find_child("Node_" + wanted, true, false)
	if not check(button != null and not button.disabled, "the panel shows a clickable node for %s" % wanted):
		map.queue_free()
		return
	check(view.find_child("VocationSlot", true, false) != null, "VocationSlot present in the real panel")
	button.pressed.emit()
	await _wait(5)
	var after: Dictionary = sim.call("settlement_detail", id)
	var construction: Dictionary = after.get("construction", {})
	check(str(construction.get("building", "")) == wanted, "click started the construction of %s, got %s" % [wanted, str(construction)])
	var tree_after: Dictionary = facade.building_tree(id)
	for node in tree_after["nodes"]:
		if str(node["building"]) == wanted:
			check(str(node["state"]) == "building", "node %s is now under construction" % wanted)
			check(int(node["turns_left"]) > 0, "node has turns left")
	check((tree_after["queue"] as Array).size() == 1, "queue shows the new work")
	var rebuilt: Button = panel.buildings_view.find_child("Node_" + wanted, true, false)
	check(rebuilt != null and rebuilt.get_meta("node_state") == "building", "the panel refreshed the node to 'building'")
	map.queue_free()
	await process_frame
