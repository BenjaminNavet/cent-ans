extends SceneTree

## CB-M1 : contours de formation en décales.
## 1. Table d'états de `BattleFormationOutline.outline_state` (fonction pure).
## 2. Textures : clés bornées, image pleine/pointillée cohérente.
## 3. Intégration : démo autonome de bataille (`battle.tscn`), une décale par régiment, plus
##    d'anneau jaune, sélection = trait plein, ennemi ciblé = pulsé, régiment absent = caché.
##
## Usage : godot --headless --path game --script res://tests/cb_m1_outline_test.gd

const S := BattleFormationOutline.State

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_state_table()
	_check_textures()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb_m1_outline_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb_m1_outline_test: " + message)
	return condition


func _check_state_table() -> void:
	var own := {"side": "attacker", "present": true, "state": "idle"}
	var foe := {"side": "defender", "present": true, "state": "melee"}
	var own_routing := {"side": "attacker", "present": true, "state": "routing"}
	var foe_routing := {"side": "defender", "present": true, "state": "routing"}
	var own_gone := {"side": "attacker", "present": false, "state": "idle"}
	var foe_gone := {"side": "defender", "present": false, "state": "idle"}
	# [unité, sélectionnée, survolée, ciblée, attendu]
	var rows := [
		[own, false, false, false, S.NONE],
		[own, true, false, false, S.SELECTED],
		[own, false, true, false, S.HOVERED],
		[own, true, true, false, S.SELECTED],
		[own, false, false, true, S.NONE],  # une troupe du joueur n'est jamais « ciblée »
		[foe, false, false, false, S.NONE],
		[foe, false, true, false, S.ENEMY_HOVERED],
		[foe, false, false, true, S.ENEMY_TARGETED],
		[foe, false, true, true, S.ENEMY_TARGETED],
		[foe, true, false, false, S.NONE],  # un ennemi n'est pas sélectionnable
		[own_routing, true, true, false, S.NONE],
		[foe_routing, false, true, true, S.NONE],
		[own_gone, true, false, false, S.NONE],
		[foe_gone, false, true, true, S.NONE],
	]
	for i in rows.size():
		var row: Array = rows[i]
		var got := BattleFormationOutline.outline_state(row[0], row[1], row[2], row[3], "attacker")
		_check(got == int(row[4]), "state row %d: got %d, expected %d" % [i, got, row[4]])
	# Le camp du joueur peut être le défenseur.
	_check(BattleFormationOutline.outline_state(foe, true, false, false, "defender") == S.SELECTED, "defender player: own unit selected")
	_check(BattleFormationOutline.outline_state(own, false, true, false, "defender") == S.ENEMY_HOVERED, "defender player: attacker is the enemy")


func _check_textures() -> void:
	var wide := BattleFormationOutline.texture_key(63.0, 9.0, false)
	_check(wide.y == 0 and wide.z == 0 and BattleFormationOutline.RATIOS[wide.x] in [6.0, 8.0], "wide key %s" % wide)
	var column := BattleFormationOutline.texture_key(8.0, 40.0, true)
	_check(column.y == 1 and column.z == 1 and BattleFormationOutline.RATIOS[column.x] in [4.0, 6.0], "column key %s" % column)
	_check(BattleFormationOutline.texture_key(10.0, 10.0, false).x == 0, "square key")
	_check(BattleFormationOutline.texture_key(1000.0, 1.0, false).x == BattleFormationOutline.RATIOS.size() - 1, "ratio clamped to the largest bucket")
	var size := Vector2i(192, 96)
	var solid := BattleFormationOutline.outline_image(size, false)
	var dashed := BattleFormationOutline.outline_image(size, true)
	_check(solid.get_size() == size and dashed.get_size() == size, "image size")
	_check(solid.get_pixel(0, 0).a > 0.9 and solid.get_pixel(96, 48).a < 0.1, "solid: frame opaque, centre clear")
	var step := BattleFormationOutline.DASH_PX + 2
	_check(dashed.get_pixel(1, 1).a > 0.9 and dashed.get_pixel(step, 1).a < 0.1, "dashed: gap after the first dash")
	_check(solid.get_pixel(step, 1).a > 0.9, "solid: no gap")


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not _check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	_scene.paused = true
	await process_frame
	_check(not ("_rings" in _scene), "old selection rings still declared")
	var outlines: BattleFormationOutline = _scene.outlines
	if not _check(outlines != null, "no outline node"):
		return
	var units: Array = _scene.units
	_check(outlines.decal_count() == units.size(), "one decal per regiment: %d decals, %d units" % [outlines.decal_count(), units.size()])
	for child in _scene.get_children():
		_check(not (child is MeshInstance3D and (child as MeshInstance3D).material_override is StandardMaterial3D and ((child as MeshInstance3D).material_override as StandardMaterial3D).albedo_color == Color(1.0, 0.85, 0.2)), "yellow ring mesh left in the scene: %s" % child.name)
	# Sélection d'une troupe du joueur, cible = un ennemi présent.
	var own := -1
	var foe := -1
	for unit in units:
		if not bool(unit["present"]):
			continue
		if own < 0 and str(unit["side"]) == _scene.player_side:
			own = int(unit["id"])
		elif foe < 0 and str(unit["side"]) != _scene.player_side:
			foe = int(unit["id"])
	if not _check(own >= 0 and foe >= 0, "no present regiment on both sides"):
		return
	_scene.selected.assign([own])
	_scene.markers.world_hover = -1
	_scene._refresh_view(true)
	var decal := outlines.decal_of(own)
	_check(decal.visible and outlines.state_of(own) == S.SELECTED, "selected decal visible and solid")
	_check(decal.size.y == BattleFormationOutline.HEIGHT, "decal height covers the relief")
	var unit := _unit(own)
	_check(absf(decal.size.x - (float(unit["width"]) + BattleFormationOutline.MARGIN)) < 0.3, "decal width follows the front")
	_check(absf(decal.position.x - float(unit["x"])) < 0.01 and absf(decal.position.z - float(unit["z"])) < 0.01, "decal follows the regiment")
	_check(absf(decal.rotation.y - float(unit["facing"])) < 0.001, "decal follows the facing")
	_check(not outlines.decal_of(foe).visible, "idle enemy has no outline")
	# Survol de l'ennemi (terrain) = pointillé rouge.
	_scene.markers.world_hover = foe
	_scene._refresh_view(true)
	_check(outlines.decal_of(foe).visible and outlines.state_of(foe) == S.ENEMY_HOVERED, "hovered enemy dashed")
	_scene.markers.world_hover = -1
	# Ordre d'attaque : la cible pulse.
	_scene.issue({"type": "attack", "units": [own], "target": foe, "run": false})
	_scene.battle.call("tick", 0.1)
	_scene._refresh_view(true)
	if _check(int(_unit(own).get("target", -1)) == foe, "attack order did not set target"):
		_check(outlines.state_of(foe) == S.ENEMY_TARGETED, "targeted enemy pulses")
		var first := outlines.decal_of(foe).modulate.a
		for _i in 20:
			await process_frame
		_check(absf(outlines.decal_of(foe).modulate.a - first) > 0.01, "pulse animates modulate alpha")
	# Désélection : plus de contour ; régiment absent : décale cachée.
	_scene.selected.clear()
	_scene._refresh_view(true)
	_check(not outlines.decal_of(own).visible and not outlines.decal_of(foe).visible, "cleared selection hides decals")
	var gone := {"id": own, "side": _scene.player_side, "present": false, "state": "idle", "x": 0.0, "z": 0.0, "y": 0.0, "facing": 0.0, "width": 10.0, "depth": 4.0}
	outlines.update([gone], [own], [own])
	_check(not outlines.decal_of(own).visible, "absent regiment hides its decal")


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}
