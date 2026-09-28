class_name RichButton
extends Button

## Bouton dont `tooltip_text` est du BBCode rendu en infobulle parchemin (`RichTooltip`).


func _make_custom_tooltip(for_text: String) -> Object:
	return RichTooltip.panel_for(for_text, self)
