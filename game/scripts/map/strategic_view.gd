class_name StrategicView
extends Node

## Lot CM2 : vue stratégique « parchemin enluminé » au zoom
## maximal. Pilote le paramètre global de shader `campaign_parchment` (fondu selon la
## distance caméra : terrain, mer, fleuves passent en carte dessinée, cf.
## `parchment_*.gdshaderinc`), la couche 2D (`ParchmentOverlay` : noms, vignettes, jetons,
## navires et monstres) et retire en fondu les couches 3D qui la doublent (étiquettes de
## provinces, étendards et plaques d'armées). Purement visuel.
## Options (après `--`) : `--no-parchment` (A/B), `--parchment=<0..1>` (poids imposé).

## Bande de fondu (distance caméra) : carte 3D en deçà de `start`, parchemin au-delà de `end`.
@export var start_distance: float = 1180.0
@export var end_distance: float = 1440.0
## Au-delà de ce poids, les marqueurs 3D d'armée cèdent la place aux jetons.
@export var marker_cutoff: float = 0.6

var weight: float = 0.0
var enabled: bool = true
var forced: float = -1.0
var decor: ParchmentDecor
var overlay: ParchmentOverlay

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
const PARCHMENT_SHADER := preload("res://shaders/terrain_parchment.gdshader")
## Poids à partir duquel le terrain ne calcule plus que le parchemin (shader substitué).
const FULL_WEIGHT := 0.999

var _map: Node
var _parchment_only: bool = false
var _layer: CanvasLayer
var _markers_hidden: bool = false


func setup(map: Node) -> void:
	_map = map
	for arg in OS.get_cmdline_user_args():
		if arg == "--no-parchment":
			enabled = false
		elif arg.begins_with("--parchment="):
			forced = clampf(float(arg.trim_prefix("--parchment=")), 0.0, 1.0)
	RenderingServer.global_shader_parameter_set("campaign_parchment", 0.0)
	if not enabled:
		return
	var map_data: MapData = map.get("map_data")
	decor = ParchmentDecor.build(map_data)
	var sea := map.get("sea") as MeshInstance3D
	if sea != null and sea.material_override is ShaderMaterial:
		var roses: Array[Vector4] = []
		for rose in decor.roses:
			roses.append(Vector4(rose.x, rose.y, rose.z, 1.0))
		while roses.size() < 4:
			roses.append(Vector4.ZERO)
		var material := sea.material_override as ShaderMaterial
		material.set_shader_parameter("pm_roses", roses)
		material.set_shader_parameter("pm_rose_count", decor.roses.size())
	_layer = CanvasLayer.new()
	_layer.name = "Parchment"
	_layer.layer = 0
	add_child(_layer)
	overlay = ParchmentOverlay.new()
	overlay.name = "ParchmentOverlay"
	overlay.camera = map.get("camera")
	overlay.map_data = map_data
	overlay.decor = decor
	overlay.armies = map.get("armies")
	overlay.visible = false
	_layer.add_child(overlay)


## Poids du parchemin [0, 1] pour une distance caméra.
func weight_at(distance: float) -> float:
	if not enabled:
		return 0.0
	if forced >= 0.0:
		return forced
	return smoothstep(start_distance, end_distance, distance)


func refresh(sim: Object) -> void:
	if overlay != null:
		overlay.refresh(sim, _map.get("settlement_data"))


func update_view(distance: float) -> void:
	if not enabled:
		return
	var w := weight_at(distance)
	if not is_equal_approx(w, weight):
		weight = w
		RenderingServer.global_shader_parameter_set("campaign_parchment", w)
		_swap_terrain_shader(w >= FULL_WEIGHT)
	overlay.set_weight(w, distance)
	_fade_armies(w)


## Au poids 1, le matériau partagé du terrain passe au shader « parchemin seul » (mêmes
## uniformes, valeurs conservées) : le rendu 3D n'est plus payé.
func _swap_terrain_shader(parchment_only: bool) -> void:
	if parchment_only == _parchment_only:
		return
	var terrain := _map.get("terrain") as TerrainBuilder
	if terrain == null or terrain.material == null:
		return
	_parchment_only = parchment_only
	terrain.material.shader = PARCHMENT_SHADER if parchment_only else TERRAIN_SHADER


## Étendards 3D et plaques d'effectif s'effacent devant les jetons à blason.
func _fade_armies(w: float) -> void:
	var armies := _map.get("armies") as ArmyMarkers
	if armies == null:
		return
	var hide_markers := w > marker_cutoff
	if hide_markers != _markers_hidden:
		_markers_hidden = hide_markers
		for marker in armies._markers.values():
			marker.visible = not hide_markers
	elif hide_markers:
		for marker in armies._markers.values():
			if marker.visible:
				marker.visible = false
	for plate in armies._plates.values():
		(plate as Control).modulate.a = 1.0 - smoothstep(0.2, marker_cutoff, w)
