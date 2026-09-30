extends SceneTree

## Test headless du lot SZ4b (suites SZ4) :
##  1. courbes : part de la forêt dense (couvert constant) ;
##  2. carte de campagne près de Crécy : emprise réelle, moulins et panaches liés à leur colonie
##     (dans / autour de l'emprise réelle) ; plus de maquette agrandie (ADR 0138) ;
##  3. forêt d'Orléans au palier vallée : couche dense semée, densité et budget d'instances, coût
##     d'une mise à jour ; éteinte au palier comté.
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
	_check(is_zero_approx(forest.fraction_for(1.0)), "no dense forest at map scale")
	_check(forest.fraction_for(forest.full_scale) > 0.99, "full dense forest at real size")
	# Couvert constant : (arbres de base + part dense × densité fine) × s² ≈ couvert de la carte.
	for s: float in [0.5, 0.2, 0.08, 0.04]:
		var cover: float = (1.0 + forest.fraction_for(s) / (forest.full_scale * forest.full_scale)) * s * s
		_check(absf(cover - 1.0) < 0.02, "constant canopy cover at scale %.2f (%.3f)" % [s, cover])
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
		detail.update_view(ORLEANS_FOREST, 20.0, true)
		_check(detail.visible_count() == 0 and not detail.visible, "dense forest off at the county tier")
	map.queue_free()
	await process_frame
