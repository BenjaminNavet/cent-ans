extends SceneTree

## Recette Q1 : joue une partie comme un joueur, en fenêtre, en envoyant de vraies entrées
## (clics souris et touches poussés dans le viewport) et en capturant chaque écran.
## Usage (avec affichage, pas en headless) :
##   godot --resolution 1920x1080 --path game --script res://tests/q1_playtest.gd -- \
##     --out=<dossier> [--faction=fac_france] [--turns=12] [--phase=all]
## Imprime « Q1 » + mesures (durées de fin de tour, FPS) ; les erreurs de script sortent sur
## la console de Godot.

var out_dir := ""
var faction := "fac_france"
var turns := 12
var phase := "all"
var shot_index := 0
var map: Node = null


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--faction="):
			faction = arg.trim_prefix("--faction=")
		elif arg.begins_with("--turns="):
			turns = int(arg.trim_prefix("--turns="))
		elif arg.begins_with("--phase="):
			phase = arg.trim_prefix("--phase=")
	if out_dir == "":
		out_dir = OS.get_user_data_dir().path_join("q1")
	DirAccess.make_dir_recursive_absolute(out_dir)
	_run.call_deferred()


func _run() -> void:
	await wait(5)
	change_scene_to_file("res://scenes/start_menu.tscn")
	await wait(90)
	await shot("menu")
	var menu: Node = current_scene
	# Carte de faction : clic sur le bouton « Choisir » de la faction voulue.
	var card_button: Button = menu.get("_card_buttons").get(faction)
	await click(card_button)
	await wait(10)
	await shot("menu-faction")
	await click(menu.get("start_button"))
	var t0 := Time.get_ticks_msec()
	# Écran de chargement puis carte.
	var loading_shot := false
	while true:
		await wait(1)
		var scene := current_scene
		if not loading_shot and scene != null and scene.get_class() != "Control" and Time.get_ticks_msec() - t0 > 1500:
			pass
		if not loading_shot and Time.get_ticks_msec() - t0 > 2500:
			loading_shot = true
			await shot("loading")
		if scene != null and scene.has_method("player_army_ids") and scene.get("load_ok"):
			break
		if Time.get_ticks_msec() - t0 > 180000:
			log_q1("TIMEOUT waiting for campaign map")
			quit(1)
			return
	map = current_scene
	log_q1("campaign ready in %d ms" % (Time.get_ticks_msec() - t0))
	await wait(120)
	await shot("campaign-start")
	await fps_probe("campaign-start")
	if phase in ["all", "turns"]:
		await play_turns(turns)
	log_q1("done")
	quit(0)


# --- Tours -------------------------------------------------------------------------


func play_turns(count: int) -> void:
	for turn in count:
		var t := Time.get_ticks_msec()
		await key(KEY_ENTER)
		await wait(3)
		log_q1("end turn %d: %d ms (date %s)" % [turn + 1, Time.get_ticks_msec() - t, map.sim.call("get_date_label")])
		await dismiss_dialogs()
		if turn % 4 == 3:
			await shot("turn-%02d" % (turn + 1))


## Ferme les fenêtres modales ouvertes après une fin de tour (rapport, bataille : auto).
func dismiss_dialogs() -> void:
	for _i in 6:
		var dialog: Node = map.get("_battle_dialog")
		if dialog != null and dialog.visible:
			await shot("prebattle")
			var auto_button := find_button(dialog, "auto")
			if auto_button != null:
				await click(auto_button)
				await wait(5)
				continue
		var ok := find_visible_button(map.ui, ["Fermer", "Continuer", "OK", "D'accord"])
		if ok == null:
			return
		await shot("dialog")
		await click(ok)
		await wait(5)


# --- Entrées -----------------------------------------------------------------------


func click(control: Control, button := MOUSE_BUTTON_LEFT) -> void:
	if control == null:
		log_q1("click on null control")
		return
	var point := window_point(control)
	await move_to(point)
	var hovered: Control = root.gui_get_hovered_control()
	if hovered != null and hovered != control and not control.is_ancestor_of(hovered):
		log_q1("OVERLAP: click on %s (%s) at %s hit %s" % [control.name, _text_of(control), point, hovered.get_path()])
	await click_at(point, button)


## Centre du contrôle en pixels de fenêtre (échelle d'interface et calques compris).
func window_point(control: Control, local := Vector2(-1, -1)) -> Vector2:
	if local.x < 0.0:
		local = control.size * 0.5
	return root.get_final_transform() * (control.get_global_transform_with_canvas() * local)


func move_to(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	await wait(3)


func click_at(point: Vector2, button := MOUSE_BUTTON_LEFT) -> void:
	await move_to(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.pressed = pressed
		event.position = point
		event.global_position = point
		Input.parse_input_event(event)
		Input.flush_buffered_events()
		await wait(2)


func key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
		await wait(2)


func find_button(node: Node, fragment: String) -> BaseButton:
	for child in node.find_children("*", "BaseButton", true, false):
		var button := child as BaseButton
		if button.is_visible_in_tree() and _text_of(button).to_lower().contains(fragment.to_lower()):
			return button
	return null


func find_visible_button(node: Node, texts: Array) -> BaseButton:
	for child in node.find_children("*", "BaseButton", true, false):
		var button := child as BaseButton
		if button.is_visible_in_tree() and _text_of(button) in texts:
			return button
	return null


func _text_of(control: Control) -> String:
	if control is Button:
		return (control as Button).text
	return ""


# --- Mesures et captures -----------------------------------------------------------


func wait(frames: int) -> void:
	for _i in frames:
		await process_frame


func shot(label: String) -> void:
	await RenderingServer.frame_post_draw
	shot_index += 1
	var path := out_dir.path_join("%03d-%s.png" % [shot_index, label])
	var image := root.get_texture().get_image()
	image.save_png(path)
	log_q1("shot %s" % path)


func fps_probe(label: String, frames := 120) -> void:
	var t := Time.get_ticks_usec()
	await wait(frames)
	var seconds := (Time.get_ticks_usec() - t) / 1000000.0
	log_q1("fps %s: %.1f (draw calls %d, primitives %d)" % [label, frames / seconds,
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)])


func log_q1(text: String) -> void:
	print("Q1 %s" % text)
