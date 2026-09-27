extends SceneTree

## Test headless de la liste « Colonies » (touche B) sur la vraie simulation, lot HL2 :
##  1. B ouvre le panneau ; autant de lignes de province que dans l'aperçu ;
##  2. chaque filtre réduit la liste au compte attendu (count_idle / count_upgrade /
##     count_endangered de l'aperçu) ;
##  3. clic sur une colonie : `focus_settlement` ouvre son panneau (`SettlementController`) ;
##  4. ouvrir la liste ferme « Mes unités ».
## Usage : godot --headless --path game --script res://tests/holdings_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("holdings_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("holdings_test: " + message)
	return condition


func _run() -> void:
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
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var holdings: Node = map.holdings_ctl  # non typé : la classe dépend des autoloads
	if not _check(holdings != null and holdings.available(), "holdings controller missing"):
		return
	var overview: Dictionary = map.sim.call("get_holdings_overview", map.player_faction)
	if not _check(not overview.is_empty(), "the player should have an overview of its holdings"):
		return
	var provinces: Array = overview.get("provinces", [])
	if not _check(not provinces.is_empty(), "the player should start with provinces"):
		return

	# 1. Ouverture par l'action B.
	var press := InputEventAction.new()
	press.action = "map_toggle_holdings"
	press.pressed = true
	holdings._unhandled_input(press)
	_check(holdings.is_open(), "B should open the holdings list")
	_check(holdings.province_row_count() == provinces.size(),
		"one province row per province of the overview: %d vs %d" % [holdings.province_row_count(), provinces.size()])

	# 2. Filtres.
	holdings.set_filter("idle")
	_check(holdings.row_count() == int(overview["count_idle"]),
		"idle filter: %d vs count_idle %d" % [holdings.row_count(), int(overview["count_idle"])])
	holdings.set_filter("upgrade")
	_check(holdings.row_count() == int(overview["count_upgrade"]),
		"upgrade filter: %d vs count_upgrade %d" % [holdings.row_count(), int(overview["count_upgrade"])])
	holdings.set_filter("endangered")
	_check(holdings.row_count() == int(overview["count_endangered"]),
		"endangered filter: %d vs count_endangered %d" % [holdings.row_count(), int(overview["count_endangered"])])
	holdings.set_filter("all")
	var total_settlements := 0
	for province in provinces:
		total_settlements += (province["settlements"] as Array).size()
	_check(holdings.row_count() == total_settlements,
		"all filter: %d vs total settlements %d" % [holdings.row_count(), total_settlements])

	# 3. Clic sur une colonie : ouvre son panneau via `SettlementController`.
	var first_settlement := str((provinces[0]["settlements"] as Array)[0].get("id", ""))
	holdings.focus_settlement(first_settlement)
	await process_frame
	_check(map.settlements_ctl != null and map.settlements_ctl.panel.visible and map.settlements_ctl.panel.settlement_id == first_settlement,
		"clicking a settlement row opens its panel")

	# 4. Exclusion mutuelle avec « Mes unités ».
	if map.units_ctl != null and map.units_ctl.available():
		map.units_ctl.toggle()
		_check(map.units_ctl.is_open(), "units roster should open")
		_check(not holdings.is_open(), "opening the units roster should close the holdings list")
		holdings.toggle()
		_check(holdings.is_open(), "holdings list should reopen")
		_check(not map.units_ctl.is_open(), "opening the holdings list should close the units roster")
	map.queue_free()
