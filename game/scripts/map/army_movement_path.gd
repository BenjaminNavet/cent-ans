class_name ArmyMovementPath
extends Node3D

## Lot M4 : chemin prévu de l'armée sélectionnée, posé sur le relief, en deux couleurs :
## la partie parcourue ce tour (or) puis celle des tours suivants (encre rouge), avec un
## jalon à la fin de chaque tour. Rendu seulement : la polyligne et ses coupures viennent de
## `CampaignSim.find_path_points` (ou du `planned_path` d'une armée déjà en marche).

## Rubans subdivisés tous les SUBDIVISION_PX pour suivre le relief.
const SUBDIVISION_PX := 6.0
const LIFT := 0.5

@export var now_color: Color = Color(0.40, 0.88, 0.32, 1.0)
@export var later_color: Color = Color(0.92, 0.36, 0.20, 0.95)
@export var marker_color: Color = Color(1.0, 0.90, 0.60, 1.0)
## Lot DP2 : chemin sans droit de passage (incident diplomatique), en rouge franc.
@export var trespass_color: Color = Color(0.93, 0.08, 0.06, 1.0)

var map_data: MapData
var _now: MeshInstance3D
var _later: MeshInstance3D
var _markers: MultiMeshInstance3D
var _points := PackedVector2Array()
var _stop_index := 0
var _turn_ends := PackedInt32Array()
var _warning := false


func setup(data: MapData) -> void:
	map_data = data
	name = "ArmyMovementPath"
	_now = _line_instance("ThisTurn", now_color, 4.0, 3)
	_later = _line_instance("LaterTurns", later_color, 3.2, 2)
	_markers = MultiMeshInstance3D.new()
	_markers.name = "TurnEnds"
	_markers.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var ring := TorusMesh.new()
	ring.inner_radius = 0.55
	ring.outer_radius = 1.0
	ring.rings = 20
	ring.ring_segments = 4
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.vertex_color_use_as_albedo = true
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.no_depth_test = true
	material.render_priority = 4
	ring.material = material
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.mesh = ring
	_markers.multimesh = multimesh
	add_child(_markers)
	visible = false


func _line_instance(node_name: String, color: Color, min_px: float, priority: int) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var material := PolylineMesh.line_material(color, min_px, priority)
	material.set_shader_parameter("thin_fade", 0.0)
	material.set_shader_parameter("center_highlight", 0.25)
	# CV3-0 (#6) : liseré sombre pour rester lisible sur tout type de terrain.
	material.set_shader_parameter("casing_width", 0.32)
	material.set_shader_parameter("casing_color", Color(0.05, 0.04, 0.03, 0.88))
	instance.material_override = material
	add_child(instance)
	return instance


## `points` : polyligne carte (départ compris) ; `stop_index` : fin de ce tour ;
## `turn_ends` : fin de chaque tour (la dernière = fin du trajet).
## `warning` (lot DP2) : la marche entre sans droit de passage sur les terres d'une faction en
## paix ; le chemin est tracé en rouge.
func show_plan(points: PackedVector2Array, stop_index: int, turn_ends: PackedInt32Array, camera_distance: float, warning: bool = false) -> void:
	if points.size() < 2 or map_data == null:
		hide_path()
		return
	_set_warning(warning)
	_points = points
	_stop_index = clampi(stop_index, 0, points.size() - 1)
	_turn_ends = turn_ends
	# CV3-0 (#6) : largeur minimale relevée (0.3 -> 0.55) pour que le liseré sombre reste visible.
	var width := clampf(camera_distance * 0.004, 0.55, 6.0)
	var now_part := _subdivide(points.slice(0, _stop_index + 1))
	var later_part := _subdivide(points.slice(_stop_index))
	_now.mesh = PolylineMesh.build_screen_lines([now_part], [width], map_data, LIFT) if now_part.size() >= 2 else null
	_later.mesh = PolylineMesh.build_screen_lines([later_part], [width * 0.8], map_data, LIFT) if later_part.size() >= 2 else null
	_place_markers(camera_distance)
	visible = true


func _set_warning(on: bool) -> void:
	if on == _warning or _now == null:
		return
	_warning = on
	for pair in [[_now, now_color], [_later, later_color]]:
		var material := (pair[0] as MeshInstance3D).material_override as ShaderMaterial
		if material != null:
			material.set_shader_parameter("color", trespass_color if on else pair[1])


## Vrai si le chemin affiché est un avertissement d'intrusion (DP2).
func is_warning() -> bool:
	return visible and _warning


func hide_path() -> void:
	_points = PackedVector2Array()
	_turn_ends = PackedInt32Array()
	_stop_index = 0
	visible = false


## Vrai si une partie « tours suivants » est affichée.
func has_later_part() -> bool:
	return visible and _stop_index < _points.size() - 1


func has_now_part() -> bool:
	return visible and _stop_index > 0


func shown_points() -> PackedVector2Array:
	return _points if visible else PackedVector2Array()


func stop_index() -> int:
	return _stop_index


func update_scale(camera_distance: float) -> void:
	if visible:
		_place_markers(camera_distance)


func _place_markers(camera_distance: float) -> void:
	var multimesh := _markers.multimesh
	var size := clampf(camera_distance * 0.012, 0.8, 9.0)
	var ends := PackedInt32Array()
	for index in _turn_ends:
		if index > 0 and index < _points.size():
			ends.append(index)
	multimesh.instance_count = ends.size()
	for i in ends.size():
		var p := _points[ends[i]]
		var basis := Basis().scaled(Vector3(size, size * 0.25, size))
		var origin := Vector3(p.x, map_data.surface_world_at(p.x, p.y) + LIFT, p.y)
		multimesh.set_instance_transform(i, Transform3D(basis, origin))
		var last := i == ends.size() - 1
		var color := marker_color if ends[i] == _stop_index and not last else (now_color if ends[i] <= _stop_index else later_color)
		multimesh.set_instance_color(i, trespass_color if _warning else color)


func _subdivide(points: PackedVector2Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	if points.is_empty():
		return out
	out.append(points[0])
	for i in range(1, points.size()):
		var previous := points[i - 1]
		var target := points[i]
		var steps := maxi(int(previous.distance_to(target) / SUBDIVISION_PX), 1)
		for k in range(1, steps + 1):
			out.append(previous.lerp(target, float(k) / steps))
	return out
