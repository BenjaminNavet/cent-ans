extends SceneTree

## Test headless du lot JR3 (interface de la faction croisée, spec JR § 4, 5, 5.1) sur la vraie
## simulation et les vraies données, vérifié par l'état des nœuds (pas de capture) :
##  1. textes de la section (état, promesse, contingent, infobulle des causes) sur des vues
##     synthétiques ;
##  2. évènements `crusade` : rubrique, « Jérusalem délivrée » lue par tous, le reste réservé au
##     joueur croisé (rapport de saison, journal, lettres, pastilles) ;
##  3. campagne croisée : section « Ferveur » du panneau de faction (jauge, aumônes, bouton),
##     repère du HUD (clic → panneau), caméra sur Limassol, passage prêché (`pending` à 1) ;
##  4. campagne française : ni section ni repère ;
##  5. choix de faction : la faction sans province est listée et cliquable sur la carte.
## Les réglages vont sur le fichier de test (`Settings.use_test_file`), jamais sur celui du joueur.
## Usage : godot --headless --path game --script res://tests/jr_crusade_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const CRUSADERS := "fac_crusaders"
const BASE := "set_limassol"

var _failures := 0
## Point visé par la caméra à l'ouverture de la campagne (avant toute image : sans souris, le
## défilement par les bords de l'écran déplacerait la vue en headless).
var _opening_focus := Vector3.ZERO


func _init() -> void:
	await process_frame
	await _run()
	print("jr_crusade_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("jr_crusade_test: " + message)
	return condition


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_crusade"),
			"CampaignSim.get_crusade missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		_check(str(settings.get("path")) != "user://settings.cfg", "the test must not use the player's settings file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
	_test_texts()
	_test_events()
	var map := await _start(CRUSADERS)
	if map != null:
		await _test_section(map)
		_test_hud(map)
		_test_camera(map)
		await _test_preach(map)
		map.queue_free()
		await process_frame
	map = await _start("fac_france")
	if map != null:
		_test_absent(map)
		map.queue_free()
		await process_frame
	await _test_faction_chooser()


func _start(faction: String) -> Node3D:
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = faction
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	map.camera_rig.edge_pan_enabled = false
	_opening_focus = map.camera_rig.target_focus
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null and facade.is_real and str(map.player_faction) == faction,
			"campaign as %s failed to start" % faction):
		map.queue_free()
		return null
	if map.sim.has_method("set_chronicle_enabled"):
		map.sim.call("set_chronicle_enabled", false)
	return map


# --- 1. Textes ----------------------------------------------------------------------------------


func _test_texts() -> void:
	var high := {"fervor": 75, "zeal_morale": 10, "desertion_percent": 0, "alms": 1800}
	_check(CrusadeSection.status_text(high).contains("Élan") and CrusadeSection.status_text(high).contains("+10"),
		"zeal line: %s" % CrusadeSection.status_text(high))
	var low := {"fervor": 12, "zeal_morale": -10, "desertion_percent": 5}
	var low_text := CrusadeSection.status_text(low)
	_check(low_text.contains("moral %s10" % Money.MINUS) and low_text.contains("Débandade") and low_text.contains("5 %"),
		"low fervour line: %s" % low_text)
	_check(not CrusadeSection.status_text({"fervor": 50}).contains("moral"), "steady fervour: no morale line")
	_check(CrusadeSection.promise_text({"passage_units": 3, "passage_delay": 2}).begins_with("3 unités de volontaires débarqueront dans 2 tours"),
		"promise: %s" % CrusadeSection.promise_text({"passage_units": 3, "passage_delay": 2}))
	_check(CrusadeSection.pending_text({"port_name": "Limassol", "units": 1, "turns_left": 1}) == "Limassol : 1 unité, dans 1 tour",
		"pending: %s" % CrusadeSection.pending_text({"port_name": "Limassol", "units": 1, "turns_left": 1}))
	var view := {"fervor": 59, "floor": 50, "target_name": "Jérusalem", "alms": 1480,
		"changes": [{"cause": "Le vœu s'use", "delta": -1}, {"cause": "Victoire sur une autre foi", "delta": 6}]}
	var tip := CrusadeSection.tooltip(view)
	_check(tip.contains("Ferveur : 59 / 100") and tip.contains("Le vœu s'use") and tip.contains("%s1" % Money.MINUS)
		and tip.contains("+6") and tip.contains("Plancher : 50"), "gauge tooltip: %s" % tip)
	_check(CrusadeSection.tooltip({"fervor": 60}).contains("Aucun mouvement"), "tooltip without changes")
	var marks := CrusadeSection.thresholds()
	_check(marks.has("high") and marks.has("low") and marks.has("desertion")
		and int(marks["desertion"]) < int(marks["low"]) and int(marks["low"]) < int(marks["high"]),
		"thresholds read from data/rules/crusade.json: %s" % str(marks))
	_check(int(CrusadeSection.thresholds({"zeal_high_threshold": 80}).get("high", 0)) == 80, "a threshold of the view wins over the file")
	var gauge := CrusadeSection.FervorGauge.new()
	gauge.set_view(10, 0, {"desertion": 20, "low": 30, "high": 70})
	_check(gauge.fill_color() == HudStyle.POOR, "gauge ink under the desertion mark")
	gauge.set_view(80, 0, {"desertion": 20, "low": 30, "high": 70})
	_check(gauge.fill_color() == HudStyle.GOLD, "gauge ink at the zeal mark")
	gauge.free()


# --- 2. Évènements ------------------------------------------------------------------------------


func _test_events() -> void:
	var freed := {"kind": "crusade", "faction": CRUSADERS, "province": "prov_jerusalem",
		"text_fr": "Jérusalem délivrée ! L'ost tient la cité de son vœu et y établit son siège."}
	var preached := {"kind": "crusade", "faction": CRUSADERS, "province": "prov_cyprus",
		"text_fr": "Les croisés prêchent le passage : 3 unité(s) de volontaires sont attendues à Limassol dans 2 tour(s)."}
	_check(SeasonReport.is_public_crusade(freed) and not SeasonReport.is_public_crusade(preached), "only the deliverance is public")
	_check(SeasonReport.section_of("crusade") == "lands" and SeasonReport.KIND_STYLES.has("crusade"), "crusade kind has a section and a style")
	var mine := func(event: Dictionary) -> bool: return str(event.get("faction", "")) == CRUSADERS
	var nobody := func(_event: Dictionary) -> bool: return false
	var own := SeasonReport.build_groups([freed, preached], mine, Callable(), CRUSADERS)
	_check(own.size() == 1 and own[0]["id"] == "lands" and (own[0]["entries"] as Array).size() == 2, "the crusader reads both under his lands: %s" % str(own))
	var deaf := func(_event: Dictionary) -> bool: return false
	for keeps in [Callable(), deaf]:
		var other := SeasonReport.build_groups([freed, preached], nobody, keeps, "fac_france")
		_check(other.size() == 1 and other[0]["id"] == "world" and (other[0]["entries"] as Array).size() == 1
			and str(other[0]["entries"][0]["text_fr"]).contains("délivrée"), "France reads the deliverance only: %s" % str(other))
	_check(SeasonReport.tone_of(freed, CRUSADERS) == SeasonReport.TONE_GAIN, "the deliverance is a gain for the crusader")
	_check(not NewsLetters.news_from_event(preached).is_empty() and NewsLetters.kind_label("crusade") == "Croisade", "crusade letters")
	_check(EndTurnCluster.short_label("crusade") == "Croisade", "crusade alert pill label")


# --- 3. Campagne croisée ------------------------------------------------------------------------


func _test_section(map: Node3D) -> void:
	var view: Dictionary = map.sim.call("get_crusade")
	if not _check(not view.is_empty(), "get_crusade is empty for the crusader"):
		return
	map.ui.faction_panel_requested.emit()
	await process_frame
	await process_frame
	var panel: FactionPanel = map.ui.faction_panel
	var section: CrusadeSection = panel.crusade_section
	if not _check(panel.visible and section != null and section.is_visible_in_tree(), "the fervour section shows in the crusader's faction panel"):
		return
	var fervor := int(view["fervor"])
	_check(section.header_label.text == "Ferveur" and section.value_label.text == "%d / 100" % fervor, "starting fervour shown: %s" % section.value_label.text)
	_check(section.gauge.fervor == fervor and section.gauge.size.x > 100.0, "gauge filled to the fervour (%d, %s px)" % [section.gauge.fervor, section.gauge.size.x])
	_check(section.gauge.tooltip_text.contains("Ferveur : %d / 100" % fervor) and section.gauge.tooltip_text.contains("Ce tour"), "gauge tooltip lists the causes")
	_check(section.gauge.marks.has("high") and section.marks_label.text.contains("Débandade sous"), "threshold marks: %s" % section.marks_label.text)
	_check(section.alms_label.text == "Aumônes : %s par tour" % Money.amount(int(view["alms"])), "alms line: %s" % section.alms_label.text)
	_check(section.status_label.text == CrusadeSection.status_text(view), "status line: %s" % section.status_label.text)
	_check(section.preach_button.text.contains(Money.amount(int(view["passage_cost"]))), "preach button carries its cost: %s" % section.preach_button.text)
	_check(section.preach_button.disabled != bool(view["passage_available"]), "preach button enabled when the passage is available")
	_check(section.error_label.visible == false and (view["pending"] as Array).is_empty() and not section.pending_box.visible, "no contingent expected at the start")
	var rule: Control = section.get_parent().get_node_or_null("CrusadeRule")
	_check(rule != null and rule.visible, "separator under the section")


func _test_hud(map: Node3D) -> void:
	var label: Label = map.ui.fervor_label
	var fervor := int((map.sim.call("get_crusade") as Dictionary).get("fervor", -1))
	if not _check(label != null and label.is_visible_in_tree(), "fervour indicator in the top bar"):
		return
	_check(label.text.contains(str(fervor)), "indicator shows the fervour: %s" % label.text)
	_check(label.get_parent() == map.ui.treasury_label.get_parent(), "indicator sits in the resource bar")
	_check(label.tooltip_text.contains("Ferveur : %d / 100" % fervor), "indicator tooltip")
	map.ui.hide_faction()
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	label.gui_input.emit(click)
	_check(map.ui.faction_panel_visible() and map.ui.faction_panel.crusade_section.visible, "clicking the indicator opens the faction panel")


func _test_camera(map: Node3D) -> void:
	var world: Vector3 = map.settlement_layer.world_position_of(BASE) if map.settlement_layer != null else Vector3.ZERO
	if not _check(world != Vector3.ZERO, "Limassol is on the campaign map"):
		return
	var gap := Vector2(_opening_focus.x - world.x, _opening_focus.z - world.z).length()
	_check(gap < 40.0, "the campaign opens on Limassol (%.1f px away)" % gap)
	map.camera_rig.look_at_point(Vector3.ZERO)
	map.call("_focus_capital")
	var focus: Vector3 = map.camera_rig.target_focus
	_check(Vector2(focus.x - world.x, focus.z - world.z).length() < 40.0, "focusing the capital of a landless faction goes to its host")
	_check(not (map.player_army_ids() as PackedStringArray).is_empty(), "the crusader starts with an army")


func _test_preach(map: Node3D) -> void:
	var section: CrusadeSection = map.ui.faction_panel.crusade_section
	var before: Dictionary = map.sim.call("get_crusade")
	if not _check(bool(before["passage_available"]), "the passage can be preached at the start: %s" % str(before["passage_blocker"])):
		return
	_check(section.preach_button.tooltip_text.contains("volontaires"), "preach tooltip promises the contingent")
	var treasury := int((map.sim.call("get_faction_summary", CRUSADERS) as Dictionary).get("treasury", 0))
	section.preach_button.pressed.emit()
	await process_frame
	await process_frame
	var after: Dictionary = map.sim.call("get_crusade")
	_check(bool(section.last_result.get("ok", false)), "preach order accepted: %s" % str(section.last_result))
	if not _check((after["pending"] as Array).size() == 1, "one contingent expected after the call: %s" % str(after["pending"])):
		return
	_check(int(after["fervor"]) > int(before["fervor"]), "the call lifts the fervour")
	_check(int((map.sim.call("get_faction_summary", CRUSADERS) as Dictionary).get("treasury", 0)) == treasury - int(before["passage_cost"]), "the call is paid")
	_check(section.value_label.text == "%d / 100" % int(after["fervor"]), "section refreshed after the call")
	_check(section.pending_box.visible and section.pending_box.get_child_count() == 2, "expected contingent listed")
	var line := (section.pending_box.get_child(1) as Label).text
	_check(line.contains(str(after["pending"][0]["port_name"])) and line.contains("unité"), "contingent line: %s" % line)
	_check(section.preach_button.disabled and section.blocker_label.visible
		and section.preach_button.tooltip_text.contains(str(after["passage_blocker"])), "cooldown: button disabled with the reason")
	_check((map.ui.fervor_label as Label).text.contains(str(int(after["fervor"]))), "indicator follows the fervour: %s" % map.ui.fervor_label.text)
	var refused := section.request_preach()
	_check(not bool(refused.get("ok", true)) and section.error_label.visible and section.error_label.text.begins_with("Refusé"),
		"a second call is refused with the reason: %s" % str(refused))
	_check(((map.sim.call("get_crusade") as Dictionary)["pending"] as Array).size() == 1, "a refused call adds no contingent")


# --- 4. Autre faction ---------------------------------------------------------------------------


func _test_absent(map: Node3D) -> void:
	_check((map.sim.call("get_crusade") as Dictionary).is_empty(), "get_crusade is empty for France")
	map.ui.faction_panel_requested.emit()
	var panel: FactionPanel = map.ui.faction_panel
	_check(panel.visible and panel.crusade_section != null and not panel.crusade_section.visible, "no fervour section for France")
	_check(not (panel.crusade_section.get_parent().get_node("CrusadeRule") as Control).visible, "no stray separator for France")
	_check(map.ui.fervor_label != null and not (map.ui.fervor_label as Label).visible, "no fervour indicator for France")


# --- 5. Choix de faction ------------------------------------------------------------------------


func _test_faction_chooser() -> void:
	_check(not FrontEndData.faction(CRUSADERS).is_empty(), "the crusaders have a selection card in front_end.json")
	var select := FactionSelect.new()
	select.size = Vector2(1600, 1000)
	root.add_child(select)
	await process_frame
	await process_frame
	var picker := select.map_picker
	if not _check(picker != null and picker.sheets.has(CRUSADERS), "the map picker lists the crusaders"):
		select.queue_free()
		return
	select.start_tabs.current_tab = 1
	await process_frame
	await process_frame
	_check(picker.landless.has(CRUSADERS), "the landless faction has a banner on the map")
	_check(select._faction_buttons.has(CRUSADERS), "the crusaders are in the faction list")
	var center := picker.faction_center(CRUSADERS)
	_check(Rect2(Vector2.ZERO, picker.size).has_point(center), "the banner is inside the framed map: %s in %s" % [center, picker.size])
	_check(picker.faction_at(center) == CRUSADERS, "the banner is found under its centre")
	var seat := picker.faction_center("fac_cyprus")
	if seat != Vector2(-1, -1):
		_check(center.distance_to(seat) > 1.0, "the banner is not the centre of Cyprus")
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	click.position = center
	picker._gui_input(click)
	_check(select.selected_faction == CRUSADERS, "clicking the banner selects the crusaders")
	_check(picker.card_text(CRUSADERS).contains("Difficulté"), "hover card of the crusaders: %s" % picker.card_text(CRUSADERS))
	select.queue_free()
	await process_frame
