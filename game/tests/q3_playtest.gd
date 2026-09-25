extends SceneTree

## Recette Q3 : joue une partie comme un joueur, en fenêtre, avec de vraies entrées (clics souris,
## molette, touches poussés dans le viewport), et capture chaque écran. Dérivé du pilote Q1.
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/q3_playtest.gd -- --out=<dossier> \
##     [--faction=fac_france] [--turns=12] [--res=1920x1080] [--phases=battle,siege,...]
## Phases : battle, siege, naval, actions, edicts, diplomacy, trade, agent, zoom, panels, turns,
## save, settings (par défaut : toutes). Imprime « Q3 » + mesures ; les erreurs de script sortent
## sur la console. Le pilote écrit dans `user://settings.cfg` (résolution, conseiller réarmé) :
## sauvegarder ce fichier avant et le restaurer après.

const ALL_PHASES := ["battle", "siege", "naval", "actions", "edicts", "diplomacy", "trade", "agent",
	"zoom", "panels", "turns", "save", "settings"]
const WATCHDOG_S := 2400.0

var out_dir := ""
var faction := "fac_france"
var turns := 12
var resolution := Vector2i(1920, 1080)
var phases: Array = ALL_PHASES.duplicate()
var shot_index := 0
var map: Node = null
var phase_label := "start"
## Voix : images où le bus « Voix » sonne (pic > -50 dB), par phase.
var voice_frames: Dictionary = {}
var voice_peak: Dictionary = {}
var _voice_bus := -1


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--faction="):
			faction = arg.trim_prefix("--faction=")
		elif arg.begins_with("--turns="):
			turns = int(arg.trim_prefix("--turns="))
		elif arg.begins_with("--res="):
			var parts := arg.trim_prefix("--res=").split("x")
			resolution = Vector2i(int(parts[0]), int(parts[1]))
		elif arg.begins_with("--phases="):
			phases = Array(arg.trim_prefix("--phases=").split(","))
	if out_dir == "":
		out_dir = OS.get_user_data_dir().path_join("q3")
	DirAccess.make_dir_recursive_absolute(out_dir)
	create_timer(WATCHDOG_S).timeout.connect(func() -> void:
		log_q("WATCHDOG: pilot still running after %d s, quitting" % int(WATCHDOG_S))
		quit(3))
	_run.call_deferred()


func _run() -> void:
	await wait(5)
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("set_value", "video/fullscreen", false)
	settings.call("set_value", "video/resolution", resolution)
	# Le conseiller redit ses « premières fois » (réglages restaurés après la partie).
	settings.call("set_value", "voice/advisor", true)
	settings.call("set_value", "voice/barks", true)
	settings.call("set_value", "voice/advisor_seen", "")
	await wait(10)
	log_q("window %s, viewport %s" % [DisplayServer.window_get_size(), root.get_visible_rect().size])
	_voice_bus = AudioServer.get_bus_index("Voix")
	log_q("voice bus %d volume %.1f dB mute %s" % [_voice_bus, AudioServer.get_bus_volume_db(_voice_bus) if _voice_bus >= 0 else 0.0, AudioServer.is_bus_mute(_voice_bus) if _voice_bus >= 0 else false])
	change_scene_to_file("res://scenes/start_menu.tscn")
	await wait(90)
	await shot("menu")
	var menu: Node = current_scene
	var select: Node = menu.get("faction_select")
	if select != null:
		await click(menu.get("new_game_button"))
		await wait(40)
		await shot("menu-factions")
		var cards: Dictionary = select.get("_cards")
		if cards.has(faction):
			await click(cards[faction])
		else:
			select.call("select", faction)
		await wait(20)
	log_q("selected faction: %s" % menu.get("selected_faction"))
	await shot("menu-faction")
	await click(menu.get("start_button"))
	var t0 := Time.get_ticks_msec()
	var loading_shot := false
	while true:
		await wait(1)
		var scene := current_scene
		if not loading_shot and Time.get_ticks_msec() - t0 > 2500:
			loading_shot = true
			await shot("loading")
		if scene != null and scene.has_method("player_army_ids") and scene.get("load_ok"):
			break
		if Time.get_ticks_msec() - t0 > 240000:
			log_q("TIMEOUT waiting for campaign map")
			quit(1)
			return
	map = current_scene
	log_q("campaign ready in %d ms" % (Time.get_ticks_msec() - t0))
	phase_label = "campaign-start"
	await wait(150)
	await shot("campaign-start")
	await fps_probe("campaign-start")
	await dismiss_dialogs()
	for phase in phases:
		phase_label = str(phase)
		log_q("=== phase %s (date %s, treasury %s)" % [phase, map.sim.call("get_date_label"), _treasury()])
		match phase:
			"battle": await phase_battle()
			"siege": await phase_siege()
			"naval": await phase_naval()
			"actions": await phase_actions()
			"edicts": await phase_edicts()
			"diplomacy": await phase_diplomacy()
			"trade": await phase_trade()
			"agent": await phase_agent()
			"zoom": await phase_zoom()
			"panels":
				await phase_panels()
				await dismiss_dialogs()
			"turns": await play_turns(turns)
			"save": await phase_save()
			"settings": await phase_settings()
		log_q("=== end %s: voice frames %d, peak %.1f dB, advisor said %s" % [phase, int(voice_frames.get(phase, 0)), float(voice_peak.get(phase, -200.0)), _advisor_said()])
	log_q("done")
	quit(0)


func _treasury() -> String:
	var info: Dictionary = root.get_node("SimFacade").faction_info(map.player_faction)
	return str(info.get("treasury", "?"))


func _advisor_said() -> String:
	var advisor: Node = Advisor.current()
	if advisor == null:
		return "[]"
	return str(advisor.get("said"))


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


func province_city_world(province: String) -> Vector3:
	var data: Object = map.map_data
	var centroid: Vector2 = data.centroid_of_id(province)
	var state: Dictionary = map.sim.call("get_province_state", province)
	var city := str(state.get("city", ""))
	if city != "" and map.settlement_layer != null:
		return map.settlement_layer.world_position_of(city)
	return Vector3(centroid.x, data.surface_world_at(centroid.x, centroid.y), centroid.y)


func capital_world() -> Vector3:
	return province_city_world(capital_id())


func _open_panel() -> Control:
	var settlement_panel: Control = map.settlements_ctl.panel if map.settlements_ctl != null else null
	if settlement_panel != null and settlement_panel.is_visible_in_tree():
		return settlement_panel
	var province_panel: Control = map.ui.province_panel
	if province_panel.is_visible_in_tree():
		return province_panel
	return null


func phase_actions() -> void:
	var capital := capital_world()
	log_q("capital %s at %s" % [capital_id(), capital])
	await look_at_world(capital, 60.0)
	await click_at(world_to_window(capital))
	await wait(20)
	var panel := _open_panel()
	log_q("click on capital: panel %s, army '%s'" % [panel.name if panel != null else "none", map.selected_army])
	if panel == null:
		# Second clic (Q2 : alternance armée / ville).
		await click_at(world_to_window(capital))
		await wait(20)
		panel = _open_panel()
		log_q("second click on capital: panel %s" % (panel.name if panel != null else "none"))
	await shot("capital-clicked")
	if panel == null:
		return
	# Recrutement.
	if panel.get("recruit_button") != null and (panel.recruit_button as Button).is_visible_in_tree():
		await click(panel.recruit_button)
		await wait(15)
		await shot("recruit-list")
		var recruited := 0
		for attempt in 3:
			var button := first_enabled(panel.recruit_list, attempt)
			if button != null:
				await click(button)
				await wait(10)
				recruited += 1
				if not panel.recruit_list.is_visible_in_tree():
					await click(panel.recruit_button)
					await wait(10)
		log_q("recruited %d units" % recruited)
		await shot("recruited")
	# Construction.
	var tabs: TabContainer = panel.tabs
	for tab in tabs.get_tab_count():
		await click_tab(tabs, tab)
		await wait(15)
		await shot("panel-%s-tab-%d" % [panel.name, tab])
		if tabs.get_current_tab_control() != null and panel.buildable_list.is_visible_in_tree():
			var building := first_enabled(panel.buildable_list, 0)
			if building != null:
				log_q("building ordered: %s" % _text_of(building).replace("\n", " / "))
				await click(building)
				await wait(15)
				await shot("building-ordered")
	await key(KEY_ESCAPE)
	await wait(10)
	# Armée du joueur : sélection au clic puis déplacement libre (clic droit au sol).
	var army_ids: PackedStringArray = map.player_army_ids()
	if army_ids.is_empty():
		log_q("no player army")
		return
	var army_id := army_ids[0]
	var army_world: Vector3 = map.armies.world_position_of(army_id)
	await look_at_world(army_world, 80.0)
	await click_at(world_to_window(map.armies._markers[army_id].pick_position()))
	await wait(20)
	log_q("selected army after click: '%s' (expected %s)" % [map.selected_army, army_id])
	await shot("army-selected")
	var target := world_to_window(army_world) + Vector2(220, 110)
	await move_to(target)
	await wait(20)
	await shot("army-move-preview")
	var before: Dictionary = map.sim.call("get_army", army_id)
	await click_at(target, MOUSE_BUTTON_RIGHT)
	await wait(120)
	var after: Dictionary = map.sim.call("get_army", army_id)
	log_q("army moved: %s -> %s (mp %s -> %s)" % [before.get("position", before.get("province", "?")), after.get("position", after.get("province", "?")), before.get("movement_points", "?"), after.get("movement_points", "?")])
	await shot("army-moved")
	await key(KEY_ESCAPE)
	await wait(5)


## Édits (C4) : panneau de province de la capitale, onglet de l'édit, « Changer d'édit », premier
## édit disponible.
func phase_edicts() -> void:
	var capital := capital_world()
	await look_at_world(capital, 60.0)
	await click_at(world_to_window(capital))
	await wait(20)
	var settlement_panel: Control = map.settlements_ctl.panel if map.settlements_ctl != null else null
	if settlement_panel != null and settlement_panel.is_visible_in_tree():
		await click(settlement_panel.province_button)
		await wait(20)
	var panel: Control = map.ui.province_panel
	if not panel.is_visible_in_tree():
		log_q("edicts: province panel not reachable by clicks, using focus_province")
		map.flow.focus_province(capital_id())
		await wait(20)
	var section: Control = panel.get("edict_section")
	if section == null:
		log_q("edicts: no edict section")
		return
	for tab in panel.tabs.get_tab_count():
		if panel.tabs.get_tab_control(tab).is_ancestor_of(section):
			await click_tab(panel.tabs, tab)
	await wait(15)
	await shot("edict-section")
	log_q("edicts: section visible %s" % section.is_visible_in_tree())
	var choose: Button = section.get("choose_button")
	if choose != null and choose.is_visible_in_tree() and not choose.disabled:
		await click(choose)
		await wait(15)
		await shot("edict-options")
		var option := first_enabled(section, 0)
		# Le premier bouton activé peut être « Changer d'édit » lui-même.
		var skip := 0
		while option != null and option == choose:
			skip += 1
			option = first_enabled(section, skip)
		if option != null:
			log_q("edicts: choosing '%s'" % _text_of(option).replace("\n", " / "))
			await click(option)
			await wait(15)
		var result: Dictionary = section.get("last_result") if section.get("last_result") != null else {}
		log_q("edicts: result %s, error '%s'" % [result, (section.get("error_label") as Label).text if section.get("error_label") != null else ""])
		await shot("edict-chosen")
	else:
		log_q("edicts: choose button unavailable (%s)" % (choose.tooltip_text if choose != null else "null"))
	await key(KEY_ESCAPE)
	await wait(10)


## Diplomatie (DP1) : P, une faction en paix, accord commercial ; puis l'ennemi, trêve ; « Que
## faudrait-il ? », « Proposer le traité ».
func phase_diplomacy() -> void:
	await key(KEY_P)
	await wait(30)
	var panels := map.ui.find_children("*", "Control", true, false).filter(func(n: Node) -> bool: return n.get_script() == load("res://scripts/ui/diplomacy_panel.gd") and (n as Control).is_visible_in_tree())
	if panels.is_empty():
		log_q("diplomacy panel not open after P")
		return
	var panel: Control = panels[0]
	await shot("diplomacy-open")
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	for target in ["peace", enemy]:
		var row: Button = null
		for child in panel.find_children("Faction_*", "Button", true, false):
			var id := str(child.name).trim_prefix("Faction_")
			var entry: Dictionary = panel.call("_entry", id)
			if (target == "peace" and str(entry.get("status", "")) == "peace") or id == target:
				row = child
				break
		if row == null:
			log_q("diplomacy: no row for %s" % target)
			continue
		await click(row)
		await wait(20)
		var selected := str(panel.get("_selected"))
		log_q("diplomacy: selected %s (status %s)" % [selected, panel.call("_entry", selected).get("status", "?")])
		await shot("diplomacy-%s" % selected)
		var clause_menu: MenuButton = panel.get("_clause_menu")
		await click(clause_menu)
		await wait(15)
		await shot("diplomacy-clause-menu")
		var popup := clause_menu.get_popup()
		var wanted := "Accord commercial" if target == "peace" else "Trêve de deux ans"
		var picked := false
		for index in popup.item_count:
			if popup.get_item_text(index) == wanted:
				popup.id_pressed.emit(popup.get_item_id(index))
				picked = true
		popup.hide()
		log_q("diplomacy: clause '%s' picked %s, articles %d" % [wanted, picked, (panel.get("_articles") as Array).size()])
		await wait(15)
		await shot("diplomacy-draft-%s" % selected)
		log_q("diplomacy: chance '%s'" % (panel.get("_chance_label") as Label).text)
		await click(find_button(panel, "Que faudrait-il"))
		await wait(15)
		log_q("diplomacy: after counter, chance '%s', articles %d" % [(panel.get("_chance_label") as Label).text, (panel.get("_articles") as Array).size()])
		await shot("diplomacy-counter-%s" % selected)
		var status_before := str(panel.call("_entry", selected).get("status", ""))
		await click(find_button(panel, "Proposer le traité"))
		await wait(30)
		log_q("diplomacy: sent to %s, status %s -> %s, toast '%s'" % [selected, status_before, panel.call("_entry", selected).get("status", "?"), map.ui.toast.get("text") if map.ui.toast is Label else _toast_text()])
		await shot("diplomacy-sent-%s" % selected)
	await key(KEY_ESCAPE)
	await wait(10)


func _toast_text() -> String:
	for label in map.ui.toast.find_children("*", "Label", true, false):
		return (label as Label).text
	return ""


## Commerce (C5) : bouton de la couche des routes commerciales, vue large, puis la touche R.
func phase_trade() -> void:
	await look_at_world(capital_world(), 600.0)
	await wait(30)
	var routes: Array = map.sim.call("get_trade_routes") if map.sim.has_method("get_trade_routes") else []
	var mine := 0
	for route in routes:
		if str(route.get("from_faction", "")) == map.player_faction or str(route.get("to_faction", "")) == map.player_faction:
			mine += 1
	log_q("trade: %d routes, %d for the player" % [routes.size(), mine])
	var button: Button = map.ui.get("trade_button")
	if button != null and button.is_visible_in_tree():
		await click(button)
		await wait(30)
		log_q("trade: layer after button: trade_mode %s" % map.trade_mode)
		await shot("trade-layer")
		await click(button)
		await wait(10)
	else:
		log_q("trade: no visible trade button")
	await key(KEY_R)
	await wait(20)
	log_q("trade: after R, trade_mode %s, diplomacy mode %s" % [map.trade_mode, map.diplomacy.get("mode") if map.diplomacy != null else "?"])
	await shot("trade-key-r")
	await key(KEY_R)
	await wait(10)


## Agent (C6) : G, recrute le premier agent possible, fin de tour, le sélectionne, l'envoie vers
## une ville ennemie (clic droit), tente la première action disponible.
func phase_agent() -> void:
	var ctl: Node = map.agents_ctl
	if ctl == null or not ctl.available():
		log_q("agent: controller unavailable")
		return
	await key(KEY_G)
	await wait(20)
	await shot("agents-registry")
	var registry: Control = ctl.registry
	var recruit_box: Control = ctl.get("_registry_recruit")
	var button := first_enabled(recruit_box, 0)
	if button != null:
		log_q("agent: recruiting '%s'" % _text_of(button))
		await click(button)
		await wait(20)
		await shot("agent-recruited")
	else:
		log_q("agent: no recruit option enabled")
	if registry.visible:
		await key(KEY_G)
	await end_turn()
	await key(KEY_G)
	await wait(20)
	var row := first_enabled(ctl.get("_registry_list"), 0)
	if row == null:
		log_q("agent: no agent in registry after a turn")
		await key(KEY_G)
		return
	await click(row)
	await wait(40)
	log_q("agent: selected '%s', %d action buttons" % [ctl.selected_agent, ctl.action_button_count()])
	if registry.visible:
		await click(find_button(registry, "✕"))
	await shot("agent-selected")
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	var target_province := ""
	for province in root.get_node("SimFacade").call("faction_info", enemy).get("provinces", []):
		target_province = str(province)
		break
	if target_province == "":
		target_province = "prov_guyenne" if map.player_faction != "fac_england" else "prov_normandie"
	var target_world := province_city_world(target_province)
	await look_at_world(target_world, 150.0)
	await click_at(world_to_window(target_world), MOUSE_BUTTON_RIGHT)
	await wait(30)
	var agent: Dictionary = {}
	for entry in map.sim.call("get_agents"):
		if str(entry.get("id", "")) == ctl.selected_agent:
			agent = entry
	log_q("agent: right-click on %s → destination '%s' location '%s'" % [target_province, agent.get("destination", ""), agent.get("location", "")])
	await shot("agent-ordered")
	var action := first_enabled(ctl.get("_bar_actions"), 0)
	if action != null:
		await click(action)
		await wait(20)
		log_q("agent: action '%s' → report %s" % [_text_of(action).replace("\n", " "), map.sim.call("get_last_agent_report")])
		await shot("agent-action")
	else:
		log_q("agent: no action available at %s" % agent.get("location_name", "?"))
	await key(KEY_ESCAPE)
	await wait(5)


## Zoom (ZG) : molette au-dessus de la capitale, de la vue stratégique au plus près.
func phase_zoom() -> void:
	var capital := capital_world()
	await look_at_world(capital, map.camera_rig.max_distance)
	await wait(40)
	await shot("zoom-max")
	await fps_probe("zoom-max", 60)
	var marks := [800.0, 300.0, 120.0, 50.0, 20.0, 8.0, 3.0, 1.0, 0.3]
	var next := 0
	var last := map.camera_rig.distance
	var stalls := 0
	for step in 160:
		var centre := world_to_window(capital)
		await wheel(centre, MOUSE_BUTTON_WHEEL_DOWN)
		await wait(6)
		var distance: float = map.camera_rig.distance
		while next < marks.size() and distance <= marks[next]:
			await wait(40)
			log_q("zoom: distance %.2f (mark %.1f), min here %.2f" % [map.camera_rig.distance, marks[next], map.camera_rig.min_distance_at(capital)])
			await shot("zoom-%s" % str(marks[next]).replace(".", "_"))
			next += 1
		if absf(distance - last) < 0.001:
			stalls += 1
			if stalls > 8:
				break
		else:
			stalls = 0
		last = distance
	await wait(40)
	log_q("zoom: closest distance reached %.2f" % map.camera_rig.distance)
	await shot("zoom-closest")
	await fps_probe("zoom-closest", 60)
	# Rotation et bascule en vue rapprochée (Q/E), comme un joueur qui regarde la ville.
	for _i in 20:
		await key(KEY_Q)
	await wait(30)
	await shot("zoom-closest-rotated")
	await look_at_world(capital, 150.0)


func wheel(point: Vector2, button: MouseButton) -> void:
	await move_to(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = button
		event.pressed = pressed
		event.factor = 1.0
		event.position = point
		event.global_position = point
		Input.parse_input_event(event)
		Input.flush_buffered_events()
	await wait(1)


func phase_panels() -> void:
	var entries := [[KEY_P, "diplomacy"], [KEY_C, "court"], [KEY_T, "tech"], [KEY_G, "agents"],
		[KEY_K, "codex"], [KEY_L, "encyclopedia"], [KEY_O, "objectives"], [KEY_F1, "help"],
		[KEY_N, "diplomacy-map"], [KEY_R, "religion-map"], [KEY_M, "unrest-map"]]
	for entry in entries:
		await key(entry[0])
		await wait(25)
		await shot("panel-%s" % entry[1])
		if str(entry[1]).ends_with("-map"):
			await key(entry[0])
		else:
			await key(KEY_ESCAPE)
		await wait(10)
		var leftover := open_panels()
		if not leftover.is_empty():
			log_q("after closing %s, still open: %s" % [entry[1], leftover])
			await shot("panel-%s-after-close" % entry[1])
			await key(KEY_ESCAPE)
			await wait(5)
	await click(map.ui.faction_label)
	await wait(15)
	await shot("faction-panel")
	await key(KEY_ESCAPE)
	await wait(10)
	await click(map.ui.log_toggle)
	await wait(10)
	await shot("journal-open")
	await click(map.ui.log_toggle)
	await wait(5)


func open_panels() -> Array:
	var names := []
	for child in map.ui.find_children("*", "Control", true, false):
		var control := child as Control
		if control.is_visible_in_tree() and (control.name.ends_with("Panel") or control.name.ends_with("Window") or control.name.ends_with("Sheet")):
			names.append(str(control.name))
	return names


# --- Bataille, siège, naval --------------------------------------------------------


func phase_battle() -> void:
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	var armies: Array = load("res://scripts/battle/battle_scene.gd").main_armies(map.sim, map.player_faction, enemy)
	if armies.is_empty() or not map.sim.has_method("debug_stage_battle"):
		log_q("battle staging unavailable")
		return
	map.sim.call("debug_stage_battle", armies[0], armies[1])
	map.refresh_all()
	map.call("_offer_pending_battles")
	await wait(30)
	await shot("prebattle")
	var dialog: Node = map.get("_battle_dialog")
	await fight(dialog.get("fight_button"), "battle")


## Clique sur « Combattre » et joue la bataille (ou le siège) jusqu'à l'écran de fin.
func fight(button: Button, label: String) -> void:
	var t := Time.get_ticks_msec()
	await click(button)
	var battle: Node = null
	var loading_shot := false
	while battle == null and Time.get_ticks_msec() - t < 90000:
		await wait(1)
		if not loading_shot and Time.get_ticks_msec() - t > 1500:
			loading_shot = true
			await shot("%s-loading" % label)
		for child in root.get_children():
			if child.get("finished_shown") != null:
				battle = child
	if battle == null:
		log_q("%s scene did not open" % label)
		return
	await wait(90)
	log_q("%s scene loaded in %d ms" % [label, Time.get_ticks_msec() - t])
	await shot("%s-deploy" % label)
	await fps_probe("%s-deploy" % label)
	var deployment: Object = battle.get("deployment")
	for attempt in 4:
		var start_button := find_button(battle, "Commencer la bataille")
		if start_button == null:
			start_button = find_button(battle, "Commencer l'assaut")
		if start_button == null or deployment == null or not deployment.get("active"):
			break
		await shot("%s-speech-%d" % [label, attempt])
		await click(start_button)
		await wait(40)
	log_q("%s deployment active after start: %s" % [label, deployment.get("active") if deployment != null else "?"])
	await shot("%s-start" % label)
	for _i in 2:
		await key(KEY_EQUAL)
	var t_fight := Time.get_ticks_msec()
	var next_shot := 15000
	var fight_limit_ms := 110000
	while not battle.get("finished_shown") and Time.get_ticks_msec() - t_fight < fight_limit_ms:
		await wait(10)
		if Time.get_ticks_msec() - t_fight > next_shot:
			await shot("%s-%ds" % [label, next_shot / 1000])
			await fps_probe("%s-%ds" % [label, next_shot / 1000], 60)
			next_shot += 30000
	if not battle.get("finished_shown"):
		var sim_battle: Object = battle.get("battle")
		log_q("%s still running after %d s (clock %.0f s); sounding the general retreat" % [label, (Time.get_ticks_msec() - t_fight) / 1000, float(sim_battle.call("get_elapsed"))])
		await click(find_button(battle, "Retraite générale"))
		await wait(10)
		await shot("%s-retreat-confirm" % label)
		await click(find_button(battle, "Sonner la retraite"))
		var t_retreat := Time.get_ticks_msec()
		while not battle.get("finished_shown") and Time.get_ticks_msec() - t_retreat < 120000:
			await wait(10)
		log_q("%s after retreat: finished %s in %d s" % [label, battle.get("finished_shown"), (Time.get_ticks_msec() - t_retreat) / 1000])
	await wait(60)
	await shot("%s-result" % label)
	var result_screen: Control = battle.get("result_screen")
	var back := find_button(result_screen, "Retour") if result_screen != null else null
	await click(back)
	await wait(90)
	log_q("%s scene still valid after return: %s" % [label, is_instance_valid(battle)])
	await shot("after-%s" % label)
	await dismiss_dialogs()


func phase_siege() -> void:
	if not map.sim.has_method("debug_stage_siege"):
		return
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	var armies: Array = load("res://scripts/battle/battle_scene.gd").main_armies(map.sim, map.player_faction, enemy)
	if armies.is_empty():
		return
	var target := "prov_guyenne" if map.player_faction != "fac_england" else "prov_normandie"
	map.sim.call("debug_stage_siege", armies[0], target)
	map.refresh_all()
	await wait(10)
	var city := province_city_world(target)
	await look_at_world(city, 60.0)
	await wait(40)
	await shot("siege-map")
	# Sélection de l'armée assiégeante et bouton « Donner l'assaut » du bandeau.
	map.select_army(armies[0])
	await wait(20)
	await shot("siege-army-bar")
	var assault: Button = map.sieges.assault_button if map.sieges != null else null
	if assault != null and assault.is_visible_in_tree():
		log_q("siege: '%s' / '%s'" % [map.sieges.status_label.text, assault.text])
		await click(assault)
	else:
		log_q("siege: no assault button, offering pending battles")
		map.call("_offer_pending_battles")
	await wait(30)
	await shot("siege-prebattle")
	var dialog: Node = map.get("_battle_dialog")
	if dialog != null and dialog.visible:
		await fight(dialog.get("fight_button"), "siege")
	else:
		log_q("siege: no pre-battle dialog")
	await dismiss_dialogs()


## Naval (NV1) : une escadre intercepte la traversée (mise en scène `debug_stage_naval`).
func phase_naval() -> void:
	if not map.sim.has_method("debug_stage_naval"):
		log_q("naval: staging unavailable")
		return
	var enemy := "fac_england" if map.player_faction != "fac_england" else "fac_france"
	var armies: Array = load("res://scripts/battle/battle_scene.gd").main_armies(map.sim, map.player_faction, enemy)
	if armies.is_empty():
		return
	var place := "set_calais" if map.player_faction == "fac_england" else "set_portsmouth"
	var index: int = map.sim.call("debug_stage_naval", armies[0], place, enemy)
	log_q("naval: staged index %d (%s)" % [index, place])
	if index < 0:
		return
	map.refresh_all()
	map.call("_offer_pending_battles")
	await wait(30)
	await shot("naval-prebattle")
	var naval: Node = map.get_node_or_null("NavalCampaign")
	var dialog: Control = naval.dialog if naval != null else null
	if dialog == null or not dialog.visible:
		log_q("naval: no pre-battle dialog")
		return
	var t := Time.get_ticks_msec()
	await click(dialog.fight_button)
	var scene: Node = null
	while scene == null and Time.get_ticks_msec() - t < 60000:
		await wait(1)
		for child in root.get_children():
			if child is NavalScene:
				scene = child
	if scene == null:
		log_q("naval: scene did not open")
		return
	await wait(120)
	log_q("naval scene loaded in %d ms" % (Time.get_ticks_msec() - t))
	await shot("naval-start")
	await fps_probe("naval-start")
	await key(KEY_3)
	var t_fight := Time.get_ticks_msec()
	var next_shot := 15000
	while not scene.get("_finished_shown") and Time.get_ticks_msec() - t_fight < 120000:
		await wait(10)
		if Time.get_ticks_msec() - t_fight > next_shot:
			await shot("naval-%ds" % (next_shot / 1000))
			next_shot += 30000
	log_q("naval: finished %s after %d s" % [scene.get("_finished_shown"), (Time.get_ticks_msec() - t_fight) / 1000])
	await wait(30)
	await shot("naval-result")
	var back := find_button(scene, "Retour")
	if back == null:
		back = find_button(scene, "Quitter")
	if back != null:
		await click(back)
	else:
		await key(KEY_ESCAPE)
	await wait(90)
	await shot("after-naval")
	await dismiss_dialogs()


# --- Sauvegarde, chargement, réglages ------------------------------------------------


func phase_save() -> void:
	var date_saved: String = map.sim.call("get_date_label")
	await key(KEY_F5)
	await wait(20)
	await shot("quicksave")
	await dismiss_dialogs()
	await end_turn()
	var date_after: String = map.sim.call("get_date_label")
	await key(KEY_F9)
	await wait(60)
	# F9 peut recharger la scène : la carte change.
	if current_scene != map and current_scene != null and current_scene.has_method("player_army_ids"):
		map = current_scene
	var t := Time.get_ticks_msec()
	while not map.get("load_ok") and Time.get_ticks_msec() - t < 60000:
		await wait(5)
	log_q("quickload: saved %s, played to %s, now %s" % [date_saved, date_after, map.sim.call("get_date_label")])
	await shot("quickload")
	await dismiss_dialogs()
	# Menu pause → « Sauvegarder » / « Charger » (fenêtre des sauvegardes).
	await key(KEY_ESCAPE)
	await wait(20)
	await shot("pause-menu")
	var save_button := find_button(root, "Sauvegarder")
	if save_button != null:
		await click(save_button)
		await wait(20)
		await shot("save-dialog")
		await key(KEY_ESCAPE)
		await wait(10)
	var load_button := find_button(root, "Charger")
	if load_button != null:
		await click(load_button)
		await wait(20)
		await shot("load-dialog")
		await key(KEY_ESCAPE)
		await wait(10)
	await key(KEY_ESCAPE)
	await wait(10)
	await dismiss_dialogs()


func phase_settings() -> void:
	await key(KEY_ESCAPE)
	await wait(15)
	await click(find_button(root, "Réglages"))
	await wait(20)
	var menus := root.find_children("*", "Control", true, false).filter(func(n: Node) -> bool: return n.get_script() == load("res://scripts/ui/settings_menu.gd"))
	if menus.is_empty():
		log_q("settings menu not found")
		return
	var menu: Control = menus[0]
	var tabs: TabContainer = menu.find_children("*", "TabContainer", true, false)[0]
	for tab in tabs.get_tab_count():
		await click_tab(tabs, tab)
		await wait(10)
		await shot("settings-tab-%d-%s" % [tab, tabs.get_tab_title(tab)])
		if tabs.get_tab_title(tab) == "Son":
			# Un curseur déplacé à la souris (clic au quart) puis les cases des voix.
			var sliders := tabs.get_current_tab_control().find_children("*", "HSlider", true, false)
			if not sliders.is_empty():
				var slider: HSlider = sliders[0]
				var before := slider.value
				await click_at(window_point(slider, Vector2(slider.size.x * 0.25, slider.size.y * 0.5)))
				await wait(10)
				log_q("settings: slider '%s' %.2f -> %.2f" % [slider.name, before, slider.value])
				await click_at(window_point(slider, Vector2(slider.size.x * (before / maxf(slider.max_value, 0.001)), slider.size.y * 0.5)))
			var checks := tabs.get_current_tab_control().find_children("*", "CheckBox", true, false)
			for check in checks:
				log_q("settings: sound checkbox '%s' = %s" % [_text_of(check), (check as CheckBox).button_pressed])
			await shot("settings-sound-after")
	await click(menu.find_child("CloseButton", true, false))
	await wait(10)
	await key(KEY_ESCAPE)
	await wait(10)
	await look_at_world(capital_world(), 40.0)
	for quality in ["low", "ultra", "high"]:
		root.get_node("Settings").set_value("video/quality", quality)
		await wait(60)
		await shot("quality-%s" % quality)
		await fps_probe("quality-%s" % quality)


# --- Tours -------------------------------------------------------------------------


func end_turn() -> void:
	var t := Time.get_ticks_msec()
	await key(KEY_ENTER)
	await wait(3)
	log_q("end turn: %d ms (date %s, treasury %s)" % [Time.get_ticks_msec() - t, map.sim.call("get_date_label"), _treasury()])
	await wait(60)
	await dismiss_dialogs()


func play_turns(count: int) -> void:
	for turn in count:
		# Comme un joueur : un clic sur la capitale et un recrutement de temps en temps.
		if turn % 3 == 1:
			var capital := capital_world()
			await look_at_world(capital, 60.0)
			await click_at(world_to_window(capital))
			await wait(20)
			var panel := _open_panel()
			if panel != null and panel.get("recruit_button") != null and (panel.recruit_button as Button).is_visible_in_tree():
				await click(panel.recruit_button)
				await wait(10)
				var button := first_enabled(panel.recruit_list, 0)
				if button != null:
					await click(button)
			await key(KEY_ESCAPE)
			await wait(10)
		await end_turn()
		if turn % 3 == 2:
			await shot("turn-%02d" % (turn + 1))
	await shot("turns-end")


## Ferme les fenêtres modales ouvertes après une fin de tour (rapport, bataille : auto).
func dismiss_dialogs() -> void:
	for _i in 8:
		var naval: Node = map.get_node_or_null("NavalCampaign")
		if naval != null and naval.dialog != null and naval.dialog.visible:
			await shot("naval-prebattle-turn")
			await click(naval.dialog.auto_button if naval.dialog.get("auto_button") != null else find_button(naval.dialog, "auto"))
			await wait(10)
			continue
		var dialog: Node = map.get("_battle_dialog")
		if dialog != null and dialog.visible:
			await shot("prebattle-turn")
			var auto_button := find_button(dialog, "auto")
			if auto_button != null:
				await click(auto_button)
				await wait(10)
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
				if (child as Control).is_visible_in_tree() and not (child as BaseButton).disabled and text not in ["×", "✕", "Plus tard", ""]:
					choice = child
					break
			log_q("chronicle decision: choosing '%s'" % (_text_of(choice) if choice != null else "?"))
			if choice == null:
				await key(KEY_ESCAPE)
			else:
				await click(choice)
			await wait(10)
			continue
		var ok := find_visible_button(root, ["Fermer", "Continuer", "OK", "D'accord", "Poursuivre"])
		if ok == null:
			return
		await shot("dialog")
		await click(ok)
		await wait(8)


# --- Entrées -----------------------------------------------------------------------


func click(control: Control, button := MOUSE_BUTTON_LEFT) -> void:
	if control == null:
		log_q("click on null control")
		return
	var ancestor := control.get_parent()
	while ancestor != null:
		if ancestor is ScrollContainer:
			(ancestor as ScrollContainer).ensure_control_visible(control)
			await wait(3)
			break
		ancestor = ancestor.get_parent()
	var point := window_point(control)
	var viewport_size := Vector2(DisplayServer.window_get_size())
	if point.x < 0 or point.y < 0 or point.x > viewport_size.x or point.y > viewport_size.y:
		log_q("OFFSCREEN: %s (%s) at %s, window %s" % [control.name, _text_of(control), point, viewport_size])
	await move_to(point)
	var hovered: Control = root.gui_get_hovered_control()
	if hovered != null and hovered != control and not control.is_ancestor_of(hovered):
		log_q("OVERLAP: click on %s (%s) at %s hit %s" % [control.name, _text_of(control), point, hovered.get_path()])
	await click_at(point, button)


func click_tab(tabs: TabContainer, index: int) -> void:
	var bar: TabBar = tabs.get_tab_bar()
	var rect := bar.get_tab_rect(index)
	await click_at(root.get_final_transform() * (bar.get_global_transform_with_canvas() * rect.get_center()))
	if tabs.current_tab != index:
		log_q("tab click missed: %d (now %d)" % [index, tabs.current_tab])
		tabs.current_tab = index


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
	if node == null:
		return null
	for child in node.find_children("*", "BaseButton", true, false):
		var button := child as BaseButton
		if not button.disabled and button.is_visible_in_tree() and not button.is_queued_for_deletion():
			if skip == 0:
				return button
			skip -= 1
	return null


func find_button(node: Node, fragment: String) -> BaseButton:
	if node == null:
		return null
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
	if control != null and control.get("text") != null:
		return str(control.get("text"))
	return ""


# --- Mesures et captures -----------------------------------------------------------


func wait(frames: int) -> void:
	for _i in frames:
		await process_frame
		_sample_voice()


func _sample_voice() -> void:
	if _voice_bus < 0:
		return
	var peak := maxf(AudioServer.get_bus_peak_volume_left_db(_voice_bus, 0), AudioServer.get_bus_peak_volume_right_db(_voice_bus, 0))
	if peak > float(voice_peak.get(phase_label, -200.0)):
		voice_peak[phase_label] = peak
	if peak > -50.0:
		voice_frames[phase_label] = int(voice_frames.get(phase_label, 0)) + 1


func shot(label: String) -> void:
	await RenderingServer.frame_post_draw
	shot_index += 1
	var path := out_dir.path_join("%03d-%s.png" % [shot_index, label])
	var image := root.get_texture().get_image()
	image.save_png(path)
	log_q("shot %s" % path)


func fps_probe(label: String, frames := 120) -> void:
	var t := Time.get_ticks_usec()
	await wait(frames)
	var seconds := (Time.get_ticks_usec() - t) / 1000000.0
	log_q("fps %s: %.1f (draw calls %d, primitives %d)" % [label, frames / seconds,
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)])


func log_q(text: String) -> void:
	print("Q3 %s" % text)
