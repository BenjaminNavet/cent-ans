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
	if phase in ["all", "battle"]:
		await phase_battle()
	if phase in ["all", "siege"]:
		await phase_siege()
	if phase in ["all", "actions"]:
		await phase_actions()
	if phase in ["all", "panels"]:
		await phase_panels()
		await dismiss_dialogs()
	if phase in ["all", "turns"]:
		await play_turns(turns)
	if phase in ["all", "save"]:
		await phase_save_settings()
	log_q1("done")
	quit(0)


# --- Actions de campagne -----------------------------------------------------------


func world_to_window(world: Vector3) -> Vector2:
	var camera: Camera3D = map.camera
	return root.get_final_transform() * camera.unproject_position(world)


func look_at_world(world: Vector3, distance: float) -> void:
	map.camera_rig.look_at_point(world, distance)
	map.camera_rig.snap()
	await wait(30)


func capital_id() -> String:
	return str(root.get_node("SimFacade").faction_info(map.player_faction).get("capital", ""))


func capital_world() -> Vector3:
	var data: Object = map.map_data
	var centroid: Vector2 = data.centroid_of_id(capital_id())
	var state: Dictionary = map.sim.call("get_province_state", capital_id())
	var city := str(state.get("city", ""))
	if city != "" and map.settlement_layer != null:
		return map.settlement_layer.world_position_of(city)
	return Vector3(centroid.x, data.surface_world_at(centroid.x, centroid.y), centroid.y)


func phase_actions() -> void:
	var capital := capital_world()
	log_q1("capital %s at %s" % [capital_id(), capital])
	# Vues rapprochées de la capitale (L1 Paris, CV1 colonies, CV2 armées).
	for distance in [60.0, 25.0, 10.0]:
		await look_at_world(capital, distance)
		await wait(60)
		await shot("capital-zoom-%d" % int(distance))
	await fps_probe("capital-zoom-10")
	await look_at_world(capital, 60.0)
	# Clic sur la capitale : panneau de province ou de colonie.
	await click_at(world_to_window(capital))
	await wait(20)
	await shot("capital-clicked")
	var panel: Control = map.ui.province_panel
	if not panel.visible:
		log_q1("province panel not opened by clicking the capital (army on top: '%s')" % map.selected_army)
		await key(KEY_ESCAPE)
		# Clic sur la campagne de la province, à l'écart de la ville et de l'armée.
		await click_at(world_to_window(capital) + Vector2(-220, 140))
		await wait(15)
		log_q1("clicked the province countryside: selected index %d, capital index %d" % [map.selected_index, map.map_data.index_of_id(capital_id())])
		if not panel.visible:
			map.flow.focus_province(capital_id())
			await wait(15)
	if panel.visible:
		await click(panel.recruit_button)
		await wait(10)
		await shot("recruit-list")
		var recruited := 0
		for attempt in 2:
			var button := first_enabled(panel.recruit_list, attempt)
			if button != null:
				await click(button)
				await wait(10)
				recruited += 1
				if not panel.recruit_list.is_visible_in_tree():
					log_q1("recruit list closed after recruiting")
					await click(panel.recruit_button)
					await wait(10)
		log_q1("recruited %d units" % recruited)
		await shot("recruited")
		for tab in panel.tabs.get_tab_count():
			panel.tabs.current_tab = tab
			await wait(10)
			await shot("province-tab-%d" % tab)
			if tab == 1:
				var built := false
				var building := first_enabled(panel.buildable_list, 0)
				if building != null:
					await click(building)
					await wait(10)
					built = true
				log_q1("building ordered: %s" % built)
				await shot("building-ordered")
		await key(KEY_ESCAPE)
		await wait(10)
	# Armée du joueur : sélection au clic puis déplacement libre (clic droit au sol).
	var army_ids: PackedStringArray = map.player_army_ids()
	if army_ids.is_empty():
		log_q1("no player army")
		return
	var army_id := army_ids[0]
	var army_world: Vector3 = map.armies.world_position_of(army_id)
	await look_at_world(army_world, 80.0)
	await click_at(world_to_window(army_world) + Vector2(0, -6))
	await wait(15)
	log_q1("selected army after click: '%s' (expected %s)" % [map.selected_army, army_id])
	if map.selected_army != army_id:
		map.select_army(army_id)
		await wait(5)
	await shot("army-selected")
	var target := world_to_window(army_world) + Vector2(260, 120)
	await move_to(target)
	await wait(20)
	await shot("army-move-preview")
	var before: Dictionary = map.sim.call("get_army", army_id)
	await click_at(target, MOUSE_BUTTON_RIGHT)
	await wait(90)
	var after: Dictionary = map.sim.call("get_army", army_id)
	log_q1("army moved: %s -> %s (mp %s -> %s)" % [before.get("position", before.get("province", "?")), after.get("position", after.get("province", "?")), before.get("movement_points", "?"), after.get("movement_points", "?")])
	await shot("army-moved")
	await key(KEY_ESCAPE)
	await wait(5)


func phase_panels() -> void:
	# Chaque panneau par son raccourci, capture, puis Échap (ou la même touche pour les modes).
	var entries := [[KEY_P, "diplomacy"], [KEY_C, "court"], [KEY_T, "tech"], [KEY_G, "agents"],
		[KEY_K, "codex"], [KEY_L, "encyclopedia"], [KEY_O, "objectives"], [KEY_F1, "help"],
		[KEY_N, "diplomacy-map"], [KEY_R, "religion-map"], [KEY_M, "unrest-map"]]
	for entry in entries:
		await key(entry[0])
		await wait(20)
		await shot("panel-%s" % entry[1])
		if str(entry[1]).ends_with("-map"):
			await key(entry[0])
		else:
			await key(KEY_ESCAPE)
		await wait(10)
		var leftover := open_panels()
		if not leftover.is_empty():
			log_q1("after closing %s, still open: %s" % [entry[1], leftover])
			await shot("panel-%s-after-close" % entry[1])
			await key(KEY_ESCAPE)
			await wait(5)
	# Cour : ouvre la fiche du premier personnage.
	await key(KEY_C)
	await wait(15)
	var court: Control = map.ui.court_panel
	await click(find_button(court, "Voir"))
	await wait(20)
	await shot("character-sheet")
	await key(KEY_ESCAPE)
	await key(KEY_ESCAPE)
	await wait(10)
	# Panneau de faction (clic sur le nom de la faction).
	await click(map.ui.faction_label)
	await wait(15)
	await shot("faction-panel")
	await key(KEY_ESCAPE)
	await wait(10)
	# Menu de la barre : chaque entrée ouvre une seule fenêtre.
	var popup: PopupMenu = map.ui.menu_button.get_popup()
	for index in popup.item_count:
		var label := popup.get_item_text(index)
		var id := popup.get_item_id(index)
		if label == "" or id in [0, 1, 3, 4] or popup.is_item_separator(index):
			continue
		var before := _window_count()
		popup.id_pressed.emit(id)
		await wait(15)
		var opened := _window_count() - before
		log_q1("menu entry '%s' (id %d): %d new window(s)" % [label, id, opened])
		await shot("menu-%d" % id)
		for _i in 2:
			await key(KEY_ESCAPE)
			await wait(5)
		for dialog in root.find_children("*", "AcceptDialog", true, false):
			(dialog as AcceptDialog).hide()
	# Journal déplié.
	await click(map.ui.log_toggle)
	await wait(10)
	await shot("journal-open")
	await click(map.ui.log_toggle)
	await wait(5)


func _window_count() -> int:
	var count := 0
	for node in root.find_children("*", "Control", true, false):
		var control := node as Control
		if control.is_visible_in_tree() and control is PanelContainer and control.get_parent() == map.ui:
			count += 1
	for node in root.find_children("*", "Window", true, false):
		if (node as Window).visible and not (node is PopupMenu):
			count += 1
	return count


func open_panels() -> Array:
	var names := []
	for child in map.ui.find_children("*", "Control", true, false):
		var control := child as Control
		if control.is_visible_in_tree() and (control.name.ends_with("Panel") or control.name.ends_with("Window") or control.name.ends_with("Sheet")):
			names.append(str(control.name))
	return names


# --- Bataille et siège -----------------------------------------------------------


func phase_battle() -> void:
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	var armies: Array = load("res://scripts/battle/battle_scene.gd").main_armies(map.sim, map.player_faction, enemy)
	if armies.is_empty() or not map.sim.has_method("debug_stage_battle"):
		log_q1("battle staging unavailable")
		return
	map.sim.call("debug_stage_battle", armies[0], armies[1])
	map.refresh_all()
	map.call("_offer_pending_battles")
	await wait(30)
	await shot("prebattle")
	var dialog: Node = map.get("_battle_dialog")
	var t := Time.get_ticks_msec()
	await click(dialog.get("fight_button"))
	var battle: Node = null
	while battle == null and Time.get_ticks_msec() - t < 60000:
		await wait(1)
		for child in root.get_children():
			if child.get("finished_shown") != null:
				battle = child
	if battle == null:
		log_q1("battle scene did not open")
		return
	await wait(120)
	log_q1("battle scene loaded in %d ms" % (Time.get_ticks_msec() - t))
	await shot("battle-deploy")
	await fps_probe("battle-deploy")
	await key(KEY_ENTER)
	await wait(60)
	await shot("battle-start")
	for _i in 3:
		await key(KEY_EQUAL)
	var t_fight := Time.get_ticks_msec()
	var next_shot := 20000
	var fight_limit_ms := 90000
	while not battle.get("finished_shown") and Time.get_ticks_msec() - t_fight < fight_limit_ms:
		await wait(10)
		if Time.get_ticks_msec() - t_fight > next_shot:
			await shot("battle-%ds" % (next_shot / 1000))
			await fps_probe("battle-%ds" % (next_shot / 1000), 60)
			next_shot += 40000
	if not battle.get("finished_shown"):
		var sim_battle: Object = battle.get("battle")
		log_q1("battle still running after %d s (clock %.0f s); sounding the general retreat" % [(Time.get_ticks_msec() - t_fight) / 1000, float(sim_battle.call("get_elapsed"))])
		await click(find_button(battle, "Retraite générale"))
		await wait(10)
		await shot("battle-retreat-confirm")
		await click(find_button(battle, "Sonner la retraite"))
		var t_retreat := Time.get_ticks_msec()
		while not battle.get("finished_shown") and Time.get_ticks_msec() - t_retreat < 120000:
			await wait(10)
		log_q1("after retreat: finished %s in %d s" % [battle.get("finished_shown"), (Time.get_ticks_msec() - t_retreat) / 1000])
	await wait(60)
	await shot("battle-result")
	var result_screen: Control = battle.get("result_screen")
	var back := find_button(result_screen, "Retour") if result_screen != null else null
	await click(back)
	await wait(60)
	log_q1("battle scene still valid after return: %s" % is_instance_valid(battle))
	await shot("after-battle")
	await dismiss_dialogs()


func phase_siege() -> void:
	if not map.sim.has_method("debug_stage_siege"):
		return
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	var armies: Array = load("res://scripts/battle/battle_scene.gd").main_armies(map.sim, map.player_faction, enemy)
	if armies.is_empty():
		return
	var target := "prov_guyenne" if map.player_faction != "fac_england" else "prov_ile_de_france"
	map.sim.call("debug_stage_siege", armies[0], target)
	map.refresh_all()
	await wait(10)
	map.flow.focus_province(target)
	await wait(40)
	await shot("siege-map")
	map.call("_offer_pending_battles")
	await wait(20)
	await shot("siege-prebattle")
	var dialog: Node = map.get("_battle_dialog")
	if dialog != null and dialog.visible:
		await click(dialog.get("auto_button"))
		await wait(30)
		await shot("siege-auto-resolved")
	await dismiss_dialogs()


# --- Sauvegarde, chargement, réglages ------------------------------------------------


func phase_save_settings() -> void:
	var date_saved: String = map.sim.call("get_date_label")
	await key(KEY_F5)
	await wait(20)
	await shot("quicksave")
	await key(KEY_ENTER)
	await wait(5)
	await dismiss_dialogs()
	await key(KEY_F9)
	await wait(30)
	log_q1("quickload: saved %s, now %s" % [date_saved, map.sim.call("get_date_label")])
	await shot("quickload")
	# Menu pause puis réglages.
	await key(KEY_ESCAPE)
	await wait(15)
	await shot("pause-menu")
	await click(find_button(root, "Réglages"))
	await wait(15)
	var menus := root.find_children("*", "Control", true, false).filter(func(n: Node) -> bool: return n.get_script() == load("res://scripts/ui/settings_menu.gd"))
	if menus.is_empty():
		log_q1("settings menu not found")
		return
	var menu: Control = menus[0]
	var tabs: TabContainer = menu.find_children("*", "TabContainer", true, false)[0]
	for tab in tabs.get_tab_count():
		tabs.current_tab = tab
		await wait(8)
		await shot("settings-tab-%d" % tab)
	await click(menu.find_child("CloseButton", true, false))
	await wait(10)
	await key(KEY_ESCAPE)
	await wait(10)
	# Qualité : basse puis ultra, sur la même vue.
	await look_at_world(capital_world(), 40.0)
	for quality in ["low", "ultra", "high"]:
		root.get_node("Settings").set_value("video/quality", quality)
		await wait(60)
		await shot("quality-%s" % quality)
		await fps_probe("quality-%s" % quality)
	for ui_size in [1.25, 0.8, 1.0]:
		root.get_node("Settings").set_value("interface/ui_size", ui_size)
		await wait(20)
		await shot("ui-size-%s" % ui_size)


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
		var skip_tutorial := find_button(map.ui, "Passer le tutoriel")
		if skip_tutorial != null:
			await shot("tutorial")
			await click(skip_tutorial)
			await wait(5)
			continue
		var chronicle: Control = map.chronicle.window if map.chronicle != null else null
		if chronicle != null and chronicle.visible:
			await shot("chronicle-decision")
			var choice: BaseButton = null
			for child in chronicle.find_children("*", "Button", true, false):
				var text := _text_of(child)
				if (child as Control).is_visible_in_tree() and not (child as BaseButton).disabled and text not in ["×", "Plus tard", ""]:
					choice = child
					break
			log_q1("chronicle decision: choosing '%s'" % (_text_of(choice) if choice != null else "?"))
			await click(choice)
			await wait(10)
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
	# Comme un joueur : fait défiler la liste jusqu'au bouton avant de cliquer.
	var ancestor := control.get_parent()
	while ancestor != null:
		if ancestor is ScrollContainer:
			(ancestor as ScrollContainer).ensure_control_visible(control)
			await wait(3)
			break
		ancestor = ancestor.get_parent()
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


func first_enabled(node: Node, skip: int) -> BaseButton:
	for child in node.find_children("*", "BaseButton", true, false):
		var button := child as BaseButton
		if not button.disabled and button.is_visible_in_tree() and not button.is_queued_for_deletion():
			if skip == 0:
				return button
			skip -= 1
	return null


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
