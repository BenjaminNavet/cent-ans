class_name RichPanel
extends PanelContainer

## Panneau (carte d'unité…) dont `tooltip_text` est du BBCode rendu en infobulle parchemin.


func _make_custom_tooltip(for_text: String) -> Object:
	return RichTooltip.panel_for(for_text, self)
