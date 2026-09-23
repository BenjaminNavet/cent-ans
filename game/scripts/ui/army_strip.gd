class_name ArmyStrip
extends PanelContainer

## Bandeau d ost. Squelette.

signal unit_selected(index: int)
signal selection_changed(indices: PackedInt32Array)
signal split_requested(indices: PackedInt32Array)


func set_army(_army: Dictionary, _capacity: int = 20, _unit_catalog: Dictionary = {}) -> void:
	pass
