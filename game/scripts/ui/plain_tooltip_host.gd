extends Control

## IB2 (ADR 0109) : script générique attaché par `RichTooltip.attach_plain` aux contrôles natifs
## (`Button`, `CheckBox`, `Label`…) qui n'ont pas de classe dédiée (`RichButton`, `IconChip`,
## `RichPanel`). Rend `tooltip_text` en infobulle en sections comme les autres (`panel_for`).
## Aucune règle de jeu ici.


func _make_custom_tooltip(for_text: String) -> Object:
	return RichTooltip.panel_for(for_text, self)
