extends SceneTree

## WR captives (ADR 0303) : le bouton « Exécuter » du panneau des captifs. Confirmation en deux
## temps, ordre `execute_captive` envoyé au cœur, infobulle chiffrée par les valeurs de `core/`.

class FakeSim:
	extends RefCounted
	var orders: Array = []

	func get_ransoms() -> Dictionary:
		return {"ours": [], "debts": [], "held": [{
			"character": "chr_jean_de_normandie", "name": "Jean de Normandie", "faction": "fac_france",
			"captor": "fac_england", "rank": "heir", "rank_label": "héritier", "prestige": 20, "ransom": 5000,
			"terms": {"kind": "money", "province": ""}, "plans": [], "cedable_provinces": [],
			"execution": {"prestige": 10, "victim_opinion": -60, "house_opinion": -25, "others_opinion": -8,
				"ransom_lost": 5000, "ruler_trait": "trait_cruel"}}]}

	func submit_order(order: Dictionary) -> Dictionary:
		orders.append(order)
		return {"ok": true}


var _failures := 0


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("FAIL: ", what)


func _initialize() -> void:
	var panel := RansomPanel.new()
	root.add_child(panel)
	var sim := FakeSim.new()
	panel.refresh(sim)
	var row: Dictionary = panel.rows.get("chr_jean_de_normandie", {})
	var button: Button = row.get("execute")
	_check(button != null, "execute button exists")
	if button == null:
		quit(1)
		return
	_check(button.text == RansomPanel.EXECUTE_LABEL, "initial label")
	var tip := RansomPanel.execution_tooltip(sim.get_ransoms()["held"][0]["execution"], "Jean")
	for needle in ["+10", "-60", "-25", "-8", Money.digits(5000)]:
		_check(tip.contains(needle), "tooltip shows " + needle)
	button.pressed.emit()
	_check(sim.orders.is_empty(), "first click only arms")
	_check(button.text == RansomPanel.EXECUTE_CONFIRM_LABEL, "armed label")
	button.pressed.emit()
	_check(sim.orders.size() == 1, "second click sends one order")
	if sim.orders.size() == 1:
		_check(sim.orders[0] == {"type": "execute_captive", "character": "chr_jean_de_normandie"}, "order payload")
	# Réouverture : désarmé.
	panel.show_data(sim.get_ransoms())
	var again: Button = panel.rows["chr_jean_de_normandie"]["execute"]
	again.pressed.emit()
	panel.show_data(sim.get_ransoms())
	_check(panel.armed_execution == "", "reopen disarms")
	print("wr_captives_ui_test: ", "OK" if _failures == 0 else "%d échec(s)" % _failures)
	quit(1 if _failures > 0 else 0)
