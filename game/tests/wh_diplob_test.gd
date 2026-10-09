extends TestCase

## Lot WH `diplob` : diplomatie active, vérifiée par l'état des nœuds et du pont (sans capture).
##  1. le pont expose la ligue, le genre d'alliance, le pacte et l'« hégémon » ;
##  2. les listes d'options portent les ennemis de chaque camp (clause « Rejoindre la guerre ») ;
##  3. menus de clauses : alliance militaire / défensive, pacte de non-agression, rejoindre la guerre ;
##  4. offres : un appel d'allié et un ultimatum ont leurs propres boutons ;
##  5. la clause JoinWar passe par `evaluate_proposal` (libellé et raisons).
## Usage : godot --headless --path game --script res://tests/wh_diplob_test.gd

var map: Node3D = null


## Simulation factice : deux offres, sans appels féodaux.
class FakeSim:
	extends RefCounted

	func get_offers() -> Array:
		return [
			{"id": 1, "from": "fac_portugal", "from_name": "Portugal", "kind": "ally_call", "ultimatum": false, "text": "Portugal réclame votre aide.", "expires_in": 2},
			{"id": 2, "from": "fac_england", "from_name": "Angleterre", "kind": "tribute", "ultimatum": true, "text": "Ultimatum.", "expires_in": 2},
		]


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


func _menu_texts(menu: MenuButton) -> PackedStringArray:
	var popup := menu.get_popup()
	var texts := PackedStringArray()
	for index in popup.item_count:
		texts.append(popup.get_item_text(index))
	return texts


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	var controller: Node = map.diplomacy
	controller.open_panel("fac_england")
	await _wait(10)
	var panel: Control = controller.panel
	var sim: Object = panel.get("sim")
	# 1. Pont.
	var league: Dictionary = sim.call("get_league")
	check(league.has("active") and not bool(league["active"]), "no league at the start of 1337: %s" % league)
	var entries: Array = sim.call("get_diplomacy", map.player_faction)
	check(not entries.is_empty(), "get_diplomacy empty")
	var first: Dictionary = entries[0]
	for key in ["alliance_kind", "non_aggression_turns_left", "hegemon"]:
		check(first.has(key), "get_diplomacy should expose %s" % key)
	# 2. Options.
	var peaceful := ""
	for e in entries:
		if str(e["status"]) in ["peace", "truce"]:
			peaceful = str(e["id"])
			break
	check(peaceful != "", "a peaceful faction is needed")
	var options: Dictionary = sim.call("treaty_options", peaceful)
	check(options["ours"].has("enemies") and options["theirs"].has("enemies"), "treaty_options sides need enemies")
	check(options.has("non_aggression_durations") and not (options["non_aggression_durations"] as PackedInt64Array).is_empty(), "pact durations come from the data")
	# 3. Menus de clauses.
	controller.open_panel(peaceful)
	await _wait(10)
	var tab: DiplomacyNegotiationTab = panel.negotiation
	tab.fill_menu(tab.clause_menu)
	var clauses := _menu_texts(tab.clause_menu)
	check(clauses.has("Alliance militaire") and clauses.has("Alliance défensive"), "alliance kinds: %s" % clauses)
	check(clauses.has("Pacte de non-agression"), "pact entry: %s" % clauses)
	tab.fill_menu(tab._demand_menu)
	var demands := _menu_texts(tab._demand_menu)
	var has_enemies: bool = not (options["ours"]["enemies"] as Array).is_empty()
	check(demands.has("Rejoindre la guerre contre…") == has_enemies, "join-war entry should follow our enemies: %s" % demands)
	# 5. Évaluation d'une clause de guerre (si un ennemi existe, sinon un refus lisible).
	var enemies: Array = options["ours"]["enemies"]
	var target := str(enemies[0]["id"]) if not enemies.is_empty() else "fac_england"
	var verdict: Dictionary = sim.call("evaluate_proposal", {"type": "propose_treaty", "target": peaceful, "articles": [{"kind": "join_war", "giver": "recipient", "target": target}]})
	check(verdict.has("accept") and not (verdict["reasons"] as Array).is_empty(), "join_war should be evaluated with reasons: %s" % verdict)
	# 4. Offres : boutons propres à l'appel d'allié et à l'ultimatum.
	var section := DiplomacyOffersSection.new()
	panel.add_child(section)
	section.show_for(FakeSim.new(), map.player_faction, "fac_portugal", {})
	await _wait(2)
	var buttons := PackedStringArray()
	for button in section.find_children("*", "Button", true, false):
		buttons.append((button as Button).text)
	for label in ["Marcher à son secours", "Se dérober", "Céder", "Refuser (la guerre)"]:
		check(buttons.has(label), "offer button %s missing: %s" % [label, buttons])
	map.queue_free()
	await process_frame
