class_name ConstructionMarkers
extends Node3D

## Chantier visible sur les provinces en construction (`get_province_city().construction`) :
## lot TB3 (ADR 0162), un échafaudage et son tas de pierres (`outbuildings/worksite_1.glb`) à la
## place du marteau « ⚒ » en `Label3D`. C'est un **signe** de carte (TB2, ADR 0151) : il garde une
## taille constante à l'écran, comme l'ancien marteau, et ne s'affiche que dans la couche
## « Signes » et les modes de carte du bloc `signs.construction`. Le chantier à l'échelle réelle,
## au pied de la ville, est posé par `OutbuildingLayer`.
## Reconstruit à chaque `refresh` ; vide si la simulation n'expose pas `get_province_city`.

const MODEL := "res://assets/models/outbuildings/worksite_1.glb"
## Largeur de la maquette (m) ramenée à `screen_fraction` de la distance caméra.
const MODEL_SPAN_M := 34.0

## Taille à l'écran : largeur du chantier en fraction de la distance à la caméra (≈ 40 px).
@export var screen_fraction: float = 0.04
## Lacet de présentation (rad) : l'échafaudage de trois quarts.
@export var yaw: float = 0.6

var _markers: Array[MeshInstance3D] = []
var _mesh: Mesh = null
var _mesh_loaded := false


## `province_ids` : ids détenus par le joueur (ou tous, au choix de l'appelant) à vérifier ;
## `is_building(id) -> bool` et `world_position_of(id) -> Vector3` fournis par l'appelant.
func refresh(province_ids: PackedStringArray, is_building: Callable, world_position_of: Callable) -> void:
	for child in get_children():
		child.queue_free()
	_markers.clear()
	var mesh := worksite_mesh()
	var map_data: Variant = get_parent().get("map_data") if get_parent() != null else null
	for province_id in province_ids:
		if not bool(is_building.call(province_id)):
			continue
		var marker := MeshInstance3D.new()
		marker.name = "Worksite_" + province_id
		marker.mesh = mesh
		marker.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var at: Vector3 = world_position_of.call(province_id)
		if map_data is MapData:  # posé au sol : un chantier ne flotte pas
			at.y = (map_data as MapData).surface_world_at(at.x, at.z)
		marker.position = at
		marker.rotation.y = yaw
		add_child(marker)
		_markers.append(marker)
	_rescale()


## Maillage du chantier aux matériaux partagés des bâtiments (null si la maquette manque).
func worksite_mesh() -> Mesh:
	if not _mesh_loaded:
		_mesh_loaded = true
		_mesh = BuildingMaterials.remap_mesh(OutbuildingLayer._load_glb_mesh(MODEL), "far")
	return _mesh


## TB2 : le signe doublait l'écu de la ville ; il ne s'affiche que dans la couche « Signes » et
## les modes de carte du bloc `signs.construction` (`MapReadability.sign_shown`).
func _process(_delta: float) -> void:
	visible = MapReadability.sign_shown("construction", MapReadability.map_mode_of(get_parent()))
	if visible:
		_rescale()


## Taille constante à l'écran : échelle proportionnelle à la distance à la caméra.
func _rescale() -> void:
	var camera := get_viewport().get_camera_3d() if is_inside_tree() else null
	if camera == null:
		return
	var eye := camera.global_position
	for marker in _markers:
		if is_instance_valid(marker):
			marker.scale = Vector3.ONE * marker_scale(eye.distance_to(marker.global_position))


## Échelle (unités monde par mètre de maquette) d'un chantier vu à `distance`.
func marker_scale(distance: float) -> float:
	return maxf(distance, 0.01) * screen_fraction / MODEL_SPAN_M


func marker_count() -> int:
	return _markers.size()
