extends TestCase

## Test headless des composants du HUD de campagne (lot F10a) : instanciation des quatre scènes,
## appels d'API avec les données de démonstration de `hud_preview.gd`, signaux émis.
## Usage : godot --headless --path game --script res://tests/hud_components_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

const PREVIEW := preload("res://tests/hud_preview.gd")

var _received: Array = []


func _init() -> void:
	await process_frame
	await _test_army_strip()
	await _test_general_seal()
	await _test_end_turn_cluster()
	await _test_news_letters()
	finish()


		print("FAIL: " + message)


func _instance(path: String) -> Control:
	var node: Control = (load(path) as PackedScene).instantiate()
	root.add_child(node)
	return node


func _record(value: Variant = null) -> void:
	_received.append(value)


func _test_army_strip() -> void:
	var strip: ArmyStrip = _instance("res://scenes/ui/army_strip.tscn")
	var catalog := ArmyStrip.load_unit_catalog(ProjectSettings.globalize_path("res://../data"))
	check(catalog.has("unit_knights"), "unit catalog should contain unit_knights")
	var army: Dictionary = PREVIEW.demo_army()
	strip.set_army(army, 20, catalog, "Ost de Philippe VI")
	await process_frame
	var cards := strip.find_children("*", "", true, false).filter(func(n: Node) -> bool: return n is ArmyStrip.RegimentCard)
	check(cards.size() == 8, "army strip should show 8 cards, got %d" % cards.size())
	var expected_upkeep := 0
	for unit in army["units"]:
		expected_upkeep += int(catalog.get(unit["unit_type"], {}).get("upkeep", 0))
	check(strip.total_upkeep() == expected_upkeep and expected_upkeep > 0, "upkeep %d != %d" % [strip.total_upkeep(), expected_upkeep])
	check(strip.unit_class(army["units"][0]) == "cavalry", "knights should be cavalry")
	check(strip.tooltip_for(army["units"][0]).contains("Effectif : 60 / 60"), "tooltip should show strength")

	_received.clear()
	strip.unit_selected.connect(_record)
	strip.split_requested.connect(_record)
	strip._on_card_clicked(2, false)
	strip._on_card_clicked(5, true)
	check(strip.get_selection() == PackedInt32Array([2, 5]), "selection should be [2, 5], got %s" % strip.get_selection())
	strip._on_split_pressed()
	check(_received == [2, 5, PackedInt32Array([2, 5])], "strip signals: %s" % [_received])
	strip._on_card_clicked(5, true)
	check(strip.get_selection() == PackedInt32Array([2]), "ctrl-click should toggle off")

	# Disposition : 20 régiments tiennent sur deux rangs au plus dans 880 px.
	var layout := strip.card_layout(20)
	check(int(layout["rows"]) <= 2, "20 units should fit in 2 rows, got %s" % layout)
	check(int(strip.card_layout(8)["rows"]) == 1, "8 units should fit in 1 row")
	check(int(strip.card_layout(40)["rows"]) <= 4, "40 units should fit in 4 rows, got %s" % strip.card_layout(40))

	# Noms : jamais coupés au milieu d'un mot (mot entier ou abréviation terminée par un point).
	var font := strip.get_theme_default_font()
	for name: String in ["Hommes d'armes à pied", "Arbalétriers génois", "Archers à l'arc long", "Arbalétriers"]:
		for width in [40.0, 54.0, 74.0]:
			var fitted := ArmyStrip.fit_name(name, font, width)
			var original: PackedStringArray = str(name).split(" ", false)
			var words := str(fitted["text"]).split(" ", false)
			check(words.size() == original.size(), "fit_name changed word count: %s" % fitted)
			for i in words.size():
				var ok: bool = words[i] == original[i] or (words[i].ends_with(".") and original[i].begins_with(words[i].trim_suffix(".")))
				check(ok, "fit_name cut a word: %s -> %s" % [original[i], words[i]])
	strip.clear()
	check(not strip.visible, "cleared strip should hide")
	strip.queue_free()


func _test_general_seal() -> void:
	var seal: GeneralSeal = _instance("res://scenes/ui/general_seal.tscn")
	seal.set_general(PREVIEW.demo_character(), PREVIEW.demo_army())
	await process_frame
	check(seal.faction_id == "fac_france", "seal faction should come from the character")
	check(seal.rank() == 3, "rank should be 1 + learned skills")
	check(seal.unspent_points() == 2, "unspent points should be 2")
	_received.clear()
	seal.general_requested.connect(_record)
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	seal._gui_input(click)
	check(_received == ["chr_philippe_vi"], "general_requested: %s" % [_received])
	seal.set_general({}, {}, "fac_england")
	check(seal.tooltip_text == "Armée sans chef", "empty general tooltip")
	seal.queue_free()


func _test_end_turn_cluster() -> void:
	var cluster: EndTurnCluster = _instance("res://scenes/ui/end_turn_cluster.tscn")
	cluster.set_date("Printemps 1337", 0)
	var alerts: Array = PREVIEW.demo_alerts()
	cluster.set_alerts(alerts)
	await process_frame
	var groups := cluster.get_alert_groups()
	check(groups.size() == 7, "7 alert kinds expected, got %d" % groups.size())
	check(str(groups[0]["kind"]) == "chronicle_decision", "decision first")
	check(not cluster.blocking_alert().is_empty(), "decision should block")
	_received.clear()
	cluster.alert_activated.connect(_record)
	cluster.end_turn_requested.connect(_record.bind("end_turn"))
	cluster._on_button_pressed()
	check(_received.size() == 1 and (_received[0] as Dictionary).get("kind") == "chronicle_decision", "blocked bell should activate the decision: %s" % [_received])
	# Deux armées ennemies : clics successifs = alertes successives, puis retour à la première.
	var enemy_index := -1
	for i in groups.size():
		if groups[i]["kind"] == "enemy_army":
			enemy_index = i
	_received.clear()
	cluster.activate_group(enemy_index)
	cluster.activate_group(enemy_index)
	cluster.activate_group(enemy_index)
	check(_received.size() == 3 and _received[0]["army_id"] == "army_12" and _received[1]["army_id"] == "army_17" and _received[2]["army_id"] == "army_12", "enemy alerts should cycle: %s" % [_received])
	cluster.set_alerts(alerts.filter(func(a: Dictionary) -> bool: return not EndTurnCluster.is_blocking(a)))
	_received.clear()
	cluster._on_button_pressed()
	check(_received == ["end_turn"], "free bell should request end of turn: %s" % [_received])
	cluster.set_end_turn_enabled(false)
	_received.clear()
	cluster._on_button_pressed()
	check(_received.is_empty(), "disabled bell should do nothing")
	cluster.queue_free()


func _test_news_letters() -> void:
	var letters: NewsLetters = _instance("res://scenes/ui/news_letters.tscn")
	for item in PREVIEW.demo_news():
		letters.push_news(item)
	await process_frame
	check(letters.get_items().size() == 6, "6 letters kept")
	check(str(letters.get_items()[0]["title"]).begins_with("Édouard III"), "newest letter first")
	var shown := letters.find_children("*", "", true, false).filter(func(n: Node) -> bool: return n is NewsLetters.Letter)
	check(shown.size() == 5, "5 letters visible by default, got %d" % shown.size())
	_received.clear()
	letters.news_activated.connect(_record)
	letters.news_dismissed.connect(_record)
	letters.activate(0)
	letters.dismiss(1)
	check(_received.size() == 2 and str(_received[1]["title"]) == "Philippe VI confisque la Guyenne", "letters signals: %s" % [_received])
	check(letters.get_items().size() == 5, "dismiss should remove a letter")
	letters.clear()
	check(letters.get_items().is_empty(), "clear should empty the pile")
	var war := NewsLetters.news_from_event({"kind": "war_declared", "text_fr": "L'Angleterre déclare la guerre à la France.", "faction": "fac_england", "province": "", "army": ""})
	check(war.get("kind") == "war_declared" and war.get("faction_id") == "fac_england", "news_from_event: %s" % war)
	check(NewsLetters.news_from_event({"kind": "income", "text_fr": "Revenus"}).is_empty(), "income is not news")
	letters.queue_free()
	await process_frame
