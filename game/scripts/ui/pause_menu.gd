class_name PauseMenu
extends Control

## F3 — menu pause de la carte (Échap) : Reprendre / Sauvegarder / Charger / Réglages /
## Aide / Menu principal / Quitter. Le contrôleur (`FlowController`) met l'arbre en pause
## pendant que le menu est ouvert ; ce nœud et ses fenêtres tournent en `PROCESS_MODE_ALWAYS`.
## Quitter ou revenir au menu avec une partie non sauvegardée demande confirmation
## (`unsaved_turns` > 0 : tours joués depuis la dernière sauvegarde, auto comprise).
## Lot P2e (ADR 0097, bible DA § 12.1) : `self` reste un `Control` coordinateur sans voile ni
## taille propres ; `_menu_panel`, `_confirm_panel` et l'instance locale de `SaveLoadDialog`
## (celle créée ici, pas celles de `start_menu.gd`/`map_ui.gd`, hors lot) rejoignent
## individuellement la zone `MODAL` de `UiLayout`, qui fournit le voile commun (un seul tant que
## l'un des trois reste visible). Tailles par `UiType` ; ouverture et fermeture par `UiMotion`.

signal resume_requested
signal save_requested(save_name: String)
signal load_requested(path: String)
signal help_requested
signal main_menu_requested
signal quit_requested

const SAVE_DIALOG_SCENE := "res://scenes/ui/save_load_dialog.tscn"

var unsaved_turns: int = 0
var default_save_name: String = "partie"
var save_dialog: SaveLoadDialog
var buttons: Dictionary = {}  # clé → Button

var _menu_panel: PanelContainer
var _confirm_panel: PanelContainer
var _confirm_label: Label
var _pending_exit: String = ""  # "main_menu" ou "quit"
var _settings_menu: SettingsMenu = null
## DF1 : rappel du niveau de difficulté de la campagne (figé), sous le titre.
var difficulty_label: Label


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	process_mode = Node.PROCESS_MODE_ALWAYS
	# `_menu_panel`, `_confirm_panel` et `save_dialog` rejoignent `UiLayout` (zone `MODAL`) : ils
	# ne sont plus des descendants de `self`, donc pas libérés avec lui — `close_pause()` (hors
	# lot, `flow_controller.gd`) ne fait que `pause_menu.queue_free()`.
	tree_exiting.connect(_free_modal_children)
	var parchment := load("res://scenes/ui/parchment_theme.tres")
	_menu_panel = PanelContainer.new()
	_menu_panel.theme = parchment
	_menu_panel.custom_minimum_size = Vector2(340, 0)
	add_child(_menu_panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	_menu_panel.add_child(box)
	var title := Label.new()
	title.text = "Pause"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	UiType.apply(title, UiType.TITLE)
	box.add_child(title)
	difficulty_label = Label.new()
	difficulty_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	UiType.apply(difficulty_label, UiType.CAPTION)
	RichTooltip.attach_plain(difficulty_label, "pause_difficulty_fixed")
	difficulty_label.mouse_filter = Control.MOUSE_FILTER_PASS
	box.add_child(difficulty_label)
	refresh_difficulty()
	visibility_changed.connect(refresh_difficulty)
	for spec in [
		["resume", "Reprendre", func() -> void: resume_requested.emit()],
		["save", "Sauvegarder…", open_save],
		["load", "Charger…", open_load],
		["settings", "Réglages…", open_settings],
		["help", "Aide (F1)", func() -> void: help_requested.emit()],
		["main_menu", "Menu principal", func() -> void: _request_exit("main_menu")],
		["quit", "Quitter le jeu", func() -> void: _request_exit("quit")],
	]:
		var button := Button.new()
		button.text = spec[1]
		UiType.apply(button, UiType.HEADING)
		button.pressed.connect(spec[2])
		box.add_child(button)
		buttons[spec[0]] = button
	_build_confirm(parchment)
	save_dialog = (load(SAVE_DIALOG_SCENE) as PackedScene).instantiate()
	save_dialog.process_mode = Node.PROCESS_MODE_ALWAYS
	UiZones.put(UiZones.Zone.MODAL, save_dialog)
	save_dialog.hide()
	save_dialog.save_confirmed.connect(func(save_name: String) -> void: save_requested.emit(save_name))
	save_dialog.load_confirmed.connect(func(path: String) -> void: load_requested.emit(path))
	save_dialog.dialog_closed.connect(func() -> void: _show_menu_panel())
	UiZones.put(UiZones.Zone.MODAL, _menu_panel)
	UiMotion.fade_in(_menu_panel)


## Relit le niveau de la campagne en cours (`SimFacade`).
func refresh_difficulty() -> void:
	if difficulty_label == null:
		return
	var facade := get_node_or_null("/root/SimFacade")
	if facade == null or not facade.has_method("current_difficulty"):
		difficulty_label.text = ""
		return
	difficulty_label.text = "Difficulté : %s" % str(facade.call("difficulty_label", str(facade.call("current_difficulty"))))


func _build_confirm(parchment: Theme) -> void:
	_confirm_panel = PanelContainer.new()
	_confirm_panel.theme = parchment
	_confirm_panel.custom_minimum_size = Vector2(460, 0)
	_confirm_panel.hide()
	add_child(_confirm_panel)
	UiZones.put(UiZones.Zone.MODAL, _confirm_panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	_confirm_panel.add_child(box)
	_confirm_label = Label.new()
	_confirm_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_confirm_label)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	row.add_theme_constant_override("separation", 8)
	box.add_child(row)
	for spec in [
		["Annuler", _cancel_exit],
		["Sauvegarder d'abord", func() -> void:
			UiMotion.fade_out(_confirm_panel)
			open_save()],
		["Quitter sans sauvegarder", _confirm_exit],
	]:
		var button := Button.new()
		button.text = spec[0]
		button.pressed.connect(spec[1])
		row.add_child(button)
		buttons["confirm_%d" % row.get_child_count()] = button


func is_busy() -> bool:
	return save_dialog.visible or _confirm_panel.visible or (_settings_menu != null and is_instance_valid(_settings_menu))


## Libère les fenêtres passées dans la zone `MODAL` (elles ne sont plus filles de `self`).
func _free_modal_children() -> void:
	for control in [_menu_panel, _confirm_panel, save_dialog]:
		if control != null and is_instance_valid(control):
			control.queue_free()
	if _settings_menu != null and is_instance_valid(_settings_menu):
		_settings_menu.queue_free()


## Échap pendant la pause : ferme d'abord la fenêtre secondaire, sinon reprend.
func _unhandled_input(event: InputEvent) -> void:
	if not visible or not event.is_action_pressed("ui_cancel"):
		return
	get_viewport().set_input_as_handled()
	if save_dialog.visible:
		save_dialog.close()
	elif _confirm_panel.visible:
		_cancel_exit()
	elif _settings_menu != null and is_instance_valid(_settings_menu):
		_settings_menu.close()
	else:
		resume_requested.emit()


func _show_menu_panel() -> void:
	_menu_panel.show()
	UiMotion.fade_in(_menu_panel)


func open_save() -> void:
	UiMotion.fade_out(_menu_panel)
	save_dialog.open_save(default_save_name)


func open_load() -> void:
	UiMotion.fade_out(_menu_panel)
	save_dialog.open_load()


func open_settings() -> void:
	UiMotion.fade_out(_menu_panel)
	_settings_menu = SettingsMenu.new()
	_settings_menu.closed.connect(func() -> void:
		_settings_menu = null
		_show_menu_panel())
	add_child(_settings_menu)


func settings_open() -> bool:
	return _settings_menu != null and is_instance_valid(_settings_menu)


func _request_exit(kind: String) -> void:
	_pending_exit = kind
	if unsaved_turns <= 0:
		_confirm_exit()
		return
	var what := "revenir au menu principal" if kind == "main_menu" else "quitter le jeu"
	_confirm_label.text = "La partie n'a pas été sauvegardée depuis %d tour%s. Voulez-vous vraiment %s ?" % [
		unsaved_turns, "s" if unsaved_turns > 1 else "", what]
	UiMotion.fade_out(_menu_panel)
	_confirm_panel.show()
	UiMotion.fade_in(_confirm_panel)


func _cancel_exit() -> void:
	_pending_exit = ""
	UiMotion.fade_out(_confirm_panel)
	_show_menu_panel()


func _confirm_exit() -> void:
	var kind := _pending_exit
	_pending_exit = ""
	UiMotion.fade_out(_confirm_panel)
	if kind == "main_menu":
		main_menu_requested.emit()
	elif kind == "quit":
		quit_requested.emit()


func confirm_visible() -> bool:
	return _confirm_panel.visible
