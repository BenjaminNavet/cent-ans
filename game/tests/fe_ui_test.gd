extends SceneTree

## Test headless du lot FE6 (interface de la féodalité, spec FE § 6) sur la vraie simulation et
## les vraies données, vérifié par l'état des nœuds (pas de capture) :
##  1. pont : fiches (France souveraine, Bourgogne vassale), fil d'Ariane, obligations, carte
##     féodale, aperçu d'escalade, fiches de départ du choix de faction ;
##  2. panneau « Arbre féodal » (France) : arbre, pastilles, sélection d'un vassal, commise
##     (refusée sans félonie : l'ordre atteint le cœur), concession d'un titre ;
##  3. panneau vu d'un vassal type (Bourgogne) : révolte et hommage soumis ;
##  4. section « Féodalité » du panneau de faction, fil d'Ariane de la province, filtre de carte,
##     « Qui peut entrer en guerre » (confirmation de guerre), arbitrage, Codex, guide en 3 étapes,
##     événements féodaux ;
##  5. choix de faction sur la carte (onglet, fiche au survol, clic, filtres).
## Usage : godot --headless --path game --script res://tests/fe_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("fe_ui_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("fe_ui_test: " + message)
	return condition


func _data_dir() -> String:
	return MAP_PATHS.default_data_dir()


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_feudal_tree"),
			"CampaignSim.get_feudal_tree missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
	_test_bridge()
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(_data_dir())
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
	await _test_tree_france(map)
	await _test_faction_section(map)
	await _test_breadcrumb(map)
	await _test_map_filter(map)
	await _test_escalation(map)
	await _test_orders(map)
	await _test_tutorial(map)
	_test_events(map)
	await _test_vassal(map)
	map.queue_free()
	await process_frame
	await _test_faction_chooser()


# --- 1. Pont --------------------------------------------------------------------------------


func _sim_for(faction: String) -> Object:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	_check(sim.call("new_campaign", _data_dir(), faction, 1337), "new_campaign %s failed" % faction)
	return sim


static func _ids(entries: Array) -> Array:
	var ids: Array = []
	for entry in entries:
		ids.append(str((entry as Dictionary).get("id", "")))
	return ids


func _test_bridge() -> void:
	var sim := _sim_for("fac_france")
	var france: Dictionary = sim.call("get_feudal_sheet", "fac_france")
	for key in ["name", "ruler", "primary", "primary_name", "rank", "titles", "liege", "liege_chain", "sovereign",
			"direct_vassals", "title_vassals", "loyalty", "status", "objectives", "independence_turns", "start_crown"]:
		_check(france.has(key), "get_feudal_sheet lacks %s" % key)
	_check(str(france.get("liege", "?")) == "" and int(france.get("loyalty", 0)) == -1, "France is sovereign: %s" % france.get("liege"))
	_check(str(france.get("rank", "")) == "kingdom", "France holds a kingdom")
	_check(_ids(france.get("direct_vassals", [])).has("fac_burgundy"), "Burgundy is a direct vassal of France")
	_check(_ids(france.get("title_vassals", [])).has("fac_england"), "England holds Guyenne of France")
	var burgundy: Dictionary = sim.call("get_feudal_sheet", "fac_burgundy")
	_check(str(burgundy.get("liege", "")) == "fac_france", "Burgundy's liege is France")
	_check(int(burgundy.get("loyalty", -1)) >= 0 and str(burgundy.get("status", "")) != "", "a vassal has a loyalty and a status")
	var crumbs: Array = sim.call("get_province_breadcrumb", "prov_charolais")
	_check(crumbs.size() == 3 and str(crumbs[0].get("title_name", "")) == "Royaume de France"
		and str(crumbs[2].get("holder", "")) == "fac_burgundy", "Charolais breadcrumb: %s" % str(crumbs))
	_check(Array(sim.call("get_province_lieges", "prov_guyenne")) == ["fac_england", "fac_france"], "Guyenne allegiance chain")
	var duties: Dictionary = sim.call("get_feudal_obligations", "fac_burgundy")
	_check(str(duties.get("liege", "")) == "fac_france" and int(duties.get("tribute_percent", 0)) > 0, "Burgundy owes tribute to France")
	for key in ["tribute", "host_against", "protect", "felons", "own_felonies", "grantable_titles", "grant_candidates", "homage_candidates"]:
		_check(duties.has(key), "get_feudal_obligations lacks %s" % key)
	var cells: Array = sim.call("get_feudal_map", PackedStringArray(["prov_guyenne", "prov_charolais", "nope"]))
	_check(cells.size() == 3 and str(cells[0].get("second_lord", "")) == "fac_france" and str(cells[0].get("holder", "")) == "fac_england",
		"Guyenne is a double allegiance: %s" % str(cells))
	_check(str(cells[1].get("sovereign", "")) == "fac_france" and str(cells[1].get("holder", "")) == "fac_burgundy", "Charolais under Burgundy and France")
	_check((cells[2] as Dictionary).is_empty(), "unknown province: empty cell")
	var steps: Array = sim.call("get_war_escalation_preview", "fac_england", "fac_burgundy")
	_check(not steps.is_empty() and str(steps[0].get("faction", "")) == "fac_france"
		and str(steps[0].get("likelihood", "")) in ["likely", "uncertain", "unlikely"] and str(steps[0].get("reason", "")) != "",
		"attacking Burgundy calls France first: %s" % str(steps))
	var store: Object = ClassDB.instantiate("GameDataStore")
	if _check(store.call("load", _data_dir()), "GameDataStore.load failed"):
		var sheets: Array = store.call("get_feudal_start_sheets")
		_check(sheets.size() >= 60, "one start sheet per playable faction: %d" % sheets.size())
		var foix := {}
		for sheet in sheets:
			if str(sheet.get("id", "")) == "fac_foix_bearn":
				foix = sheet
		_check(str(foix.get("kingdom", "")) != "" and not (foix.get("provinces", PackedStringArray()) as PackedStringArray).is_empty(),
			"Foix-Béarn start sheet has a kingdom and provinces: %s" % str(foix.keys()))


# --- 2. Arbre féodal (France) -----------------------------------------------------------------


func _test_tree_france(map: Node) -> void:
	var feudal: FeudalController = map.get("feudal")
	if not _check(feudal != null and feudal.available(), "FeudalController missing on the campaign map"):
		return
	var popup: PopupMenu = map.ui.menu_button.get_popup()
	_check(popup.get_item_index(FeudalController.MENU_FEUDAL_ID) >= 0, "Menu → Arbre féodal")
	popup.id_pressed.emit(FeudalController.MENU_FEUDAL_ID)
	await process_frame
	_check(feudal.panel.visible, "Menu → Arbre féodal opens the feudal tree")
	var france_item := feudal.tree_item("fac_france")
	var burgundy_item := feudal.tree_item("fac_burgundy")
	_check(france_item != null and burgundy_item != null, "France and Burgundy in the tree")
	if burgundy_item != null:
		_check(burgundy_item.get_text(1) != "" and burgundy_item.get_tooltip_text(0).contains("Loyauté"), "vassal badge and loyalty tooltip")
	if france_item != null:
		_check(france_item.get_text(0).contains("(vous)"), "the player is marked in the tree")
	var liege := _find(feudal.panel, "Liege") as Label
	_check(liege != null and liege.text.contains("Souverain"), "France's position: sovereign")
	feudal.select("fac_burgundy")
	await process_frame
	var commise := _find(feudal.panel, "CommiseButton") as Button
	_check(commise != null and commise.disabled, "no felony: the forfeiture button is disabled")
	var name_label := _find(feudal.panel, "SelectedName") as Label
	_check(name_label != null and name_label.text != "", "selected vassal sheet")


# --- 4. Section du panneau de faction, fil d'Ariane, filtre, escalade -----------------------------


func _test_faction_section(map: Node) -> void:
	map.call("_show_faction_panel", "fac_france")
	await process_frame
	var section: FeudalSection = map.ui.faction_panel.feudal_section
	if not _check(section != null and section.visible, "Féodalité section in the faction panel"):
		return
	var titles := _find(section, "Titles") as Label
	_check(titles != null and titles.text.contains("Royaume de France"), "titles listed")
	var vassals := _find(section, "Vassals") as Label
	_check(vassals != null and vassals.text.contains("Bourgogne"), "direct vassals listed")
	_check(_find(section, "Independence") != null, "generic victory path shown for a sovereign")
	var feudal: FeudalController = map.get("feudal")
	feudal.panel.hide()
	section.tree_button.emit_signal("pressed")
	await process_frame
	_check(feudal.panel.visible and feudal.focus == "fac_france", "Arbre féodal… opens the tree")
	map.ui.hide_faction()


func _test_breadcrumb(map: Node) -> void:
	var panel: Node = map.ui.province_panel
	panel.show_province({"id": "prov_charolais", "display_name": "Charolais"}, {}, [], false)
	await process_frame
	var crumbs: HFlowContainer = panel.breadcrumb
	_check(crumbs.visible and crumbs.get_child_count() == 5, "Charolais breadcrumb: 3 titles and 2 separators")
	var first := crumbs.get_child(0) as LinkButton
	_check(first != null and first.text == "Royaume de France", "breadcrumb starts at the kingdom")
	var feudal: FeudalController = map.get("feudal")
	feudal.panel.hide()
	var last := crumbs.get_child(crumbs.get_child_count() - 1) as LinkButton
	if _check(last != null, "last crumb is a link"):
		last.emit_signal("pressed")
		await process_frame
		_check(feudal.panel.visible and feudal.focus == "fac_burgundy", "a crumb opens the tree on the title's holder")
	map.ui.hide_province()


func _test_map_filter(map: Node) -> void:
	var modes: Node = map.map_modes
	modes.set_mode("feudal")
	await process_frame
	_check(modes.mode == "feudal", "Féodalité filter selected")
	var guyenne: Dictionary = modes.feudal_lens.cells.get("prov_guyenne", {})
	_check(str(guyenne.get("second_lord", "")) == "fac_france", "Guyenne read as a double allegiance")
	_check(modes.feudal_lens.shield_count() >= 1, "party shields placed on the map")
	_check(modes.legend() != null, "legend of the feudal filter")
	var hover := str(modes.hover_text("prov_charolais"))
	_check(hover.contains("Bourgogne"), "hover text names the holder: %s" % hover)
	modes.set_mode("political")
	await process_frame
	_check(modes.feudal_lens.shield_count() == 0, "shields cleared with the political map")


func _test_escalation(map: Node) -> void:
	var dialog := WarDeclarationDialog.new()
	map.ui.add_child(dialog)
	dialog.ask(map.sim, "fac_albret", "Albret", "la place d'Albret")
	await process_frame
	_check(dialog.escalation.visible and not dialog.escalation.steps.is_empty(), "attacking Albret shows who may enter the war")
	if not dialog.escalation.steps.is_empty():
		_check(str(dialog.escalation.steps[0].get("faction", "")) == "fac_england", "England is called first for Albret: %s" % str(dialog.escalation.steps))
		_check(_find(dialog.escalation, "Step0") != null, "one row per link")
	dialog.queue_free()
	var hint := EscalationPreview.bbcode(map.sim, "fac_france", "fac_albret")
	_check(hint.contains("Qui peut entrer en guerre"), "diplomacy hint carries the escalation chain")


# --- Ordres ----------------------------------------------------------------------------------


func _test_orders(map: Node) -> void:
	var feudal: FeudalController = map.get("feudal")
	var sim: Object = map.sim
	var refused: Dictionary = sim.call("feudal_declare_commise", "fac_burgundy")
	_check(not bool(refused.get("ok", true)) and str(refused.get("error", "")).contains("commise"),
		"forfeiture without felony reaches the core and is refused: %s" % refused)
	_check(not feudal.declare_commise("fac_burgundy"), "controller forwards the forfeiture order")
	var bad: Dictionary = sim.call("feudal_arbitrate", 999, "nonsense", "")
	_check(not bool(bad.get("ok", true)) and str(bad.get("error", "")).contains("invalide"), "unknown verdict refused by the bridge")
	var no_offer: Dictionary = sim.call("feudal_arbitrate", 999, "impose_peace", "")
	_check(not bool(no_offer.get("ok", true)), "arbitration of an unknown offer refused by the core")
	var duties: Dictionary = sim.call("get_feudal_obligations", "fac_france")
	var titles: Array = duties.get("grantable_titles", [])
	if _check(not titles.is_empty(), "France holds titles it may grant"):
		var title := str(titles[0].get("id", ""))
		feudal.open_for("")
		feudal.select("fac_burgundy")
		await process_frame
		var choice := _find(feudal.panel, "GrantChoice") as OptionButton
		_check(choice != null and choice.item_count == titles.size(), "grant choice lists France's grantable titles")
		_check(feudal.grant_title(title, "fac_burgundy"), "granting %s to Burgundy is accepted" % title)
		var titles_after := _ids((sim.call("get_feudal_sheet", "fac_burgundy") as Dictionary).get("titles", []))
		_check(titles_after.has(title), "Burgundy now holds %s" % title)


# --- Guide et événements -----------------------------------------------------------------------


func _test_tutorial(map: Node) -> void:
	var feudal: FeudalController = map.get("feudal")
	feudal.panel.hide()
	feudal.start_tutorial()
	await process_frame
	_check(feudal.tutorial_active() and feudal.tutorial.visible, "feudal guide shown")
	_check(FeudalController.TUTORIAL_STEPS.size() == 3, "three steps")
	feudal.open_for("")
	_check(feudal.tutorial_objective_met(), "step 1 met once the tree is open")
	feudal.advance_tutorial()
	map.map_modes.set_mode("feudal")
	_check(feudal.tutorial_objective_met(), "step 2 met with the feudal filter")
	feudal.advance_tutorial()
	_check(feudal.tutorial_index == 2, "step 3 reached")
	feudal.advance_tutorial()
	_check(not feudal.tutorial_active() and not feudal.tutorial.visible, "guide finished")
	map.map_modes.set_mode("political")
	var codex: Node = root.get_node_or_null("/root/CodexStore")
	_check(codex != null and codex.call("has_entry", "cdx_vassalite"), "Codex entry « Vassalité »")


func _test_events(map: Node) -> void:
	var feudal: FeudalController = map.get("feudal")
	_check(FeudalController.is_feudal_event({"kind": "vassalage", "text_fr": "x"}), "vassalage is feudal")
	_check(FeudalController.is_feudal_event({"kind": "diplomacy", "text_fr": "Félonie de Bretagne envers France"}), "felony text is feudal")
	_check(not FeudalController.is_feudal_event({"kind": "battle", "text_fr": "Bataille de Crécy"}), "a battle is not feudal")
	feudal.recent.clear()
	feudal.after_end_turn([{"kind": "vassalage", "text_fr": "Bourgogne prête hommage à France.", "faction": "fac_burgundy"},
		{"kind": "battle", "text_fr": "Bataille"}])
	_check(feudal.recent.size() == 1, "one feudal event recorded")
	feudal.open_for("")
	_check(_find(feudal.panel, "Recent0") != null, "recent feudal events listed in the panel")


# --- 3. Vu d'un vassal type (Bourgogne) -------------------------------------------------------


func _test_vassal(map: Node) -> void:
	var feudal: FeudalController = map.get("feudal")
	var france_sim: Object = map.sim
	var sim := _sim_for("fac_burgundy")
	map.sim = sim
	map.player_faction = "fac_burgundy"
	feudal.open_for("")
	await process_frame
	var liege := _find(feudal.panel, "Liege") as Label
	_check(liege != null and liege.text.contains("France") and liege.text.contains("loyauté"), "vassal position: liege and loyalty")
	_check(_find(feudal.panel, "Tribute") != null, "vassal owes tribute")
	_check(_find(feudal.panel, "RevoltButton") != null, "revolt button for a vassal")
	_check(feudal.tree_item("fac_france") != null and feudal.tree_item("fac_burgundy") != null, "tree rooted at the sovereign")
	var lords: Array = (sim.call("get_feudal_obligations", "fac_burgundy") as Dictionary).get("homage_candidates", [])
	var choice := _find(feudal.panel, "HomageChoice") as OptionButton
	_check(lords.is_empty() or (choice != null and choice.item_count == lords.size()), "homage choice lists the candidates")
	if not lords.is_empty():
		var other := _sim_for("fac_burgundy")
		var lord := str(lords[0].get("id", ""))
		var result: Dictionary = other.call("feudal_switch_allegiance", lord)
		_check(bool(result.get("ok", false)) and str((other.call("get_feudal_sheet", "fac_burgundy") as Dictionary).get("liege", "")) == lord,
			"homage to %s submitted: %s" % [lord, result])
	_check(feudal.revolt(), "revolt order accepted")
	_check(str((sim.call("get_feudal_sheet", "fac_burgundy") as Dictionary).get("liege", "?")) == "", "after the revolt Burgundy has no liege")
	map.sim = france_sim
	map.player_faction = "fac_france"


# --- 5. Choix de faction sur la carte ---------------------------------------------------------


func _test_faction_chooser() -> void:
	var select := FactionSelect.new()
	select.size = Vector2(1600, 1000)
	root.add_child(select)
	await process_frame
	await process_frame
	_check(select.start_tabs != null and select.start_tabs.get_tab_count() == 2, "two tabs: recommended starts and the map")
	_check(select.card_count() >= 3 and select.card_count() <= 6, "recommended cards only: %d" % select.card_count())
	var picker := select.map_picker
	if not _check(picker != null and picker.sheets.size() >= 60, "map picker loaded the start sheets"):
		select.queue_free()
		return
	select.start_tabs.current_tab = 1
	await process_frame
	await process_frame
	_check(picker.size.x > 100 and picker.provinces.size() > 100, "map picker laid out with provinces")
	_check(not picker.kingdoms().is_empty() and select.kingdom_filter.item_count == picker.kingdoms().size() + 1, "kingdom filter filled")
	var center := picker.faction_center("fac_foix_bearn")
	_check(picker.faction_at(center) == "fac_foix_bearn", "Foix-Béarn found under its centre")
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	click.position = center
	picker._gui_input(click)
	_check(select.selected_faction == "fac_foix_bearn", "clicking the map selects the faction")
	var card := picker.card_text("fac_foix_bearn")
	_check(card.contains("Suzerain") and card.contains("Difficulté") and card.contains("Objectifs"), "hover card: %s" % card)
	picker.set_filters("", "county")
	_check(not picker.matches("fac_france") and picker.matches("fac_foix_bearn"), "rank filter")
	select.queue_free()
	await process_frame


func _find(node: Node, node_name: String) -> Node:
	if node == null:
		return null
	if node.name == node_name:
		return node
	for child in node.get_children():
		var found := _find(child, node_name)
		if found != null:
			return found
	return null
