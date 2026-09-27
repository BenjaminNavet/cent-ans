extends SceneTree

## Test headless du lot CV3-4 (interface de campagne : postures, rencontres, classes de résultat)
## sur la vraie simulation et les vraies données, vérifié par l'état des nœuds :
##  1. boutons de posture du sceau : six boutons, grisés avec la raison quand le cœur refuse ;
##     un clic sur une posture permise passe l'ordre, rafraîchit la rangée et l'étendard ;
##  2. pastille de posture sur l'étendard, armée du joueur semi-transparente en embuscade ;
##  3. un site de rencontre placé près de l'armée donne un marqueur (bulle au survol) ; un clic
##     y fait marcher l'armée, la rencontre en attente ouvre la fenêtre, le choix appelle le pont ;
##  4. bandeau de classe : notice de résolution automatique sur la carte, écran de fin de bataille.
## Usage : godot --headless --path game --script res://tests/cv3_4_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("cv3_4_ui_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cv3_4_ui_test: " + message)
	return condition


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("debug_place_encounter"),
			"CampaignSim.debug_place_encounter missing (run core/build.sh)"):
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
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)
	var army_id := ""
	for id in map.player_army_ids():
		army_id = id
		break
	if not _check(army_id != "", "France has no army"):
		map.queue_free()
		return
	_test_stances(map, sim, army_id)
	await process_frame
	await _test_outcomes(map, sim)
	# L'armée de la bataille d'essai a pu périr : une armée du joueur encore debout.
	var survivor := ""
	for id in map.player_army_ids():
		survivor = id
		break
	if _check(survivor != "", "no player army left for the encounter"):
		await _test_encounters(map, sim, survivor)
	map.queue_free()
	await process_frame


# --- 1-2. Postures --------------------------------------------------------------------------


func _test_stances(map: Node, sim: Object, army_id: String) -> void:
	map.select_army(army_id)
	var seal: GeneralSeal = map.ui.general_seal
	var bar: StanceBar = seal.stance_bar
	if not _check(bar != null and bar.visible, "the stance bar should show for a player army"):
		return
	_check(bar.get_child_count() == 6, "six stance buttons, got %d" % bar.get_child_count())
	var options: Dictionary = sim.call("get_stance_options", army_id)
	_check(options.size() == 6, "get_stance_options should list six stances: %s" % [options])
	var current := str(sim.call("get_army", army_id).get("stance", "normal"))
	var refused := 0
	var allowed := ""
	for stance: String in StanceBar.STANCES:
		var button := bar.button(stance)
		if not _check(button != null, "no button for %s" % stance):
			continue
		var reason := str(options.get(stance, ""))
		if stance == current:
			_check(button.button_pressed and not button.disabled, "the current stance %s should be pressed" % stance)
		elif reason != "":
			refused += 1
			_check(button.disabled, "%s refused by the core should be disabled" % stance)
			_check(button.tooltip_text.contains(reason), "the tooltip of %s should give the reason « %s »" % [stance, reason])
		else:
			_check(not button.disabled, "%s allowed by the core should be enabled" % stance)
			if allowed == "" and stance != "normal":
				allowed = stance
	_check(refused > 0, "the core should refuse at least one stance at the start (%s)" % [options])
	print("cv3_4: army %s stance %s, %d refused, first allowed %s" % [army_id, current, refused, allowed])
	if _check(allowed != "", "no allowed stance to pick (%s)" % [options]):
		bar.button(allowed).pressed.emit()
		_check(str(sim.call("get_army", army_id).get("stance", "")) == allowed, "the click should set the stance %s" % allowed)
		_check(map.ui.general_seal.stance_bar.current == allowed and map.ui.general_seal.stance_bar.button(allowed).button_pressed,
			"the bar should be refreshed on %s" % allowed)
		var marker: Node3D = map.armies.get("_markers").get(army_id)
		if _check(marker != null, "no marker for the army"):
			var badge := marker.get_node_or_null("Finial/" + StanceBadge.NODE_NAME) as Sprite3D
			_check(badge != null and badge.texture != null and str(badge.get_meta("stance", "")) == allowed,
				"the standard should carry the %s badge" % allowed)
			# Embuscade : rendu fantôme pour le propriétaire (la vision du cœur la cache à l'ennemi).
			StanceBadge.apply(marker, "ambush", true)
			var ghosts := 0
			for node in marker.find_children("*", "GeometryInstance3D", true, false):
				if node.name != StanceBadge.NODE_NAME and is_equal_approx((node as GeometryInstance3D).transparency, StanceBadge.AMBUSH_TRANSPARENCY):
					ghosts += 1
			_check(ghosts > 0, "an ambushing player army should be semi-transparent")
			StanceBadge.apply(marker, "ambush", false)
			_check(float(marker.get_meta("stance_transparency", -1.0)) == 0.0, "no ghost for another faction's army")
		map.call("_on_stance_changed", army_id, "normal")
		_check(str(sim.call("get_army", army_id).get("stance", "")) == "normal", "back to the normal stance")


# --- 3. Rencontres --------------------------------------------------------------------------


func _test_encounters(map: Node, sim: Object, army_id: String) -> void:
	var ctl: EncounterController = map.encounters
	if not _check(ctl != null and ctl.available(), "the encounter controller should be set up"):
		return
	map.select_army(army_id)
	var army: Dictionary = sim.call("get_army", army_id)
	var start: Vector2 = army["position"]
	var rect: Rect2 = map.movement_ctl.bubble.covered_rect()
	var radius := maxf(rect.size.x, rect.size.y) * 0.5
	var spot := Vector2(-1, -1)
	for i in 24:
		var point := start + Vector2.RIGHT.rotated(TAU * float(i) / 24.0) * radius * 0.3
		var plan: Dictionary = sim.call("find_path_points", army_id, point.x, point.y)
		if plan.get("ok", false) and bool(plan.get("reachable_this_turn", false)):
			spot = point
			break
	if not _check(spot.x >= 0.0, "no reachable spot near the army at %s" % start):
		return
	var site := int(sim.call("debug_place_encounter", "enc_marchands_lombards", spot.x, spot.y))
	if not _check(site >= 0, "debug_place_encounter refused %s" % spot):
		return
	map.refresh_all()
	await process_frame
	var node: Control = ctl.marker(site)
	if not _check(node != null, "the site should have a marker (%d markers)" % ctl.marker_count()):
		return
	_check(node.tooltip_text.contains("Rencontre") and node.tooltip_text.contains(str(node.get("site").get("title", "?"))),
		"the marker bubble should give the title: %s" % node.tooltip_text)
	_check(node.tooltip_text.contains(str(node.get("site").get("province_name", "?"))), "the bubble should give the province")
	_check(not ctl.window.visible, "no window before the army meets the site")
	# Le rapport de saison (ouvert par la bataille automatique) passe avant la fenêtre.
	var report: Control = map.flow.season_report
	if report != null:
		report.show()
	ctl.site_clicked(site)
	await process_frame
	if report != null:
		_check(not ctl.window.visible, "the encounter window should wait for the season report")
		report.hide()
		await process_frame
	var queue: Array = sim.call("get_pending_encounters")
	if not _check(queue.size() == 1, "the march should meet the site (%d pending)" % queue.size()):
		return
	_check(ctl.window.visible, "a pending encounter should open the choice window")
	var shown: Dictionary = ctl.window.current_encounter()
	_check(int(shown.get("site", -1)) == site and str(shown.get("army", "")) == army_id, "the window should show the met site")
	var buttons := ctl.window.option_buttons()
	var options: Array = queue[0].get("options", [])
	_check(buttons.size() == options.size() and buttons.size() >= 2, "one button per option (%d / %d)" % [buttons.size(), options.size()])
	var chosen := -1
	for i in mini(buttons.size(), options.size()):
		var option: Dictionary = options[i]
		_check(buttons[i].disabled == not bool(option.get("available", true)), "option %d availability should follow the core" % i)
		if not bool(option.get("available", true)):
			_check(buttons[i].tooltip_text.contains(str(option.get("reason", ""))), "a refused option should give its reason")
		elif chosen < 0 and str(option.get("outcome", "")) == "":
			chosen = i
	# « Plus tard » : fermée sans réponse, pas rouverte au rafraîchissement.
	ctl.window.call("_close")
	map.refresh_all()
	_check(not ctl.window.visible, "a dismissed encounter should not reopen by itself")
	_check(ctl.open_window() and ctl.window.visible, "the encounter window should reopen on demand")
	if _check(chosen >= 0, "no available option without a battle"):
		ctl.window.option_buttons()[chosen].pressed.emit()
		await process_frame
		_check((sim.call("get_pending_encounters") as Array).is_empty(), "the choice should answer the encounter")
		_check(not ctl.window.visible, "the window should close after the choice")
		_check(ctl.marker(site) == null, "the answered site should lose its marker")


# --- 4. Classes de résultat -----------------------------------------------------------------


func _test_outcomes(map: Node, sim: Object) -> void:
	# Écran de fin de bataille (B2) : bandeau de classe tiré de `resolve_battle().outcome`.
	var screen := BattleResultScreen.new()
	root.add_child(screen)
	var sides := {
		"attacker": {"name": "France", "faction": "fac_france", "color": Color(0.2, 0.3, 0.8)},
		"defender": {"name": "Angleterre", "faction": "fac_england", "color": Color(0.8, 0.2, 0.2)},
	}
	var outcome := {"winner": "attacker", "duration": 600.0, "attacker": {"total_losses": 20}, "defender": {"total_losses": 400}}
	var aftermath := {"campaign_outcome": {"attacker_class": "heroic", "attacker_label": "Victoire héroïque", "defender_class": "disaster", "defender_label": "Désastre"}}
	screen.show_result("Bataille d'essai", "attacker", sides, [], outcome, aftermath)
	await process_frame
	_check(screen.outcome_band != null and screen.outcome_band.visible and screen.outcome_band.label.text == "Victoire héroïque",
		"the result screen should show the class band of the player's side")
	if screen.outcome_band != null:
		_check(screen.outcome_band.get_theme_stylebox("panel").bg_color == OutcomeBand.color_for("heroic"), "the band colour should follow the class")
	screen.queue_free()

	# Notice de résolution automatique sur la carte.
	var enemy := "fac_england"
	var armies := BattleScene.main_armies(sim, str(map.player_faction), enemy)
	if not _check(armies.size() >= 2 and sim.has_method("debug_stage_battle"), "no armies to stage a battle"):
		return
	sim.call("debug_stage_battle", armies[0], armies[1])
	var pending: Array = sim.call("get_pending_battles")
	if not _check(not pending.is_empty(), "the staged battle should be pending"):
		return
	map.call("_on_battle_auto", int(pending[0].get("index", 0)))
	await process_frame
	var last: Dictionary = sim.call("get_last_battle_outcome")
	var notice: OutcomeNotice = map.outcome_notice
	_check(str(last.get("player_label", "")) != "", "the auto-resolved battle should have a class for the player: %s" % [last])
	_check(notice.band.visible and notice.band.label.text.begins_with(str(last.get("player_label", "?"))),
		"the map notice should show the class « %s »: %s" % [last.get("player_label", ""), notice.band.label.text])
	_check(notice.band.outcome_class == str(last.get("player_class", "")), "the notice colour should follow the class")
	_check(not notice.check(), "the same outcome should not be announced twice")
	print("cv3_4: auto-resolved battle → %s" % notice.band.label.text)
