class_name WallCollapseFx
extends Node3D

## Effondrement physique des murailles (lot S1) : rendu seulement (ADR 0007).
## Squelette : API publique, implémentation à venir.

var settings: Dictionary = {}


func setup(_height_at: Callable) -> void:
	pass


func sync_piece(_index: int, _view: Dictionary, _previous_ratio: float, _ratio: float, _intact: bool) -> void:
	pass


func prime() -> void:
	pass


func active_body_count() -> int:
	return 0
