extends SceneTree

## Test headless du lot NT5 (N6 plafond d'unités, N7 engins de siège), par l'état des nœuds :
##  1. le pont expose `army_unit_cap` (20, donnée du cœur) et `get_assault_odds` ;
##  2. « Former une armée » est grisé, avec une infobulle « army_full », dès 21 unités cochées,
##     et redevient actif à 20 ;
##  3. la ligne des engins de siège du panneau de siège (prêts / tours restants).
## Usage : godot --headless --path game --script res://tests/nt5_cap_engines_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_run()
	print("nt5_cap_engines_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("nt5_cap_engines_test: " + message)
	return condition


func _run() -> void:
	# 1. Pont.
	if _check(ClassDB.class_exists("CampaignSim"), "CampaignSim missing (run core/build.sh)"):
		var sim: Object = ClassDB.instantiate("CampaignSim")
		_check(sim.has_method("army_unit_cap"), "CampaignSim.army_unit_cap missing")
		_check(sim.has_method("get_assault_odds"), "CampaignSim.get_assault_odds missing")
		if sim.has_method("army_unit_cap"):
			_check(int(sim.call("army_unit_cap")) == 20, "army cap is not 20 without data")
	var facade: Node = root.get_node_or_null("/root/SimFacade")
	_check(facade != null and int(facade.call("army_unit_cap")) == 20, "SimFacade.army_unit_cap != 20")

	# 2. « Former une armée » borné.
	var widgets: GDScript = load("res://scripts/map/panel_widgets.gd")
	var button := Button.new()
	root.add_child(button)
	var checks: Array[CheckBox] = []
	for i in 21:
		var check := CheckBox.new()
		check.button_pressed = true
		root.add_child(check)
		checks.append(check)
	widgets.call("bind_army_cap", button, checks, 20)
	_check(button.disabled, "21 units checked: the button must be greyed")
	_check(button.tooltip_text.contains("army_full"), "21 units checked: army_full tooltip expected, got '%s'" % button.tooltip_text)
	checks[0].button_pressed = false
	_check(not button.disabled, "20 units checked: the button must be active")
	_check(not button.tooltip_text.contains("army_full"), "20 units checked: no army_full tooltip")

	# 3. Ligne des engins.
	var siege: GDScript = load("res://scripts/map/siege_controller.gd")
	var text := str(siege.call("engines_text", [
		{"name": "Échelles", "kind": "ladders", "ready": true, "turns_left": 0},
		{"name": "Bélier", "kind": "ram", "ready": false, "turns_left": 1},
		{"name": "Beffroi", "kind": "tower", "ready": false, "turns_left": 3}]))
	_check(text == "Engins de siège — échelles : prêtes, bélier : 1 tour, beffroi : 3 tours.", "engines line: '%s'" % text)
