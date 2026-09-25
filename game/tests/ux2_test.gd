extends SceneTree

## Test headless du lot UX2 « Premiers pas » : choix du conseil « que faire maintenant »
## selon l'état (`NextHint`), placement du parchemin du tutoriel hors de sa cible
## (`TutorialOverlay.place_panel`, rectangles synthétiques), « Plus tard » puis reprise à la
## même étape et sommaire (`TutorialController` sur une carte factice).
## Usage : godot --headless --path game --script res://tests/ux2_test.gd

var _failures := 0


class FakeSim:
	extends RefCounted

	func get_turn() -> int:
		return 0

	func get_faction_summary(_faction: String) -> Dictionary:
		return {"treasury": 1000, "income": 0, "tax_rate": "normal"}


class FakeMap:
	extends Node

	var sim: Object = FakeSim.new()
	var player_faction := "fac_france"
	var camera_rig: Object = null
	var map_data: Object = null
	var ui: Object = null

	func player_army_ids() -> PackedStringArray:
		return PackedStringArray()


func _init() -> void:
	await process_frame
	_test_hint_choice()
	_test_army_idle()
	_test_panel_placement()
	await _test_postpone_resume()
	if _failures == 0:
		print("ux2 OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("ux2: " + message)
	return condition


func _test_hint_choice() -> void:
	var config := NextHint.data()
	_check(not (config.get("hints", []) as Array).is_empty(), "data/ui/next_hints.json should load")
	var research := {"kind": "research_idle", "text": "Aucune recherche en cours"}
	var base := {"alerts": [research], "idle_army": "army_1", "idle_army_name": "l'ost de Philippe VI",
		"treasury": 60000, "income": 0, "constructions": 0, "can_build": true, "tutorial_step": -1, "dismissed": []}
	var hint := NextHint.choose(base)
	_check(str(hint.get("id", "")) == "research_idle" and str(hint.get("action", "")) == "alert", "idle research first, got %s" % [hint])
	_check(str(hint.get("title", "")).contains("(T)"), "the research hint names its key: %s" % [hint.get("title", "")])
	_check(hint.get("alert", {}) == research, "the hint carries the bell alert (same click path)")
	var no_research := base.duplicate()
	no_research["alerts"] = []
	hint = NextHint.choose(no_research)
	_check(str(hint.get("id", "")) == "army_idle" and str(hint.get("army_id", "")) == "army_1", "then the idle army, got %s" % [hint])
	_check(str(hint.get("text", "")).contains("clic droit") and str(hint.get("text", "")).contains("Philippe VI"), "army hint explains the right click: %s" % [hint.get("text", "")])
	var dismissed := no_research.duplicate()
	dismissed["dismissed"] = ["army_idle"]
	hint = NextHint.choose(dismissed)
	_check(str(hint.get("id", "")) == "idle_treasury" and str(hint.get("action", "")) == "open_city", "dismissed army hint gives way to the idle treasury, got %s" % [hint])
	var building := dismissed.duplicate()
	building["constructions"] = 1
	hint = NextHint.choose(building)
	_check(str(hint.get("id", "")) == "end_turn", "all done: end the season, got %s" % [hint])
	var poor := dismissed.duplicate()
	poor["treasury"] = 1500
	_check(str(NextHint.choose(poor).get("id", "")) == "end_turn", "a small treasury is not idle")
	var rich_income := dismissed.duplicate()
	rich_income["income"] = 40000
	_check(str(NextHint.choose(rich_income).get("id", "")) == "end_turn", "treasury under two seasons of income is not idle")
	var postponed := building.duplicate()
	postponed["tutorial_step"] = 3
	hint = NextHint.choose(postponed)
	_check(str(hint.get("id", "")) == "tutorial_postponed" and str(hint.get("text", "")).contains("étape 4"), "postponed guide offered, got %s" % [hint])
	var siege := base.duplicate()
	siege["alerts"] = [research, {"kind": "siege", "province_id": "prov_x", "province_name": "Paris"}]
	hint = NextHint.choose(siege)
	_check(str(hint.get("title", "")) == "Défendez Paris", "a siege outranks research, got %s" % [hint])
	var blocked := base.duplicate()
	blocked["alerts"] = [research, {"kind": "chronicle_decision", "blocking": true}]
	_check(str(NextHint.choose(blocked).get("id", "")) == "chronicle_decision", "a pending decision comes first")


func _test_army_idle() -> void:
	_check(NextHint.army_is_idle({"path": [], "movement_left": 10, "movement_max": 10, "stance": "normal"}), "fresh army is idle")
	_check(not NextHint.army_is_idle({"path": [], "movement_left": 4, "movement_max": 10}), "an army that marched is not idle")
	_check(not NextHint.army_is_idle({"path": ["prov_a"], "movement_left": 10, "movement_max": 10}), "an army with orders is not idle")
	_check(not NextHint.army_is_idle({"path": [], "planned_path": PackedVector2Array([Vector2.ONE]), "movement_left": 10, "movement_max": 10}), "a planned march is an order")
	_check(not NextHint.army_is_idle({"path": [], "movement_left": 10, "movement_max": 10, "stance": "siege"}), "a besieging army is busy")
	_check(NextHint.army_is_idle({"path": [], "movement_points": 3}), "mock armies without allowance")


func _test_panel_placement() -> void:
	var view := Rect2(0, 0, 1600, 900)
	var panel_size := Vector2(500, 260)
	var area := Rect2(view.position + Vector2(16, 60), view.size - Vector2(32, 60 + 18))
	# Cible sans conflit : en bas au centre (position habituelle).
	var far := Rect2(1200, 8, 160, 40)
	var spot := TutorialOverlay.place_panel(view, panel_size, far)
	_check(is_equal_approx(spot.x, roundf(16 + (1568 - 500) * 0.5)) and is_equal_approx(spot.y, 900 - 18 - 260), "default spot is bottom centre, got %s" % [spot])
	# Cible en bas au centre (bandeau d'ost) : le parchemin passe au-dessus, sans la couvrir.
	var strip := Rect2(420, 740, 760, 150)
	spot = TutorialOverlay.place_panel(view, panel_size, strip)
	var rect := Rect2(spot, panel_size)
	_check(not rect.intersects(strip.grow(TutorialOverlay.TARGET_GAP)), "panel avoids the army strip, got %s" % [rect])
	_check(area.encloses(rect), "panel stays on screen, got %s" % [rect])
	# Armée (point) au centre bas et panneau de province à droite : côté libre.
	var army := Rect2(Vector2(800, 600) - Vector2.ONE * 34, Vector2.ONE * 68)
	var province_panel := Rect2(1050, 60, 530, 700)
	spot = TutorialOverlay.place_panel(view, panel_size, army, [province_panel])
	rect = Rect2(spot, panel_size)
	_check(not rect.intersects(army.grow(TutorialOverlay.TARGET_GAP)) and not rect.intersects(province_panel), "panel avoids the army and the open panel, got %s" % [rect])
	# Position courante gardée tant qu'elle reste libre (pas de saut à chaque image).
	var current := Vector2(40, 120)
	_check(TutorialOverlay.place_panel(view, panel_size, strip, [], current) == current, "a free current spot is kept")
	# Position courante devenue gênante : recalculée.
	spot = TutorialOverlay.place_panel(view, panel_size, Rect2(60, 160, 100, 60), [], current)
	_check(not Rect2(spot, panel_size).intersects(Rect2(60, 160, 100, 60).grow(TutorialOverlay.TARGET_GAP)), "a covering current spot moves, got %s" % [spot])
	# Petit écran encombré : le moins mauvais reste à l'écran.
	var small := Rect2(0, 0, 640, 480)
	spot = TutorialOverlay.place_panel(small, panel_size, Rect2(0, 60, 640, 420))
	_check(Rect2(small.position + Vector2(16, 60), small.size - Vector2(32, 78)).grow(1.0).has_point(spot), "fallback stays on screen, got %s" % [spot])


func _test_postpone_resume() -> void:
	var fake := FakeMap.new()
	root.add_child(fake)
	var overlay: TutorialOverlay = (load("res://scenes/ui/tutorial.tscn") as PackedScene).instantiate()
	root.add_child(overlay)
	await process_frame
	var controller := TutorialController.new()
	controller.map = fake
	controller.overlay = overlay
	controller.persist_progress = false
	overlay.later_pressed.connect(controller.postpone.bind(false))
	overlay.step_chosen.connect(controller.jump_to)
	controller.start(0)
	_check(controller.active and overlay.visible and controller.current_step_id() == "intro", "tutorial starts at the intro")
	_check(overlay.toc_box.get_child_count() == TutorialSteps.count(), "table of contents lists every step")
	for _step in 3:
		controller.advance()
	_check(controller.step_index == 3, "advanced to step 4")
	overlay.later_button.pressed.emit()
	_check(not controller.active and not overlay.visible, "« Plus tard » hides the guide")
	_check(controller.postponed_step() == 3, "postponed step remembered, got %d" % controller.postponed_step())
	controller.reopen()
	_check(controller.active and controller.step_index == 3 and overlay.visible, "reopen resumes at the same step, got %d" % controller.step_index)
	_check(controller.postponed_step() == -1, "resumed guide is no longer postponed")
	await process_frame
	overlay.set_toc_open(true)
	var rows := overlay.toc_box.get_children()
	_check(rows.size() == TutorialSteps.count() and str((rows[3] as Button).text).begins_with("▸"), "current step marked in the table of contents")
	(rows[7] as Button).pressed.emit()
	_check(controller.step_index == 7 and not overlay.toc_open(), "a table-of-contents row jumps to its step and closes the list")
	overlay.queue_free()
	fake.queue_free()
