extends SceneTree

## CB-M4 : portée au sol et comparaison au survol.
## 1. Fonctions pures : ouverture du panneau (une seule sélection + un ennemi survolé), couleurs
##    selon les drapeaux, textes issus du dictionnaire, tireurs retenus pour l'arc (portée > 0).
## 2. Intégration sur la démo autonome (`battle.tscn`) : `get_units` donne `effective_range` et
##    `fire_arc` ; tireur sélectionné + ennemi survolé (terrain, puis bannière) = arc et panneau
##    aux chiffres de `hover_context` ; deux troupes sélectionnées = pas de panneau ; troupe sans
##    tir = pas d'arc.
##
## Usage : godot --headless --path game --script res://tests/cb_m4_range_compare_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_pure()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb_m4_range_compare_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb_m4_range_compare_test: " + message)
	return condition


func _fake_compare() -> Dictionary:
	var ours := {"soldiers": 120, "melee": 8.0, "defense": 6.0, "charge": 2.0, "ranged": 11.0, "range": 212.4, "morale": 70.0, "fatigue": 10.0, "bonus_vs": 100.0}
	var theirs := {"soldiers": 80, "melee": 14.0, "defense": 12.0, "charge": 9.0, "ranged": 0.0, "range": 0.0, "morale": 68.0, "fatigue": 30.0, "bonus_vs": 100.0}
	var advantages := {"soldiers": "ours", "melee": "theirs", "defense": "theirs", "charge": "theirs", "ranged": "ours", "range": "ours", "morale": "even", "fatigue": "ours", "bonus_vs": "even"}
	return {"ours": ours, "theirs": theirs, "advantages": advantages, "lines": advantages.keys()}


func _check_pure() -> void:
	_check(BattleComparePanel.wanted(1, 7), "one selected + enemy: open")
	_check(not BattleComparePanel.wanted(2, 7), "two selected: closed")
	_check(not BattleComparePanel.wanted(0, 7), "nothing selected: closed")
	_check(not BattleComparePanel.wanted(1, -1), "no enemy: closed")
	_check(BattleComparePanel.value_color("ours", true) == BattleUiKit.GOOD, "our advantage: ours green")
	_check(BattleComparePanel.value_color("ours", false) == BattleUiKit.RUBRIC, "our advantage: theirs red")
	_check(BattleComparePanel.value_color("theirs", true) == BattleUiKit.RUBRIC, "their advantage: ours red")
	_check(BattleComparePanel.value_color("even", false) == BattleUiKit.INK, "even: ink")
	var panel := BattleComparePanel.new()
	root.add_child(panel)
	panel.show_compare(_fake_compare(), "Archers", "Chevaliers")
	_check(panel.visible, "panel shown with a comparison")
	_check(panel.shown.size() == 9, "nine lines shown (got %d)" % panel.shown.size())
	_check(panel.shown.get("soldiers", []) == ["120", "80", "ours"], "soldiers from the dictionary (%s)" % [panel.shown.get("soldiers")])
	_check(panel.shown.get("range", []) == ["212 m", BattleComparePanel.NONE, "ours"], "range in metres, none without missiles (%s)" % [panel.shown.get("range")])
	_check(panel.shown.get("melee", [])[2] == "theirs", "advantage flag kept")
	panel.show_compare({}, "", "")
	_check(not panel.visible, "empty comparison closes the panel")
	panel.free()
	var units := [
		{"id": 1, "side": "attacker", "effective_range": 200.0, "fire_arc": 1.0, "present": true},
		{"id": 2, "side": "attacker", "effective_range": 0.0, "fire_arc": 1.0, "present": true},
		{"id": 3, "side": "defender", "effective_range": 150.0, "fire_arc": 1.0, "present": true},
	]
	var ids := BattleRangeArc.shooters(units, [1, 2], [3, 1]).map(func(u: Dictionary) -> int: return int(u["id"]))
	_check(ids == [1, 3], "arcs: shooters selected then hovered, no duplicate, none for id 2 (got %s)" % [ids])
	_check(BattleRangeArc.shooters(units, [2], []).is_empty(), "no arc for a regiment without missiles")
	_check(BattleComparePanel.hovered_enemy(units, [1, 3], "attacker") == 3, "hovered enemy found")
	_check(BattleComparePanel.hovered_enemy(units, [1, 2], "attacker") == -1, "friends only: no enemy")


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not _check(_scene.get("battle") != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	if not _check(_scene.has_method("_update_outlines") and _scene.get("compare_panel") != null and _scene.get("range_arc") != null, "no compare panel / range arc in the scene"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var shooter := -1
	var foot := -1
	var foe := -1
	for unit in _scene.units:
		if not bool(unit["present"]):
			continue
		_check(unit.has("effective_range") and unit.has("fire_arc"), "get_units exposes effective_range and fire_arc")
		if str(unit["side"]) == _scene.player_side:
			if float(unit["effective_range"]) > 0.0:
				if shooter < 0:
					shooter = int(unit["id"])
			elif foot < 0:
				foot = int(unit["id"])
		elif foe < 0:
			foe = int(unit["id"])
	if not _check(shooter >= 0 and foot >= 0 and foe >= 0, "demo lacks a shooter, a non-shooter or an enemy"):
		return
	var panel: BattleComparePanel = _scene.compare_panel
	var arc: BattleRangeArc = _scene.range_arc
	var info := _unit(shooter)
	_check(float(info["fire_arc"]) > 0.0 and float(info["fire_arc"]) < PI, "fire sector from the rules (%s)" % info["fire_arc"])

	# Tireur sélectionné, ennemi survolé sur le terrain : arc et panneau aux chiffres du cœur.
	_scene.selected.assign([shooter])
	_scene.markers.world_hover = foe
	_scene._update_outlines()
	_check(arc.shown.has(shooter), "arc of the selected shooter (%s)" % [arc.shown])
	_check(_arc_drawn(arc), "arc mesh built on the ground")
	_check(panel.visible and panel.enemy == foe, "panel open on the hovered enemy")
	var foe_info := _unit(foe)
	var hover: Dictionary = _scene.battle.call("hover_context", float(foe_info["x"]), float(foe_info["z"]), PackedInt32Array([shooter]))
	var compare: Dictionary = hover.get("compare", {})
	if _check(not compare.is_empty(), "core gives a comparison"):
		var ours: Dictionary = compare["ours"]
		var theirs: Dictionary = compare["theirs"]
		var advantages: Dictionary = compare["advantages"]
		for line in BattleComparePanel.LINES:
			var key: String = line[0]
			var expected := [
				BattleComparePanel.value_text(key, float(ours[key]), str(line[2])),
				BattleComparePanel.value_text(key, float(theirs[key]), str(line[2])),
				str(advantages[key]),
			]
			_check(panel.shown.get(key, []) == expected, "%s from hover_context (%s vs %s)" % [key, panel.shown.get(key), expected])
	# Pas d'appel au cœur à chaque image tant que la paire ne change pas.
	var calls := panel.core_calls
	_scene._update_outlines()
	_check(panel.core_calls == calls, "throttled refresh")

	# Deux troupes sélectionnées : pas de panneau.
	_scene.selected.assign([shooter, foot])
	_scene._update_outlines()
	_check(not panel.visible, "two selected: no comparison")

	# Survol par la bannière de l'ennemi.
	_scene.selected.assign([shooter])
	_scene.markers.world_hover = -1
	var key_of: Dictionary = _scene.markers.get("_key_of") if _scene.markers.get("_key_of") != null else {}
	if key_of.has(foe):
		_scene.markers.hovered = int(key_of[foe])
		_scene._update_outlines()
		_check(panel.visible and panel.enemy == foe, "banner hover opens the comparison")
		_scene.markers.hovered = -1
	else:
		print("cb_m4_range_compare_test: no banner key for the enemy, banner case skipped")

	# Troupe sans tir sélectionnée, rien survolé : ni arc ni panneau.
	_scene.selected.assign([foot])
	_scene._update_outlines()
	_check(not arc.shown.has(foot), "no arc for a regiment without missiles")
	_check(arc.shown.is_empty(), "no arc at all (got %s)" % [arc.shown])
	_check(not panel.visible, "nothing hovered: no comparison")
	_scene.selected.clear()


func _arc_drawn(arc: BattleRangeArc) -> bool:
	for child in arc.get_children():
		var node := child as MeshInstance3D
		if node != null and node.visible and node.mesh != null and node.mesh.get_surface_count() > 0:
			return true
	return false


func _unit(id: int) -> Dictionary:
	for unit in _scene.units:
		if int(unit["id"]) == id:
			return unit
	return {}
