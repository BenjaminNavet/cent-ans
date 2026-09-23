class_name ConstructionMarkers
extends Node3D

## Icône marteau (Label3D "⚒") sur les provinces en construction (`get_province_city().construction`).
## Reconstruit à chaque `refresh` ; vide si la simulation n'expose pas `get_province_city`.

@export var font_size: int = 36
@export var label_color: Color = Color(0.35, 0.22, 0.08)

var _labels: Array[Label3D] = []


## `province_ids` : ids détenus par le joueur (ou tous, au choix de l'appelant) à vérifier ;
## `is_building(id) -> bool` et `world_position_of(id) -> Vector3` fournis par l'appelant.
func refresh(province_ids: PackedStringArray, is_building: Callable, world_position_of: Callable) -> void:
	for child in get_children():
		child.queue_free()
	_labels.clear()
	for province_id in province_ids:
		if not bool(is_building.call(province_id)):
			continue
		var label := Label3D.new()
		label.text = "⚒"
		label.font_size = font_size
		label.outline_size = 10
		label.modulate = label_color
		label.outline_modulate = Color(0.97, 0.92, 0.80)
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.fixed_size = true
		label.pixel_size = 0.0014
		label.no_depth_test = true
		label.render_priority = 3
		label.position = world_position_of.call(province_id)
		add_child(label)
		_labels.append(label)


func marker_count() -> int:
	return _labels.size()
