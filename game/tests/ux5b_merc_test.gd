extends TestCase

## Mercenaires en cartes (UX5b) : une carte par compagnie, les disponibles d'abord, raison sur les
## refus, clic gauche = `hire_requested`, clic sur une carte grisée = rien.
## Usage : godot --headless --path game --script res://tests/ux5b_merc_test.gd

const INFO := {
	"region_name": "Guyenne", "blocked": "", "hires_left": 1, "premium_last_turn": 0,
	"options": [
		{"unit_type": "u_a", "name": "Arbalétriers", "band_name": "Génois", "cost": 30, "upkeep": 3, "available": false, "reason": "trésor insuffisant", "pool_available": 2, "pool_cap": 3, "pool_label": "2 disponibles, +1 dans 2 saisons", "soldiers": 60, "recruit_time_turns": 1},
		{"unit_type": "u_b", "name": "Routiers", "band_name": "", "cost": 12, "upkeep": 2, "available": true, "reason": "", "pool_available": 3, "pool_cap": 3, "pool_label": "3 disponibles", "soldiers": 80, "recruit_time_turns": 1},
	],
}


func _init() -> void:
	await process_frame
	var panel: Control = (load("res://scripts/ui/mercenary_panel.gd") as GDScript).new()
	root.add_child(panel)
	var hired: Array = []
	panel.hire_requested.connect(func(army: String, unit: String) -> void: hired.append([army, unit]))
	panel.show_market("army_1", INFO)
	var list: Container = panel.find_child("List", true, false)
	check(list is HFlowContainer and list.get_child_count() == 2, "two cards in a grid")
	check(str(list.get_child(0).name) == "Card_u_b", "available company first")
	var blocked: Control = panel.find_child("Card_u_a", true, false)
	check(blocked.find_child("ReasonLabel", true, false).text.contains("trésor insuffisant"), "refusal reason on the card")
	check(blocked.find_child("NameLabel", true, false).text.contains("Génois"), "band name in the title")
	var ok: Control = panel.find_child("Card_u_b", true, false)
	check(ok.find_child("PriceLabel", true, false).text == RecruitBasket.price_text(INFO["options"][1]), "hire price and upkeep")
	check(ok.find_child("PoolLabel", true, false).text == "3 disponibles", "core reserve text")
	_click(blocked)
	check(hired.is_empty(), "greyed card does nothing")
	_click(ok)
	check(hired == [["army_1", "u_b"]], "click emits hire_requested: %s" % str(hired))
	finish()


func _click(card: Control) -> void:
	var click := InputEventMouseButton.new()
	click.button_index = MOUSE_BUTTON_LEFT
	click.pressed = true
	card.gui_input.emit(click)
