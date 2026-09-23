class_name SaveLoadDialog
extends CenterContainer

## Boîte de dialogue parchemin pour sauvegarder / charger (`user://saves/*.json`).
## Mode sauvegarde : liste + champ de nom (cliquer une entrée pré-remplit le nom).
## Mode chargement : liste seule. Signaux `save_confirmed(name)` et `load_confirmed(path)`.

signal save_confirmed(save_name: String)
signal load_confirmed(path: String)
signal dialog_closed

enum Mode { SAVE, LOAD }

@onready var title_label: Label = %TitleLabel
@onready var saves_list: ItemList = %SavesList
@onready var name_row: HBoxContainer = %NameRow
@onready var name_edit: LineEdit = %NameEdit
@onready var status_label: Label = %StatusLabel
@onready var cancel_button: Button = %CancelButton
@onready var confirm_button: Button = %ConfirmButton

var mode: Mode = Mode.SAVE
var _saves: Array[Dictionary] = []


func _ready() -> void:
	cancel_button.pressed.connect(close)
	confirm_button.pressed.connect(_on_confirm)
	saves_list.item_selected.connect(_on_item_selected)
	saves_list.item_activated.connect(func(_index: int) -> void: _on_confirm())
	name_edit.text_submitted.connect(func(_text: String) -> void: _on_confirm())


func open_save(default_name: String) -> void:
	mode = Mode.SAVE
	title_label.text = "Sauvegarder la partie"
	confirm_button.text = "Sauvegarder"
	name_row.show()
	name_edit.text = default_name
	_refresh_list()
	show()
	name_edit.grab_focus()


func open_load() -> void:
	mode = Mode.LOAD
	title_label.text = "Charger une partie"
	confirm_button.text = "Charger"
	name_row.hide()
	_refresh_list()
	status_label.text = "Aucune sauvegarde." if _saves.is_empty() else ""
	confirm_button.disabled = _saves.is_empty()
	show()


func close() -> void:
	hide()
	dialog_closed.emit()


func _refresh_list() -> void:
	saves_list.clear()
	_saves = SimFacade.list_saves()
	for save in _saves:
		var faction := SimFacade.faction_short_name(save["faction"]) if save["faction"] != "" else "?"
		saves_list.add_item("%s — %s, %s (%s)" % [save["name"], faction, save["date"], save["timestamp"]])
	status_label.text = ""
	confirm_button.disabled = false


func _on_item_selected(index: int) -> void:
	if mode == Mode.SAVE and index < _saves.size():
		name_edit.text = _saves[index]["name"]


func _on_confirm() -> void:
	if mode == Mode.SAVE:
		var save_name := name_edit.text.strip_edges()
		if save_name == "":
			status_label.text = "Donnez un nom à la sauvegarde."
			return
		save_confirmed.emit(save_name)
	else:
		var selected := saves_list.get_selected_items()
		if selected.is_empty():
			status_label.text = "Choisissez une sauvegarde."
			return
		load_confirmed.emit(_saves[selected[0]]["path"])
	hide()
	dialog_closed.emit()
