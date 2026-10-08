class_name TutorialController
extends Node

## F8 — tutoriel des premiers tours et encyclopédie sur la carte de campagne. Créé par
## `campaign_map.gd`, qui n'appelle que `setup` (et `stage_screenshot` pour les captures).
##
## Tutoriel : 14 étapes (`TutorialSteps`) affichées par `TutorialOverlay`. Chaque objectif est
## vérifié en lisant l'état de l'interface (panneaux ouverts, onglet, sélection) et de la
## simulation (chemin d'une armée, chantier, recherche, tour, impôt, gouverneur) ; l'étape
## suivante s'ouvre dès qu'il est rempli. Progression persistée dans `Settings`
## (`tutorial/step`, `tutorial/done`) ; désactivable (`tutorial/enabled`, bouton « Passer le
## tutoriel ») ; ne démarre qu'en début de partie et jamais pendant les captures.
##
## Encyclopédie : touche L ou Menu → Encyclopédie. Menu → Tutoriel relance le guide.
##
## Lot UX2 (U15) : « Plus tard » range le guide (`postpone`, réglage `tutorial/postponed`) ;
## il reprend à la même étape (`resume`) par le conseil « que faire maintenant », l'aide (F1)
## ou Menu → Tutoriel. Le sommaire du parchemin mène à n'importe quelle étape (`jump_to`).
## Aucune règle de jeu : lecture de l'état seulement.

const TUTORIAL_SCENE := "res://scenes/ui/tutorial.tscn"
const ENCYCLOPEDIA_SCENE := "res://scenes/ui/encyclopedia.tscn"
const MENU_ENCYCLOPEDIA_ID := 920
const MENU_TUTORIAL_ID := 921
const CHECK_INTERVAL := 0.25
const DONE_PAUSE := 0.8
## Tours après lesquels le tutoriel ne démarre plus de lui-même (partie déjà avancée).
const EARLY_TURNS := 2

var map: Node = null  # CampaignMap
var settings: Node = null
var overlay: TutorialOverlay = null
var encyclopedia: Encyclopedia = null
var steps: Array[Dictionary] = []
var step_index: int = -1
var active := false
var royal_army: String = ""
## Instantané pris à l'entrée de chaque étape (tour, chemins, chantiers, impôt…).
var snapshot: Dictionary = {}
var _check_timer := 0.0
var _done_timer := -1.0
## Faux en capture : la progression du joueur n'est pas modifiée.
var persist_progress := true
## UX2 : étape où le guide a été rangé par « Plus tard » (-1 : aucun guide en attente).
var _postponed_step := -1


func setup(campaign_map: Node) -> void:
	map = campaign_map
	settings = get_node_or_null("/root/Settings")
	persist_progress = not capture_mode()
	# Q8 : le menu pause met l'arbre en pause ; le contrôleur doit tourner pour ranger le guide.
	process_mode = Node.PROCESS_MODE_ALWAYS
	var ui: Node = map.get("ui")
	encyclopedia = (load(ENCYCLOPEDIA_SCENE) as PackedScene).instantiate()
	encyclopedia.hide()
	ui.add_child(encyclopedia)
	overlay = (load(TUTORIAL_SCENE) as PackedScene).instantiate()
	overlay.hide()
	PanelStack.set_tier(overlay, PanelStack.Tier.TUTORIAL)  # Q4 : au-dessus de tout le reste
	ui.add_child(overlay)
	overlay.continue_pressed.connect(advance)
	overlay.skip_step_pressed.connect(advance)
	overlay.skip_all_pressed.connect(skip_all)
	overlay.later_pressed.connect(postpone)
	overlay.step_chosen.connect(jump_to)
	if bool(_setting("tutorial/postponed", false)) and not bool(_setting("tutorial/done", false)):
		_postponed_step = int(_setting("tutorial/step", 0))
	var menu_button: MenuButton = ui.get("menu_button")
	if menu_button != null:
		var popup := menu_button.get_popup()
		popup.add_item("Encyclopédie (L)", MENU_ENCYCLOPEDIA_ID)
		popup.add_item("Tutoriel", MENU_TUTORIAL_ID)
		popup.id_pressed.connect(func(id: int) -> void:
			if id == MENU_ENCYCLOPEDIA_ID:
				encyclopedia.open_window()
			elif id == MENU_TUTORIAL_ID:
				reopen())
	if should_autostart():
		start(int(_setting("tutorial/step", 0)))


func _setting(key: String, fallback: Variant) -> Variant:
	return settings.call("get_value", key) if settings != null else fallback


func _set_setting(key: String, value: Variant) -> void:
	if settings != null and persist_progress:
		settings.call("set_value", key, value)


## Vrai en capture d'écran (`--screenshot`, `--flow-stage`…) : le guide ne s'y affiche pas,
## sauf mise en scène `--stage=tutorial`.
static func capture_mode() -> bool:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot=") or arg.begins_with("--flow-stage=") or arg.begins_with("--loading-shot="):
			return true
	return false


func should_autostart() -> bool:
	var sim: Object = map.get("sim")
	if sim == null or capture_mode() or not bool(_setting("tutorial/enabled", true)) or bool(_setting("tutorial/done", false)):
		return false
	if _postponed_step >= 0:  # UX2 : rangé par « Plus tard », repris à la demande
		return false
	return int(sim.call("get_turn")) <= EARLY_TURNS or int(_setting("tutorial/step", 0)) > 0


# --- Déroulé --------------------------------------------------------------------------


## Contexte des textes pour `faction_id` (faction du joueur par défaut).
func _context(faction_id: String = _player()) -> Dictionary:
	var faction: Dictionary = GameCatalog.definitions("factions").get(faction_id, {})
	var context := {}
	var name: Variant = faction.get("name", {})
	if name is Dictionary:
		context["faction"] = str(name.get("display", faction_id))
	# Fiche féodale de la simulation : dirigeant des factions sans `ruler` dans les données,
	# suzerain, et objectifs de titre des factions sans bloc `victory`.
	var sim := _sim()
	var sheet: Dictionary = sim.call("get_feudal_sheet", faction_id) if sim != null and sim.has_method("get_feudal_sheet") else {}
	if str(faction.get("ruler", "")) != "":
		context["ruler"] = Encyclopedia._character_name(str(faction["ruler"]))
	elif str(sheet.get("ruler", "")) != "":
		context["ruler"] = str(sheet["ruler"])
	context["liege"] = str(sheet.get("liege_name", ""))
	context["description"] = str(faction.get("description", ""))
	if str(faction.get("capital_city", "")) != "":
		context["capital"] = str(faction["capital_city"])
	var victory: Dictionary = faction.get("victory", {})
	var lines := PackedStringArray()
	for objective in victory.get("objectives", []):
		lines.append("• %s" % str(objective.get("title", "")))
	if lines.is_empty():
		for objective in sheet.get("objectives", []):
			lines.append("• %s" % str(objective.get("title", "")))
		if not lines.is_empty():
			context["objectives_title"] = "Les objectifs de votre titre"
	if not lines.is_empty():
		context["objectives"] = "\n".join(lines)
	if victory.has("end_year"):
		context["end_year"] = str(int(victory["end_year"]))
	return context


func start(from_step: int = 0) -> void:
	if map.get("sim") == null:
		return
	steps = TutorialSteps.steps(str(map.get("player_faction")), _context())
	royal_army = find_royal_army()
	active = true
	_clear_postponed()
	var titles := PackedStringArray()
	for step in steps:
		titles.append(str(step.get("title", "")))
	overlay.set_steps(titles)
	_enter_step(clampi(from_step, 0, steps.size() - 1))


## Menu → Tutoriel : relance depuis le début (même si le guide avait été passé).
func restart() -> void:
	_set_setting("tutorial/done", false)
	_set_setting("tutorial/enabled", true)
	start(0)


## UX2 : « Plus tard » — le guide se range et garde son étape (reprise par `resume`).
func postpone(notify: bool = true) -> void:
	if not active:
		return
	var index := step_index
	active = false
	step_index = -1
	overlay.hide()
	overlay.set_target({})
	_postponed_step = index
	_set_setting("tutorial/step", index)
	_set_setting("tutorial/postponed", true)
	if notify:
		var ui: Node = map.get("ui")
		ui.call("show_toast", "Guide mis de côté à l'étape %d : reprenez-le par le conseil en haut à gauche, l'aide (F1) ou Menu → Tutoriel." % (index + 1))


## UX2 : reprend le guide à l'étape où « Plus tard » l'avait laissé (sinon au début).
func resume() -> void:
	if active:
		return
	var index := maxi(_postponed_step, 0)
	_set_setting("tutorial/done", false)
	_set_setting("tutorial/enabled", true)
	start(index)


## UX2 : Menu → Tutoriel et bouton de l'aide : reprise du guide rangé, sinon depuis le début.
func reopen() -> void:
	if active:
		return
	if _postponed_step >= 0:
		resume()
	else:
		restart()


## UX2 : étape du guide rangé par « Plus tard », -1 sinon (conseil « Reprendre le guide »).
func postponed_step() -> int:
	return -1 if active else _postponed_step


## UX2 : sommaire — aller directement à l'étape `index`.
func jump_to(index: int) -> void:
	if not active or index < 0 or index >= steps.size():
		return
	_enter_step(index)


func _clear_postponed() -> void:
	if _postponed_step >= 0 or bool(_setting("tutorial/postponed", false)):
		_set_setting("tutorial/postponed", false)
	_postponed_step = -1


func current_step_id() -> String:
	return str(steps[step_index]["id"]) if active and step_index >= 0 and step_index < steps.size() else ""


func _enter_step(index: int) -> void:
	step_index = index
	_done_timer = -1.0
	_set_setting("tutorial/step", index)
	snapshot = _take_snapshot()
	overlay.show_step(steps[index], index, steps.size())
	_focus_for_step(current_step_id())


## Étape suivante (objectif rempli, « Continuer » ou « Passer l'étape ») ; fin après la dernière.
func advance() -> void:
	if not active:
		return
	if step_index + 1 >= steps.size():
		finish()
		return
	_enter_step(step_index + 1)


func finish() -> void:
	active = false
	step_index = -1
	overlay.hide()
	overlay.set_target({})
	_set_setting("tutorial/done", true)
	_set_setting("tutorial/step", 0)
	_clear_postponed()


## « Passer le tutoriel » : fermé pour de bon (relançable par Menu → Tutoriel).
func skip_all() -> void:
	finish()
	var ui: Node = map.get("ui")
	ui.call("show_toast", "Tutoriel passé : Menu → Tutoriel ou l'aide (F1) pour le relancer.")


## Vérifie l'objectif tout de suite ; passe à l'étape suivante s'il est rempli. Sert au smoke.
func check_now() -> bool:
	if not active or bool(steps[step_index].get("manual", false)):
		return false
	if objective_met(current_step_id()):
		advance()
		return true
	return false


func _process(delta: float) -> void:
	if not active or not map.get("visible"):
		return
	# Q2 : le guide s'efface sous le menu pause, les réglages et la décision de chronique.
	var covered := modal_open()
	if overlay.visible == covered:
		overlay.visible = not covered
	if covered:
		return
	overlay.set_target(resolve_target(str(steps[step_index].get("target", ""))))
	var avoid: Array = []
	# UX2 : aussi le bandeau d'ost, la cloche, le sceau et le journal (le parchemin ne les
	# couvre que faute de place ailleurs).
	for panel in [_ui_node("province_panel") as Control, _settlement_panel(), _ui_node("army_strip") as Control,
			_ui_node("end_turn_cluster") as Control, _ui_node("general_seal") as Control, _ui_node("event_log") as Control,
			_ui_node("tech_panel") as Control, _ui_node("court_panel") as Control, _ui_node("faction_panel") as Control]:
		if panel != null and panel.is_visible_in_tree():
			avoid.append(panel.get_global_rect())
	overlay.set_avoid(avoid)
	if _done_timer >= 0.0:
		_done_timer -= delta
		if _done_timer < 0.0:
			advance()
		return
	_check_timer -= delta
	if _check_timer > 0.0:
		return
	_check_timer = CHECK_INTERVAL
	if not bool(steps[step_index].get("manual", false)) and objective_met(current_step_id()):
		overlay.mark_done()
		_done_timer = DONE_PAUSE


func _unhandled_input(event: InputEvent) -> void:
	if not (event is InputEventKey) or not event.pressed or event.echo or not map.get("visible"):
		return
	var key := event as InputEventKey
	if get_tree().paused:
		return
	if key.is_action_pressed("encyclopedia_open") and not key.ctrl_pressed and not key.meta_pressed:  # U7
		encyclopedia.toggle()
		get_viewport().set_input_as_handled()


## Q2 : vrai quand une fenêtre modale couvre la carte (menu pause, réglages, fenêtre de
## chronique ou rapport de saison hors de l'étape qui les demande) : le guide ne s'affiche pas
## par-dessus.
func modal_open() -> bool:
	var flow: Node = map.get("flow")
	if flow != null:
		if bool(flow.call("is_paused")):
			return true
		var settings_menu: Variant = flow.get("_settings_menu")
		if settings_menu != null and is_instance_valid(settings_menu):
			return true
		var report := flow.get("season_report") as Control
		if report != null and report.visible and current_step_id() != "season_report":
			return true
	var chronicle: Node = map.get("chronicle")
	if chronicle != null and current_step_id() != "chronicle":
		var window := chronicle.get("window") as Control
		if window != null and window.is_visible_in_tree():
			return true
	return false


# --- Objectifs ------------------------------------------------------------------------


func _sim() -> Object:
	return map.get("sim")


func _player() -> String:
	return str(map.get("player_faction"))


func _ui_node(node_name: String) -> Node:
	var ui: Node = map.get("ui")
	return ui.get(node_name) if ui != null else null


## Contrôle exposé par un accesseur stable de `MapUI` (`end_turn_control()`… du nouveau HUD)
## s'il existe, sinon le nœud actuel `fallback_name`.
func _ui_control(accessor: String, fallback_name: String) -> Control:
	var ui: Node = map.get("ui")
	if ui == null:
		return null
	if ui.has_method(accessor):
		var control := ui.call(accessor) as Control
		if control != null:
			return control
	return ui.get(fallback_name) as Control


## Armée principale : celle que commande le souverain, sinon la plus nombreuse.
func find_royal_army() -> String:
	var sim := _sim()
	var ruler := str(GameCatalog.definitions("factions").get(_player(), {}).get("ruler", ""))
	var best := ""
	var best_units := -1
	for army_id in map.call("player_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if ruler != "" and str(army.get("general", "")) == ruler:
			return str(army_id)
		var units := (army.get("units", []) as Array).size()
		if units > best_units:
			best_units = units
			best = str(army_id)
	return best


func _player_provinces() -> PackedStringArray:
	var result := PackedStringArray()
	var sim := _sim()
	var map_data: MapData = map.get("map_data")
	for index in range(1, map_data.province_count + 1):
		var province_id := str(map_data.get_province(index).get("id", ""))
		var state: Dictionary = sim.call("get_province_state", province_id)
		if str(state.get("owner", "")) == _player():
			result.append(province_id)
	return result


func _constructions() -> int:
	var sim := _sim()
	if not sim.has_method("get_province_city"):
		return 0
	var count := 0
	var settlements := sim.has_method("province_settlements")
	for province_id in _player_provinces():
		var city: Dictionary = sim.call("get_province_city", province_id)
		if not (city.get("construction", {}) as Dictionary).is_empty():
			count += 1
		if not settlements:
			continue
		# Q2 (C5) : chantiers lancés depuis le panneau d'une colonie.
		for settlement_id in sim.call("province_settlements", province_id):
			var detail: Dictionary = sim.call("settlement_detail", settlement_id)
			if str(detail.get("owner", "")) == _player() and not (detail.get("construction", {}) as Dictionary).is_empty():
				count += 1
	return count


## Q2 (C5) : panneau de colonie ouvert (clic sur une ville), sinon null.
func _settlement_panel() -> Control:
	var controller: Node = map.get("settlements_ctl")
	var panel: Control = controller.get("panel") if controller != null else null
	return panel if panel != null and panel.is_visible_in_tree() else null


func _governors() -> Dictionary:
	var sim := _sim()
	var result := {}
	if not sim.has_method("get_faction_characters"):
		return result
	for character_id in sim.call("get_faction_characters", _player()):
		result[str(character_id)] = str((sim.call("get_character", character_id) as Dictionary).get("governor_of", ""))
	return result


func _army_orders() -> Dictionary:
	var sim := _sim()
	var result := {}
	for army_id in map.call("player_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		result[str(army_id)] = _army_place(army)
	return result


## Lieu d'une armée pour le tutoriel : colonie, et position libre (lot M4 : une marche courte
## peut laisser l'armée près de la même colonie).
static func _army_place(army: Dictionary) -> String:
	return "%s@%s" % [army.get("location", ""), army.get("position", "")]


func _tax_rate() -> String:
	return str((_sim().call("get_faction_summary", _player()) as Dictionary).get("tax_rate", ""))


func _research_id() -> String:
	var sim := _sim()
	if not sim.has_method("get_research"):
		return ""
	return str((sim.call("get_research", _player()) as Dictionary).get("technology", ""))


func _take_snapshot() -> Dictionary:
	var sim := _sim()
	var result := {"turn": int(sim.call("get_turn"))}
	match current_step_id():
		"move_army":
			result["locations"] = _army_orders()
		"build":
			result["constructions"] = _constructions()
		"research":
			result["research"] = _research_id()
		"tax":
			result["tax"] = _tax_rate()
		"governor":
			result["governors"] = _governors()
	return result


func _visible(node_name: String) -> bool:
	var node := _ui_node(node_name) as Control
	return node != null and node.is_visible_in_tree()


## Objectif de l'étape `step_id` rempli d'après l'état actuel (et l'instantané d'entrée).
func objective_met(step_id: String) -> bool:
	var sim := _sim()
	if sim == null:
		return false
	match step_id:
		"select_army":
			var selected := str(map.get("selected_army"))
			return selected != "" and selected in map.call("player_army_ids")
		"move_army":
			var before: Dictionary = snapshot.get("locations", {})
			for army_id in map.call("player_army_ids"):
				var army: Dictionary = sim.call("get_army", army_id)
				if not (army.get("path", []) as Array).is_empty():
					return true
				if before.has(army_id) and str(before[army_id]) != _army_place(army):
					return true
			return false
		"open_province":
			# Q2 : un clic sur la ville ouvre le panneau de la colonie (C5), qui compte aussi.
			var settlement_panel := _settlement_panel()
			if settlement_panel != null and bool(settlement_panel.get("is_player_owner")):
				return true
			var panel := _ui_node("province_panel")
			if panel == null or not (panel as Control).is_visible_in_tree():
				return false
			var state: Dictionary = sim.call("get_province_state", str(panel.get("province_id")))
			return str(state.get("owner", "")) == _player()
		"city_tab":
			var settlement_panel := _settlement_panel()
			if settlement_panel != null:
				var settlement_tabs: TabContainer = settlement_panel.get("tabs")
				if settlement_tabs != null and settlement_tabs.current_tab == SettlementPanel.TAB_BUILDINGS:
					return true
			var panel := _ui_node("province_panel")
			if panel == null or not (panel as Control).is_visible_in_tree():
				return false
			var tabs: TabContainer = panel.get("tabs")
			return tabs != null and tabs.current_tab == 1
		"build":
			if not sim.has_method("get_province_city"):
				return true
			return _constructions() > int(snapshot.get("constructions", 0))
		"research":
			if not sim.has_method("get_research"):
				return true
			var research := _research_id()
			return research != "" and research != str(snapshot.get("research", ""))
		"diplomacy":
			var diplomacy: Node = map.get("diplomacy")
			return diplomacy != null and (diplomacy.get("panel") as Control).visible
		"end_turn":
			return int(sim.call("get_turn")) > int(snapshot.get("turn", 0))
		"season_report":
			var flow: Node = map.get("flow")
			var report: Control = flow.get("season_report") if flow != null else null
			return report == null or not report.visible  # lu puis fermé, ou aucun rapport à lire
		"chronicle":
			var chronicle: Node = map.get("chronicle")
			return chronicle != null and (chronicle.get("window") as Control).visible
		"tax":
			return _tax_rate() != str(snapshot.get("tax", ""))
		"governor":
			var before: Dictionary = snapshot.get("governors", {})
			var now := _governors()
			for character_id in now:
				if str(now[character_id]) != "" and str(now[character_id]) != str(before.get(character_id, "")):
					return true
			return false
	return false


# --- Cibles (flèche et halo) ----------------------------------------------------------


func _control_rect(control: Control) -> Dictionary:
	if control == null or not control.is_visible_in_tree():
		return {}
	var rect := control.get_global_rect()
	if rect.size.x < 2.0 or rect.size.y < 2.0:
		return {}
	return {"rect": rect}


func _world_point(world: Vector3) -> Dictionary:
	var camera: Camera3D = map.get("camera")
	if camera == null or camera.is_position_behind(world):
		return {}
	var point := camera.unproject_position(world)
	var viewport_size := map.get_viewport().get_visible_rect().size
	if point.x < 0.0 or point.y < 0.0 or point.x > viewport_size.x or point.y > viewport_size.y:
		return {}
	return {"point": point}


func _province_point(province_id: String) -> Dictionary:
	var map_data: MapData = map.get("map_data")
	if map_data == null or map_data.index_of_id(province_id) <= 0:
		return {}
	var centroid := map_data.centroid_of_id(province_id)
	return _world_point(Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y))


## Q2 : la ville capitale (maquette ou icône de colonie), sinon le centre de la province.
func _capital_point() -> Dictionary:
	var sim := _sim()
	var layer: Node = map.get("settlement_layer")
	var city := str((sim.call("get_province_state", _capital()) as Dictionary).get("city", "")) if sim != null else ""
	if city != "" and layer != null:
		var world: Vector3 = layer.call("world_position_of", city)
		if world != Vector3.ZERO:
			return _world_point(world)
	return _province_point(_capital())


func _capital() -> String:
	return str(GameCatalog.definitions("factions").get(_player(), {}).get("capital", ""))


## Cible de la flèche selon l'étape et l'interface ouverte : `{rect}`, `{point}` ou vide.
func resolve_target(target: String) -> Dictionary:
	match target:
		"royal_army":
			# Armée déjà sélectionnée à l'étape « marche » : le chemin se donne sur la carte,
			# la flèche reste sur le marqueur ; le widget d'armée du HUD sert de repli.
			var armies: ArmyMarkers = map.get("armies")
			var army_id := royal_army
			if armies == null or not armies.has_army(army_id):
				var ids: PackedStringArray = map.call("player_army_ids")
				army_id = ids[0] if not ids.is_empty() else ""
			if army_id == "" or armies == null or not armies.has_army(army_id):
				return {}
			var marker := _world_point(armies.world_position_of(army_id))
			if marker.is_empty() and str(map.get("selected_army")) != "":
				return _control_rect(_ui_control("selected_army_widget", "army_panel"))
			return marker
		"capital":
			return _capital_point()
		"city_tab", "buildable":
			var settlement_panel := _settlement_panel()
			if settlement_panel != null:  # Q2 (C5) : panneau de la ville cliquée
				var settlement_tabs: TabContainer = settlement_panel.get("tabs")
				if target == "buildable" and settlement_tabs != null and settlement_tabs.current_tab == SettlementPanel.TAB_BUILDINGS:
					var list := _control_rect(settlement_panel.get("buildable_list") as Control)
					if not list.is_empty():
						return list
				if settlement_tabs != null and settlement_tabs.get_tab_count() > SettlementPanel.TAB_BUILDINGS:
					var settlement_bar := settlement_tabs.get_tab_bar()
					var settlement_rect := settlement_bar.get_tab_rect(SettlementPanel.TAB_BUILDINGS)
					settlement_rect.position += settlement_bar.get_global_rect().position
					return {"rect": settlement_rect}
				return _control_rect(settlement_panel)
			var panel := _ui_node("province_panel") as Control
			if panel == null or not panel.is_visible_in_tree():
				return _capital_point()
			var tabs: TabContainer = panel.get("tabs")
			if target == "buildable" and tabs != null and tabs.current_tab == 1:
				var buildable: Control = panel.get("buildable_list")
				var found := _control_rect(buildable)
				if not found.is_empty():
					return found
			if tabs != null and tabs.get_tab_count() > 1:
				var bar := tabs.get_tab_bar()
				var tab_rect := bar.get_tab_rect(1)
				tab_rect.position += bar.get_global_rect().position
				return {"rect": tab_rect}
			return _control_rect(panel)
		"research":
			if _visible("tech_panel"):
				return _control_rect(_ui_node("tech_panel") as Control)
			var box := _control_rect(_ui_node("research_box") as Control)
			return box if not box.is_empty() else _control_rect(_ui_node("tech_button") as Control)
		"diplomacy":
			var diplomacy: Node = map.get("diplomacy")
			return _control_rect(diplomacy.get("button") as Control) if diplomacy != null else {}
		"end_turn":
			return _control_rect(_ui_control("end_turn_control", "end_turn_button"))
		"season_report":
			var flow: Node = map.get("flow")
			return _control_rect(flow.get("season_report") as Control) if flow != null else {}
		"chronicle":
			var chronicle: Node = map.get("chronicle")
			if chronicle == null:
				return {}
			return _control_rect(chronicle.get("button") as Control)
		"tax":
			var panel := _ui_node("faction_panel") as Control
			if panel != null and panel.is_visible_in_tree():
				var tax_button: Control = panel.get("tax_normal")
				if tax_button != null:
					return _control_rect(tax_button.get_parent() as Control)
				return _control_rect(panel)
			return _control_rect(_ui_node("faction_swatch") as Control)
		"governor":
			var sheet := _ui_node("character_sheet") as Control
			if sheet != null and sheet.is_visible_in_tree():
				var picker: Control = sheet.get("picker_panel")
				if picker != null and picker.is_visible_in_tree():
					return _control_rect(picker)
				return _control_rect(sheet.get("governor_button") as Control)
			if _visible("court_panel"):
				return _control_rect(_ui_node("court_panel") as Control)
			return _control_rect(_ui_node("court_button") as Control)
	return {}


## Caméra sur l'armée principale ou la capitale au début des étapes concernées.
func _focus_for_step(step_id: String) -> void:
	var rig: CampaignCamera = map.get("camera_rig")
	var map_data: MapData = map.get("map_data")
	if rig == null or map_data == null:
		return
	var extent := maxf(map_data.size.x, map_data.size.y)
	match step_id:
		"select_army":
			var armies: ArmyMarkers = map.get("armies")
			if armies != null and armies.has_army(royal_army):
				rig.look_at_point(armies.world_position_of(royal_army), extent * 0.12)
		"open_province":
			var capital := _capital()
			if map_data.index_of_id(capital) > 0:
				var centroid := map_data.centroid_of_id(capital)
				rig.look_at_point(Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y), extent * 0.12)
