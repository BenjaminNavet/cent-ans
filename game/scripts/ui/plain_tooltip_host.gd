extends Control

## script générique attaché par `TooltipHost.attach_plain` aux contrôles natifs
## (`Button`, `CheckBox`, `Label`…) qui n'ont pas de classe dédiée (`RichButton`, `IconChip`,
## `RichPanel`). Rend `tooltip_text` en infobulle en sections comme les autres (`bubble`).
## Aucune règle de jeu ici.


func _make_custom_tooltip(for_text: String) -> Object:
	return TooltipHost.bubble(for_text, self)
