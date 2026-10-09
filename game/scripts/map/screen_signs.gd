class_name ScreenSigns
extends Node3D

## Base des signes de carte de taille constante à l'écran (SC MC13) : `RuinMarkers`,
## `ConstructionMarkers`. Tient la liste des signes posés, vide l'ensemble et recalcule la taille
## de chacun d'après sa distance à la caméra ; la sous-classe donne seulement `_apply_size`.

var _signs: Array[Node3D] = []


## Libère les signes existants.
func _clear_signs() -> void:
	for child in get_children():
		child.queue_free()
	_signs.clear()


func _add_sign(sign_node: Node3D) -> void:
	add_child(sign_node)
	_signs.append(sign_node)


## Dimensionne `sign_node` pour qu'il garde sa taille à l'écran à `distance` de la caméra.
func _apply_size(_sign_node: Node3D, _distance: float) -> void:
	pass


func _rescale() -> void:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return
	var eye := camera.global_position
	for sign_node in _signs:
		if is_instance_valid(sign_node):
			_apply_size(sign_node, eye.distance_to(sign_node.global_position))


func marker_count() -> int:
	return _signs.size()
