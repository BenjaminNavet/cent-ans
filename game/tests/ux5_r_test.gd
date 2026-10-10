extends TestCase

## Lot UX5-R : recrutement par cartes et panier local (sans capture).
##  1. panier : clic +1, Maj+clic +5, clic droit −1, plafonné par places libres et réserve ;
##  2. pied « Montre » : total, trésor après, bouton « Sceller la levée » (désactivé si vide / trop cher) ;
##  3. cartes : légende « N hommes · K tours », gouttes de réserve, onglets de catégorie, tri par prix ;
##  4. le panneau de colonie émet un ordre par recrue scellée.
## Usage : godot --headless --path game --script res://tests/ux5_r_test.gd

const ROWS := [
	{"unit_type": "u_pike", "name": "Piquiers", "cost": 10, "upkeep": 1, "available": true, "group": "ready", "category": "infantry", "soldiers": 80, "recruit_time_turns": 2, "pool_available": 3, "pool_cap": 4, "pool_seasons_to_next": 2},
	{"unit_type": "u_bow", "name": "Archers", "cost": 6, "upkeep": 1, "available": true, "group": "ready", "category": "ranged", "soldiers": 60, "recruit_time_turns": 1, "pool_available": 10, "pool_cap": 10, "pool_seasons_to_next": -1},
	{"unit_type": "u_knight", "name": "Chevaliers", "cost": 40, "upkeep": 4, "available": false, "group": "blocked", "category": "cavalry", "soldiers": 30, "recruit_time_turns": 3, "reason": "trésor insuffisant"},
]


func _init() -> void:
	await process_frame
	_run()
	finish()


func _run() -> void:
	var basket := RecruitBasket.new()
	root.add_child(basket)
	basket.set_rows(ROWS, 6, 100)
	# 1. Panier et plafonds.
	check(basket.add("u_pike") == 1 and basket.counts["u_pike"] == 1, "click adds one")
	check(basket.add("u_pike", 5) == 2, "pool of 3 caps shift-click (1 already): %d" % basket.counts["u_pike"])
	check(basket.room_for("u_pike") == 0, "pool exhausted")
	check(basket.add("u_bow", 5) == 3, "slots cap: 6 free, 3 taken -> 3 left")
	check(basket.total_count() == 6 and basket.add("u_bow") == 0, "slots full")
	check(basket.add("u_knight") == 0, "unavailable unit cannot be added")
	check(basket.remove("u_bow") == 1 and basket.counts["u_bow"] == 2, "right click removes one")
	# 2. Total, trésor après, sceau.
	check(basket.total_cost() == 3 * 10 + 2 * 6, "total cost %d" % basket.total_cost())
	check(basket.total_upkeep() == 5, "total upkeep")
	check(basket.footer_label.text.contains("trésor après"), "footer: %s" % basket.footer_label.text)
	check(not basket.seal_button.disabled, "seal enabled")
	basket.treasury = 10
	basket._refresh_view()
	check(basket.seal_button.disabled, "seal disabled when treasury short")
	basket.treasury = 100
	basket._refresh_view()
	var sealed_orders: Array = []
	basket.sealed.connect(func(order: Array) -> void: sealed_orders.append_array(order))
	basket.seal_button.pressed.emit()
	check(sealed_orders.size() == 5 and sealed_orders.count("u_pike") == 3, "one order per recruit: %s" % str(sealed_orders))
	check(basket.total_count() == 0 and basket.seal_button.disabled, "basket emptied, seal disabled")
	# 3. Cartes, onglets, tri.
	check(basket.find_child("Card_u_pike", true, false) != null, "card per unit")
	check(RecruitBasket.legend_text(ROWS[0]) == "80 hommes · 2 tours", RecruitBasket.legend_text(ROWS[0]))
	check(RecruitBasket.pool_drops(ROWS[0]) == "●●●○", "wax drops")
	check(RecruitBasket.pool_note(ROWS[0]).contains("+1 dans 2 saisons"), RecruitBasket.pool_note(ROWS[0]))
	check(basket.find_child("Tab_ranged", true, false) != null and basket.find_child("Tab_all", true, false) != null, "category tabs")
	basket.set_category("ranged")
	check(basket.grid.get_child_count() == 1 and basket.find_child("Card_u_bow", true, false) != null, "filter by category")
	basket.set_category("")
	basket.cycle_sort()
	check(str(basket.grid.get_child(0).name) == "Card_u_bow", "price ascending puts Archers first")
	basket.cycle_sort()
	check(str(basket.grid.get_child(0).name) == "Card_u_knight", "price descending puts Chevaliers first")
	# Le panier survit au rafraîchissement, borné aux nouvelles limites.
	basket.add("u_bow", 4)
	basket.set_rows(ROWS, 2, 100)
	check(basket.total_count() == 2, "clamped to new free slots")
	# 4. Panneau de colonie.
	var panel := SettlementPanel.new()
	root.add_child(panel)
	var emitted: Array = []
	panel.recruit_requested.connect(func(_s: String, unit_type: String) -> void: emitted.append(unit_type))
	panel.treasury = 100
	var detail := {"id": "set_x", "province": "prov_x", "name": "Paris", "kind": "city", "owner": "fac_france", "recruit_slots_free": 3, "recruit_slots": 3}
	panel.show_settlement(detail, ROWS, [], true)
	panel.recruit_basket.add("u_bow", 2)
	panel.recruit_basket.add("u_pike")
	panel.recruit_basket.seal_button.pressed.emit()
	check(emitted.size() == 3 and emitted.count("u_bow") == 2, "panel emits one order per recruit: %s" % str(emitted))
