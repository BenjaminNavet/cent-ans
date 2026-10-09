extends TestCase

## Test headless des lots UI3 (audit A3) : rubriques du rapport de saison (U5), filtre d'intérêt
## des nouvelles (U5), pastilles d'alerte libellées (U5), fiche des raccourcis (U7), motifs des
## actions grisées (U10), pluriels et dates (U8), symboles d'accessibilité (U12).
## Usage : godot --headless --path game --script res://tests/ui3_test.gd


func _init() -> void:
	await process_frame
	_test_season_report()
	_test_news_interest()
	await _test_alert_pills()
	_test_shortcuts()
	_test_blockers()
	_test_french()
	finish()


func _test_season_report() -> void:
	var player := "fac_france"
	var events := [
		{"kind": "birth", "text_fr": "Naissance d'un prince.", "faction": player},
		{"kind": "province_captured", "text_fr": "Wissant tombe aux mains d'Angleterre (auparavant France).", "faction": "fac_england", "province": "prov_calais"},
		{"kind": "province_captured", "text_fr": "Bordeaux tombe aux mains de France.", "faction": player, "province": "prov_guyenne"},
		{"kind": "building_completed", "text_fr": "Marché achevé à Paris.", "faction": player, "province": "prov_ile_de_france"},
		{"kind": "battle", "text_fr": "Bataille de Sluys.", "faction": player, "army": "army_1"},
		{"kind": "income", "text_fr": "Revenus : 100 livres.", "faction": player},
		{"kind": "alliance_formed", "text_fr": "Alliance Vérone–Autriche.", "faction": "fac_verona"},
		{"kind": "war_declared", "text_fr": "L'Angleterre déclare la guerre.", "faction": "fac_england"},
	]
	var relevant := func(event: Dictionary) -> bool:
		return str(event.get("faction", "")) == player or str(event.get("text_fr", "")).contains("auparavant France")
	var keeps_world := func(event: Dictionary) -> bool: return str(event.get("faction", "")) == "fac_england"
	var groups := SeasonReport.build_groups(events, relevant, keeps_world, player)
	var ids: Array = groups.map(func(group: Dictionary) -> String: return str(group["id"]))
	check(ids == ["lands", "armies", "works", "world"], "sections in order, got %s" % [ids])
	var lands: Array = groups[0]["entries"]
	check(str(lands[0]["_tone"]) == SeasonReport.TONE_LOSS and str(lands[0]["text_fr"]).begins_with("Wissant"), "the loss comes first: %s" % [lands[0]])
	check(str(lands[1]["_tone"]) == SeasonReport.TONE_GAIN, "then the gain")
	var world: Array = groups[3]["entries"]
	check(world.size() == 1 and str(world[0]["kind"]) == "war_declared", "world keeps only interesting news: %s" % [world])
	var with_treasury := SeasonReport.with_summary(groups, "treasury", [{"kind": "summary", "text_fr": "Le trésor gagne 10 ₶."}])
	check(str(with_treasury[1]["id"]) == "treasury", "treasury summary inserted in second position")
	check(not SeasonReport.has_news(SeasonReport.with_summary([], "treasury", [{"kind": "summary", "text_fr": "x"}])), "a summary alone is no news")
	check(SeasonReport.action_label({"kind": "technology_researched"}) == "Technologies", "tech action")


func _test_news_interest() -> void:
	var interest := NewsInterest.new()
	interest.player = "fac_france"
	interest.factions = {"fac_france": NewsInterest.Interest.PLAYER, "fac_england": NewsInterest.Interest.CLOSE, "fac_castile": NewsInterest.Interest.GREAT_POWER}
	interest.owners = {"prov_calais": "fac_france", "prov_lucca": "fac_florence"}
	check(interest.keeps({"kind": "province_captured", "faction": "fac_england", "province": "prov_calais"}), "news about the player's land")
	check(not interest.keeps({"kind": "province_captured", "faction": "fac_florence", "province": "prov_lucca"}), "distant capture filtered")
	check(interest.keeps({"kind": "war_declared", "faction": "fac_castile"}), "great power war kept")
	check(not interest.keeps({"kind": "battle", "faction": "fac_castile"}), "great power battle far away filtered")
	interest.mode = NewsInterest.MODE_ALL
	check(interest.keeps({"kind": "province_captured", "faction": "fac_florence", "province": "prov_lucca"}), "all mode keeps everything")


func _test_alert_pills() -> void:
	var cluster: EndTurnCluster = (load("res://scenes/ui/end_turn_cluster.tscn") as PackedScene).instantiate()
	root.add_child(cluster)
	cluster.set_alerts([{"kind": "siege", "text": "Paris assiégée"}, {"kind": "siege", "text": "Rouen assiégée"}, {"kind": "debt", "text": "Dette"}])
	await process_frame
	var groups := cluster.get_alert_groups()
	check(groups.size() == 2, "two alert kinds")
	var pills := cluster.find_children("*", "", true, false).filter(func(node: Node) -> bool: return node is EndTurnCluster.AlertBadge)
	check(pills.size() == 2 and (pills[0] as EndTurnCluster.AlertBadge).label == "Siège" and (pills[0] as EndTurnCluster.AlertBadge).count == 2, "labelled pill with counter")
	check(cluster.custom_minimum_size.y > cluster.bell_height(), "pills stacked above the bell")
	cluster.queue_free()


func _test_shortcuts() -> void:
	check(ShortcutSheet.action_keys("map_toggle_court") == "C", "court key from the InputMap: %s" % ShortcutSheet.action_keys("map_toggle_court"))
	check(ShortcutSheet.action_keys("map_toggle_agents") == "G", "agents key")
	check(ShortcutSheet.action_keys("help_open") == "F1", "help key")
	var sections := ShortcutSheet.sections()
	check(sections.size() >= 4 and ShortcutSheet.bbcode().contains("Finir la saison"), "shortcut sheet generated")
	check(ShortcutSheet.physical_label(KEY_ENTER) == "Entrée", "French key names")


func _test_blockers() -> void:
	# Chargée à l'exécution : la fiche dépend des autoloads (IconLibrary), enregistrés après la compilation.
	var sheet: GDScript = load("res://scripts/ui/character_sheet.gd")
	if not check(sheet != null and sheet.can_instantiate(), "character sheet script"):
		return
	var dead := {"alive": false, "sex": "female"}
	check(str(sheet.call("action_blocker", "marry", dead, [1])) == "défunte", "dead woman")
	check(str(sheet.call("action_blocker", "governor", {"alive": true, "army": "army_1"}, [1])) == "commande déjà une armée", "general cannot govern")
	check(str(sheet.call("action_blocker", "general", {"alive": true}, [])) == "aucune armée sans chef", "no army")
	check(str(sheet.call("action_blocker", "marry", {"alive": true}, [1])) == "", "free to marry")


func _test_french() -> void:
	check(FrText.count(1, "tour") == "1 tour" and FrText.count(3, "tour") == "3 tours" and FrText.count(0, "tour") == "0 tour", "plurals")
	check(FrText.from_iso("2026-09-24T23:19:05") == "24 sept. 2026, 23 h 19", "French date: %s" % FrText.from_iso("2026-09-24T23:19:05"))
	check(Accessibility.relation_symbol("war") == "⚔" and Accessibility.level_symbol(0.1) == "▼", "accessibility symbols")
