extends SceneTree

## Test headless du lot SZ4 (objets à l'échelle aux paliers intermédiaires) :
##  1. `MapPropScale` : 1 au loin (lisibilité stratégique conservée), taille réelle de près,
##     décroissance monotone et continue (aucun saut entre deux distances voisines) ;
##  2. réglages lus depuis `res://resources/map_prop_scale.tres` ;
##  3. carte de campagne (headless) près de Crécy : hameaux, moulins et panaches à l'échelle de la
##     carte au palier stratégique, à leur taille réelle au palier vallée, visibles au palier site ;
##     coût d'une réécriture d'échelle ;
##  4. sol des villes ZG6 : matériau « masse de toits » réglé depuis `town_render.tres`.
## Usage : godot --headless --path game --script res://tests/sz4_prop_scale_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_curve()
	await _test_map()
	if _failures > 0:
		push_error("sz4_prop_scale_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("sz4_prop_scale_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_curve() -> void:
	var props := MapPropScale.shared()
	_check(props != null and ResourceLoader.exists("res://resources/map_prop_scale.tres"), "shared resource")
	for d in [60.0, 150.0, 620.0, props.shrink_start]:
		_check(is_equal_approx(props.tree_scale(d), 1.0) and is_equal_approx(props.hamlet_scale(d), 1.0), "far scale is 1 at d=%.1f" % d)
	_check(is_equal_approx(props.windmill_scale(props.shrink_end), props.windmill_ratio), "real size at shrink_end")
	_check(is_equal_approx(props.hamlet_scale(1.0), props.hamlet_ratio), "real size below shrink_end")
	var previous := 1.0
	var max_jump := 0.0
	var d := props.shrink_start + 1.0
	while d > props.shrink_end - 1.0:
		var s := props.windmill_scale(d)
		_check(s <= previous + 1e-6, "monotonic at d=%.2f" % d)
		max_jump = maxf(max_jump, absf(log(previous) - log(s)))
		previous = s
		d -= 0.05
	# Pas de saut : sur un pas de 0,05 unité, l'échelle varie de moins de 6 %.
	_check(max_jump < 0.06, "continuous (max log step %.3f)" % max_jump)
	_check(not props.needs_rewrite(0.5, 0.51) and props.needs_rewrite(0.5, 0.45) and props.needs_rewrite(0.9, 1.0), "rewrite step")


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
	var data: MapData = map.map_data
	var layer: SettlementLayer = map.get("settlement_layer")
	var life: CampaignLife = map.get("life")
	var crecy := Vector2(2191.5, 1706.0)
	var focus := Vector3(crecy.x, data.surface_world_at(crecy.x, crecy.y), crecy.y)
	var samples := {}
	for d: float in [60.0, 14.0, 6.0, 2.0]:
		rig.look_at_point(focus, d)
		rig.snap()
		for i in 20:
			await process_frame
		layer.flush()
		for i in 3:
			await process_frame
		# Serveur de rendu factice (headless) : les transformations d'instance ne se relisent pas ;
		# échelles appliquées lues dans l'état des couches.
		var effects := life.effects if life != null else null
		var mill: float = effects.get("_windmill_scale") if effects != null else -1.0
		var chimney: Variant = effects.get("_chimney_material").get_shader_parameter("prop_scale") if effects != null else null
		samples[d] = [float(layer.get("_hamlet_scale")) * ModelLibrary.HAMLET_SCALE, mill, chimney, layer.get_node("Hamlets").visible]
	print("sz4_prop_scale_test: samples (hamlet, windmill, chimney, hamlets visible) %s" % samples)
	var hamlet_base := ModelLibrary.HAMLET_SCALE
	var far: Array = samples[60.0]
	_check(is_equal_approx(far[0], hamlet_base), "hamlets at map scale far away")
	_check(is_equal_approx(far[1], 1.0), "windmills at map scale far away")
	var valley: Array = samples[6.0]
	_check(valley[0] > 0.0 and valley[0] < hamlet_base * props.hamlet_scale(6.0) * 1.3, "hamlets shrunk at the valley tier (%s)" % valley[0])
	_check(valley[1] > 0.0 and valley[1] <= props.windmill_scale(6.0) * (1.0 + props.rewrite_step) + 1e-4, "windmills shrunk at the valley tier")
	_check(valley[2] != null and absf(float(valley[2]) - props.chimney_scale(6.0)) < 1e-4, "chimney smoke scale at the valley tier")
	_check(bool((samples[2.0] as Array)[3]), "hamlets kept at the site tier")
	# Coût : une réécriture d'échelle des moulins (toutes les instances).
	if life != null and life.effects != null:
		var t0 := Time.get_ticks_usec()
		life.effects.call("_rewrite_windmills")
		var ms := (Time.get_ticks_usec() - t0) / 1000.0
		var t2 := Time.get_ticks_usec()
		for p: Array in life.effects.get("_windmill_points"):
			life.effects.call("_windmill_transforms", p)
		print("transforms only %.2f ms" % ((Time.get_ticks_usec() - t2) / 1000.0))
		print("sz4_prop_scale_test: windmill rewrite %.2f ms (%d)" % [ms, int(life.effects.stats.get("windmills", 0))])
	# Coût : réécriture d'échelle des hameaux des tuiles chargées.
	layer.set("_hamlet_scale", 0.5)
	var t1 := Time.get_ticks_usec()
	layer.call("_update_hamlet_scale", 60.0)
	print("sz4_prop_scale_test: hamlet rescale %.2f ms for %d instances in %d chunks" % [(Time.get_ticks_usec() - t1) / 1000.0, layer.hamlet_instance_count(), (layer.get("_hamlet_nodes") as Dictionary).size()])
	# Sol des villes : matériau « masse de toits ».
	var profile := TownRenderProfile.load_default()
	var ground := TownBuilder.material(1, false, 0.7, 719.0, 0, true)
	_check(absf(float(ground.get_shader_parameter("roofscape")) - profile.roofscape_strength) < 1e-5, "roofscape material strength")
	var streets := TownBuilder.material(1, false, 0.9, 719.0)
	_check(streets.get_shader_parameter("roofscape") == null or float(streets.get_shader_parameter("roofscape")) == 0.0, "streets without roofscape")
	map.queue_free()
	await process_frame
