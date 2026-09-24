class_name CoastRenderer
extends MeshInstance3D

## Trait de côte : liseré sombre et discret, d'épaisseur constante à l'écran
## (`terrain_line.gdshader`) ; l'écume et le sable du shader font l'essentiel de la transition.

@export var color: Color = Color(0.10, 0.09, 0.07, 0.28)
@export var width: float = 0.0
@export var min_px: float = 1.0
@export var lift: float = 0.05


func build(map_data: MapData) -> void:
	var lines := []
	var widths := []
	for line in map_data.coastlines:
		lines.append(line)
		widths.append(width)
	mesh = PolylineMesh.build_screen_lines(lines, widths, map_data, lift)
	material_override = PolylineMesh.line_material(color, min_px)
