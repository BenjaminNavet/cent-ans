class_name SaveLoadDialog
extends CenterContainer

## Boîte de dialogue parchemin pour sauvegarder / charger (`user://saves/*.json`, F3 :
## `SaveSlots`). Chaque emplacement montre sa vignette, son nom, la faction, la date de jeu,
## le tour et la date réelle ; les sauvegardes automatiques tournantes sont signalées.
## Mode sauvegarde : liste + champ de nom (cliquer une entrée pré-remplit le nom ; écraser
## demande une seconde confirmation). Mode chargement : liste seule (les sauvegardes d'un
## autre moteur sont grisées). « Supprimer » efface l'emplacement choisi.
## Signaux `save_confirmed(name)` et `load_confirmed(path)`.

signal save_confirmed(save_name: String)
signal load_confirmed(path: String)
signal dialog_closed

enum Mode { SAVE, LOAD }

@onready var title_label: Label = %TitleLabel
@onready var saves_list: ItemList = %SavesList
@onready var name_row: HBoxContainer = %NameRow
@onready var name_edit: LineEdit = %NameEdit
@onready var status_label: Label = %StatusLabel
@onready var delete_button: Button = %DeleteButton
@onready var cancel_button: Button = %CancelButton
@onready var confirm_button: Button = %ConfirmButton

var mode: Mode = Mode.SAVE
var _saves: Array[Dictionary] = []
var _overwrite_armed: String = ""
var _delete_armed: String = ""


func _ready() -> void:
	cancel_button.pressed.connect(close)
	confirm_button.pressed.connect(_on_confirm)
	delete_button.pressed.connect(_on_delete)
	saves_list.item_selected.connect(_on_item_selected)
	saves_list.item_activated.connect(func(_index: int) -> void: _on_confirm())
	name_edit.text_submitted.connect(func(_text: String) -> void: _on_confirm())
	name_edit.text_changed.connect(func(_text: String) -> void: _overwrite_armed = "")


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


func save_count() -> int:
	return _saves.size()


func _refresh_list() -> void:
	saves_list.clear()
	_saves = SaveSlots.list()
	_overwrite_armed = ""
	_delete_armed = ""
	var facade := get_node_or_null("/root/SimFacade")
	for save in _saves:
		var faction_id := str(save.get("faction", ""))
		var faction := str(facade.call("faction_short_name", faction_id)) if facade != null and faction_id != "" else "?"
		var text := "%s\n%s — %s, tour %d\nsauvegardée le %s" % [
			save["label"], faction, save.get("date", "?"), int(save.get("turn", 0)),
			str(save.get("timestamp", "")).replace("T", " à ")]
		var icon: Texture2D = SaveSlots.thumbnail_texture(save["name"]) if save.get("thumbnail", "") != "" else null
		if icon == null:
			icon = PortraitLoader.heraldry_texture(faction_id)
		var index := saves_list.add_item(text, icon)
		if mode == Mode.LOAD and not SaveSlots.loadable(save):
			saves_list.set_item_disabled(index, true)
			saves_list.set_item_tooltip(index, "Sauvegarde de la simulation réelle, indisponible ici.")
	status_label.text = ""
	confirm_button.disabled = false
	delete_button.disabled = true


func _on_item_selected(index: int) -> void:
	delete_button.disabled = index >= _saves.size()
	_delete_armed = ""
	delete_button.text = "Supprimer"
	if mode == Mode.SAVE and index < _saves.size():
		name_edit.text = _saves[index]["name"]


func _on_delete() -> void:
	var selected := saves_list.get_selected_items()
	if selected.is_empty():
		return
	var save_name: String = _saves[selected[0]]["name"]
	if _delete_armed != save_name:
		_delete_armed = save_name
		delete_button.text = "Confirmer ?"
		status_label.text = "Cliquez encore pour supprimer « %s »." % _saves[selected[0]]["label"]
		return
	SaveSlots.delete(save_name)
	delete_button.text = "Supprimer"
	_refresh_list()
	status_label.text = "Sauvegarde supprimée."


func _on_confirm() -> void:
	if mode == Mode.SAVE:
		var save_name := name_edit.text.strip_edges()
		if save_name == "":
			status_label.text = "Donnez un nom à la sauvegarde."
			return
		var exists := _saves.any(func(save: Dictionary) -> bool: return save["name"] == save_name.validate_filename())
		if exists and _overwrite_armed != save_name:
			_overwrite_armed = save_name
			status_label.text = "« %s » existe : confirmez pour l'écraser." % save_name
			return
		save_confirmed.emit(save_name)
	else:
		var selected := saves_list.get_selected_items()
		if selected.is_empty():
			status_label.text = "Choisissez une sauvegarde."
			return
		if not SaveSlots.loadable(_saves[selected[0]]):
			status_label.text = "Cette sauvegarde exige la simulation réelle."
			return
		load_confirmed.emit(_saves[selected[0]]["path"])
	hide()
	dialog_closed.emit()
