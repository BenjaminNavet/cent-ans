class_name PathPreview
extends MeshInstance3D

## Aperçu de chemin : ruban orange posé sur le relief entre les centroïdes des provinces
## du chemin (segments subdivisés pour suivre le terrain). Largeur selon la distance caméra.
## ZG7a : aux paliers vallée et site, ruban fin (largeur ≈ constante en pixels, plus de
## plancher de 0,8 unité ≈ 575 m), soulèvement proportionnel à la distance (0,6 unité ≈ 430 m
## au-dessus du sol de près auparavant) et subdivision plus fine ; reconstruit quand la
## distance de la caméra a changé de plus de `REBUILD_RATIO` (`update_view`, chaque image).

const SUBDIVISION_PX := 12.0
## Largeur (unités monde) par unité de distance caméra : ≈ 5 px à l'écran.
const WIDTH_PER_DISTANCE := 0.005
const MAX_WIDTH := 8.0
## Largeur minimale de près (unités, ≈ 7 m) ; les anciens planchers (0,8 et 0,25) ne valent
## plus qu'au-delà de `FAR_DISTANCE`.
const NEAR_MIN_WIDTH := 0.01
const FAR_DISTANCE := 160.0
## Soulèvement par unité de distance, borné à `lift` (valeur historique, vue stratégique).
const LIFT_PER_DISTANCE := 0.004
const MIN_LIFT := 0.002
## Pas de subdivision par unité de distance (px carte), borné à `SUBDIVISION_PX`.
const STEP_PER_DISTANCE := 0.04
const MIN_STEP := 0.04
## Plafond de points (coût de reconstruction : une hauteur par point).
const MAX_POINTS := 2000
const REBUILD_RATIO := 1.3

@export var color: Color = Color(1.0, 0.55, 0.15, 0.95)
@export var lift: float = 0.6

var map_data: MapData
var _shown_ids: PackedStringArray = PackedStringArray()
## Sommets du chemin (non subdivisés) et plancher de largeur de la vue stratégique.
var _waypoints := PackedVector2Array()
var _far_min_width := 0.8
var _built_distance := -1.0


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
		points.append(centroid)
	_show(points, camera_distance, 0.8)


## Lot C5 : chemin sur le graphe des colonies. `points` : positions carte (x, z) des colonies
## successives (départ compris) ; chaque arête est un segment subdivisé posé sur le relief.
## `ids` : identifiants des colonies (retournés par `shown_ids`).
func show_points(points_in: PackedVector2Array, camera_distance: float, ids: PackedStringArray = PackedStringArray()) -> void:
	if points_in.size() < 2 or map_data == null:
		hide_path()
		return
	_shown_ids = ids
	_show(points_in, camera_distance, 0.25)


func hide_path() -> void:
	_shown_ids = PackedStringArray()
	_waypoints = PackedVector2Array()
	visible = false


func shown_ids() -> PackedStringArray:
	return _shown_ids


## ZG7a : appelé à chaque image par `CampaignMap` ; reconstruit le ruban si la distance de la
## caméra a assez changé (largeur, soulèvement et subdivision en dépendent).
func update_view(camera_distance: float) -> void:
	if not visible or _waypoints.size() < 2 or _built_distance <= 0.0:
		return
	var ratio := camera_distance / _built_distance
	if ratio > REBUILD_RATIO or ratio < 1.0 / REBUILD_RATIO:
		_build(camera_distance)


## Largeur (unités monde) du ruban à cette distance : plancher historique en vue stratégique,
## ruban fin de près.
func width_at(camera_distance: float) -> float:
	var floor_width := lerpf(NEAR_MIN_WIDTH, _far_min_width, smoothstep(FAR_DISTANCE * 0.25, FAR_DISTANCE, camera_distance))
	return clampf(camera_distance * WIDTH_PER_DISTANCE, floor_width, MAX_WIDTH)


func lift_at(camera_distance: float) -> float:
	return clampf(camera_distance * LIFT_PER_DISTANCE, MIN_LIFT, lift)


func _show(waypoints: PackedVector2Array, camera_distance: float, far_min_width: float) -> void:
	_waypoints = waypoints
	_far_min_width = far_min_width
	if _waypoints.size() < 2:
		hide_path()
		return
	_build(camera_distance)
	visible = true


func _build(camera_distance: float) -> void:
	_built_distance = maxf(camera_distance, 0.001)
	var length := 0.0
	for i in range(1, _waypoints.size()):
		length += _waypoints[i - 1].distance_to(_waypoints[i])
	var step := clampf(camera_distance * STEP_PER_DISTANCE, MIN_STEP, SUBDIVISION_PX)
	step = maxf(step, length / MAX_POINTS)
	var points := PackedVector2Array([_waypoints[0]])
	for i in range(1, _waypoints.size()):
		var previous := _waypoints[i - 1]
		var target := _waypoints[i]
		var steps := maxi(int(previous.distance_to(target) / step), 1)
		for k in range(1, steps + 1):
			points.append(previous.lerp(target, float(k) / steps))
	mesh = PolylineMesh.build([points], [width_at(camera_distance)], map_data, lift_at(camera_distance))
