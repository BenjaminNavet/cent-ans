class_name CoastRenderer
extends MeshInstance3D

## Trait de côte : ruban fin brun sombre au niveau de la mer.

@export var color: Color = Color(0.25, 0.20, 0.14)
@export var width: float = 1.2
@export var lift: float = 0.15


func build(map_data: MapData) -> void:
	var lines := []
	var widths := []
	for line in map_data.coastlines:
		lines.append(line)
		widths.append(width)
	mesh = PolylineMesh.build(lines, widths, map_data, lift)
	material_override = PolylineMesh.flat_material(color)
