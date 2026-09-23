class_name RiversRenderer
extends MeshInstance3D

## Rivières : rubans fins posés sur le relief (`terrain_line.gdshader`), largeur monde selon
## l'importance, jamais moins de `major_min_px` / `minor_min_px` pixels écran (les cours d'eau
## plus fins s'estompent) ; couleur accordée à la mer, légère transparence.
## Les fleuves majeurs (`importance` ≥ MAJOR_IMPORTANCE) sont toujours visibles ; les
## cours d'eau mineurs (maillage enfant) n'apparaissent qu'en vue rapprochée.

const MAJOR_IMPORTANCE := 3

@export var color: Color = Color(0.21, 0.37, 0.44, 0.9)
@export var base_width: float = 0.1
@export var width_per_importance: float = 0.1
@export var major_min_px: float = 1.6
@export var minor_min_px: float = 1.0
@export var lift: float = 0.05
## Distance caméra au-delà de laquelle les rivières mineures sont masquées (réglée par la scène).
@export var minor_max_distance: float = 1500.0

var _minor: MeshInstance3D
var _minor_visible := true


func build(map_data: MapData) -> void:
	var major_lines := []
	var major_widths := []
	var minor_lines := []
	var minor_widths := []
	for river in map_data.rivers:
		var importance: int = river["importance"]
		var width := base_width + width_per_importance * importance
		if importance >= MAJOR_IMPORTANCE:
			major_lines.append(river["points"])
			major_widths.append(width)
		else:
			minor_lines.append(river["points"])
			minor_widths.append(width)
	mesh = PolylineMesh.build_screen_lines(major_lines, major_widths, map_data, lift)
	material_override = PolylineMesh.line_material(color, major_min_px, 2)
	if _minor != null:
		_minor.queue_free()
	_minor = MeshInstance3D.new()
	_minor.name = "MinorRivers"
	_minor.mesh = PolylineMesh.build_screen_lines(minor_lines, minor_widths, map_data, lift)
	var minor_color := color
	minor_color.a *= 0.8
	_minor.material_override = PolylineMesh.line_material(minor_color, minor_min_px, 2)
	add_child(_minor)


func update_visibility(camera_distance: float) -> void:
	var should_show := camera_distance < minor_max_distance
	if should_show == _minor_visible or _minor == null:
		return
	_minor_visible = should_show
	_minor.visible = should_show
