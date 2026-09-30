class_name LakesRenderer
extends MeshInstance3D

## Lot SS3 (ADR 0141 §4) : nappes d'eau des lacs de la carte de campagne, rendu seulement.
## Squelette : implémentation à venir.

const LAKES_FILE := "lakes.json"

var stats: Dictionary = {}


func build(_data: MapData) -> void:
	pass


func update_view(_camera_distance: float, _strategic_weight: float) -> void:
	pass
