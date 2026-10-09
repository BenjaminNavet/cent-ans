extends TestCase

## Test headless du lot WH turn (ADR 0281) : sauvegarde auto (5 emplacements + avant bataille),
## modes du rapport de saison, suivi de missions et cloche, bilan de fin, sur la vraie simulation.
## Usage : godot --headless --path game --script res://tests/wh_turn_test.gd


class FakeSim:
	extends RefCounted
	var missions: Array = []

	func get_missions() -> Array:
		return missions


func _init() -> void:
	await process_frame
	await _run()
	finish()


## Chargées à l'exécution : leur compilation tire la chaîne `MapUI`, qui exige les autoloads.
func _run() -> void:
	var MissionTrackerScript: GDScript = load("res://scripts/ui/mission_tracker.gd")
	var AlertsScript: GDScript = load("res://scripts/ui/alerts.gd")
	var ClusterScript: GDScript = load("res://scripts/ui/end_turn_cluster.gd")
	var VictoryScript: GDScript = load("res://scripts/map/victory_controller.gd")
	# 1. Sauvegarde automatique.
	check(SaveSlots.AUTOSAVE_SLOTS == 5, "five rotating autosave slots")
	check(SaveSlots.autosave_name_for(1, 1) == "auto_1" and SaveSlots.autosave_name_for(5, 1) == "auto_5"
		and SaveSlots.autosave_name_for(6, 1) == "auto_1", "one autosave per season, rotating over 5 slots")
	check(SaveSlots.is_autosave(SaveSlots.BATTLE_SAVE) and SaveSlots.display_name(SaveSlots.BATTLE_SAVE) == "Avant la bataille",
		"the pre-battle save is an autosave with its own label")
	check(SaveSlots.autosave_battle(0) == "", "no pre-battle save when autosave is off")
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		check(int(settings.DEFAULTS["game/autosave_interval"]) == 1, "autosave defaults to every season")
		settings.call("set_value", "interface/season_report", true, false)
		check(str(settings.call("get_value", "interface/season_report")) == "auto", "old boolean season_report migrates to auto")
		settings.call("set_value", "interface/season_report", false, false)
		check(str(settings.call("get_value", "interface/season_report")) == "off", "false migrates to off")
		check(str(settings.DEFAULTS["interface/season_report"]) == "auto", "default mode is auto")

	# 2. Rapport de saison : modal seulement si important.
	var calm := [{"id": "world", "entries": [{"kind": "peace_signed", "_tone": "", "text_fr": "Paix."}]},
		{"id": "works", "entries": [{"kind": "building_completed", "_tone": "", "text_fr": "Fini."}]}]
	check(not SeasonReport.needs_modal(calm), "a calm season stays a toast")
	check(SeasonReport.quiet_line("Hiver 1338", calm).contains("2 nouvelles"), "quiet line counts the news")
	var loss := [{"id": "lands", "entries": [{"kind": "province_captured", "_tone": SeasonReport.TONE_LOSS, "text_fr": "Perdue."}]}]
	check(SeasonReport.needs_modal(loss), "a loss opens the report")
	var gain := [{"id": "lands", "entries": [{"kind": "province_captured", "_tone": SeasonReport.TONE_GAIN, "text_fr": "Prise."}]}]
	check(SeasonReport.needs_modal(gain), "a capture opens the report")
	var fight := [{"id": "armies", "entries": [{"kind": "battle", "_tone": "", "text_fr": "Bataille."}]}]
	check(SeasonReport.needs_modal(fight), "a battle opens the report")
	var foreign := [{"id": "world", "entries": [{"kind": "battle", "_tone": "", "text_fr": "Ailleurs."}]}]
	check(not SeasonReport.needs_modal(foreign), "a foreign battle does not")

	# 3. Suivi de missions et cloche.
	var missions := [
		{"id": 1, "title": "Prendre Calais", "progress": "tenue par England", "progress_ratio": 0.0, "turns_left": 6,
			"objective": "o", "reward": "800 livres", "province": "prov_boulonnais", "source": "Froissart"},
		{"id": 2, "title": "Une victoire", "progress": "0/1", "progress_ratio": 0.0, "turns_left": 1,
			"objective": "o", "reward": "400 livres", "province": ""}]
	var tracker: Control = MissionTrackerScript.new()
	root.add_child(tracker)
	var activated: Array = []
	tracker.mission_activated.connect(func(m: Dictionary) -> void: activated.append(m))
	tracker.set_missions(missions)
	check(tracker.visible, "the tracker shows with missions")
	check(tracker.find_child("Mission1", true, false) != null and tracker.find_child("Mission2", true, false) != null, "one row per mission")
	var head: Button = tracker.find_child("Head", true, false) as Button
	if check(head != null, "row head exists"):
		check(head.text.contains("6 saisons"), "turns left shown: %s" % head.text)
		head.pressed.emit()
		check(activated.size() == 1 and str(activated[0]["province"]) == "prov_boulonnais", "click emits the mission")
	tracker.set_collapsed(true)
	check(tracker.find_child("Mission1", true, false) == null, "collapsed: no rows")
	tracker.toggle()
	check(tracker.find_child("Mission1", true, false) != null, "expanded again")
	tracker.set_missions([])
	check(not tracker.visible, "hidden without mission")
	tracker.queue_free()
	var fake := FakeSim.new()
	fake.missions = missions
	var alerts: Array = AlertsScript.mission_alerts(fake)
	check(alerts.size() == 1 and alerts[0]["kind"] == "mission_due" and int(alerts[0]["mission_id"]) == 2, "only the due mission rings the bell")
	check("mission_due" in ClusterScript.KIND_ORDER and ClusterScript.SHORT_LABELS.has("mission_due"), "bell knows mission_due")

	# 4. Bilan de fin.
	var report := {"start_year": 1337, "year": 1360, "turns": 92, "provinces_start": 20, "provinces_peak": 30, "provinces_end": 25,
		"battles_won": 7, "battles_lost": 2, "objectives_done": 2, "objectives_total": 4, "prestige": 50,
		"score_provinces": 250, "score_objectives": 200, "score_prestige": 50, "score_treasury": 3, "score_other": 0, "score": 503}
	var lines: PackedStringArray = VictoryScript.report_lines(report)
	check(lines.size() == 5 and lines[2].contains("7 gagnée"), "report lines: %s" % str(lines))
	check(VictoryScript.score_breakdown(report).ends_with("= 503"), "score breakdown ends with the total")
	check(VictoryScript.report_lines({}).is_empty(), "no lines without a report")

	# 5. Vraie simulation : bilan, missions de faction, options de dilemme.
	if not ClassDB.class_exists("CampaignSim") or not ClassDB.instantiate("CampaignSim").has_method("get_campaign_report"):
		check(false, "CampaignSim.get_campaign_report missing (run core/build.sh)")
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
	sim.call("end_turn")
	var real: Dictionary = sim.call("get_campaign_report")
	for key in ["start_year", "year", "turns", "provinces_start", "provinces_end", "battles_won", "battles_lost", "score", "score_other"]:
		check(real.has(key), "report key %s" % key)
	check(int(real.get("provinces_start", 0)) > 0, "provinces at the start counted")
	map.victory.show_mission_notices()  # comme `after_end_turn` : avis puis suivi
	check(map.victory.tracker != null and map.victory.tracker.missions.size() == (sim.call("get_missions") as Array).size(),
		"the tracker follows get_missions")
	for mission: Dictionary in sim.call("get_missions"):
		check(mission.has("faction_mission") and mission.has("source"), "mission carries faction_mission and source")
	map.queue_free()
