class_name MapUI
extends CanvasLayer

## Couche UI de la carte : barre supérieure (date, fin du tour), panneau de province,
## étiquette de survol.

signal end_turn_pressed

@onready var date_label: Label = %DateLabel
@onready var end_turn_button: Button = %EndTurnButton
@onready var hover_label: Label = %HoverLabel
@onready var province_panel: ProvincePanel = %ProvincePanel


func _ready() -> void:
	end_turn_button.pressed.connect(func() -> void: end_turn_pressed.emit())
	province_panel.hide()
	hover_label.text = ""
	hover_label.hide()


func set_date(text: String) -> void:
	date_label.text = text


func set_hovered(province: Dictionary) -> void:
	hover_label.text = province.get("name", "") if not province.is_empty() else ""
	hover_label.visible = hover_label.text != ""


func show_province(province: Dictionary) -> void:
	province_panel.show_province(province)
