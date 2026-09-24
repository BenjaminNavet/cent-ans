class_name CityMarkers
extends Node3D

## Marqueurs de capitales : cylindre placeholder + Label3D (nom de province) à `capital_px`.

@export var marker_color: Color = Color(0.55, 0.12, 0.10)
@export var label_color: Color = Color(0.16, 0.10, 0.05)
@export var marker_radius: float = 2.5
@export var marker_height: float = 4.0
@export var font_size: int = 22
## Distance caméra au-delà de laquelle les étiquettes sont masquées (réglée par la scène).
@export var label_max_distance: float = 1500.0

## Anti-chevauchement (lot V2b) : période de mise à jour (s) et marge entre étiquettes (px).
@export var declutter_interval: float = 0.2
@export var declutter_margin: float = 4.0

var _labels: Array[Label3D] = []
var _labels_visible := true
## Étiquettes triées par priorité (plus grande province d'abord).
var _priority: Array[Label3D] = []
var _declutter_timer := 0.0

var _marker_material: StandardMaterial3D
var _cylinder: CylinderMesh


func build(map_data: MapData) -> void:
	for child in get_children():
		child.queue_free()
	_labels.clear()
	_marker_material = StandardMaterial3D.new()
	_marker_material.albedo_color = marker_color
	_marker_material.roughness = 0.7
	_cylinder = CylinderMesh.new()
	_cylinder.top_radius = marker_radius
	_cylinder.bottom_radius = marker_radius * 1.15
	_cylinder.height = marker_height
	_cylinder.radial_segments = 12
	for index in map_data.provinces:
		var province: Dictionary = map_data.provinces[index]
		var capital: Vector2 = province["capital_px"]
		var y := map_data.surface_world_at(capital.x, capital.y)
		var marker := Node3D.new()
		marker.name = "City_%d" % index
		marker.position = Vector3(capital.x, y, capital.y)
		# M10 assets : modèle 3D (château / ville / village / cathédrale), sinon cylindre.
		var model := ModelLibrary.city_model(str(province.get("id", "")))
		if model != null:
			marker.add_child(model)
		else:
			var body := MeshInstance3D.new()
			body.mesh = _cylinder
			body.material_override = _marker_material
			body.position = Vector3(0.0, marker_height * 0.5, 0.0)
			marker.add_child(body)
		var label := Label3D.new()
		var capital_name: String = province.get("capital_name", "")
		label.text = capital_name if capital_name != "" else province["name"]
		label.font_size = font_size
		label.outline_size = 8
		label.modulate = label_color
		label.outline_modulate = Color(0.95, 0.90, 0.78)
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.fixed_size = true
		label.pixel_size = 0.0012
		label.no_depth_test = true
		label.render_priority = 2
		label.position = Vector3(0.0, (8.0 if model != null else marker_height) + 3.0, 0.0)
		marker.add_child(label)
		label.set_meta("area", float(province.get("area_px", 0.0)))
		_labels.append(label)
		add_child(marker)
	_priority = _labels.duplicate()
	_priority.sort_custom(func(a: Label3D, b: Label3D) -> bool: return float(a.get_meta("area")) > float(b.get_meta("area")))


## Masque les étiquettes quand la caméra est trop loin (lisibilité à 120 provinces).
func update_visibility(camera_distance: float) -> void:
	var should_show := camera_distance < label_max_distance
	if should_show == _labels_visible:
		return
	_labels_visible = should_show
	for label in _labels:
		label.visible = should_show
	_declutter_timer = 0.0
	if should_show:
		declutter()


func _process(delta: float) -> void:
	if not _labels_visible or _priority.is_empty():
		return
	_declutter_timer -= delta
	if _declutter_timer <= 0.0:
		_declutter_timer = declutter_interval
		declutter()


## Masque les étiquettes qui en chevauchent une plus prioritaire (rectangles écran estimés
## d'après la taille de police et la longueur du texte).
func declutter() -> void:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return
	var screen := get_viewport().get_visible_rect()
	var placed: Array[Rect2] = []
	for label in _priority:
		var world := label.global_position
		if camera.is_position_behind(world):
			label.visible = false
			continue
		var center := camera.unproject_position(world)
		var width := label.text.length() * font_size * 0.52 + declutter_margin * 2.0
		var height := font_size * 1.1 + declutter_margin * 2.0
		var rect := Rect2(center - Vector2(width, height) * 0.5, Vector2(width, height))
		if not screen.intersects(rect):
			label.visible = false
			continue
		var free := true
		for other in placed:
			if other.intersects(rect):
				free = false
				break
		label.visible = free
		if free:
			placed.append(rect)
