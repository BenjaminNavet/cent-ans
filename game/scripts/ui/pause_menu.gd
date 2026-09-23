class_name PauseMenu
extends Control

## F3 — menu pause de la carte (Échap) : Reprendre / Sauvegarder / Charger / Réglages /
## Aide / Menu principal / Quitter. Le contrôleur (`FlowController`) met l'arbre en pause
## pendant que le menu est ouvert ; ce nœud et ses fenêtres tournent en `PROCESS_MODE_ALWAYS`.
## Quitter ou revenir au menu avec une partie non sauvegardée demande confirmation
## (`unsaved_turns` > 0 : tours joués depuis la dernière sauvegarde, auto comprise).

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


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	process_mode = Node.PROCESS_MODE_ALWAYS
	var veil := ColorRect.new()
	veil.color = Color(0.05, 0.03, 0.01, 0.55)
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(veil)
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	_menu_panel = PanelContainer.new()
	_menu_panel.custom_minimum_size = Vector2(340, 0)
	center.add_child(_menu_panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	_menu_panel.add_child(box)
	var title := Label.new()
	title.text = "Pause"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 28)
	box.add_child(title)
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
		button.add_theme_font_size_override("font_size", 20)
		button.pressed.connect(spec[2])
		box.add_child(button)
		buttons[spec[0]] = button
	_build_confirm(center)
	save_dialog = (load(SAVE_DIALOG_SCENE) as PackedScene).instantiate()
	save_dialog.process_mode = Node.PROCESS_MODE_ALWAYS
	add_child(save_dialog)
	save_dialog.hide()
	save_dialog.save_confirmed.connect(func(save_name: String) -> void: save_requested.emit(save_name))
	save_dialog.load_confirmed.connect(func(path: String) -> void: load_requested.emit(path))
	save_dialog.dialog_closed.connect(func() -> void: _menu_panel.show())


func _build_confirm(center: CenterContainer) -> void:
	_confirm_panel = PanelContainer.new()
	_confirm_panel.custom_minimum_size = Vector2(460, 0)
	_confirm_panel.hide()
	center.add_child(_confirm_panel)
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
			_confirm_panel.hide()
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


func open_save() -> void:
	_menu_panel.hide()
	save_dialog.open_save(default_save_name)


func open_load() -> void:
	_menu_panel.hide()
	save_dialog.open_load()


func open_settings() -> void:
	_menu_panel.hide()
	_settings_menu = SettingsMenu.new()
	_settings_menu.closed.connect(func() -> void:
		_settings_menu = null
		_menu_panel.show())
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
	_menu_panel.hide()
	_confirm_panel.show()


func _cancel_exit() -> void:
	_pending_exit = ""
	_confirm_panel.hide()
	_menu_panel.show()


func _confirm_exit() -> void:
	var kind := _pending_exit
	_pending_exit = ""
	_confirm_panel.hide()
	if kind == "main_menu":
		main_menu_requested.emit()
	elif kind == "quit":
		quit_requested.emit()


func confirm_visible() -> bool:
	return _confirm_panel.visible
