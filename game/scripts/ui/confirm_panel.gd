class_name ConfirmPanel
extends PanelContainer

## Fenêtre modale oui / non du projet (thème parchemin, pas de dialogue OS), réutilisable via
## `open` ; `ConfirmDialog.ask` en fait un dialogue jetable. Les sous-classes ajoutent leur contenu dans `body`
## (entre le texte et les boutons) ; aucune règle de jeu ici.

signal confirmed
signal cancelled

## Ancrage vertical des fenêtres de la carte ; le HUD de bataille repose les siens.
const TOP_OFFSET := 110

var title_label: Label
var text_label: Label
var body: VBoxContainer
var button_row: HBoxContainer
var yes_button: Button
var no_button: Button
## Échap équivaut à « non » ; le HUD de bataille gère Échap lui-même.
var close_on_escape := true
var focus_yes_on_open := true


func _init(yes_label := "Confirmer", no_label := "Renoncer", width := 440) -> void:
	name = "ConfirmPanel"
	PanelStack.set_tier(self, PanelStack.Tier.MODAL, true)
	theme = load("res://scenes/ui/parchment_theme.tres")
	set_anchors_preset(Control.PRESET_CENTER_TOP)
	offset_left = -width / 2.0
	offset_right = width / 2.0
	offset_top = TOP_OFFSET
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.name = "Box"
	box.add_theme_constant_override("separation", 10)
	add_child(box)
	title_label = Label.new()
	UiType.apply(title_label, UiType.HEADING)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(title_label)
	text_label = Label.new()
	text_label.name = "Text"
	UiType.apply(text_label, UiType.BODY)
	text_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	text_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text_label.custom_minimum_size = Vector2(width - 40, 0)
	box.add_child(text_label)
	body = VBoxContainer.new()
	box.add_child(body)
	button_row = HBoxContainer.new()
	button_row.name = "Buttons"
	button_row.alignment = BoxContainer.ALIGNMENT_CENTER
	button_row.add_theme_constant_override("separation", 12)
	box.add_child(button_row)
	no_button = Button.new()
	no_button.name = "Cancel"
	no_button.text = no_label
	no_button.pressed.connect(_on_cancel)
	button_row.add_child(no_button)
	yes_button = Button.new()
	yes_button.name = "Confirm"
	yes_button.text = yes_label
	yes_button.pressed.connect(_on_confirm)
	button_row.add_child(yes_button)
	hide()


func open(title: String, text: String) -> void:
	title_label.text = title
	title_label.visible = title != ""
	text_label.text = text
	text_label.visible = text != ""
	show()
	if focus_yes_on_open:
		yes_button.grab_focus()


func _on_confirm() -> void:
	hide()
	confirmed.emit()


func _on_cancel() -> void:
	hide()
	cancelled.emit()


func _unhandled_input(event: InputEvent) -> void:
	if close_on_escape and visible and event.is_action_pressed("ui_cancel"):
		_on_cancel()
		get_viewport().set_input_as_handled()
