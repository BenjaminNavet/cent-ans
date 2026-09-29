class_name FolkPool
extends Node3D

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 2.2, § 5) : réservoir de
## figurines de la carte vivante. Squelette : API publique, rempli ensuite.


func setup(_map_data: MapData, _terrain: TerrainBuilder, _cap: int) -> void:
	pass


func register(_provider: Object) -> void:
	pass


func refresh(_sim: Object) -> void:
	pass


func update_view(_focus: Vector2, _camera_distance: float, _near_weight: float) -> void:
	pass
