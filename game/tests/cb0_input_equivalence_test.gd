extends SceneTree

## CB0 : test d'équivalence des entrées de bataille. Rejoue, par `get_viewport().push_input()`,
## une séquence scriptée sur la démo autonome de bataille (France-Angleterre, graine 1337,
## `battle.tscn` sans campagne) : clic, cadre, Maj-clic, clic droit, glisser-droit, double clic
## droit, clic droit sur un ennemi, touches F/G/H/C/Échap/Espace/+/-, puis Ctrl+1 et 1, 1, 1.
## Compare les commandes obtenues (`BattleScene.issued_log`, rempli par `issue()` sous
## `log_orders_for_test`) au golden `game/tests/data/cb0_orders_golden.json`.
##
## Ce test verrouille le comportement d'avant l'extraction des entrées vers `battle_input.gd`
## (CB0) : il doit rester vert, sans modifier le golden, une fois l'extraction faite.
##
## Usage :
##   godot --headless --path game --script res://tests/cb0_input_equivalence_test.gd
##   godot --headless --path game --script res://tests/cb0_input_equivalence_test.gd -- --record
##       (régénère le golden sur le code courant ; à n'utiliser qu'avant l'extraction CB0)

const GOLDEN_PATH := "res://tests/data/cb0_orders_golden.json"

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	await _run()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb0_input_equivalence_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb0_input_equivalence_test: " + message)
	return condition


func _run() -> void:
	var record := OS.get_cmdline_user_args().has("--record")
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	# Script de scène non compilé (classe non importée) : une erreur d'appel interromprait _run()
	# sans compter d'échec, et le test sortirait « OK ».
	if not _check(_scene.has_method("issue"), "battle scene script failed to load (run godot --import?)"):
		return
	_scene.autoplay = true  # F5c : saute le déploiement (comme la démo autonome, les captures).
	_scene.log_orders_for_test = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not _check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)  # entrées manuelles seulement
	_scene.paused = true  # aucun tic pendant la séquence : positions stables
	_scene.issued_log.clear()
	await _play_sequence()
	var actual: Array = _scene.issued_log
	await _check_quick_select()  # CB0 étape 4 : n'affecte pas `issued_log` (déjà capturé ci-dessus)
	if record:
		var file := FileAccess.open(GOLDEN_PATH, FileAccess.WRITE)
		file.store_string(JSON.stringify(actual, "\t"))
		file.close()
		print("cb0_input_equivalence_test: golden written (%d commands) at %s" % [actual.size(), GOLDEN_PATH])
		return
	var golden_text := FileAccess.get_file_as_string(GOLDEN_PATH)
	if not _check(golden_text != "", "golden missing: %s (run once with -- --record)" % GOLDEN_PATH):
		return
	var golden: Variant = JSON.parse_string(golden_text)
	if not _check(golden is Array and golden.size() == actual.size(), "command count: got %d, expected %s" % [actual.size(), golden.size() if golden is Array else "?"]):
		print("cb0_input_equivalence_test: actual = %s" % JSON.stringify(actual))
		return
	for i in golden.size():
		_check(_approx_equal(golden[i], actual[i]), "command %d differs:\n  golden: %s\n  actual: %s" % [i, golden[i], actual[i]])


## Compare deux valeurs JSON en tolérant les écarts d'arrondi flottant (ré-encodage JSON, calculs
## géométriques du survol/glisser) : les entiers et types simples sont exacts.
func _approx_equal(a: Variant, b: Variant) -> bool:
	if a is Dictionary and b is Dictionary:
		if a.keys().size() != b.keys().size():
			return false
		for key in a.keys():
			if not b.has(key) or not _approx_equal(a[key], b[key]):
				return false
		return true
	if a is Array and b is Array:
		if a.size() != b.size():
			return false
		for i in a.size():
			if not _approx_equal(a[i], b[i]):
				return false
		return true
	if (a is float or a is int) and (b is float or b is int):
		return absf(float(a) - float(b)) < 0.05
	return a == b


## La séquence elle-même (une fonction par étape, ordre imposé par le plan CB0).
func _play_sequence() -> void:
	var units: Array = _scene.battle.call("get_units")
	var mine: Array = units.filter(func(u: Dictionary) -> bool: return str(u["side"]) == _scene.player_side and bool(u["present"]))
	var foes: Array = units.filter(func(u: Dictionary) -> bool: return str(u["side"]) == _scene.enemy_side and bool(u["present"]))
	if not _check(mine.size() >= 3 and foes.size() >= 1, "not enough regiments to script the sequence (%d allies, %d foes)" % [mine.size(), foes.size()]):
		return
	# Vue plongeante sur tout le champ (les deux camps sont visibles) plutôt que sur le seul
	# camp du joueur : la cible ennemie (étape 7) doit être à l'écran.
	var field_center: Vector2 = _scene.terrain.field_center()
	_scene.camera_rig.look_at_point(Vector3(field_center.x, 0.0, field_center.y), 750.0, 0.0)
	for _i in 3:
		await process_frame

	var s0: Vector2 = _scene._unit_screen(mine[0])
	var s1: Vector2 = _scene._unit_screen(mine[1])
	var s2: Vector2 = _scene._unit_screen(mine[2])
	var enemy_screen: Vector2 = _scene._unit_screen(foes[0])
	var view: Vector2 = _scene.get_viewport().get_visible_rect().size

	_left_click(s0, false)  # 1. Clic simple.
	OS.delay_msec(400)
	var rect_a := Vector2(minf(s0.x, s1.x) - 60.0, minf(s0.y, s1.y) - 60.0)
	var rect_b := Vector2(maxf(s0.x, s1.x) + 60.0, maxf(s0.y, s1.y) + 60.0)
	_drag_left(rect_a, rect_b, false)  # 2. Cadre.
	OS.delay_msec(400)
	_left_click(s2, true)  # 3. Maj-clic.
	OS.delay_msec(400)
	_right_click(Vector2(view.x * 0.1, view.y * 0.12))  # 4. Clic droit (sol, loin des troupes).
	OS.delay_msec(400)
	var center_screen := view * 0.5
	_drag_right(center_screen, center_screen + Vector2(120.0, 0.0))  # 5. Glisser-droit.
	OS.delay_msec(400)
	_right_click(center_screen)  # 6. Double clic droit (deux relâchers rapprochés).
	_right_click(center_screen)
	OS.delay_msec(400)
	_right_click(enemy_screen)  # 7. Clic droit sur un ennemi.
	OS.delay_msec(400)
	_key(KEY_F)  # 8-11. F / G / H / C.
	_key(KEY_G)
	_key(KEY_H)
	_key(KEY_C)
	_key(KEY_ESCAPE)  # 12-15. Échap / Espace / + / -.
	_key(KEY_SPACE)
	_key(KEY_EQUAL)
	_key(KEY_MINUS)
	OS.delay_msec(700)
	_key(KEY_1, true)  # 16-19. Ctrl+1 (mémorise), puis 1, 1, 1 (rappelle).
	OS.delay_msec(700)
	_key(KEY_1)
	OS.delay_msec(700)
	_key(KEY_1)
	OS.delay_msec(700)
	_key(KEY_1)
	await process_frame


## CB0 étape 4 (sélection rapide) : assertions directes, sans toucher au golden ci-dessus.
##  - Ctrl/Cmd+A = toutes les troupes du joueur présentes, hors déroute ;
##  - double clic gauche (350 ms) sur une troupe au sol = même `type` ;
##  - double clic sur une carte (`_on_card_double_clicked`) = même `type` et recentrage caméra.
func _check_quick_select() -> void:
	_scene.selected.clear()
	_key(KEY_A, true)
	var units: Array = _scene.battle.call("get_units")
	var expected_all: Array[int] = []
	for unit in units:
		if str(unit["side"]) == _scene.player_side and bool(unit["present"]) and str(unit["state"]) != "routing":
			expected_all.append(int(unit["id"]))
	_check(_ids_match(_scene.selected, expected_all), "Ctrl+A: got %s, expected %s" % [_scene.selected, expected_all])

	_scene.selected.clear()
	var mine: Array = units.filter(func(u: Dictionary) -> bool: return str(u["side"]) == _scene.player_side and bool(u["present"]))
	if not _check(mine.size() >= 1, "no player regiment for the quick-select check"):
		return
	var target: Dictionary = mine[0]
	var kind := str(target["type"])
	var expected_same_type: Array[int] = []
	for unit in mine:
		if str(unit["type"]) == kind:
			expected_same_type.append(int(unit["id"]))
	var s: Vector2 = _scene._unit_screen(target)
	_left_click(s, false)  # simple clic : sélection normale
	_left_click(s, false)  # aussitôt après : double clic, même type
	_check(_ids_match(_scene.selected, expected_same_type), "double left click on a %s: got %s, expected %s" % [kind, _scene.selected, expected_same_type])

	_scene.selected.clear()
	_scene.camera_rig.look_at_point(Vector3.ZERO, 750.0, 0.0)  # loin de la cible : le recentrage doit bouger la caméra
	_scene._on_card_double_clicked(int(target["id"]))
	_check(_ids_match(_scene.selected, expected_same_type), "card double click on a %s: got %s, expected %s" % [kind, _scene.selected, expected_same_type])
	var recentered: float = _scene.camera_rig.target.distance_to(Vector3(float(target["x"]), 0.0, float(target["z"])))
	_check(recentered < 5.0, "card double click should recenter the camera on the regiment (off by %.1f m)" % recentered)


func _ids_match(actual: Array, expected: Array[int]) -> bool:
	if actual.size() != expected.size():
		return false
	for id in expected:
		if not actual.has(id):
			return false
	return true


func _left_click(pos: Vector2, shift: bool) -> void:
	_mouse_button(pos, MOUSE_BUTTON_LEFT, true, shift)
	_mouse_button(pos, MOUSE_BUTTON_LEFT, false, shift)


func _drag_left(a: Vector2, b: Vector2, shift: bool) -> void:
	_mouse_button(a, MOUSE_BUTTON_LEFT, true, shift)
	_mouse_motion(b)
	_mouse_button(b, MOUSE_BUTTON_LEFT, false, shift)


func _right_click(pos: Vector2) -> void:
	_mouse_button(pos, MOUSE_BUTTON_RIGHT, true, false)
	_mouse_button(pos, MOUSE_BUTTON_RIGHT, false, false)


func _drag_right(a: Vector2, b: Vector2) -> void:
	_mouse_button(a, MOUSE_BUTTON_RIGHT, true, false)
	_mouse_motion(b)
	_mouse_button(b, MOUSE_BUTTON_RIGHT, false, false)


func _mouse_button(pos: Vector2, button: int, pressed: bool, shift: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = button
	event.pressed = pressed
	event.shift_pressed = shift
	event.position = pos
	event.global_position = pos
	_scene.get_viewport().push_input(event)


func _mouse_motion(pos: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = pos
	event.global_position = pos
	_scene.get_viewport().push_input(event)


func _key(code: Key, ctrl: bool = false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		event.ctrl_pressed = ctrl
		event.meta_pressed = false
		_scene.get_viewport().push_input(event)
