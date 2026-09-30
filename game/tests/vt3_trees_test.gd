extends SceneTree

## Test headless du chantier VT3 (arbres de la carte à l'échelle 1:1, ADR 0138 addendum VT3) :
##  1. `MapPropScale` : échelle des arbres constante, hauteurs réelles 12-30 m par essence, portée
##     où un arbre de 20 m fait ≈ 1 px (1080p, fov 55°) ;
##  2. carte de campagne (headless) : arbres individuels (tuiles et forêt dense) coupés au-delà de
##     la portée, présents en deçà ; forêt dense pleine près du point visé.
## Usage : godot --headless --path game --script res://tests/vt3_trees_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_scale()
	await _test_map()
	if _failures > 0:
		push_error("vt3_trees_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("vt3_trees_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


const ORLEANS_FOREST := Vector2(2180.0, 3333.0)


func _test_scale() -> void:
	var props := MapPropScale.shared()
	_check(is_equal_approx(props.tree_scale(), props.tree_ratio), "constant tree scale")
	# Hauteurs de modèle extrêmes par essence (VegetationTileJob._make_instance) → mètres.
	var ranges := {"oak": Vector2(1.1, 1.7), "beech": Vector2(1.35, 1.95), "conifer": Vector2(1.3, 2.1)}
	for essence: String in ranges:
		var r: Vector2 = ranges[essence] * props.tree_scale() * 719.0
		_check(r.x >= 12.0 and r.y <= 30.0, "%s real height %.1f-%.1f m" % [essence, r.x, r.y])
	# Portée : un arbre de 20 m y fait ≈ 1 px (1080p, fov 55°).
	var px := MapPropScale.pixels_for(20.0, props.tree_view_range)
	_check(px > 0.7 and px < 1.4, "20 m tree at the view range: %.2f px" % px)
	_check(props.trees_visible(props.tree_max_distance - 1.0) and not props.trees_visible(props.tree_max_distance + 0.1), "tree range")
	_check(props.trees_weight(1.0) == 1.0 and props.trees_weight(props.tree_max_distance) == 0.0, "tree range weight")


func _settle(rig: CampaignCamera, focus: Vector3, distance: float) -> void:
	rig.look_at_point(focus, distance)
	rig.snap()
	for i in 20:
		await process_frame


func _test_map() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var props := MapPropScale.shared()
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.map_data
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	if not _check(vegetation != null, "vegetation layer"):
		map.queue_free()
		return
	var focus := Vector3(ORLEANS_FOREST.x, data.surface_world_at(ORLEANS_FOREST.x, ORLEANS_FOREST.y), ORLEANS_FOREST.y)
	for d: float in [300.0, 60.0]:
		await _settle(rig, focus, d)
		_check(not vegetation.visible, "no individual trees at d=%.0f" % d)
		_check(vegetation.forest_detail == null or not vegetation.forest_detail.visible, "no dense forest at d=%.0f" % d)
	await _settle(rig, focus, 10.0)
	_check(vegetation.visible and vegetation.tile_count() > 0, "map trees drawn at d=10")
	# Hauteurs réelles des arbres semés (hors haies) : 12-30 m pour l'essentiel.
	var heights: Array[float] = []
	for entry: Dictionary in (vegetation.get("_tiles") as Dictionary).values():
		var buffers: Array = entry["buffers"]
		for slot in buffers.size():
			if slot % VegetationTileJob.KIND_COUNT == VegetationTileJob.Kind.HEDGE:
				continue
			var buffer: PackedFloat32Array = buffers[slot]
			var k := 0
			while k + 15 < buffer.size() and heights.size() < 4000:
				heights.append(Vector3(buffer[k + 1], buffer[k + 5], buffer[k + 9]).length() * props.tree_scale() * 719.0)
				k += 16 * 7
	heights.sort()
	if _check(heights.size() > 50, "tree instances sampled (%d)" % heights.size()):
		var median := heights[heights.size() / 2]
		print("vt3_trees_test: tree heights p10 %.1f, median %.1f, p90 %.1f m" % [heights[heights.size() / 10], median, heights[heights.size() * 9 / 10]])
		_check(median > 12.0 and median < 26.0, "median tree height %.1f m" % median)
	map.queue_free()
	await process_frame
