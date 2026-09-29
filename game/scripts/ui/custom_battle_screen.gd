class_name CustomBattleScreen
extends PanelContainer

## NT2 — « Bataille personnalisée » du menu principal (squelette).

signal closed


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	UiZones.put(UiZones.Zone.MODAL, self)


func close() -> void:
	closed.emit()
	UiMotion.fade_out(self, UiMotion.DURATION, true)
