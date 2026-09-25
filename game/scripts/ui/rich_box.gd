class_name RichBox
extends VBoxContainer

## Boîte verticale dont `tooltip_text` est du BBCode rendu en infobulle parchemin (`RichTooltip`).
## B1 : peut être posée sur une `VBoxContainer` de scène : `box.set_script(RichBox)`.


func _make_custom_tooltip(for_text: String) -> Object:
	return RichTooltip.make_panel(for_text)
