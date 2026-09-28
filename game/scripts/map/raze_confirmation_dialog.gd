class_name RazeConfirmationDialog
extends PanelContainer

## RS-N : confirmation « Raser ce bâtiment ? » avant l'ordre `demolish`. Calqué sur
## `WarDeclarationDialog` (même fenêtre modale du projet, pas de dialogue OS). Le remboursement
## et l'entretien économisé viennent du cœur (`settlement_demolition_preview`) ; aucune règle ici.

signal confirmed
signal cancelled

var _title: Label
var _body: Label
var _confirm_button: Button


func _init() -> void:
	name = "RazeConfirmationDialog"
	PanelStack.set_tier(self, PanelStack.Tier.MODAL, true)
	theme = load("res://scenes/ui/parchment_theme.tres")
	set_anchors_preset(Control.PRESET_CENTER_TOP)
	offset_left = -220
	offset_right = 220
	offset_top = 110
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	add_child(box)
	_title = Label.new()
	UiType.apply(_title, UiType.HEADING)
	_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_title)
	_body = Label.new()
	UiType.apply(_body, UiType.BODY)
	_body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_body.custom_minimum_size = Vector2(400, 0)
	box.add_child(_body)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 12)
	box.add_child(row)
	var cancel_button := Button.new()
	cancel_button.text = "Renoncer"
	cancel_button.pressed.connect(_on_cancel)
	row.add_child(cancel_button)
	_confirm_button = Button.new()
	_confirm_button.text = "Raser"
	_confirm_button.pressed.connect(_on_confirm)
	row.add_child(_confirm_button)
	hide()


## Ouvre la confirmation pour `building_name`, avec le remboursement et l'entretien
## économisé donnés par `preview` (`settlement_demolition_preview`, `{refund, upkeep_saved}`).
func ask(building_name: String, preview: Dictionary) -> void:
	_title.text = "Raser %s ?" % building_name
	var refund := int(preview.get("refund", 0))
	var upkeep_saved := int(preview.get("upkeep_saved", 0))
	_body.text = "Le bâtiment est détruit sans retour possible. Rembourse %s ; économise %s d'entretien par saison." % [Money.amount(refund), Money.amount(upkeep_saved)]
	show()
	_confirm_button.grab_focus()


func _on_confirm() -> void:
	hide()
	confirmed.emit()


func _on_cancel() -> void:
	hide()
	cancelled.emit()


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		_on_cancel()
		get_viewport().set_input_as_handled()
