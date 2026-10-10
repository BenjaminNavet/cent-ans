extends TestCase

## Test headless du lot CO-C (ADR 0292) : onglet « Bâtiments » refondu. Vraie simulation (France, graine
## 1337) : pour une cité et un village, le panneau montre le palier (« Palier n/6 — nom ») d'après le cœur,
## l'en-tête se replie sans image, le nombre de cartes égale le plafond d'emplacements, une carte libre
## déplie le choix de construction. Plus un test sur maquette : image de bâtiment absente → repli.
## Usage : godot --headless --path game --script res://tests/co_c_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _wait(frames: int) -> void:
	for i in frames:
		await process_frame


func _run() -> void:
	_check_mock_view()
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("settlement_tier"),
			"CampaignSim.settlement_tier missing (run core/build.sh)"):
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
	for kind in ["city", "village"]:
		var id := _settlement_of_kind(sim, kind)
		if not check(id != "", "no settlement of kind %s" % kind):
			continue
		map.settlement_layer.select(id)
		await _wait(5)
		var panel: Control = map.settlements_ctl.panel
		var view: SettlementBuildingsView = panel.buildings_view
		var tier := int(sim.call("settlement_tier", id))
		var usage: Dictionary = sim.call("settlement_slot_usage", id)
		check(tier >= 1 and tier <= 6, "%s tier in 1..6, got %d" % [kind, tier])
		check(view.caption.text.begins_with("Palier %d/6 — " % tier), "%s caption: %s" % [kind, view.caption.text])
		check(view.tier() == tier, "%s view tier %d vs core %d" % [kind, view.tier(), tier])
		var expected := maxi(int(usage.get("max", 0)), int(usage.get("used", 0)))
		check(view.card_count() == expected, "%s: %d cards, expected %d (%s)" % [kind, view.card_count(), expected, str(usage)])
		check(not view.header_has_image() or ResourceLoader.exists(SettlementBuildingsView.tier_image_path(kind, tier)), "%s header image fallback" % kind)
		print("co_c: %s %s tier %d/6 '%s', cards %d, usage %s, header image %s" % [kind, id, tier, SettlementBuildingsView.tier_name(kind, tier), view.card_count(), str(usage), str(view.header_has_image())])
	map.queue_free()
	await process_frame


func _check_mock_view() -> void:
	var view := SettlementBuildingsView.new()
	root.add_child(view)
	var rows := [{"root": "bld_market", "built": "bld_market", "built_name": "Marché", "level": 1, "max_level": 3, "state": "upgradable",
		"next": [{"building": "bld_fair", "name": "Foire", "cost": 300, "turns": 3, "available": true}]}]
	var detail := {"kind": "village", "construction": {"building": "bld_fair", "name": "Foire", "turns_left": 2}, "build_queue": []}
	view.set_data("set_x", rows, {"used": 1, "max": 3}, 2, detail, {}, true)
	check(view.card_count() == 3, "mock: 3 cards for 3 slots, got %d" % view.card_count())
	check(view.caption.text == "Palier 2/6 — Village", "mock caption: %s" % view.caption.text)
	check(not view.header_has_image(), "mock: no tier image yet, parchment fallback")
	check(view.find_child("CardState", true, false) != null and (view.find_child("CardState", true, false) as Label).text.begins_with("Chantier"), "mock: the upgrade shows its construction")
	check(view.find_child("CardImage", true, false) != null or view.find_child("CardIcon", true, false) != null, "mock: card image or icon fallback")
	var asked: Array = []
	view.free_slot_requested.connect(func(id: String) -> void: asked.append(id))
	(view.find_child("FreeSlotButton", true, false) as Button).pressed.emit()
	check(asked == ["set_x"], "mock: free card requests the build choice")
	check(SettlementBuildingsView.roman(4) == "IV", "roman numerals")
	view.queue_free()


func _settlement_of_kind(sim: Object, kind: String) -> String:
	for entry in sim.call("settlements"):
		if str(entry["kind"]) == kind and str(entry["owner"]) == "fac_france" and str(entry["controller"]) == "fac_france":
			return str(entry["id"])
	return ""
