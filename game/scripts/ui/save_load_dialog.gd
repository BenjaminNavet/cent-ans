class_name SaveLoadDialog
extends CenterContainer

## Boîte de dialogue parchemin pour sauvegarder / charger (`user://saves/*.json`, F3 :
## `SaveSlots`). Chaque emplacement montre sa vignette, son nom, la faction, la date de jeu,
## le tour et la date réelle ; les sauvegardes automatiques tournantes sont signalées.
## Mode sauvegarde : liste + champ de nom (cliquer une entrée pré-remplit le nom ; écraser
## demande une seconde confirmation). Mode chargement : liste seule (les sauvegardes d'un
## autre moteur sont grisées). « Supprimer » efface l'emplacement choisi.
## Signaux `save_confirmed(name)` et `load_confirmed(path)`.
## Lot P2e (ADR 0097, bible DA § 12.2) : tailles de texte par `UiType`, ouverture et fermeture
## par `UiMotion`. Lot P2g : ses trois propriétaires (`pause_menu.gd`, `start_menu.gd`,
## `map_ui.gd`) le réclament dans la zone `MODAL` de `UiLayout` (voile, centré sur sa taille).

signal save_confirmed(save_name: String)
signal load_confirmed(path: String)
signal dialog_closed

enum Mode { SAVE, LOAD }

@onready var title_label: Label = %TitleLabel
@onready var saves_rows: VBoxContainer = %SavesRows
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
var _selected: int = -1
var _rows: Array[PanelContainer] = []
var _row_style: StyleBoxFlat
var _row_selected_style: StyleBoxFlat


func _ready() -> void:
	UiType.apply(title_label, UiType.TITLE)
	UiType.apply(status_label, UiType.CAPTION)
	cancel_button.pressed.connect(close)
	confirm_button.pressed.connect(_on_confirm)
	delete_button.pressed.connect(_on_delete)
	_row_style = StyleBoxFlat.new()
	_row_style.bg_color = Color(0.96, 0.92, 0.80, 0.9)
	_row_style.set_content_margin_all(6)
	_row_style.set_corner_radius_all(3)
	_row_selected_style = _row_style.duplicate()
	_row_selected_style.bg_color = Color(0.80, 0.68, 0.45, 1.0)
	_row_selected_style.border_color = Color(0.42, 0.29, 0.16)
	_row_selected_style.set_border_width_all(2)
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
	UiMotion.fade_in(self)
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
	UiMotion.fade_in(self)


func close() -> void:
	UiMotion.fade_out(self)
	dialog_closed.emit()


func save_count() -> int:
	return _saves.size()


func _refresh_list() -> void:
	for row in _rows:
		row.queue_free()
	_rows.clear()
	_selected = -1
	_saves = SaveSlots.list()
	_overwrite_armed = ""
	_delete_armed = ""
	for index in _saves.size():
		var row := _make_row(_saves[index], index)
		saves_rows.add_child(row)
		_rows.append(row)
	status_label.text = ""
	confirm_button.disabled = false
	delete_button.disabled = true
	delete_button.text = "Supprimer"


## Une ligne : vignette (ou écu de la faction), nom, faction et date de jeu, date réelle.
func _make_row(save: Dictionary, index: int) -> PanelContainer:
	var facade := get_node_or_null("/root/SimFacade")
	var faction_id := str(save.get("faction", ""))
	var faction := str(facade.call("faction_short_name", faction_id)) if facade != null and faction_id != "" else "?"
	var row := PanelContainer.new()
	row.add_theme_stylebox_override("panel", _row_style)
	row.mouse_filter = Control.MOUSE_FILTER_STOP
	var hbox := HBoxContainer.new()
	hbox.add_theme_constant_override("separation", 12)
	hbox.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(hbox)
	var icon := TextureRect.new()
	icon.custom_minimum_size = Vector2(144, 81)
	icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var texture: Texture2D = SaveSlots.thumbnail_texture(save["name"]) if save.get("thumbnail", "") != "" else null
	icon.texture = texture if texture != null else PortraitLoader.heraldry_texture(faction_id)
	hbox.add_child(icon)
	var text := VBoxContainer.new()
	text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	text.alignment = BoxContainer.ALIGNMENT_CENTER
	text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hbox.add_child(text)
	var lines := [
		[str(save["label"]), UiType.HEADING, Color(0.22, 0.14, 0.07)],
		["%s — %s, tour %d — %s" % [faction, save.get("date", "?"), int(save.get("turn", 0)) + 1, _difficulty_label(str(save.get("difficulty", "normal")))], UiType.CAPTION, Color(0.35, 0.22, 0.10)],
		["Sauvegardée le %s" % french_timestamp(str(save.get("timestamp", ""))), UiType.CAPTION, Color(0.45, 0.36, 0.26)],
	]
	var usable := mode != Mode.LOAD or SaveSlots.loadable(save)
	if not usable:
		lines.append(["Simulation réelle requise", UiType.CAPTION, Color(0.55, 0.12, 0.10)])
	for spec in lines:
		var label := Label.new()
		label.text = spec[0]
		UiType.apply(label, spec[1])
		label.add_theme_color_override("font_color", spec[2])
		label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		text.add_child(label)
	if not usable:
		row.modulate = Color(1, 1, 1, 0.55)
	row.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			select(index)
			if event.double_click:
				_on_confirm())
	return row


## Sélectionne l'emplacement `index` (clic, ou tests).
func select(index: int) -> void:
	_selected = index if index >= 0 and index < _saves.size() else -1
	for i in _rows.size():
		_rows[i].add_theme_stylebox_override("panel", _row_selected_style if i == _selected else _row_style)
	_on_item_selected(_selected)


func _on_item_selected(index: int) -> void:
	delete_button.disabled = index < 0
	_delete_armed = ""
	delete_button.text = "Supprimer"
	if mode == Mode.SAVE and index >= 0:
		name_edit.text = _saves[index]["name"]


func _on_delete() -> void:
	if _selected < 0:
		return
	var save_name: String = _saves[_selected]["name"]
	if _delete_armed != save_name:
		_delete_armed = save_name
		delete_button.text = "Confirmer ?"
		status_label.text = "Cliquez encore pour supprimer « %s »." % _saves[_selected]["label"]
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
		if _selected < 0:
			status_label.text = "Choisissez une sauvegarde."
			return
		if not SaveSlots.loadable(_saves[_selected]):
			status_label.text = "Cette sauvegarde exige la simulation réelle."
			return
		load_confirmed.emit(_saves[_selected]["path"])
	hide()
	dialog_closed.emit()


const MONTHS_SHORT := ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."]


## « 2026-09-24T23:19:05 » → « 24 sept. 2026, 23 h 19 » (audit A3 T4) ; tel quel si illisible.
static func french_timestamp(iso: String) -> String:
	var parts := iso.split("T")
	var date := parts[0].split("-")
	if date.size() != 3 or not date[1].is_valid_int():
		return iso
	var month := int(date[1])
	if month < 1 or month > 12:
		return iso
	var text := "%d %s %s" % [int(date[2]), MONTHS_SHORT[month - 1], date[0]]
	if parts.size() > 1:
		var time := parts[1].split(":")
		if time.size() >= 2:
			text += ", %d h %s" % [int(time[0]), time[1]]
	return text


## DF1 : libellé français du niveau de difficulté d'une sauvegarde (Normale par défaut).
func _difficulty_label(id: String) -> String:
	var facade := get_node_or_null("/root/SimFacade")
	if facade != null and facade.has_method("difficulty_label"):
		return "difficulté %s" % str(facade.call("difficulty_label", id)).to_lower()
	return "difficulté %s" % id
