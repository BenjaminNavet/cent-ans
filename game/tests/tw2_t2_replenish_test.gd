extends TestCase

## Test headless du lot TW2-T2 (reconstitution des armées, réserves de recrutement) sur la vraie
## simulation et les vraies données, vérifié par l'état des nœuds (pas de capture) :
##  1. `get_army_replenishment` : taux, facteurs et info-bulle française ;
##  2. le sceau d'une armée du joueur affiche le taux avec l'info-bulle détaillant les facteurs ;
##  3. `get_recruitable` expose la réserve (`pool_*`) et la ligne de recrutement l'affiche
##     (« N disponibles… ») ; un recrutement consomme la réserve.
## Usage : godot --headless --path game --script res://tests/tw2_t2_replenish_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_army_replenishment"),
			"CampaignSim.get_army_replenishment missing (run core/build.sh)"):
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
	var sim: Object = map.sim
	var army_id := ""
	for id in map.player_army_ids():
		army_id = id
		break
	if check(army_id != "", "France has no army"):
		_test_bridge(sim, army_id)
		await _test_seal(map, sim, army_id)
		_test_recruitment(sim, army_id)
	map.queue_free()
	await process_frame


# --- 1. Pont --------------------------------------------------------------------------------


func _test_bridge(sim: Object, army_id: String) -> void:
	var info: Dictionary = sim.call("get_army_replenishment", army_id)
	if not check(not info.is_empty(), "get_army_replenishment should describe the army"):
		return
	for key in ["percent", "men", "missing", "cost", "territory", "blocked", "factors", "tooltip"]:
		check(info.has(key), "get_army_replenishment lacks %s: %s" % [key, info])
	var factors: Array = info.get("factors", [])
	check(not factors.is_empty() and str((factors[0] as Dictionary).get("kind", "")) == "base",
		"the first factor should be the territory base rate: %s" % [factors])
	check(str(info.get("tooltip", "")).begins_with("Reconstitution"), "French tooltip expected: %s" % info.get("tooltip", ""))
	check(str(info.get("territory", "")) in ["own", "ally", "neutral", "hostile"], "territory key: %s" % info.get("territory", ""))
	check((sim.call("get_army_replenishment", "army_999999") as Dictionary).is_empty(), "an unknown army gives an empty dictionary")
	print("tw2_t2: army %s replenishes %d %% (%s)" % [army_id, int(info.get("percent", 0)), str(info.get("tooltip", "")).replace("\n", " | ")])


# --- 2. Sceau -------------------------------------------------------------------------------


func _test_seal(map: Node, sim: Object, army_id: String) -> void:
	map.select_army(army_id)
	await process_frame
	var seal: GeneralSeal = map.ui.general_seal
	var row: Container = seal.get("_status_row")
	if not check(row != null, "the seal has no status row"):
		return
	var info: Dictionary = sim.call("get_army_replenishment", army_id)
	var found: Control = null
	for child in row.get_children():
		if (child as Control).tooltip_text.begins_with("Reconstitution"):
			found = child
	if not check(found != null, "the seal should show the replenishment status"):
		return
	check(found.tooltip_text == str(info.get("tooltip", "")), "the seal tooltip should detail the factors")
	var label: Label = null
	for child in found.get_children():
		if child is Label:
			label = child
	check(label != null and label.text == GeneralSeal.replenishment_text(info),
		"the seal should read %s" % GeneralSeal.replenishment_text(info))


# --- 3. Recrutement -------------------------------------------------------------------------


func _test_recruitment(sim: Object, army_id: String) -> void:
	var army: Dictionary = sim.call("get_army", army_id)
	var place := str(army.get("location", ""))
	var rows: Array = sim.call("get_recruitable", place)
	if not check(not rows.is_empty(), "no recruitment option at %s" % place):
		return
	var chosen: Dictionary = {}
	for row: Dictionary in rows:
		for key in ["pool_available", "pool_cap", "pool_seasons_to_next", "pool_label"]:
			check(row.has(key), "get_recruitable row lacks %s" % key)
		if chosen.is_empty() and bool(row.get("available", false)):
			chosen = row
	var list := VBoxContainer.new()
	root.add_child(list)
	# Chargé à l'exécution : `PanelWidgets` dépend de l'autoload `IconLibrary`, inconnu à la
	# compilation d'un script `SceneTree`.
	var widgets: GDScript = load("res://scripts/map/panel_widgets.gd")
	widgets.call("fill_recruitable", list, rows, func(_unit: String) -> void: pass)
	var labels := list.find_children("PoolLabel", "Label", true, false)
	check(labels.size() == rows.size(), "one reserve label per row (%d for %d)" % [labels.size(), rows.size()])
	if not labels.is_empty():
		check((labels[0] as Label).text.contains("disponible"), "reserve label: %s" % (labels[0] as Label).text)
	list.queue_free()
	if not check(not chosen.is_empty(), "nothing recruitable at %s" % place):
		return
	var unit := str(chosen["unit_type"])
	var before := int(chosen["pool_available"])
	var result: Dictionary = sim.call("submit_order", {"type": "recruit", "settlement": place, "unit_type": unit})
	if not check(bool(result.get("ok", false)), "recruit refused: %s" % result.get("error", "")):
		return
	for row: Dictionary in sim.call("get_recruitable", place):
		if str(row["unit_type"]) == unit:
			check(int(row["pool_available"]) == before - 1, "the recruit should draw the reserve (%d → %d)" % [before, int(row["pool_available"])])
			check(int(row["pool_seasons_to_next"]) > 0 and str(row["pool_label"]).contains("+1 dans"),
				"the drawn reserve should say when it refills: %s" % row["pool_label"])
			print("tw2_t2: %s at %s: %s" % [unit, place, row["pool_label"]])
