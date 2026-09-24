class_name RichLabel
extends Label

## Libellé dont `tooltip_text` est du BBCode rendu en infobulle parchemin (`RichTooltip`).
## Peut être posé sur un `Label` existant d'une scène : `label.set_script(RichLabel)`.


func _make_custom_tooltip(for_text: String) -> Object:
	return RichTooltip.make_panel(for_text)
