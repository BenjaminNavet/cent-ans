extends TestCase

## Test headless du lot TW2-T3 (compagnies de mercenaires) sur la vraie simulation et les vraies
## données, vérifié par l'état des nœuds (pas de capture) :
##  1. `get_mercenaries` : région, engagements restants, lignes au format du recrutement ;
##  2. le bandeau d'une armée du joueur montre « Mercenaires » ; le bouton ouvre le panneau, une
##     ligne par compagnie avec sa réserve ;
##  3. un clic engage la compagnie : elle rejoint l'armée sur-le-champ, la réserve baisse, le
##     panneau se met à jour ; une ville refuse de lever une unité de mercenaires.
## Usage : godot --headless --path game --script res://tests/tw2_t3_mercenaries_test.gd

## Chargé à l'exécution : le panneau dépend d'autoloads (`IconLibrary`) inconnus à la compilation
## d'un script `SceneTree`.
var _panel_script: GDScript


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_mercenaries"),
			"CampaignSim.get_mercenaries missing (run core/build.sh)"):
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
	_panel_script = load("res://scripts/ui/mercenary_panel.gd")
	# Une armée du joueur dans une région où une compagnie se loue en 1337 (Génois des galées,
	# Brabançons en France du Nord...).
	var army_id := ""
	for id in map.player_army_ids():
		if _hireable(sim.call("get_mercenaries", id)) != "":
			army_id = id
			break
	if check(army_id != "", "no French army stands where a company can be hired in 1337"):
		_test_bridge(sim, army_id)
		await _test_panel(map, sim, army_id)
		_test_towns_refuse(sim, facade)
	map.queue_free()
	await process_frame


## Première compagnie engageable du marché `info`, "" sinon.
func _hireable(info: Dictionary) -> String:
	for option: Dictionary in info.get("options", []):
		if bool(option.get("available", false)):
			return str(option.get("unit_type", ""))
	return ""


# --- 1. Pont --------------------------------------------------------------------------------


func _test_bridge(sim: Object, army_id: String) -> void:
	var info: Dictionary = sim.call("get_mercenaries", army_id)
	for key in ["region", "region_name", "hires_left", "blocked", "premium_last_turn", "options"]:
		check(info.has(key), "get_mercenaries lacks %s: %s" % [key, info])
	check(str(info.get("region_name", "")) != "", "the region should have a French name")
	check(int(info.get("hires_left", 0)) > 0 and str(info.get("blocked", "x")) == "", "a fresh turn allows hires: %s" % info)
	for option: Dictionary in info.get("options", []):
		for key in ["unit_type", "name", "band_name", "cost", "upkeep", "available", "reason", "pool_available", "pool_cap", "pool_label"]:
			check(option.has(key), "mercenary option lacks %s" % key)
	check((sim.call("get_mercenaries", "army_999999") as Dictionary).is_empty(), "an unknown army gives an empty dictionary")
	print("tw2_t3: army %s in %s, %d companies" % [army_id, info.get("region_name", ""), (info.get("options", []) as Array).size()])


# --- 2-3. Panneau et engagement --------------------------------------------------------------


func _test_panel(map: Node, sim: Object, army_id: String) -> void:
	map.select_army(army_id)
	await process_frame
	var strip: Node = map.ui.army_strip
	var button: Button = strip.find_child("MercenaryButton", true, false)
	if not check(button != null and button.visible, "the army strip should show « Mercenaires »"):
		return
	button.pressed.emit()
	await process_frame
	var panel: Control = map.hud.mercenary_panel
	if not check(panel != null and panel.visible, "the button should open the mercenary panel"):
		return
	var info: Dictionary = sim.call("get_mercenaries", army_id)
	var title: Label = panel.find_child("Title", true, false)
	check(title.text == "Mercenaires — %s" % str(info["region_name"]), "panel title: %s" % title.text)
	var rows := panel.find_children("PoolLabel", "Label", true, false)
	check(rows.size() == (info["options"] as Array).size(), "one reserve label per company (%d)" % rows.size())
	var unit := _hireable(info)
	var before_pool := -1
	for option: Dictionary in info["options"]:
		if str(option["unit_type"]) == unit:
			before_pool = int(option["pool_available"])
	var units_before := (sim.call("get_army", army_id)["units"] as Array).size()
	# Clic sur la ligne de la compagnie (bouton de `fill_recruitable`).
	var clicked := false
	for candidate in panel.find_children("*", "Button", true, false):
		var line := candidate as Button
		if line.name != "Close" and not line.disabled and line.text.begins_with(_name_of(info, unit)):
			line.pressed.emit()
			clicked = true
			break
	if not check(clicked, "no enabled row for %s" % unit):
		return
	await process_frame
	var units_after := (sim.call("get_army", army_id)["units"] as Array).size()
	check(units_after == units_before + 1, "the company should join at once (%d → %d)" % [units_before, units_after])
	var after: Dictionary = sim.call("get_mercenaries", army_id)
	for option: Dictionary in after["options"]:
		if str(option["unit_type"]) == unit:
			check(int(option["pool_available"]) == before_pool - 1, "the hire should draw the reserve")
	check(int(after["hires_left"]) == int(info["hires_left"]) - 1, "one hire less this turn")
	check(panel.visible and panel.find_child("Status", true, false).text == _panel_script.call("status_text", after),
		"the panel should refresh after the hire")
	print("tw2_t3: hired %s, %s" % [unit, panel.find_child("Status", true, false).text])


func _name_of(info: Dictionary, unit: String) -> String:
	for option: Dictionary in info["options"]:
		if str(option["unit_type"]) == unit:
			return str(_panel_script.call("row_for", option)["name"])
	return unit


func _test_towns_refuse(sim: Object, facade: Node) -> void:
	var capital := str(facade.call("faction_info", "fac_france").get("capital", ""))
	for row: Dictionary in sim.call("get_recruitable", capital):
		if str(row["unit_type"]) == "unit_genoese_crossbowmen":
			check(not bool(row["available"]) and str(row["reason"]).contains("mercenaires"),
				"a town should not levy a company: %s" % row.get("reason", ""))
