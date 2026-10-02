extends SceneTree

## Test headless du lot SZ4b (suites SZ4) :
##  1. courbes : part de la forêt dense (VT3 : pleine de près, nulle au-delà de la portée des arbres) ;
##  2. carte de campagne près de Crécy : emprise réelle, moulins et panaches liés à leur colonie
##     (dans / autour de l'emprise réelle) ; plus de maquette agrandie (ADR 0138) ;
##  3. forêt d'Orléans au palier vallée : couche dense semée, densité et budget d'instances, coût
##     d'une mise à jour ; VT3 : encore semée à d = 20, éteinte au-delà de la portée des arbres.
## Usage : godot --headless --path game --script res://tests/sz4b_colonies_forests_test.gd

const CRECY := Vector2(2191.5, 2986.0)
const ORLEANS_FOREST := Vector2(2180.0, 3333.0)

var _failures := 0


func _init() -> void:
	await process_frame
	_test_curves()
	await _test_map()
	if _failures > 0:
		push_error("sz4b_colonies_forests_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("sz4b_colonies_forests_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_curves() -> void:
	_check(ResourceLoader.exists("res://resources/forest_detail.tres"), "forest detail resource")
	var forest := ForestDetailProfile.shared()
	var props := MapPropScale.shared()
	# VT3 : part pleine de près, plus claire vers la portée des arbres 1:1, nulle au-delà.
	_check(is_equal_approx(forest.fraction_at(1.0), 1.0) and is_equal_approx(forest.fraction_at(forest.dense_full_distance), 1.0), "full dense forest near the ground")
	_check(forest.fraction_at(props.tree_max_distance + 0.5) == 0.0 and forest.fraction_at(200.0) == 0.0, "no dense forest beyond the tree range")
	var previous := 1.0
	var d := 0.5
	while d < props.tree_max_distance + 1.0:
		var f := forest.fraction_at(d)
		_check(f <= previous + 1e-6, "dense share decreasing at d=%.1f" % d)
		previous = f
		d += 0.25
	_check(forest.fraction_at(props.tree_max_distance * 0.75) > 0.2, "dense forest still shown at 3/4 of the range")
	_check(forest.keep_for(0.01) <= 0.0625 and forest.keep_for(0.9) == 1.0, "keep levels")


func _settle(rig: CampaignCamera, focus: Vector3, distance: float, layer: SettlementLayer) -> void:
	rig.look_at_point(focus, distance)
	rig.snap()
	for i in 20:
		await process_frame
	layer.flush()
	for i in 3:
		await process_frame


func _test_map() -> void:
	# GC (ADR 0158) : ce test contrôle les emprises réelles ; il force les villes 1:1.
	TownMaquetteData.set_style(TownMaquetteData.STYLE_REAL)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false  # souris au bord de la fenêtre : pas de défilement pendant l'attente
	var data: MapData = map.map_data
	var layer: SettlementLayer = map.get("settlement_layer")
	var life: CampaignLife = map.get("life")
	var index: int = -1
	var best := INF
	for i in layer.data.settlements.size():
		var dist := (layer.data.settlements[i]["px"] as Vector2).distance_to(CRECY)
		if dist < best:
			best = dist
			index = i
	var focus := Vector3(CRECY.x, data.surface_world_at(CRECY.x, CRECY.y), CRECY.y)
	await _settle(rig, focus, 6.0, layer)
	var real := layer.real_radius(index)
	print("sz4b: %s real radius %.3f u, model radius %.3f u" % [layer.data.settlements[index]["id"], real, layer.model_radius(index)])
	_check(real > 0.0, "real footprint known")
	_check(layer.model_holder(index) == null, "no enlarged model (ADR 0138)")
	# Moulins et panaches de la colonie : autour / dans l'emprise réelle.
	var effects := life.effects if life != null else null
	if _check(effects != null, "life effects"):
		var center := layer.model_px(index)
		var mills := 0
		for point: Array in effects.get("_windmill_points"):
			if int(point[5]) == index:
				mills += 1
				var r := (point[0] as Vector2).distance_to(center)
				_check(r > real * 1.2 and r < real * 2.6, "windmill just outside the real town (%.3f / %.3f)" % [r, real])
		var smokes := 0
		for point: Array in effects.get("_chimney_points"):
			if int(point[3]) == index:
				smokes += 1
				_check((point[0] as Vector2).distance_to(center) <= real * 0.8, "chimney smoke inside the real town")
		print("sz4b: %d windmills, %d chimneys follow the model" % [mills, smokes])
	# Forêt dense.
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	var detail: ForestDetail = vegetation.forest_detail if vegetation != null else null
	if _check(detail != null, "forest detail layer (native scatter)"):
		var forest_focus := Vector3(ORLEANS_FOREST.x, data.surface_world_at(ORLEANS_FOREST.x, ORLEANS_FOREST.y), ORLEANS_FOREST.y)
		await _settle(rig, forest_focus, 6.0, layer)
		# VT3 : les tuiles de base (grilles grossières de la couche dense) ne sont semées qu'en
		# deçà de la portée des arbres : attendre celles du nouveau point visé.
		for i in 600:
			if vegetation.pending_jobs() == 0 and not vegetation.tile_coarse(map.terrain.chunk_index_at(ORLEANS_FOREST.x, ORLEANS_FOREST.y)).is_empty():
				break
			await process_frame
		var t0 := Time.get_ticks_usec()
		detail.flush(ORLEANS_FOREST, 6.0)
		var flush_ms := (Time.get_ticks_usec() - t0) / 1000.0
		for i in 3:
			await process_frame
		var t1 := Time.get_ticks_usec()
		detail.update_view(ORLEANS_FOREST, 6.0, true)
		var update_ms := (Time.get_ticks_usec() - t1) / 1000.0
		print("sz4b: forest detail at d=6 %s ; flush %.0f ms, update %.2f ms" % [detail.stats, flush_ms, update_ms])
		_check(int(detail.stats["cells"]) > 0 and detail.visible_count() > 20000, "dense forest scattered around the focus")
		_check(detail.visible_count() <= ForestDetailProfile.shared().instance_budget * 1.1, "instance budget")
		await _settle(rig, forest_focus, 20.0, layer)
		detail.flush(ORLEANS_FOREST, 20.0)
		detail.update_view(ORLEANS_FOREST, 20.0, true)
		_check(detail.visible_count() > 0 and detail.visible, "dense forest still drawn at d=20 (VT3, trees ~1.5 px)")
		_check(detail.visible_count() <= ForestDetailProfile.shared().instance_budget * 1.1, "instance budget at d=20")
		var beyond := MapPropScale.shared().tree_max_distance + 5.0
		await _settle(rig, forest_focus, beyond, layer)
		detail.update_view(ORLEANS_FOREST, beyond, true)
		_check(detail.visible_count() == 0 and not detail.visible, "dense forest off beyond the tree range")
		_check(not vegetation.visible, "map trees off beyond the tree range (VT3)")
	map.queue_free()
	await process_frame
