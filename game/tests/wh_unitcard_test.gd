extends TestCase

## WH uicards top1 / top10 : chevrons d'expérience et infobulle du bandeau d'ost, clic droit =
## fiche détaillée. Usage : godot --headless --path game --script res://tests/wh_unitcard_test.gd

var _details: Array = []


func _init() -> void:
	await process_frame
	var strip: Control = (load("res://scenes/ui/army_strip.tscn") as PackedScene).instantiate()
	root.add_child(strip)
	var catalog: Dictionary = strip.get_script().load_unit_catalog(ProjectSettings.globalize_path("res://../data"))
	var units: Array = []
	for experience in [0, 4, 9]:
		units.append({"unit_type": "unit_knights", "name": "Chevaliers", "strength": 60, "max_strength": 60,
			"morale": 70, "experience": experience})
	strip.set_army({"units": units}, 20, catalog, "Ost")
	await process_frame
	var cards := strip.find_children("*", "", true, false).filter(func(n: Node) -> bool: return n.has_method("chevron_count"))
	check(cards.size() == 3, "3 cards, got %d" % cards.size())
	check(cards[0].chevron_count() == 0 and cards[1].chevron_count() == 4 and cards[2].chevron_count() == 9,
		"chevrons 0/4/9: %d %d %d" % [cards[0].chevron_count(), cards[1].chevron_count(), cards[2].chevron_count()])
	var tip: String = strip.tooltip_for(units[1])
	check(tip.contains("Expérience : 4/10 (Éprouvé)"), "tooltip experience: %s" % tip)
	check(strip.tooltip_for(units[0]).contains("Expérience : 0/10 (Levée)"), "tooltip level 0")
	strip.unit_details_requested.connect(func(index: int) -> void: _details.append(index))
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_RIGHT
	click.pressed = true
	cards[2]._gui_input(click)
	check(_details == [2], "right click emits unit_details_requested(2): %s" % [_details])
	check(strip.card_at(1) == cards[1] and strip.card_at(7) == null, "card_at")
	# La fiche contient attaque, défense et vitesse.
	var spec: Dictionary = RichTooltip.unit_spec("unit_knights", {"strength": 60, "max_strength": 60, "morale": 70,
		"effects": [{"key": "experience", "text": UnitRank.summary(units[1]), "sign": 0}]})
	var keys := []
	for stat in spec["stats"]:
		keys.append(stat["key"])
	check(keys.has("melee") and keys.has("armor") and keys.has("speed"), "unit sheet stats: %s" % [keys])
	finish()
