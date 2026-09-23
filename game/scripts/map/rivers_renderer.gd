class_name RiversRenderer
extends MeshInstance3D

## Rivières : rubans bleus légèrement au-dessus du relief, largeur selon l'ordre de Strahler.

@export var color: Color = Color(0.18, 0.40, 0.70)
@export var base_width: float = 0.5
@export var width_per_strahler: float = 0.5
@export var lift: float = 0.25


func build(map_data: MapData) -> void:
	var lines := []
	var widths := []
	for river in map_data.rivers:
		lines.append(river["points"])
		widths.append(base_width + width_per_strahler * float(river["strahler"]))
	mesh = PolylineMesh.build(lines, widths, map_data, lift)
	material_override = PolylineMesh.flat_material(color)
