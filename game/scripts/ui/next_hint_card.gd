class_name NextHintCard
extends PanelContainer

## Lot UX2 — encart parchemin du conseil « que faire maintenant » (haut gauche de la carte,
## sous la barre). Un titre court, une phrase, une croix pour le masquer jusqu'à la saison
## suivante. Un clic sur l'encart émet `activated` (le contrôleur exécute ou ouvre l'action).
## Ne connaît ni la simulation ni les règles : `NextHintController` fournit le conseil.

signal activated(hint: Dictionary)
signal dismissed(hint: Dictionary)

const WIDTH := 330.0

var hint: Dictionary = {}
var title_label: Label
var text_label: Label
var close_button: Button


func _ready() -> void:
	name = "NextHintCard"
	mouse_filter = Control.MOUSE_FILTER_STOP
	mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	custom_minimum_size = Vector2(WIDTH, 0)
	var box_style := HudStyle.note_box(8)
	box_style.content_margin_left = 14
	add_theme_stylebox_override("panel", box_style)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 2)
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(column)
	var header := HBoxContainer.new()
	header.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(header)
	var kicker := Label.new()
	kicker.text = "Conseil :"
	kicker.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	kicker.add_theme_color_override("font_color", HudStyle.RUBRIC)
	kicker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header.add_child(kicker)
	title_label = Label.new()
	title_label.name = "Title"
	title_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	title_label.add_theme_color_override("font_color", HudStyle.INK)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	title_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header.add_child(title_label)
	close_button = Button.new()
	close_button.name = "Dismiss"
	close_button.text = "×"
	close_button.flat = true
	close_button.focus_mode = Control.FOCUS_NONE
	close_button.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	close_button.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	close_button.tooltip_text = "Masquer ce conseil jusqu'à la saison prochaine.\nRéglages → Carte → « Conseil : que faire maintenant » pour ne plus en voir."
	close_button.pressed.connect(func() -> void: dismissed.emit(hint))
	header.add_child(close_button)
	text_label = Label.new()
	text_label.name = "Text"
	text_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text_label.custom_minimum_size = Vector2(WIDTH - 24.0, 0)
	text_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	text_label.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	text_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(text_label)
	gui_input.connect(_on_gui_input)
	mouse_entered.connect(func() -> void: _set_hover(true))
	mouse_exited.connect(func() -> void: _set_hover(false))
	hide()


## Affiche `new_hint` (`NextHint.choose`) ; vide : masque l'encart.
func set_hint(new_hint: Dictionary) -> void:
	if new_hint == hint and (visible or new_hint.is_empty()):
		return
	hint = new_hint
	if hint.is_empty():
		hide()
		return
	title_label.text = str(hint.get("title", ""))
	text_label.text = str(hint.get("text", ""))
	tooltip_text = "Cliquez pour agir."
	size = Vector2.ZERO
	reset_size()


func _on_gui_input(event: InputEvent) -> void:
	var button := event as InputEventMouseButton
	if button != null and button.pressed and button.button_index == MOUSE_BUTTON_LEFT and not hint.is_empty():
		accept_event()
		activated.emit(hint)


func _set_hover(hovered: bool) -> void:
	var style := get_theme_stylebox("panel") as StyleBoxFlat
	if style != null:
		style.border_color = HudStyle.WAX if hovered else HudStyle.GOLD
