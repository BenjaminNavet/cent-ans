class_name ReachableMarkers
extends MultiMeshInstance3D

## Anneaux au sol sur les colonies atteignables ce tour par l'armée sélectionnée (lot C5).
## Rendu seulement : la liste vient de `CampaignSim.get_reachable_settlements`. Taille à
## peu près constante à l'écran (proportionnelle à la distance caméra, bornée).

@export var color: Color = Color(0.45, 0.95, 0.40, 0.9)
@export var target_color: Color = Color(1.0, 0.62, 0.18, 1.0)
@export var lift: float = 0.4

var _positions: PackedVector3Array = PackedVector3Array()
var _ids: PackedStringArray = PackedStringArray()
var _target := ""
var _scale := -1.0


func _init() -> void:
	name = "ReachableMarkers"
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var ring := TorusMesh.new()
	ring.inner_radius = 0.78
	ring.outer_radius = 1.0
	ring.rings = 24
	ring.ring_segments = 4
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.vertex_color_use_as_albedo = true
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.no_depth_test = true
	material.render_priority = 1
	ring.material = material
	multimesh = MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.mesh = ring
	visible = false


## `positions[i]` : position monde de la colonie `ids[i]`.
func show_markers(ids: PackedStringArray, positions: PackedVector3Array, camera_distance: float) -> void:
	_ids = ids
	_positions = positions
	multimesh.instance_count = positions.size()
	_scale = -1.0
	update_scale(camera_distance)
	visible = positions.size() > 0


func clear() -> void:
	_ids = PackedStringArray()
	_positions = PackedVector3Array()
	_target = ""
	multimesh.instance_count = 0
	visible = false


## Colonie visée par l'aperçu de chemin (anneau orange), "" = aucune.
func set_target(id: String) -> void:
	if id == _target:
		return
	_target = id
	_apply_colors()


func marker_count() -> int:
	return _positions.size()


func has_marker(id: String) -> bool:
	return _ids.has(id)


func update_scale(camera_distance: float) -> void:
	var size := clampf(camera_distance * 0.012, 0.9, 10.0)
	if is_equal_approx(size, _scale):
		return
	_scale = size
	for i in _positions.size():
		var basis := Basis().scaled(Vector3(size, size * 0.3, size))
		multimesh.set_instance_transform(i, Transform3D(basis, _positions[i] + Vector3(0.0, lift, 0.0)))
	_apply_colors()


func _apply_colors() -> void:
	for i in _positions.size():
		multimesh.set_instance_color(i, target_color if _ids[i] == _target else color)
