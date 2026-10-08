class_name NextHintCard
extends PanelContainer

## Lot UX2 — encart parchemin du conseil « que faire maintenant » (haut gauche de la carte,
## sous la barre). Un titre court, une phrase, une croix pour le masquer jusqu'à la saison
## suivante. Un clic sur l'encart émet `activated` (le contrôleur exécute ou ouvre l'action).
## Ne connaît ni la simulation ni les règles : `NextHintController` fournit le conseil.

signal activated(hint: Dictionary)
signal dismissed(hint: Dictionary)

## Q6 : pas de largeur minimale — l'encart prend la largeur de la zone `TOASTS` (330 px la
## dépassaient en vue étroite : titre tronqué, texte coupé au bord).

var hint: Dictionary = {}
var title_label: Label
var text_label: Label
var close_button: Button


func _ready() -> void:
	name = "NextHintCard"
	mouse_filter = Control.MOUSE_FILTER_STOP
	mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	var box_style := HudStyle.note_box(8)
	box_style.content_margin_left = 14
	add_theme_stylebox_override("panel", box_style)
	var column := UiBuild.vbox(2)
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(column)
	var header := HBoxContainer.new()
	header.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(header)
	var kicker := UiBuild.label("Conseil :", HudStyle.FONT_SMALL, HudStyle.RUBRIC)
	kicker.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header.add_child(kicker)
	title_label = Label.new()
	title_label.name = "Title"
	title_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	title_label.add_theme_color_override("font_color", HudStyle.INK)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART  # Q6 : replié, plus tronqué
	title_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header.add_child(title_label)
	close_button = UiBuild.button("×")
	close_button.name = "Dismiss"
	close_button.flat = true
	close_button.focus_mode = Control.FOCUS_NONE
	close_button.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	close_button.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	RichTooltip.attach_plain(close_button, "hint_card_hide")
	close_button.pressed.connect(func() -> void: dismissed.emit(hint))
	header.add_child(close_button)
	text_label = Label.new()
	text_label.name = "Text"
	text_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
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
	RichTooltip.attach_plain(self, "hint_card_act")
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


## Infobulle en sections (`attach_plain` ne pose pas `plain_tooltip_host.gd` sur une classe scriptée).
func _make_custom_tooltip(for_text: String) -> Object:
	return RichTooltip.panel_for(for_text, self)
