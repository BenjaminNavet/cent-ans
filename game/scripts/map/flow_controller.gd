class_name FlowController
extends Node

## F3 — « enveloppe » de la carte de campagne : menu pause (Échap), réglages appliqués à la
## carte (caméra, pan par bords, batailles interactives), sauvegardes avec fiche et vignette,
## sauvegarde automatique tournante, confirmation de fin de tour et rapport de saison (les
## alertes persistantes passent par la cloche du HUD, `HudController`, F10b).
## `campaign_map.gd` n'appelle que `setup`, `refresh`, `before_end_turn` et `after_end_turn` ; tout le reste passe par les signaux de `MapUI`.
## Aucune règle de jeu : lecture de l'état et envoi des demandes existantes.
##
## Captures (après `--`) : `--flow-stage=pause|save|settings|report|alerts|confirm`
## `--flow-shot=<chemin.png>` met l'écran en scène, capture puis quitte (la pause arrête
## `CampaignMap._process`, d'où une capture propre au contrôleur).

const START_MENU_SCENE := "res://scenes/start_menu.tscn"
const PAUSE_MENU_SCENE := "res://scenes/ui/pause_menu.tscn"
const SEASON_REPORT_SCENE := "res://scenes/ui/season_report.tscn"
const MENU_SETTINGS_ID := 910
const MENU_PAUSE_ID := 911
## Q2 : « Son… » ouvre l'onglet Son des réglages (une seule fenêtre de volumes).
const MENU_SOUND_ID := 930
const BASE_PAN_SPEED := 1.2

var map: Node = null  # CampaignMap
var settings: Node = null
var pause_menu: PauseMenu = null
var season_report: SeasonReport = null
## Panneaux déjà ouverts quand le rapport de saison s'est affiché (Q5).
var _panels_under_report: Array[Control] = []
var last_events: Array = []
var last_autosave: String = ""
var _last_saved_turn: int = 0
var _pause_snapshot: Image = null
var _end_turn_confirmed := false
var _confirm_panel: PanelContainer = null
var _confirm_label: Label = null
var _confirm_yes: Button = null
var _settings_menu: SettingsMenu = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	settings = get_node_or_null("/root/Settings")
	var ui: Node = map.get("ui")
	season_report = (load(SEASON_REPORT_SCENE) as PackedScene).instantiate()
	season_report.name = "SeasonReport"
	ui.add_child(season_report)
	# Q6 : juste au-dessus de la zone `MODAL` (diplomatie, Cour, techniques…) et de son voile : la
	# fin de tour ouvre la diplomatie sur une offre nouvelle dans l'image du rapport, qui doit
	# rester cliquable par-dessus (même étage `MODAL`, `restack` garde cet ordre). L'ancien
	# placement sous la fenêtre de chronique prenait son index dans la zone latérale où elle vit
	# désormais, et mettait le rapport sous le voile et la diplomatie. Les dialogues ajoutés plus
	# tard (bataille, fin de partie) restent au-dessus.
	var modal_zone: Control = UiZones.layout().zone_node(UiZones.Zone.MODAL)
	if modal_zone != null and modal_zone.get_parent() == ui:
		ui.move_child(season_report, modal_zone.get_index() + 1)
	season_report.entry_selected.connect(_on_report_entry)
	# Q5 : le rapport cède la place à tout panneau que le joueur ouvre après lui (il restait
	# par-dessus la fiche de colonie et la diplomatie, recrutement caché dessous).
	season_report.visibility_changed.connect(func() -> void:
		if season_report.visible:
			_panels_under_report = ui.panels.visible_panels())
	ui.panels.changed.connect(_on_panels_changed.bind(ui))
	season_report.disable_requested.connect(func() -> void:
		if settings != null:
			settings.call("set_value", "interface/season_report", false))
	ui.end_turn_gate = end_turn_would_proceed  # U5 : bandeau des autres factions
	ui.save_requested.connect(_on_save_requested)
	ui.load_requested.connect(_on_load_requested)
	var popup: PopupMenu = ui.menu_button.get_popup()
	popup.add_item("Réglages…", MENU_SETTINGS_ID)
	popup.add_item("Son…", MENU_SOUND_ID)
	popup.add_item("Menu pause (Échap)", MENU_PAUSE_ID)
	popup.id_pressed.connect(func(id: int) -> void:
		if id == MENU_SETTINGS_ID:
			open_settings()
		elif id == MENU_SOUND_ID:
			open_settings("Son")
		elif id == MENU_PAUSE_ID:
			open_pause())
	if settings != null:
		settings.changed.connect(_on_setting_changed)
	_last_saved_turn = _turn()
	apply_settings()
	_run_stage.call_deferred()


func _setting(key: String, fallback: Variant) -> Variant:
	return settings.call("get_value", key) if settings != null else fallback


func _turn() -> int:
	var sim: Object = map.get("sim")
	return int(sim.call("get_turn")) if sim != null else 0


func unsaved_turns() -> int:
	return maxi(0, _turn() - _last_saved_turn)


# --- Réglages ---------------------------------------------------------------------


func apply_settings() -> void:
	var rig: CampaignCamera = map.get("camera_rig")
	if rig != null:
		rig.edge_pan_enabled = bool(_setting("camera/edge_pan", true))
		rig.pan_speed = BASE_PAN_SPEED * float(_setting("camera/speed", 1.0))
	var sim: Object = map.get("sim")
	if sim != null and sim.has_method("set_interactive_battles"):
		sim.call("set_interactive_battles", bool(_setting("game/interactive_battles", true)))


func _on_setting_changed(key: String) -> void:
	if key.begins_with("camera/") or key == "game/interactive_battles":
		apply_settings()


## `tab` : onglet ouvert d'emblée (« Son » depuis Menu → Son…).
func open_settings(tab: String = "") -> void:
	if _settings_menu != null and is_instance_valid(_settings_menu):
		return
	_settings_menu = SettingsMenu.new()
	_settings_menu.initial_tab = tab
	_settings_menu.closed.connect(func() -> void: _settings_menu = null)
	map.get("ui").add_child(_settings_menu)


# --- Menu pause -------------------------------------------------------------------


func is_paused() -> bool:
	return pause_menu != null and is_instance_valid(pause_menu)


func open_pause() -> void:
	if is_paused() or not map.get("visible"):
		return
	_pause_snapshot = _capture_now()
	pause_menu = (load(PAUSE_MENU_SCENE) as PackedScene).instantiate()
	pause_menu.name = "PauseMenu"
	pause_menu.unsaved_turns = unsaved_turns()
	pause_menu.default_save_name = _default_save_name()
	var ui: Node = map.get("ui")
	ui.add_child(pause_menu)
	pause_menu.resume_requested.connect(close_pause)
	pause_menu.save_requested.connect(func(save_name: String) -> void:
		ui.save_requested.emit(save_name)
		pause_menu.unsaved_turns = unsaved_turns())
	pause_menu.load_requested.connect(func(path: String) -> void:
		close_pause()
		ui.load_requested.emit(path))
	pause_menu.help_requested.connect(func() -> void:
		close_pause()
		var help: Node = map.get("help")
		if help != null:
			help.call("toggle"))
	pause_menu.main_menu_requested.connect(go_to_main_menu)
	pause_menu.quit_requested.connect(func() -> void:
		get_tree().paused = false
		get_tree().quit())
	get_tree().paused = true


func close_pause() -> void:
	if is_paused():
		pause_menu.queue_free()
	pause_menu = null
	_pause_snapshot = null
	get_tree().paused = false


## Menu principal / quitter depuis la barre du haut : même confirmation que le menu pause
## (partie non sauvegardée).
func request_exit(kind: String) -> void:
	open_pause()
	pause_menu._request_exit(kind)


func toggle_pause() -> void:
	if is_paused():
		close_pause()
	else:
		open_pause()


func go_to_main_menu() -> void:
	get_tree().paused = false
	SceneFader.go(START_MENU_SCENE)


func _default_save_name() -> String:
	var sim: Object = map.get("sim")
	if sim == null:
		return "partie"
	var facade := get_node_or_null("/root/SimFacade")
	var faction := str(facade.call("faction_short_name", map.get("player_faction"))) if facade != null else "partie"
	# Audit A3 T4 : nom lisible (« France, printemps 1337 ») plutôt qu'un identifiant à tirets bas.
	return "%s, %s" % [faction, str(sim.call("get_date_label")).to_lower()]


## Échap : ferme ce qui est ouvert (réglages, rapport), laisse la carte désélectionner
## l'armée, sinon ouvre ou ferme le menu pause. La pause gère elle-même Échap.
func _unhandled_input(event: InputEvent) -> void:
	if not (event.is_action_pressed("campaign_pause") or event.is_action_pressed("ui_cancel")) or not map.get("visible"):
		return
	if _settings_menu != null and is_instance_valid(_settings_menu):
		return  # la fenêtre de réglages se ferme elle-même
	if _confirm_panel != null and _confirm_panel.visible:
		_confirm_panel.hide()
	elif season_report != null and season_report.visible:
		season_report.close()
	elif str(map.get("selected_army")) != "":
		return  # campaign_map désélectionne
	else:
		open_pause()
	get_viewport().set_input_as_handled()


# --- Sauvegardes ------------------------------------------------------------------


func _capture_now() -> Image:
	if DisplayServer.get_name() == "headless":
		return null
	var texture := get_viewport().get_texture()
	return texture.get_image() if texture != null else null


## Après `CampaignMap._on_save` (connecté avant) : fiche et vignette de l'emplacement.
## La fiche est écrite par `SaveSlots.save` ; rien si l'écriture a échoué (ancienne fiche intacte).
func _on_save_requested(save_name: String) -> void:
	if not bool(map.get("last_save_ok")):
		return
	_last_saved_turn = _turn()
	if _pause_snapshot != null:
		SaveSlots.write_thumbnail(save_name, _pause_snapshot)
	elif DisplayServer.get_name() != "headless":
		# La boîte de dialogue se ferme juste après : vignette sur l'image suivante.
		await RenderingServer.frame_post_draw
		await RenderingServer.frame_post_draw
		SaveSlots.write_thumbnail(save_name, _capture_now())


func _on_load_requested(_path: String) -> void:
	_last_saved_turn = _turn()
	last_events = []
	if season_report != null:
		season_report.hide()
	apply_settings()
	refresh()


func autosave() -> String:
	var interval := int(_setting("game/autosave_interval", 4))
	var saved := SaveSlots.autosave(_turn(), interval, _capture_now())
	if saved != "":
		last_autosave = saved
		_last_saved_turn = _turn()
	return saved


# --- Fin de tour ------------------------------------------------------------------


## Lot U5 : vrai si `before_end_turn` laissera passer la fin de tour (sans effet de bord).
func end_turn_would_proceed() -> bool:
	return not is_paused() and (_end_turn_confirmed or not bool(_setting("interface/confirm_end_turn", false)))


## Vrai si la fin de tour peut avoir lieu ; sinon ouvre la confirmation (réglage).
func before_end_turn() -> bool:
	if is_paused():
		return false
	if _end_turn_confirmed or not bool(_setting("interface/confirm_end_turn", false)):
		_end_turn_confirmed = false
		return true
	_show_end_turn_confirm()
	return false


func _show_end_turn_confirm() -> void:
	if _confirm_panel == null:
		_confirm_panel = PanelContainer.new()
		PanelStack.set_tier(_confirm_panel, PanelStack.Tier.MODAL, true)  # Q4
		_confirm_panel.theme = load("res://scenes/ui/parchment_theme.tres")
		_confirm_panel.set_anchors_preset(Control.PRESET_CENTER_TOP)
		_confirm_panel.offset_left = -220
		_confirm_panel.offset_right = 220
		_confirm_panel.offset_top = 90
		var box := VBoxContainer.new()
		_confirm_panel.add_child(box)
		_confirm_label = Label.new()
		_confirm_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		_confirm_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		box.add_child(_confirm_label)
		var row := HBoxContainer.new()
		row.alignment = BoxContainer.ALIGNMENT_CENTER
		row.add_theme_constant_override("separation", 12)
		box.add_child(row)
		var no := Button.new()
		no.text = "Pas encore"
		no.pressed.connect(func() -> void: _confirm_panel.hide())
		row.add_child(no)
		_confirm_yes = Button.new()
		_confirm_yes.text = "Terminer le tour"
		_confirm_yes.pressed.connect(confirm_end_turn)
		row.add_child(_confirm_yes)
		map.get("ui").add_child(_confirm_panel)
	var armies_idle := 0
	var sim: Object = map.get("sim")
	for army_id in map.call("player_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if (army.get("path", []) as Array).is_empty() and int(army.get("movement_points", 0)) > 0:
			armies_idle += 1
	var question := "Terminer le tour (%s) ?" % sim.call("get_date_label")
	if armies_idle > 0:
		question += "\n%d armée%s sans ordre de marche." % [armies_idle, "s" if armies_idle > 1 else ""]
	_confirm_label.text = question
	_confirm_panel.show()
	_confirm_yes.grab_focus()


func confirm_end_turn() -> void:
	if _confirm_panel != null:
		_confirm_panel.hide()
	_end_turn_confirmed = true
	map.get("ui").call("request_end_turn")  # U5 : avec le bandeau des autres factions


## Après `end_turn` : sauvegarde auto, alertes, rapport de saison.
func after_end_turn(events: Array) -> void:
	last_events = events
	autosave()
	refresh()
	if bool(_setting("interface/season_report", true)):
		show_season_report(events)


## Lot U5 : rubriques du rapport ; « Le monde » filtré par intérêt (`MapUI.keeps_news`) ; bilan
## du trésor en tête de la rubrique « Trésor ».
func report_groups(events: Array) -> Array:
	var ui: Node = map.get("ui")
	var groups := SeasonReport.build_groups(events, _concerns_player, Callable(ui, "keeps_news"), str(map.get("player_faction")))
	return SeasonReport.with_summary(groups, "treasury", treasury_summary())


## Ligne de synthèse du trésor (`get_faction_economy` : solde prévu, variation de la saison).
func treasury_summary() -> Array:
	var sim: Object = map.get("sim")
	if sim == null or not sim.has_method("get_faction_economy"):
		return []
	var economy: Dictionary = sim.call("get_faction_economy", str(map.get("player_faction")))
	var history: Array = economy.get("budget_history", [])
	if economy.is_empty() or history.is_empty():
		return []
	var last: Dictionary = history.back()
	var change := int(last.get("change", 0))
	var net := int(economy.get("net_income", 0))
	var text := "Le trésor %s %s cette saison ; solde prévu %s par saison." % [
		"gagne" if change >= 0 else "perd", Money.amount(absi(change)), Money.signed(net)]
	var other := int(last.get("other", 0))
	if other != 0:
		text += " Hors budget : %s (rançons, tributs, agents, chronique)." % Money.signed(other)
	return [{"kind": "summary", "action": "faction", "text_fr": text, "_tone": SeasonReport.TONE_LOSS if net < 0 else ""}]


func show_season_report(events: Array) -> bool:
	var sim: Object = map.get("sim")
	if sim == null or season_report == null:
		return false
	return season_report.show_report(str(sim.call("get_date_label")), report_groups(events))


## Bataille résolue après la fin du tour (dialogue d'avant-bataille, combat ou résolution
## automatique) : ses événements arrivent après `after_end_turn` et sont fusionnés dans le
## rapport déjà construit (rouvert s'il avait été fermé entre-temps).
func report_late_events(events: Array) -> void:
	if events.is_empty() or season_report == null:
		return
	last_events += events
	if bool(_setting("interface/season_report", true)):
		var sim: Object = map.get("sim")
		season_report.add_events(events, _concerns_player, Callable(map.get("ui"), "keeps_news"), str(map.get("player_faction")),
			str(sim.call("get_date_label")) if sim != null else "")


func _concerns_player(event: Dictionary) -> bool:
	var player := str(map.get("player_faction"))
	if str(event.get("faction", "")) == player:
		return true
	# U5 : une place perdue par le joueur (« … (auparavant France) ») le concerne au premier chef.
	if str(event.get("kind", "")) == "province_captured":
		var facade := get_node_or_null("/root/SimFacade")
		var player_name := str(facade.call("faction_short_name", player)) if facade != null else ""
		if player_name != "" and str(event.get("text_fr", "")).contains("auparavant %s" % player_name):
			return true
	var sim: Object = map.get("sim")
	var province_id := str(event.get("province", ""))
	if province_id != "":
		var state: Dictionary = sim.call("get_province_state", province_id)
		if str(state.get("owner", "")) == player or str(state.get("controller", "")) == player:
			return true
	var army_id := str(event.get("army", ""))
	if army_id != "":
		return str((sim.call("get_army", army_id) as Dictionary).get("faction", "")) == player
	return false


# --- Navigation --------------------------------------------------------


func refresh() -> void:
	pass  # F10b : les alertes sont rafraîchies par `HudController.refresh`.


func _on_panels_changed(ui: Node) -> void:
	if season_report == null or not season_report.visible:
		return
	var stack: PanelStack = ui.panels
	# La fin de tour ouvre elle-même des panneaux (diplomatie sur offre nouvelle) dans l'image du
	# rapport : ils restent dessous ; seuls ceux que le joueur ouvre ensuite le referment.
	var same_frame := Engine.get_process_frames() - season_report.filled_frame <= 1
	for panel in stack.visible_panels():
		# Q6 : pause, réglages, sauvegarde (zone `MODAL`, sous le rapport) ferment aussi le rapport
		# quand le joueur les ouvre après lui, comme tout autre panneau.
		if panel == season_report or _panels_under_report.has(panel):
			continue
		if same_frame:
			_panels_under_report.append(panel)
		else:
			season_report.close()
			return


func _on_report_entry(event: Dictionary) -> void:
	var ui: Node = map.get("ui")
	match str(event.get("kind", "")):
		"technology_researched":
			ui.emit_signal("tech_panel_requested")
			return
		"summary":
			ui.emit_signal("faction_panel_requested")
			return
	var army_id := str(event.get("army", ""))
	var armies: ArmyMarkers = map.get("armies")
	if army_id != "" and armies != null and armies.has_army(army_id):
		focus_army(army_id)
	else:
		focus_province(str(event.get("province", "")))


func focus_province(province_id: String) -> void:
	var map_data: MapData = map.get("map_data")
	var index := map_data.index_of_id(province_id)
	if index <= 0:
		return
	var centroid := map_data.centroid_of_id(province_id)
	var rig: CampaignCamera = map.get("camera_rig")
	var extent := maxf(map_data.size.x, map_data.size.y)
	rig.look_at_point(Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y), extent * 0.09)
	(map.get("picker") as ProvincePicker).select_index(index)


func focus_army(army_id: String) -> void:
	var armies: ArmyMarkers = map.get("armies")
	if armies == null or not armies.has_army(army_id):
		return
	var map_data: MapData = map.get("map_data")
	var rig: CampaignCamera = map.get("camera_rig")
	rig.look_at_point(armies.world_position_of(army_id), maxf(map_data.size.x, map_data.size.y) * 0.09)
	map.call("select_army", army_id)


# --- Captures (--flow-stage) ------------------------------------------------------


func _run_stage() -> void:
	var stage := ""
	var shot := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--flow-stage="):
			stage = arg.trim_prefix("--flow-stage=")
		elif arg.begins_with("--flow-shot="):
			shot = arg.trim_prefix("--flow-shot=")
	if stage == "" or map.get("sim") == null:
		return
	var rig: CampaignCamera = map.get("camera_rig")
	rig.edge_pan_enabled = false
	for _i in 5:
		await get_tree().process_frame
	var sim: Object = map.get("sim")
	match stage:
		"pause":
			open_pause()
		"save":
			SaveSlots.autosave(4, 4, _capture_now())
			open_pause()
			pause_menu.open_save()
		"settings":
			open_settings()
			# U7 / U12 : `--flow-tab=Commandes` ouvre directement un onglet.
			for arg in OS.get_cmdline_user_args():
				if arg.begins_with("--flow-tab="):
					for tabs in _settings_menu.find_children("*", "TabContainer", true, false):
						for index in (tabs as TabContainer).get_tab_count():
							if (tabs as TabContainer).get_tab_title(index) == arg.trim_prefix("--flow-tab="):
								(tabs as TabContainer).current_tab = index
		"confirm":
			settings.call("set_value", "interface/confirm_end_turn", true, false)
			map.call("_on_end_turn")
		"report", "alerts":
			var interactive: bool = sim.call("get_interactive_battles") if sim.has_method("get_interactive_battles") else true
			if sim.has_method("set_interactive_battles"):
				sim.call("set_interactive_battles", false)
			var chronicle: Node = map.get("chronicle")
			for _turn_index in 6:
				map.call("_on_end_turn")
				if chronicle != null:
					(chronicle.get("window") as Control).hide()
				if stage == "report" and season_report.visible and season_report.line_count() >= 4:
					break
			if sim.has_method("set_interactive_battles"):
				sim.call("set_interactive_battles", interactive)
			if stage == "alerts":
				season_report.close()
				map.get("ui").call("hide_province")
	for _i in 45:
		await get_tree().process_frame
	if shot == "":
		return
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(shot.get_base_dir())
	var err := image.save_png(shot)
	print("FlowController: screenshot %s (%s)" % [shot, error_string(err)])
	get_tree().paused = false
	get_tree().quit(0 if err == OK else 1)
