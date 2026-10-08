class_name IconChip
extends HBoxContainer

## Icône + libellé sur une ligne, avec infobulle riche (BBCode de `RichTooltip`) sur toute
## la surface. `IconChip.create("res_wine", "Vin", RichTooltip.resource("res_wine"))`.

var icon_rect: TextureRect
var label: Label


static func create(icon_id: String, text: String, tooltip_bbcode: String = "", icon_size: float = 20.0, font_size: int = 14, category: String = "") -> IconChip:
	var chip := IconChip.new()
	chip.add_theme_constant_override("separation", 4)
	chip.mouse_filter = Control.MOUSE_FILTER_STOP
	var library := RichTooltip.icons()
	if library != null:
		chip.icon_rect = library.call("make_rect", icon_id, icon_size, category)
	else:
		chip.icon_rect = TextureRect.new()
		chip.icon_rect.custom_minimum_size = Vector2(icon_size, icon_size)
	chip.add_child(chip.icon_rect)
	chip.label = Label.new()
	chip.label.text = text
	chip.label.add_theme_font_size_override("font_size", font_size)
	chip.label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	chip.label.visible = text != ""
	chip.add_child(chip.label)
	chip.tooltip_text = tooltip_bbcode
	return chip


func _make_custom_tooltip(for_text: String) -> Object:
	return TooltipHost.bubble(for_text, self)
