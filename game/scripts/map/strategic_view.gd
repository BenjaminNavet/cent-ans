class_name StrategicView
extends Node

## Lot CM2 : vue stratégique « parchemin enluminé » au zoom
## maximal. Pilote le paramètre global de shader `campaign_parchment` (fondu selon la
## distance caméra : terrain, mer, fleuves passent en carte dessinée, cf.
## `parchment_*.gdshaderinc`), la couche 2D (`ParchmentOverlay` : noms, vignettes, jetons,
## navires et monstres) et retire en fondu les couches 3D qui la doublent (étiquettes de
## provinces, étendards et plaques d'armées). Purement visuel.
## Options (après `--`) : `--no-parchment` (A/B), `--parchment=<0..1>` (poids imposé).

## Lot DV (ADR 0124) : la bande de fondu vue normale → parchemin est celle de
## `ZoomTiers.strategic_weight` (seuil 1200, largeur 200), seule source des deux vues.
## Au-delà de ce poids, les marqueurs 3D d'armée cèdent la place aux jetons.
@export var marker_cutoff: float = 0.6

var weight: float = 0.0
var enabled: bool = true
var forced: float = -1.0
var decor: ParchmentDecor
var overlay: ParchmentOverlay
var region_labels: RegionLabels  # TB2

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
const PARCHMENT_SHADER := preload("res://shaders/terrain_parchment.gdshader")
## Poids à partir duquel le terrain ne calcule plus que le parchemin (shader substitué).
const FULL_WEIGHT := 0.999

var _map: Node
var _tiers: ZoomTiers
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
	# TB2 : noms de région de la vue moyenne (même source que les noms du parchemin).
	region_labels = RegionLabels.new()
	region_labels.name = "RegionLabels"
	region_labels.camera = overlay.camera
	region_labels.source = overlay
	region_labels.visible = false
	region_labels.obstacles = func(view_camera: Camera3D) -> Array:
		var layer: Variant = map.get("settlement_layer")
		return (layer as SettlementLayer).screen_label_rects(view_camera) if layer is SettlementLayer else []
	region_labels.top_inset = func() -> float:
		var ui: Variant = map.get("ui")
		var bar := (ui as Node).get_node_or_null("TopBar") as Control if ui is Node else null
		return bar.get_global_rect().end.y if bar != null and bar.visible else 0.0
	_layer.add_child(region_labels)


## Poids du parchemin [0, 1] pour une distance caméra.
func weight_at(distance: float) -> float:
	if not enabled:
		return 0.0
	if forced >= 0.0:
		return forced
	if _tiers == null:
		_tiers = _map.get("zoom_tiers") as ZoomTiers if _map != null else null
		if _tiers == null:
			_tiers = ZoomTiers.load_default()
	return _tiers.strategic_weight(distance)


func refresh(sim: Object) -> void:
	if overlay != null:
		# DV (ADR 0124, remplace DA3) : plus de marqueurs peints ; sur le parchemin, les villes
		# sont les vignettes à l'encre de la couche 2D.
		overlay.draw_towns = true
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
	if region_labels != null:  # TB2
		region_labels.set_view(distance, w)
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
