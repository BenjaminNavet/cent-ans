class_name PathPreview
extends MeshInstance3D

## Aperçu de chemin : ruban orange posé sur le relief entre les centroïdes des provinces
## du chemin (segments subdivisés pour suivre le terrain). Largeur selon la distance caméra.

const SUBDIVISION_PX := 12.0

@export var color: Color = Color(1.0, 0.55, 0.15, 0.95)
@export var lift: float = 0.6

var map_data: MapData
var _shown_ids: PackedStringArray = PackedStringArray()


func setup(data: MapData) -> void:
	map_data = data
	material_override = PolylineMesh.flat_material(color)
	material_override.render_priority = 2
	visible = false


## `province_ids` : province de départ suivie du chemin. Vide → masque l'aperçu.
func show_path(province_ids: PackedStringArray, camera_distance: float) -> void:
	if province_ids.size() < 2 or map_data == null:
		hide_path()
		return
	_shown_ids = province_ids
	var points := PackedVector2Array()
	for i in province_ids.size():
		var centroid := map_data.centroid_of_id(province_ids[i])
		if centroid.x < 0.0:
			continue
		if points.is_empty():
			points.append(centroid)
			continue
		var previous := points[points.size() - 1]
		var steps := maxi(int(previous.distance_to(centroid) / SUBDIVISION_PX), 1)
		for k in range(1, steps + 1):
			points.append(previous.lerp(centroid, float(k) / steps))
	var width := clampf(camera_distance * 0.005, 0.8, 8.0)
	mesh = PolylineMesh.build([points], [width], map_data, lift)
	visible = true


func hide_path() -> void:
	_shown_ids = PackedStringArray()
	visible = false


func shown_ids() -> PackedStringArray:
	return _shown_ids
