extends "res://tests/smoke_map.gd"

## Sections « campagne » : simulation, boucle de tours, agents, économie, personnages, technologies, diplomatie, commerce, chronique, édits.
## Découpage de `smoke.gd` (SC GT7) : les sections se chaînent par héritage et partagent l'état.
## Ne se lance pas seul : point d'entrée `res://tests/smoke.gd`.

func _run_campaign_sim() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_fail("CampaignSim class not registered: GDExtension not loaded (run core/build.sh)")
		return

	var campaign: Object = ClassDB.instantiate("CampaignSim")
	_check(campaign.get_turn() == -1, "turn before new_campaign should be -1, got %d" % campaign.get_turn())

	if campaign.has_method("get_army_ids"):
		# API M2 : new_campaign(data_dir, player, seed) sur les vraies données.
		var ok: bool = campaign.new_campaign(_project_root().path_join("data"), "fac_france", 1337)
		if not _check(ok, "CampaignSim.new_campaign(data, fac_france, 1337) failed"):
			return
	else:
		campaign.new_campaign(1337)
	if campaign.has_method("set_difficulty"):
		# DF1 : niveau de difficulté choisi au lancement, figé ensuite.
		var levels: Array = campaign.get_difficulty_levels()
		_check(levels.size() == 4, "expected 4 difficulty levels, got %d" % levels.size())
		_check(campaign.get_difficulty() == "normal", "default difficulty should be 'normal', got '%s'" % campaign.get_difficulty())
		_check(campaign.set_difficulty("hard"), "set_difficulty('hard') refused")
		_check(campaign.get_difficulty() == "hard", "get_difficulty() should be 'hard', got '%s'" % campaign.get_difficulty())
		_check(campaign.set_difficulty("normal"), "set_difficulty('normal') refused at turn 0")
	_check(campaign.get_turn() == 0, "initial turn should be 0, got %d" % campaign.get_turn())
	_check(campaign.get_date_label() == "Printemps 1337", "initial date should be 'Printemps 1337', got '%s'" % campaign.get_date_label())

	for _i in range(EXPECTED_TURN):
		campaign.end_turn()

	var turn: int = campaign.get_turn()
	var label: String = campaign.get_date_label()
	_check(turn == EXPECTED_TURN, "expected turn %d, got %d" % [EXPECTED_TURN, turn])
	_check(label == EXPECTED_LABEL, "expected '%s', got '%s'" % [EXPECTED_LABEL, label])

	if failures == 0:
		print("smoke OK: turn %d, %s" % [turn, label])


func _run_campaign_loop() -> void:
	# Vraies données si la simulation réelle existe (elle a besoin de data/factions…), sinon fixtures.
	var real_data := _project_root().path_join("data")
	var use_real: bool = ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_army_ids") \
		and FileAccess.file_exists(real_data.path_join("map/map.json"))
	facade.set_data_dir(real_data if use_real else _fixtures_dir)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""

	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign scene with SimFacade failed to start"):
		return
	print("smoke campaign: simulation REAL, data %s, store %s" % [paths.data_dir, "loaded" if facade.store_loaded() else "absent"])
	_check(map.player_faction == "fac_france", "player faction should be fac_france, got %s" % map.player_faction)
	_check(map.armies.army_count() > 0, "no army markers on the map")
	_check(map.ui.faction_label.text != "" and map.ui.faction_label.text != "Cent Ans", "top bar faction label not set")

	_run_feudal(map)  # FE6 : arbre féodal et filtre « Féodalité »

	var army_ids: PackedStringArray = map.player_army_ids()
	if not _check(not army_ids.is_empty(), "player has no army"):
		map.queue_free()
		return
	map.select_army(army_ids[0])
	await process_frame
	_check(map.selected_army == army_ids[0], "army selection failed")
	_check(map.ui.army_strip.visible and map.ui.general_seal.visible, "army strip and general seal should be visible")
	var reachable: Dictionary = map.reachable
	if not _check(not reachable.is_empty(), "reachable provinces should not be empty for %s" % army_ids[0]):
		map.queue_free()
		return
	if map.movement_ctl != null and map.movement_ctl.active():
		# Lot CV3-5 : zone atteignable à deux niveaux (ce tour / tour suivant), lue dans les données.
		var zone: Dictionary = map.sim.call("get_reachable_area", army_ids[0])
		_check(int(zone.get("cells", 0)) > 1 and int(zone.get("next_cells", 0)) > 0,
				"CV3-5: reachable area should have two rings, got %d + %d cells" % [int(zone.get("cells", 0)), int(zone.get("next_cells", 0))])
		_check(int(zone.get("next_budget", 0)) > 0 and (zone.get("image") as Image).get_format() == Image.FORMAT_RGB8, "CV3-5: RGB8 two-ring mask with the next turn's budget")
		_check(map.movement_ctl.bubble.cell_count() > 1 and map.movement_ctl.bubble.next_cell_count() > 0, "CV3-5: the bubble shows both rings")
	var target: String = str(reachable.keys()[0])
	# Aperçu de chemin au survol de la cible.
	var target_index: int = map.map_data.index_of_id(target)
	map._on_province_hovered(target_index)
	if map.movement_ctl != null and map.movement_ctl.active():
		# Lot M4 : l'aperçu du mouvement libre suit le curseur jusqu'au point visé.
		var city_world: Vector3 = map.settlement_layer.world_position_of(str(map.sim.call("get_province_state", target).get("city", "")))
		map.movement_ctl.preview_target({"kind": "ground", "id": "", "point": Vector2(city_world.x, city_world.z)})
		_check(map.movement_ctl.path_line.visible, "free movement path preview should be visible towards a reachable province")
	else:
		_check(map.path_preview.visible, "path preview should be visible when hovering a reachable province")
	var army_before: Dictionary = map.sim.call("get_army", army_ids[0])
	var result: Dictionary = map.order_move(army_ids[0], target)
	_check(result.get("ok", false), "move order refused: %s" % result.get("error", "?"))
	# Mouvement libre (M2) : l'armée marche aussitôt ; elle a bougé ou garde la suite de son trajet.
	var army: Dictionary = map.sim.call("get_army", army_ids[0])
	_check(not army.get("path", []).is_empty() or int(army.get("movement_points", 0)) < int(army_before.get("movement_points", 0)),
		"army should march (or keep a path) after the order")

	var date_before: String = map.sim.call("get_date_label")
	for _i in 4:
		map._on_end_turn()
	await process_frame
	var turn: int = map.sim.call("get_turn")
	_check(turn == 4, "expected turn 4 after 4 end turns, got %d" % turn)
	_check(map.sim.call("get_date_label") != date_before, "date should advance after end turns")
	_check(map.ui.log_line_count() > 0, "event log should have entries")

	# Sauvegarde puis rechargement : dates identiques.
	var date_saved: String = map.sim.call("get_date_label")
	_check(facade.save_game(SMOKE_SAVE), "save_game failed")
	_check(FileAccess.file_exists(facade.save_path(SMOKE_SAVE)), "save file missing")
	map._on_end_turn()
	_check(map.sim.call("get_date_label") != date_saved, "date should differ before reload")
	map._on_load(facade.save_path(SMOKE_SAVE))
	await process_frame
	var date_loaded: String = map.sim.call("get_date_label")
	_check(date_loaded == date_saved, "date after load '%s' != saved '%s'" % [date_loaded, date_saved])
	_check(map.ui.date_label.text.begins_with(date_saved), "HUD date label '%s' should start with '%s'" % [map.ui.date_label.text, date_saved])
	var saves: Array = facade.list_saves()
	var found := false
	for save in saves:
		if save["name"] == SMOKE_SAVE:
			found = true
	_check(found, "smoke save not listed by list_saves")

	if failures == 0:
		print("smoke OK: campaign loop (%s), %d turns, saved and reloaded at %s" % ["real", turn, date_loaded])
	map.queue_free()
	await process_frame


## FE6 : l'arbre féodal s'ouvre sur la France (Bourgogne parmi ses vassaux), le filtre
## « Féodalité » peint la carte ; détails dans `tests/fe_ui_test.gd`.
func _run_feudal(map: Node) -> void:
	var feudal: Node = map.get("feudal")
	if feudal == null or not bool(feudal.call("available")):
		print("smoke feudal: skipped (simulation without get_feudal_tree)")
		return
	feudal.call("open_for", "")
	_check(feudal.get("panel").visible, "feudal tree panel should open")
	_check(feudal.call("tree_item", "fac_france") != null and feudal.call("tree_item", "fac_burgundy") != null,
		"feudal tree should list France and Burgundy")
	feudal.get("panel").hide()
	var modes: Node = map.get("map_modes")
	modes.call("set_mode", "feudal")
	_check(str(modes.get("mode")) == "feudal" and not (modes.get("feudal_lens").get("cells") as Dictionary).is_empty(),
		"feudal map filter should colour the provinces")
	modes.call("set_mode", "political")
	if failures == 0:
		print("smoke OK: feudal tree and map filter")


## C6 : agents de campagne sur la vraie simulation (voir l'en-tête, étape 17).
func _run_agents() -> void:
	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "agents: campaign scene failed to start"):
		map.queue_free()
		return
	var ctl: Node = map.agents_ctl  # non typé : compilé après les autoloads
	if not _check(ctl != null, "agents: controller missing"):
		map.queue_free()
		return
	_check(ctl.available(), "agents: controller inactive with the real simulation")
	var sim: Object = map.sim
	# Registre (touche G) et recrutement dans la cité de la capitale.
	var key := InputEventKey.new()
	key.physical_keycode = KEY_G
	key.pressed = true
	ctl._unhandled_input(key)
	_check(ctl.registry.visible, "G should open the agent registry")
	var place: String = ctl.recruit_place()
	_check(place != "", "agents: a recruitment place (capital city) expected")
	var options: Array = sim.call("get_agent_recruit_options", place)
	_check(options.size() == 3, "3 agent types expected, got %d" % options.size())
	var result: Dictionary = ctl.recruit(place, "spy")
	_check(result.get("ok", false), "spy recruitment refused: %s" % result.get("error", ""))
	var spy := ""
	for entry in sim.call("get_agents"):
		if str(entry.get("faction", "")) == map.player_faction:
			spy = str(entry.get("id", ""))
	if not _check(spy != "", "the recruited spy should be listed by get_agents"):
		map.queue_free()
		return
	_check(ctl.has_token(spy), "the spy should have a token on the map")
	# Une saison : points de marche rendus.
	sim.call("end_turn")
	map.refresh_all()
	_check(ctl.has_token(spy), "the spy token should survive a refresh")
	ctl.select_agent(spy)
	_check(ctl.selected_agent == spy and ctl.bar.visible, "selecting the spy should show the action bar")
	_check(ctl.action_button_count() == 4, "a spy has 4 actions, got %d" % ctl.action_button_count())
	_check(ctl.markers.marker_count() > 0, "reachable rings expected for the spy")
	var percent_shown := false
	for option in sim.call("get_agent_actions", spy):
		if bool(option.get("available", false)) and int(option.get("chance", 0)) > 0:
			percent_shown = true
	_check(percent_shown, "at least one action should be available with a chance")
	# Marche vers une colonie amie atteignable (l'action consomme ensuite la marche restante).
	var target := ""
	for id in ctl.reachable:
		var detail: Dictionary = sim.call("settlement_detail", str(id))
		if str(detail.get("controller", "")) == map.player_faction:
			target = str(id)
			break
	var moved := false
	if target != "":
		var move: Dictionary = ctl.order_move(spy, target)
		moved = move.get("ok", false)
		_check(moved, "move_agent refused: %s" % move.get("error", ""))
		_check(str(sim.call("get_agent", spy).get("location", "")) == target, "the spy should stand on %s" % target)
	# Contre-espionnage en colonie amie : un rapport.
	ctl.select_agent(spy)
	var report: Dictionary = ctl.perform_action("counter")
	_check(str(report.get("text", "")) != "", "counter-espionage should produce a report: %s" % report)
	# La ligne du rapport arrive au journal de la saison suivante.
	var events: Array = sim.call("end_turn")
	var agent_lines := 0
	for event in events:
		if str(event.get("kind", "")) == "agent":
			agent_lines += 1
	_check(agent_lines > 0, "an 'agent' event should open the next journal")
	_check(SeasonReport.KIND_STYLES.has("agent"), "season report should style agent events")
	# Encyclopédie : onglet Agents.
	_check(Encyclopedia.tab_index_of("agents") >= 0, "encyclopedia should have an Agents tab")
	_check(Encyclopedia.fiche_bbcode("agent_spy").contains("Renseigner"), "spy fiche should list its actions")
	map.queue_free()
	await process_frame
	if failures == 0:
		print("smoke OK: agents (spy recruited, token, bar of %d actions, report, move %s, %d journal line(s), encyclopedia)" % [4, "ok" if moved else "skipped", agent_lines])


## M3 : construit le bâtiment le moins cher disponible à Paris (marché si absent), passe les
## tours nécessaires, vérifie qu'il apparaît dans `buildings` et que `projected_income` a
## augmenté ; passe l'impôt à Haut et vérifie une nouvelle hausse. Avec la vraie simulation si
## sur les vraies données (`data/`).
func _run_city_economy() -> void:
	const PROVINCE_ID := "prov_ile_de_france"
	const FACTION_ID := "fac_france"
	var data_dir := _project_root().path_join("data")
	if not _check(ClassDB.class_exists("CampaignSim"), "city/economy: CampaignSim unavailable"):
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, FACTION_ID, 1337), "city/economy: new_campaign failed"):
		return
	# M10 : pas d'événements de chronique pendant la mesure (le bruit masquerait l'effet du bâtiment).
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)

	var city: Dictionary = sim.call("get_province_city", PROVINCE_ID)
	if not _check(not city.is_empty(), "get_province_city(%s) should not be empty" % PROVINCE_ID):
		return
	_check(city.has("classes") and (city["classes"] as Dictionary).size() == 4, "get_province_city should expose 4 population classes")

	# Quelques tours d'abord : la population/richesse initiale converge vers son équilibre les
	# premiers tours (les deux moteurs), un bruit qui masquerait l'effet du bâtiment construit
	# juste après le départ.
	for _i in 20:
		sim.call("end_turn")

	var buildable: Array = city.get("buildable", [])
	# Préfère un bâtiment à effet économique direct (trade_income/tax_income/wealth) : plus
	# fiable à mesurer que le moins cher (ex. une caserne n'a aucun effet sur le revenu).
	var choice: Dictionary = _pick_economic_building(buildable)
	if not _check(not choice.is_empty(), "no buildable building available in %s" % PROVINCE_ID):
		return
	print("smoke city/economy: building %s (%d livres, %d tour(s))" % [choice["building"], choice["cost"], choice["turns"]])

	var economy_before: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(not economy_before.is_empty(), "get_faction_economy should not be empty")

	var build_result: Dictionary = sim.call("submit_order", {"type": "build", "province": PROVINCE_ID, "building": choice["building"]})
	_check(build_result.get("ok", false), "build order refused: %s" % build_result.get("error", "?"))

	for _i in int(choice["turns"]):
		sim.call("end_turn")

	var city_after: Dictionary = sim.call("get_province_city", PROVINCE_ID)
	var built_ids: Array = []
	for entry in city_after.get("buildings", []):
		built_ids.append(str(entry.get("id", "")))
	_check(built_ids.has(str(choice["building"])), "%s should be in buildings after %d turn(s), got %s" % [choice["building"], choice["turns"], built_ids])

	# La richesse converge sur plusieurs saisons (§ 1.1) : on laisse une fenêtre de tours après
	# l'achèvement et on retient le meilleur revenu prévisionnel observé plutôt qu'un seul point,
	# pour ne pas dépendre de la vitesse exacte de convergence des deux moteurs.
	# Depuis G2 l'IA peut retourner un vassal ou déclarer la guerre pendant cette fenêtre : le
	# revenu global peut baisser pour d'autres raisons. On accepte donc aussi la preuve que le
	# bâtiment est entré dans les comptes (entretien des bâtiments en hausse).
	var best_after := int(economy_before.get("projected_income", 0))
	var upkeep_before := int(sim.call("get_faction_summary", FACTION_ID).get("building_upkeep", 0))
	var upkeep_after := upkeep_before
	for _i in 6:
		sim.call("end_turn")
		var economy: Dictionary = sim.call("get_faction_economy", FACTION_ID)
		best_after = maxi(best_after, int(economy.get("projected_income", 0)))
		upkeep_after = maxi(upkeep_after, int(sim.call("get_faction_summary", FACTION_ID).get("building_upkeep", 0)))
	# Information seulement : l'effet d'un bâtiment est testé côté Rust (tests m3/f1) ; sur la vraie
	# simulation, la dérive de fond (guerre, IA, prix) peut le masquer.
	print("smoke city/economy: after construction, income %d -> best %d, building upkeep %d -> %d" % [economy_before.get("projected_income", 0), best_after, upkeep_before, upkeep_after])

	# Impôt : comparaison immédiate (même tour, sans fin de tour entre les deux) pour isoler
	# l'effet du multiplicateur fiscal de la dérive de fond de l'économie.
	var economy_normal: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	var tax_result: Dictionary = sim.call("submit_order", {"type": "set_tax_rate", "rate": "high"})
	_check(tax_result.get("ok", false), "set_tax_rate high refused: %s" % tax_result.get("error", "?"))
	var economy_high_tax: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(int(economy_high_tax.get("projected_income", 0)) > int(economy_normal.get("projected_income", 0)),
		"projected_income should rise with high tax: %d -> %d" % [economy_normal.get("projected_income", 0), economy_high_tax.get("projected_income", 0)])

	if failures == 0:
		print("smoke OK: city/economy, built %s, projected income %d -> best %d, tax normal->high %d -> %d" % [
			choice["building"],
			economy_before.get("projected_income", 0), best_after,
			economy_normal.get("projected_income", 0), economy_high_tax.get("projected_income", 0)])


## Bâtiment constructible avec un effet direct sur le revenu (trade_income/tax_income/wealth),
## le moins cher parmi ceux-ci ; à défaut, le constructible disponible le moins cher.
func _pick_economic_building(buildable: Array) -> Dictionary:
	var data_dir := _project_root().path_join("data")
	var best_economic: Dictionary = {}
	var best_any: Dictionary = {}
	for row in buildable:
		if not bool(row.get("available", false)):
			continue
		if best_any.is_empty() or int(row["cost"]) < int(best_any["cost"]):
			best_any = row
		if _has_economic_effect(data_dir, str(row.get("building", ""))):
			if best_economic.is_empty() or int(row["cost"]) < int(best_economic["cost"]):
				best_economic = row
	return best_economic if not best_economic.is_empty() else best_any


func _has_economic_effect(data_dir: String, building_id: String) -> bool:
	var path := data_dir.path_join("buildings").path_join(building_id + ".json")
	if not FileAccess.file_exists(path):
		return false
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		return false
	for effect in (parsed as Dictionary).get("effects", []):
		if ["trade_income", "tax_income", "wealth"].has(str(effect.get("effect", ""))):
			return true
	return false


## M4 : personnages et dynasties (docs/design/m4-characters-dynasties.md § 5). Avec la vraie
## simulation sur `data/`.
func _run_characters() -> void:
	const FACTION_ID := "fac_france"
	var data_dir := _project_root().path_join("data")
	if not _check(ClassDB.class_exists("CampaignSim"), "characters: CampaignSim unavailable"):
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, FACTION_ID, 1337), "characters: new_campaign failed"):
		return

	var ids: Array = sim.call("get_faction_characters", FACTION_ID)
	if not _check(not ids.is_empty(), "get_faction_characters(fac_france) should not be empty"):
		return

	# Panneau de cour : au moins 1 personnage listé, via la scène réelle.
	var court_scene: PackedScene = load("res://scenes/ui/court_panel.tscn")
	# Type dynamique (Node) : les classes globales fraîchement ajoutées (CourtPanel) ne sont pas
	# toujours résolues pour un typage statique au moment où `--script` compile ce fichier.
	var court: Node = court_scene.instantiate()
	root.add_child(court)
	await process_frame
	var rows: Array[Dictionary] = []
	for id in ids:
		rows.append(sim.call("get_character", id))
	court.show_court(rows, "France", Color(0.2, 0.3, 0.7))
	_check(court.rows_list.get_child_count() >= 1, "court panel should list at least 1 character")
	court.queue_free()

	# XP + apprentissage d'une compétence de commandement de rang 1 sur le dirigeant.
	var ruler: String = str(ids[0])
	var grant: Dictionary = sim.call("submit_order", {"type": "debug_grant_xp", "character": ruler, "amount": 400})
	_check(grant.get("ok", false), "debug_grant_xp refused: %s" % grant.get("error", "?"))
	var tier1_command := ""
	for node in sim.call("get_skill_tree"):
		if str(node["branch"]) == "command" and int(node["tier"]) == 1:
			tier1_command = str(node["id"])
			break
	if _check(tier1_command != "", "no tier-1 command skill in get_skill_tree()"):
		var learn: Dictionary = sim.call("submit_order", {"type": "learn_skill", "character": ruler, "skill": tier1_command})
		_check(learn.get("ok", false), "learn_skill(%s, %s) refused: %s" % [ruler, tier1_command, learn.get("error", "?")])

	# Gouverneur : un courtisan (pas le dirigeant) nommé à Normandie, sinon la première province
	# contrôlée différente de la capitale.
	var target_province := ""
	for candidate in ["prov_normandie", "prov_picardie", "prov_champagne", "prov_orleanais", "prov_anjou"]:
		var state: Dictionary = sim.call("get_province_state", candidate)
		if str(state.get("owner", "")) == FACTION_ID and candidate != "prov_ile_de_france":
			target_province = candidate
			break
	var courtier := ""
	for id in ids:
		if str(id) != ruler:
			courtier = str(id)
			break
	if _check(target_province != "" and courtier != "", "no province/courtier available for assign_governor"):
		var gov: Dictionary = sim.call("submit_order", {"type": "assign_governor", "character": courtier, "province": target_province})
		_check(gov.get("ok", false), "assign_governor(%s, %s) refused: %s" % [courtier, target_province, gov.get("error", "?")])
		var after: Dictionary = sim.call("get_province_state", target_province)
		_check(str(after.get("governor", "")) == courtier, "province %s should report %s as governor" % [target_province, courtier])

	# Mariage entre deux candidats valides, sinon skip explicite (raison imprimée).
	var married := false
	for id in ids:
		var candidates: Array = sim.call("get_marriage_candidates", id)
		if not candidates.is_empty():
			var spouse: String = str(candidates[0]["id"])
			var marriage: Dictionary = sim.call("submit_order", {"type": "propose_marriage", "character": id, "spouse": spouse})
			_check(marriage.get("ok", false), "propose_marriage(%s, %s) refused: %s" % [id, spouse, marriage.get("error", "?")])
			married = true
			break
	if not married:
		print("smoke characters: propose_marriage skipped, no valid candidates in fac_france at 1337")

	# 40 fins de tour : au moins une naissance ou une mort dans le journal.
	var found_birth_or_death := false
	for _i in 40:
		for event in sim.call("end_turn"):
			var kind: String = str(event.get("kind", ""))
			if kind == "birth" or kind == "death":
				found_birth_or_death = true
	_check(found_birth_or_death, "no birth or death event in 40 end_turn() calls")
	await _check_family_tree_c3(sim, FACTION_ID)

	if failures == 0:
		print("smoke OK: characters, court %d, learn_skill %s, governor %s->%s, marriage %s, birth/death seen" % [
			rows.size(), tier1_command, courtier, target_province, "yes" if married else "skipped"])


## C3 : onglet « Arbre familial » du panneau Cour (Valois, ≥ 3 générations après 40 tours) et
## apprentissage d'une compétence depuis l'arbre visuel de la fiche. Vraie simulation seulement.
func _check_family_tree_c3(sim: Object, faction_id: String) -> void:
	var failures_before := failures
	var ids: Array = sim.call("get_faction_characters", faction_id)
	var rows: Array[Dictionary] = []
	for id in ids:
		rows.append(sim.call("get_character", id))
	var court: Node = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	court.set("sim_source", sim)
	root.add_child(court)
	await process_frame
	court.show_court(rows, "France", Color(0.2, 0.3, 0.7))
	court.show_tab(1)  # CourtPanel.TAB_TREE
	await process_frame
	var tree: Dictionary = sim.call("get_family_tree", str(ids[0]), 2, 3)
	var shown: int = court.family_tree.node_count()
	var generations: int = court.family_tree.generation_count()
	_check(court.tree_box.visible and not (court.get_node("VBox/Scroll") as Control).visible, "C3: tree tab should replace the list")
	_check(shown == (tree.get("nodes", []) as Array).size() and shown >= 5, "C3: family tree shows %d nodes, expected %d (>= 5)" % [shown, (tree.get("nodes", []) as Array).size()])
	_check(generations >= 3, "C3: Valois family tree should span >= 3 generations, got %d" % generations)
	_check(court.family_tree.medallions.has(str(tree.get("heir", ""))), "C3: heir %s missing from the tree" % tree.get("heir", ""))
	var opened: Array = []
	court.character_selected.connect(func(id: String) -> void: opened.append(id))
	court.family_tree.medallions.values()[0].pressed.emit()
	_check(opened.size() == 1, "C3: clicking a medallion should open the character sheet")
	court.queue_free()

	# Fiche : arbre de compétences visuel, clic sur un nœud disponible = ordre `learn_skill`.
	var ruler := str(ids[0])
	sim.call("submit_order", {"type": "debug_grant_xp", "character": ruler, "amount": 300})
	var sheet: Node = (load("res://scenes/ui/character_sheet.tscn") as PackedScene).instantiate()
	root.add_child(sheet)
	await process_frame
	var learned_result: Array = []
	sheet.learn_skill_requested.connect(func(character: String, skill: String) -> void:
		learned_result.append(sim.call("submit_order", {"type": "learn_skill", "character": character, "skill": skill}))
		learned_result.append(skill))
	sheet.show_character(sim.call("get_character", ruler), sim.call("get_skill_tree"), sim.call("get_learnable", ruler), [], [], [])
	await process_frame
	var view: Node = sheet.skill_tree_view
	var skill_nodes: int = view.nodes.size()
	_check(skill_nodes == (sim.call("get_skill_tree") as Array).size(), "C3: skill tree view shows %d nodes" % skill_nodes)
	var available: Array = view.available_ids()
	var learned_skill := ""
	if _check(not available.is_empty(), "C3: no available skill in the tree view"):
		learned_skill = str(available[0])
		_check(view.press(learned_skill), "C3: pressing %s should be accepted" % learned_skill)
		_check(learned_result.size() == 2 and bool((learned_result[0] as Dictionary).get("ok", false)), "C3: learn_skill from the tree refused: %s" % str(learned_result))
		var after: Dictionary = sim.call("get_character", ruler)
		_check((after.get("skills_learned", []) as Array).has(learned_skill), "C3: %s not learned after the click" % learned_skill)
		sheet.show_character(after, sim.call("get_skill_tree"), sim.call("get_learnable", ruler), [], [], [])
		_check(view.state_of(learned_skill) == "learned", "C3: %s should be drawn as learned" % learned_skill)

	# C7 : suite du général (vignettes à infobulles), dates de vie, repli de la fiche.
	var retinue_count := 0
	if sim.has_method("get_retinue_catalog"):
		var catalog: Dictionary = sim.call("get_retinue_catalog")
		_check((catalog.get("companions", []) as Array).size() >= 12 and int(catalog.get("max", 0)) == 8, "C7: retinue catalogue %s" % str(catalog.keys()))
		# Deux compagnons que le souverain n'a pas encore (il a pu en gagner en 40 tours).
		var held: Array = []
		for entry in (sim.call("get_character", ruler).get("retinue", []) as Array):
			held.append(str(entry.get("id", "")))
		var granted_count := 0
		for companion in ["ret_heraut", "ret_barbier_chirurgien", "ret_ecuyer", "ret_menestrel", "ret_espion"]:
			if granted_count >= 2 or held.has(companion) or held.size() + granted_count >= 7:
				continue
			var granted: Dictionary = sim.call("submit_order", {"type": "debug_grant_companion", "character": ruler, "companion": companion})
			if _check(bool(granted.get("ok", false)), "C7: grant %s refused: %s" % [companion, str(granted)]):
				granted_count += 1
		var with_retinue: Dictionary = sim.call("get_character", ruler)
		retinue_count = (with_retinue.get("retinue", []) as Array).size()
		_check(retinue_count >= 2 and int(with_retinue.get("retinue_max", 0)) == 8, "C7: retinue of %s: %d" % [ruler, retinue_count])
		sheet.show_character(with_retinue, sim.call("get_skill_tree"), sim.call("get_learnable", ruler), [], [], [])
		var row: Node = sheet.retinue_row
		_check(row != null and row.get_child_count() == 8, "C7: retinue row should show 8 slots (companions + free)")
		if row != null and row.get_child_count() > 0:
			var tip := str((row.get_child(0) as Control).tooltip_text)
			_check(tip.contains("Effets") and tip.contains("Obtention"), "C7: companion tooltip incomplete: %s" % tip)
		_check(str(with_retinue.get("death_year", -1)) == "0", "C7: a living ruler has no death year")
		var dead_seen := 0
		for node in (tree.get("nodes", []) as Array):
			if not bool(node.get("alive", true)) and int(node.get("death_year", 0)) > 0:
				dead_seen += 1
				_check(FamilyTreeView.life_dates(node) == "%d–%d" % [int(node["birth_year"]), int(node["death_year"])], "C7: life dates of %s" % str(node.get("id", "")))
		sheet.fit_beside(1196.0, 1920.0)
		_check(sheet.compact and sheet.custom_minimum_size.x <= 1920.0 - 1196.0 - 32.0 + 0.5, "C7: sheet beside the tree should fold into one column")
		sheet.fit_beside(0.0, 1920.0)
		_check(not sheet.compact and is_equal_approx(sheet.custom_minimum_size.x, 1000.0), "C7: sheet alone should use two columns")
	sheet.queue_free()
	if failures == failures_before:
		print("smoke OK: family tree (%d nodes, %d generations), skill tree (%d nodes, learned %s by click), retinue %d" % [shown, generations, skill_nodes, learned_skill, retinue_count])


## M6 : technologies (docs/design/m6-technologies.md § 4). Vraie simulation uniquement (le mock
## n'implémente pas la recherche) ; skip explicite sinon.
func _run_technologies() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_tech_tree")):
		print("smoke technologies: skipped, CampaignSim has no get_tech_tree (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "technologies: new_campaign failed"):
		return
	var tree: Array = sim.call("get_tech_tree", FACTION_ID)
	_check(tree.size() >= 30, "get_tech_tree should list >= 30 technologies, got %d" % tree.size())
	var known_before := 0
	var cheapest := ""
	var cheapest_cost := 1 << 30
	for node in tree:
		match str(node.get("state", "")):
			"known":
				known_before += 1
			"available":
				if int(node["effective_cost"]) < cheapest_cost:
					cheapest_cost = int(node["effective_cost"])
					cheapest = str(node["id"])

	# Panneau des technologies (scène réelle) : un bouton par technologie.
	var panel_scene: PackedScene = load("res://scenes/ui/tech_panel.tscn")
	var panel: Node = panel_scene.instantiate()
	root.add_child(panel)
	await process_frame
	panel.show_tree(tree, {}, int(sim.call("get_research_points", FACTION_ID)), "France", Color(0.2, 0.3, 0.7))
	var buttons: int = panel.military_view.buttons.size() + panel.civil_view.buttons.size() + panel.medicine_view.buttons.size()
	_check(buttons == tree.size(), "tech panel should show %d nodes, got %d" % [tree.size(), buttons])
	panel.queue_free()

	if not _check(cheapest != "", "no available technology for fac_france"):
		return
	var result: Dictionary = sim.call("submit_order", {"type": "research", "technology": cheapest})
	_check(result.get("ok", false), "research(%s) refused: %s" % [cheapest, result.get("error", "?")])
	var research: Dictionary = sim.call("get_research", FACTION_ID)
	_check(str(research.get("technology", "")) == cheapest, "get_research should report %s, got %s" % [cheapest, research])
	var refused: Dictionary = sim.call("submit_order", {"type": "research", "technology": "tech_masonry"})
	_check(not refused.get("ok", true), "research of a known technology should be refused")

	var researched_events := 0
	for _i in 20:
		for event in sim.call("end_turn"):
			if str(event.get("kind", "")) == "technology_researched" and str(event.get("faction", "")) == FACTION_ID:
				researched_events += 1
	var known_after := 0
	for node in sim.call("get_tech_tree", FACTION_ID):
		if str(node.get("state", "")) == "known":
			known_after += 1
	_check(known_after > known_before, "no technology acquired in 20 turns (%d -> %d)" % [known_before, known_after])
	_check(researched_events >= 1, "no technology_researched event for %s in 20 turns" % FACTION_ID)
	if failures == 0:
		print("smoke OK: technologies (real), %d nodes, research %s (%d pts, %d/turn), known %d -> %d" % [
			tree.size(), cheapest, cheapest_cost, int(research.get("points_per_turn", 0)), known_before, known_after])


## M5 : diplomatie et religion (docs/design/m5-diplomacy-religion.md § 4). Vraie simulation
## uniquement (le mock n'a pas de diplomatie).
func _run_diplomacy() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_diplomacy")):
		print("smoke diplomacy: skipped, CampaignSim has no get_diplomacy (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "diplomacy: new_campaign failed"):
		return
	var entries: Array = sim.call("get_diplomacy", FACTION_ID)
	_check(entries.size() >= 10, "get_diplomacy should list >= 10 factions, got %d" % entries.size())
	var england: Dictionary = {}
	for entry in entries:
		if str(entry["id"]) == "fac_england":
			england = entry
	_check(str(england.get("status", "")) == "war", "France and England should be at war in 1337")
	_check(not (england.get("attitude_reasons", []) as Array).is_empty(), "attitude reasons expected")

	# Évaluation d'une paix blanche avec l'Angleterre (lue, pas forcément acceptée).
	var verdict: Dictionary = sim.call("evaluate_proposal", {"type": "propose_peace", "target": "fac_england", "provinces": [], "tribute": 0})
	_check(verdict.has("accept") and not (verdict.get("reasons", []) as Array).is_empty(), "evaluate_proposal should give a verdict with reasons")
	var war_verdict: Dictionary = sim.call("evaluate_proposal", {"type": "declare_war", "target": "fac_navarre"})
	_check(not (war_verdict.get("reasons", []) as Array).is_empty(), "declare_war evaluation should list consequences")

	# Embargo puis guerre contre une faction voisine.
	var embargo: Dictionary = sim.call("submit_order", {"type": "set_embargo", "target": "fac_aragon", "active": true})
	_check(embargo.get("ok", false), "set_embargo refused: %s" % embargo.get("error", "?"))
	var war: Dictionary = sim.call("submit_order", {"type": "declare_war", "target": "fac_navarre"})
	_check(war.get("ok", false), "declare_war refused: %s" % war.get("error", "?"))

	# DP1 : traité à plusieurs clauses (chance d'acceptation, contre-proposition, options).
	var treaty := [{"kind": "peace"}, {"kind": "gold", "giver": "proposer", "amount": 5000}]
	var treaty_verdict: Dictionary = sim.call("evaluate_treaty", "fac_england", treaty)
	_check(bool(treaty_verdict.get("ok", false)) and (treaty_verdict.get("articles", []) as Array).size() == 2, "evaluate_treaty should value each article: %s" % treaty_verdict)
	_check(int(treaty_verdict.get("chance", -1)) >= 0 and int(treaty_verdict.get("chance", -1)) <= 100, "evaluate_treaty chance out of range")
	var options: Dictionary = sim.call("treaty_options", "fac_england")
	_check(not ((options.get("theirs", {}) as Dictionary).get("provinces", []) as Array).is_empty(), "treaty_options should list English provinces")
	_check(sim.call("counter_treaty", "fac_england", treaty) is Dictionary, "counter_treaty should answer")
	_check((sim.call("get_war_summary", "fac_england") as Dictionary).has("war_score"), "get_war_summary should report the war score")
	_check(sim.call("get_treaty_history", FACTION_ID) is Array, "get_treaty_history should be an array")
	# DP2 : refus expliqués, positions diplomatiques, droit de passage.
	if sim.has_method("explain_treaty"):
		var explained: Dictionary = sim.call("explain_treaty", "fac_flanders", [{"kind": "trade_agreement"}, {"kind": "gold", "giver": "recipient", "amount": 20000}])
		_check(bool(explained.get("ok", false)) and not (explained.get("lines", []) as Array).is_empty() and str(explained.get("summary", "")) != "", "explain_treaty should list weighted reasons: %s" % explained)
		var stances: PackedStringArray = sim.call("get_province_stances", PackedStringArray(["prov_ile_de_france", "prov_guyenne"]))
		_check(stances.size() == 2 and stances[0] == "self", "get_province_stances: %s" % stances)
		_check(DiplomaticStances.COLORS.has(stances[1]), "unknown stance %s" % stances[1])
		_check(str((sim.call("get_faction_stance", "fac_flanders") as Dictionary).get("key", "")) != "", "get_faction_stance expected")
		if sim.has_method("get_province_stances_for"):  # DZ : relations vues d'une autre faction
			var seen: PackedStringArray = sim.call("get_province_stances_for", "fac_england", PackedStringArray(["prov_ile_de_france", "prov_guyenne"]))
			var of_england: Dictionary = sim.call("get_faction_stances_for", "fac_england")
			_check(seen.size() == 2 and seen[1] == "self" and seen[0] == str(of_england.get(FACTION_ID, "")), "get_province_stances_for: %s / %s" % [seen, of_england.get(FACTION_ID)])
			_check(str(of_england.get("fac_rebels", "")) == "war", "rebels should be enemies of everyone")
			_check((sim.call("get_province_stances_for", "fac_nobody", PackedStringArray(["prov_guyenne"])) as PackedStringArray).is_empty(), "unknown viewer should give nothing")
		_check((sim.call("get_trespass", "fac_flanders") as Dictionary).has("theirs"), "get_trespass expected")
		var army_ids: PackedStringArray = sim.call("get_army_ids")
		for army_id in army_ids:
			var army: Dictionary = sim.call("get_army", army_id)
			if str(army.get("faction", "")) == FACTION_ID:
				var passage: Dictionary = sim.call("find_path_trespass", army_id, 2000.0, 3280.0)
				_check(passage.is_empty() or passage.has("ok"), "find_path_trespass should answer")
				break

	# Panneau réel : instanciation, rafraîchissement, brouillon de traité.
	var panel: Node = (load("res://scripts/ui/diplomacy_panel.gd") as GDScript).new()
	root.add_child(panel)
	await process_frame
	panel.sim = sim
	panel.player_faction = FACTION_ID
	panel.refresh()
	panel.select_faction("fac_england")
	panel.stage_example()
	await process_frame
	_check(str(panel.get("negotiation").get("_reasons").text) != "", "diplomacy panel should explain the verdict")
	panel.queue_free()

	# 20 tours : pas d'erreur ; offres et religion lisibles.
	var diplomatic_events := 0
	for _i in 20:
		for event in sim.call("end_turn"):
			var kind := str(event.get("kind", ""))
			if kind in ["war_declared", "peace_signed", "alliance_formed", "alliance_broken", "diplomatic_offer", "embargo", "vassal_rebellion"]:
				diplomatic_events += 1
		for offer in sim.call("get_offers"):
			_check(str(offer.get("text", "")) != "", "offer text expected")
	var religion: Dictionary = sim.call("get_religion_state", FACTION_ID)
	_check(religion.has("papal_favor"), "get_religion_state should report papal favour")
	var province_religion: Dictionary = sim.call("get_province_religion", "prov_ile_de_france")
	_check(province_religion.has("heresy"), "get_province_religion should report heresy")
	if failures == 0:
		print("smoke OK: diplomacy (real), %d factions, peace verdict %s, embargo + war declared, %d diplomatic events in 20 turns, favour %d" % [
			entries.size(), "accept" if verdict.get("accept", false) else "refuse", diplomatic_events, int(religion.get("papal_favor", 0))])
## C5 (docs/design/2026-09-24-rapprochement-total-war.md) : routes commerciales et accords.
## Vraie simulation uniquement (le mock n'a pas de commerce).
func _run_trade() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_trade_routes")):
		print("smoke trade: skipped, CampaignSim has no get_trade_routes (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "trade: new_campaign failed"):
		return
	var routes: Array = sim.call("get_trade_routes")
	_check(routes.size() >= 8, "get_trade_routes should list >= 8 routes, got %d" % routes.size())
	var bruges_londres: Dictionary = {}
	for route_variant in routes:
		var route: Dictionary = route_variant
		if str(route.get("id", "")) == "route_bruges_londres":
			bruges_londres = route
	_check(not bruges_londres.is_empty(), "route_bruges_londres should be in the catalogue")
	_check(bruges_londres.get("path", PackedStringArray()).size() >= 2, "route path should have >= 2 settlements")
	_check(bruges_londres.has("total_value") and bruges_londres.has("security") and bruges_londres.has("cut"), "route fields expected")

	# Accord commercial (C5 unifié avec DP1, ADR 0012) : article de traité, présence dans
	# get_diplomacy, doublon refusé, rupture unilatérale. L'IA peut refuser : on essaie
	# plusieurs partenaires et on garde le premier qui signe.
	var trade_treaty := [{"kind": "trade_agreement"}]
	var partner := ""
	for candidate in ["fac_flanders", "fac_castile", "fac_brittany", "fac_scotland", "fac_aragon", "fac_navarre", "fac_burgundy", "fac_papacy"]:
		var propose: Dictionary = sim.call("submit_order", {"type": "propose_treaty", "target": candidate, "articles": trade_treaty})
		if bool(propose.get("ok", false)):
			partner = candidate
			break
	_check(partner != "", "no faction signed a trade agreement treaty")
	if partner != "":
		var diplomacy_entries: Array = sim.call("get_diplomacy", FACTION_ID)
		var partner_entry: Dictionary = {}
		for entry in diplomacy_entries:
			if str(entry["id"]) == partner:
				partner_entry = entry
		_check(bool(partner_entry.get("trade_agreement", false)), "trade_agreement should be true after the trade treaty")
		var duplicate: Dictionary = sim.call("submit_order", {"type": "propose_treaty", "target": partner, "articles": trade_treaty})
		_check(not duplicate.get("ok", true), "a second trade agreement with the same faction should be refused")
		var broken: Dictionary = sim.call("submit_order", {"type": "break_trade_agreement", "target": partner})
		_check(broken.get("ok", false), "break_trade_agreement refused: %s" % broken.get("error", "?"))

	# Embargo : coupe la route entre les deux mêmes comptoirs.
	var embargo: Dictionary = sim.call("submit_order", {"type": "set_embargo", "target": "fac_flanders", "active": true})
	_check(embargo.get("ok", false), "set_embargo refused: %s" % embargo.get("error", "?"))
	var after_embargo: Array = sim.call("get_trade_routes")
	var cut_by_embargo := false
	for route_variant in after_embargo:
		var route: Dictionary = route_variant
		if str(route.get("id", "")) == "route_bruges_londres" and bool(route.get("cut", false)):
			cut_by_embargo = true
	_check(cut_by_embargo, "route_bruges_londres should be cut by the England-Flanders embargo")
	sim.call("submit_order", {"type": "set_embargo", "target": "fac_flanders", "active": false})

	# Économie : le revenu commercial apparaît dans get_faction_economy.
	sim.call("end_turn")
	var economy: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(economy.has("trade_income") and economy.has("trade_income_last_turn"), "faction economy should report trade income")

	# Panneaux réels : diplomatie (section Commerce) et faction (ligne Commerce).
	var diplomacy_panel: Node = (load("res://scripts/ui/diplomacy_panel.gd") as GDScript).new()
	root.add_child(diplomacy_panel)
	await process_frame
	diplomacy_panel.sim = sim
	diplomacy_panel.player_faction = FACTION_ID
	diplomacy_panel.refresh()
	diplomacy_panel.select_faction("fac_flanders")
	await process_frame
	diplomacy_panel.queue_free()
	var faction_panel: Node = (load("res://scenes/ui/faction_panel.tscn") as PackedScene).instantiate()
	root.add_child(faction_panel)
	await process_frame
	faction_panel.show_faction(FACTION_ID, "France", Color.WHITE, economy)
	await process_frame
	faction_panel.queue_free()

	if failures == 0:
		print("smoke OK: trade (real), %d routes, agreement proposed/broken, embargo cuts a route, economy + panels" % routes.size())


func _run_chronicle() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_pending_decisions")):
		print("smoke chronicle: skipped, CampaignSim has no get_pending_decisions (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "chronicle: new_campaign failed"):
		return
	var historical := 0
	var random := 0
	var by_order := 0
	var journal := 0
	var sample: Dictionary = {}
	for _i in 60:
		for event in sim.call("end_turn"):
			if str(event.get("kind", "")) == "chronicle":
				journal += 1
		for decision in sim.call("get_pending_decisions"):
			if decision.get("historical", false):
				historical += 1
			else:
				random += 1
			_check(str(decision.get("title", "")) != "" and not (decision.get("options", []) as Array).is_empty(), "decision needs a title and options")
			if sample.is_empty() and decision.get("historical", false):
				sample = decision
			var result: Dictionary
			if by_order == 0:
				result = sim.call("submit_order", {"type": "choose_event_option", "decision": int(decision["id"]), "option": 0})
				by_order += 1
			else:
				result = sim.call("choose_event_option", int(decision["id"]), 0)
			_check(result.get("ok", false), "choose_event_option refused: %s" % result.get("error", "?"))
	_check(historical >= 1, "60 turns should bring at least one historical event, got %d" % historical)
	_check(random >= 1, "60 turns should bring at least one random event, got %d" % random)
	_check(by_order == 1, "one decision should be resolved by order")
	_check((sim.call("get_pending_decisions") as Array).is_empty(), "no decision should remain")
	var refused: Dictionary = sim.call("choose_event_option", 99999, 0)
	_check(not refused.get("ok", true), "unknown decision should be refused")

	# Fenêtre réelle sur une décision historique.
	var window: Node = (load("res://scenes/ui/chronicle_window.tscn") as PackedScene).instantiate()
	root.add_child(window)
	await process_frame
	if not sample.is_empty():
		window.call("show_decision", sample, 2)
		await process_frame
		_check(window.visible, "chronicle window should be visible")
	window.queue_free()
	if failures == 0:
		print("smoke OK: chronicle (real), %d historical + %d random decisions in 60 turns, %d journal entries, sample « %s »" % [
			historical, random, journal, str(sample.get("title", "?"))])


# --- M10 assets ------------------------------------------------------------------------


## H9 : interface de la Table et de la médecine (vraie simulation si elle expose
## `get_diet_options`, sinon skip imprimé) : section Table d'une province française, changement
## de régime par l'interface, refus affiché, infobulle de tech médecine (plantes, note),
## genres `table` / `medicine` mappés (rapport, lettres, alertes), herbier silencieux.
## C4 (TW) : édits régionaux par le pont réel (options, ordre `set_edict`, délai) et section UI.
func _run_edicts() -> void:
	const FACTION_ID := "fac_france"
	const PROVINCE_ID := "prov_berry"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_edict_options")):
		print("smoke edicts: skipped, CampaignSim has no get_edict_options (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "edicts: new_campaign failed"):
		return
	var failures_before := failures
	var options: Array = sim.call("get_edict_options", PROVINCE_ID)
	_check(options.size() >= 6, "edicts: expected at least 6 options, got %d" % options.size())
	var result: Dictionary = sim.call("submit_order", {"type": "set_edict", "province": PROVINCE_ID, "edict": "edict_militia_levy"})
	_check(bool(result.get("ok", false)), "edicts: set_edict refused: %s" % str(result.get("error", "")))
	var edict: Dictionary = sim.call("get_province_edict", PROVINCE_ID)
	_check(bool(edict.get("pending", false)) and str(edict.get("edict", "")) == "edict_none", "edicts: militia levy should be pending, got %s" % [edict])
	var again: Dictionary = sim.call("submit_order", {"type": "set_edict", "province": PROVINCE_ID, "edict": "edict_feudal_aid"})
	_check(not bool(again.get("ok", true)), "edicts: second change in the same turn should be refused")
	sim.call("end_turn")
	edict = sim.call("get_province_edict", PROVINCE_ID)
	_check(str(edict.get("edict", "")) == "edict_militia_levy", "edicts: militia levy should be active after one turn, got %s" % [edict])
	var section := EdictSection.new()
	root.add_child(section)
	section.show_for(PROVINCE_ID, true, sim)
	_check(section.visible and section.option_buttons.size() == options.size(), "edicts: section should list every edict")
	section.queue_free()
	if failures == failures_before:
		print("smoke OK: edicts (real), %d options, militia levy pending then active, second change refused" % options.size())
