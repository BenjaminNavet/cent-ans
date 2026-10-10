extends TestCase

## Test headless du lot WR turn (ADR 0304) : offre de mission en choix (candidates, accepter, refuser,
## fenêtre) sur la vraie simulation, et filtre par genre du journal.
## Usage : godot --headless --path game --script res://tests/wr_turn_ui_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	# 1. Genres du journal (données) : regroupement des genres d'événements.
	check(JournalView.genre_of("battle") == "war" and JournalView.genre_of("peace_signed") == "diplomacy"
		and JournalView.genre_of("coinage") == "economy" and JournalView.genre_of("death") == "people"
		and JournalView.genre_of("heresy") == "faith" and JournalView.genre_of("mission") == "missions",
		"event kinds map to journal genres")
	check(JournalView.genre_of("no_such_kind") == "other", "unknown kinds fall under « other »")
	var choices := JournalView.genre_choices()
	check(str(choices[0].get("id")) == "all" and choices.size() >= 6, "choices start with « Tout »")
	# Filtre pur : une date n'est gardée que si une ligne du genre la suit.
	var lines := PackedStringArray(["[b]— B —[/b]", "guerre B", "paix B", "[b]— A —[/b]", "paix A"])
	var genres := PackedStringArray(["", "war", "diplomacy", "", "diplomacy"])
	check(JournalView.filter_lines(lines, genres, "all").size() == 5, "« all » keeps every line")
	var war := JournalView.filter_lines(lines, genres, "war")
	check(war.size() == 2 and war[0].contains("B") and war[1] == "guerre B", "war keeps one dated line")
	check(JournalView.filter_lines(lines, genres, "faith").is_empty(), "an empty genre shows nothing")

	# 2. Offre de mission en choix.
	check(MissionOfferController.REFUSE == -1, "refusal index")
	var offer_stub := {"id": 3, "turns_left": 2, "candidates": [
		{"index": 0, "title": "A", "objective": "a", "duration": 6, "reward": "100 livres", "source": ""},
		{"index": 1, "title": "B", "objective": "b", "duration": 8, "reward": "+10 prestige", "source": "Histoire"}]}
	var decision := MissionOfferController.decision_for(offer_stub)
	var options: Array = decision["options"]
	check(options.size() == 3 and str(options[2]["text"]).begins_with("Refuser"), "two candidates plus a refusal button")
	check(str(options[1]["effects_text"]).contains("8 tours") and str(options[1]["effects_text"]).contains("prestige"),
		"each choice shows its delay and reward")
	check(int(decision["expires_in"]) == 2, "the window shows the remaining turns")

	if not ClassDB.class_exists("CampaignSim") or not ClassDB.instantiate("CampaignSim").has_method("get_mission_offer"):
		check(false, "CampaignSim.get_mission_offer missing (run core/build.sh)")
		return
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
	check((sim.call("get_mission_offer") as Dictionary).is_empty(), "no offer before the first end of turn")
	sim.call("end_turn")
	var offer: Dictionary = sim.call("get_mission_offer")
	check(not offer.is_empty(), "an offer after the first turn")
	var candidates: Array = offer.get("candidates", [])
	check(candidates.size() >= 2 and candidates.size() <= 3, "2 or 3 candidates: %d" % candidates.size())
	for candidate: Dictionary in candidates:
		check(str(candidate["title"]) != "" and str(candidate["reward"]) != "" and int(candidate["duration"]) >= 3,
			"candidate has title, reward and delay")
	check((sim.call("get_missions") as Array).is_empty(), "no mission is imposed")
	# Fenêtre : le contrôleur affiche l'offre et traduit le bouton choisi.
	check(map.mission_offer != null and map.mission_offer.available(), "offer controller is wired")
	map.mission_offer.open_window()
	await process_frame
	check(map.mission_offer.window.visible, "the offer window opens")
	check(map.mission_offer.choice_for(candidates.size()) == MissionOfferController.REFUSE
		and map.mission_offer.choice_for(1) == 1, "buttons map to candidates, the last to a refusal")
	map.mission_offer._on_option_chosen(int(offer["id"]), 1)  # le joueur accepte la deuxième
	await process_frame
	var missions: Array = sim.call("get_missions")
	check(missions.size() == 1 and str(missions[0]["title"]) == str(candidates[1]["title"]), "the chosen mission is active")
	check((sim.call("get_mission_offer") as Dictionary).is_empty() and not map.mission_offer.window.visible, "the offer is closed")
	# Refus : une nouvelle offre vient après le délai de carence, on la refuse.
	var refused := false
	for _i in 6:
		sim.call("end_turn")
		if not (sim.call("get_mission_offer") as Dictionary).is_empty():
			var before := (sim.call("get_missions") as Array).size()
			var result: Dictionary = sim.call("choose_mission", -1)
			refused = bool(result.get("ok", false)) and (sim.call("get_mission_offer") as Dictionary).is_empty() \
				and (sim.call("get_missions") as Array).size() == before
			break
	check(refused, "a later offer can be refused")

	# 3. Filtre du journal sur la vraie carte, choix retenu pour la session.
	var journal: JournalView = map.ui.journal
	journal.clear()
	journal.add_events([
		{"kind": "battle", "text_fr": "Bataille de Test."},
		{"kind": "peace_signed", "text_fr": "Paix de Test."},
		{"kind": "building_completed", "text_fr": "Halle de Test."}], "Printemps 1337")
	check(journal.line_genres.size() == journal.lines.size(), "one genre per journal line")
	check(journal.line_genres.has("war") and journal.line_genres.has("diplomacy") and journal.line_genres.has("economy"),
		"lines carry their genre")
	journal.set_expanded(true)
	check(journal.genre_bar != null and journal.genre_bar.visible, "the genre bar shows when the journal is open")
	journal.set_genre_filter("diplomacy")
	var shown := JournalView.filter_lines(journal.lines, journal.line_genres, journal.genre_filter)
	check(shown.size() == 2 and shown[1].contains("Paix"), "only diplomacy is shown, under its date")
	check(journal.lines.size() == 4, "the filter hides, it does not delete")
	check(JournalView.remembered_genre == "diplomacy", "the choice is remembered for the session")
	journal.set_genre_filter("all")
	map.queue_free()
