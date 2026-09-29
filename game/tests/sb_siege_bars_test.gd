extends SceneTree

## SB (TW2, ADR 0107) : barres de vie flottantes des ouvrages de siège.
## 1. `SiegeHealthBars.states` (fonction pure) : visibilité (endommagée ou visée, masquée
##    intacte ou tombée), ratio, étiquette « Porte : 324/540 », engins entamés.
## 2. Nœud seul avec une caméra : la barre d'une pièce visée est à l'écran, étiquette visible,
##    taille constante ; celle d'une pièce intacte n'existe pas.
## 3. Intégration : siège de Guyenne (`debug_stage_siege`) — `get_siege` expose `under_attack`
##    et `engines` ; la porte ramenée à mi-vie montre sa barre à 50 %.
##
## Usage : godot --headless --path game --script res://tests/sb_siege_bars_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_states()
	await _check_node()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("sb_siege_bars_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("sb_siege_bars_test: " + message)
	return condition


func _fake_siege() -> Dictionary:
	return {
		"pieces": [
			{"index": 0, "kind": "wall", "a": Vector2(0, 0), "b": Vector2(20, 0), "hp": 700.0, "max_hp": 700.0, "intact": true, "under_attack": false},
			{"index": 1, "kind": "gate", "a": Vector2(20, 0), "b": Vector2(34, 0), "hp": 324.0, "max_hp": 540.0, "intact": true, "under_attack": true},
			{"index": 2, "kind": "wall", "a": Vector2(34, 0), "b": Vector2(54, 0), "hp": 420.0, "max_hp": 700.0, "intact": true, "under_attack": false},
			{"index": 3, "kind": "wall", "a": Vector2(54, 0), "b": Vector2(74, 0), "hp": 700.0, "max_hp": 700.0, "intact": true, "under_attack": true},
			{"index": 4, "kind": "wall", "a": Vector2(74, 0), "b": Vector2(94, 0), "hp": 0.0, "max_hp": 700.0, "intact": false, "under_attack": false},
		],
		"engines": [
			{"unit": 7, "kind": "ram", "side": "attacker", "x": 27.0, "z": 8.0, "hp": 20.0, "max_hp": 20.0},
			{"unit": 8, "kind": "tower", "side": "attacker", "x": 60.0, "z": 8.0, "hp": 9.0, "max_hp": 36.0},
		],
	}


func _check_states() -> void:
	var ground := func(_x: float, _z: float) -> float: return 2.0
	var s := SiegeHealthBars.states(_fake_siege(), ground, 8.0)
	_check(not s["piece:0"]["shown"], "intact untouched wall hidden")
	_check(s["piece:1"]["shown"], "battered gate shown")
	_check(absf(float(s["piece:1"]["ratio"]) - 0.6) < 1e-4, "gate ratio 0.6: %s" % s["piece:1"]["ratio"])
	_check(str(s["piece:1"]["text"]) == "Porte : 324/540", "gate caption: %s" % s["piece:1"]["text"])
	_check(s["piece:2"]["shown"], "damaged wall shown although not under attack")
	_check(str(s["piece:2"]["text"]) == "Muraille : 420/700", "wall caption: %s" % s["piece:2"]["text"])
	_check(s["piece:3"]["shown"], "wall under attack shown although intact")
	_check(not s["piece:4"]["shown"], "breached wall hidden (rubble)")
	_check(not s["engine:7"]["shown"], "full-strength ram hidden")
	_check(s["engine:8"]["shown"] and absf(float(s["engine:8"]["ratio"]) - 0.25) < 1e-4, "hurt tower shown at 25 %")
	_check(str(s["engine:8"]["text"]) == "Beffroi : 9/36", "tower caption: %s" % s["engine:8"]["text"])
	var journal: Array[Rect2] = [Rect2(10, 60, 360, 240)]
	_check(SiegeHealthBars._covered(Rect2(300, 250, 172, 33), journal), "bar under the battle log is covered")
	_check(not SiegeHealthBars._covered(Rect2(500, 250, 172, 33), journal), "bar clear of the battle log is shown")
	var world: Vector3 = s["piece:1"]["world"]
	_check(absf(world.y - (2.0 + 8.0 + SiegeHealthBars.WALL_LIFT)) < 1e-4 and absf(world.x - 27.0) < 1e-4, "gate bar above the wall walk: %s" % world)
	_check(str(s["piece:1"]["side"]) == "defender" and str(s["engine:8"]["side"]) == "attacker", "owner sides")


func _check_node() -> void:
	var holder := Node3D.new()
	root.add_child(holder)
	var camera := Camera3D.new()
	holder.add_child(camera)
	camera.position = Vector3(47, 40, 120)
	camera.look_at(Vector3(47, 10, 0))
	camera.current = true
	var bars := SiegeHealthBars.new()
	bars.wall_height = 8.0
	bars.height_at = func(_x: float, _z: float) -> float: return 0.0
	bars.side_colors = {"attacker": Color(0.1, 0.2, 0.7), "defender": Color(0.7, 0.1, 0.1)}
	# The headless mouse position is arbitrary: keep it off every bar.
	bars.mouse_override = Vector2(-1e6, -1e6)
	holder.add_child(bars)
	bars.sync(_fake_siege())
	await process_frame
	_check(bars.bar("piece:0") == null, "no bar built for an intact untouched wall")
	var gate: Control = bars.bar("piece:1")
	if _check(gate != null, "gate bar built"):
		_check(gate.visible, "gate bar on screen")
		_check(gate.size == SiegeHealthBars.BAR_SIZE, "constant screen size: %s" % gate.size)
		_check(absf(float(gate.get("ratio")) - 0.6) < 1e-4, "gate bar ratio")
		_check((gate.get("caption") as Label).visible, "caption shown on a piece under attack")
		_check((gate.get("fill") as Color).is_equal_approx(Color(0.7, 0.1, 0.1)), "defender colour")
	var wall: Control = bars.bar("piece:2")
	if _check(wall != null, "damaged wall bar built"):
		_check(not (wall.get("caption") as Label).visible, "no caption on a quiet damaged wall (hover only)")
	# Repaired to full and no longer attacked: hidden.
	var healed := _fake_siege()
	healed["pieces"][1]["hp"] = 540.0
	healed["pieces"][1]["under_attack"] = false
	bars.sync(healed)
	await process_frame
	_check(not gate.visible, "gate bar hidden once intact and quiet")
	holder.queue_free()
	await process_frame


func _check_integration() -> void:
	if not ClassDB.class_exists("CampaignSim") or not ClassDB.instantiate("CampaignSim").has_method("debug_stage_siege"):
		_check(false, "CampaignSim.debug_stage_siege not registered (run core/build.sh)")
		return
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.configure(sim, index, 7)
	root.add_child(_scene)
	for _i in 20:
		await process_frame
	var battle: Object = _scene.battle
	var siege: Dictionary = battle.call("get_siege")
	var pieces: Array = siege.get("pieces", [])
	if not _check(not pieces.is_empty(), "siege pieces"):
		return
	_check((pieces[0] as Dictionary).has("under_attack"), "pieces expose under_attack")
	_check(siege.has("engines"), "siege exposes engines")
	for engine in siege.get("engines", []):
		_check(str(engine["kind"]) in ["ram", "tower"] and float(engine["max_hp"]) > 0.0, "engine entry: %s" % engine)
	var view: BattleSiege = _scene.siege_view
	if not _check(view != null and view.health_bars != null, "siege view has health bars"):
		return
	var gate := int(siege["gate"])
	var key := "piece:%d" % gate
	_check(not view.health_bars.state(key).get("shown", true), "intact gate bar hidden at the start")
	var max_hp := float(pieces[gate]["max_hp"])
	battle.call("debug_set_piece_hp", gate, max_hp * 0.5)
	for _i in 3:
		await process_frame
	var s := view.health_bars.state(key)
	_check(bool(s.get("shown", false)), "gate bar shown at half HP")
	_check(absf(float(s.get("ratio", 0.0)) - 0.5) < 0.01, "gate bar at 50 %%: %s" % s.get("ratio"))
	_check(str(s.get("text", "")).begins_with("Porte : "), "gate caption: %s" % s.get("text"))
	_check(view.health_bars.side_colors.get("defender") == _scene.side_colors.get("defender"), "bars use the scene's side colours")
