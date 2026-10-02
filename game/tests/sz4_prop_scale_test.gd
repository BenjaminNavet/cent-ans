extends SceneTree

## Test headless du lot SZ4 (objets à l'échelle aux paliers intermédiaires) :
##  1. `MapPropScale` : incendies à 1 au loin (lisibilité stratégique conservée), taille réelle de
##     près, décroissance monotone et continue (aucun saut) ; arbres 1:1 constants (VT3) ;
##  2. réglages lus depuis `res://resources/map_prop_scale.tres` ;
##  3. carte de campagne (headless) près de Crécy : hameaux, moulins et panaches de cheminée à
##     l'échelle 1:1 à toute distance (VT, VT2) ; moulins et panaches coupés au-delà de leur
##     portée ; hameaux visibles au palier site ;
##  4. sol des villes ZG6 : matériau « masse de toits » réglé depuis `town_render.tres`.
## Usage : godot --headless --path game --script res://tests/sz4_prop_scale_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	# GC2 (ADR 0158) : ce test porte sur les accessoires et les villes à l'échelle 1:1 (masse de
	# toits de `TownLayer`) : style `real`, le style `maquette` par défaut a son test
	# (`gc_maquettes_test.gd`).
	TownMaquetteData.set_style(TownMaquetteData.STYLE_REAL)
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
		_check(is_equal_approx(props.fire_scale(d), 1.0), "far fire scale is 1 at d=%.1f" % d)
		_check(is_equal_approx(props.hamlet_scale(d), props.hamlet_ratio), "hamlets 1:1 at d=%.1f (VT)" % d)
	_check(is_equal_approx(props.fire_scale(props.shrink_end), props.fire_ratio), "fires at real size at shrink_end")
	# VT3 : arbres 1:1 à toute distance (échelle constante).
	_check(is_equal_approx(props.tree_scale(), props.tree_ratio), "trees 1:1 (VT3)")
	_check(is_equal_approx(props.hamlet_scale(1.0), props.hamlet_ratio), "real size below shrink_end")
	# VT2 : moulins et panaches 1:1 à toute distance (échelle constante, sans argument).
	_check(is_equal_approx(props.windmill_scale(), props.windmill_ratio), "windmills 1:1 (VT2)")
	_check(is_equal_approx(props.chimney_scale(), props.chimney_ratio), "chimney smoke 1:1 (VT2)")
	# Taille réelle : moulin de 15-25 m hors tout (faîte 0,36, ailes 0,576 u de modèle), panache de
	# 20-40 m de haut (1 u = 719 m).
	var mill_m := (0.3 + 0.288) * LifeEffects.WINDMILL_SCALE * props.windmill_scale() * 719.0
	var plume_m := LifeEffects.CHIMNEY_SIZE.y * props.chimney_scale() * 719.0
	_check(mill_m > 15.0 and mill_m < 25.0, "real windmill height %.1f m" % mill_m)
	_check(plume_m > 20.0 and plume_m < 40.0, "real chimney plume height %.1f m" % plume_m)
	# Portées : moulins et panaches coupés au-delà, panaches éteints en fondu.
	_check(props.windmills_visible(props.windmill_max_distance - 1.0) and not props.windmills_visible(props.windmill_max_distance + 1.0), "windmill range")
	_check(is_equal_approx(props.chimney_alpha(1.0), props.chimney_real_alpha) and props.chimney_alpha(props.chimney_max_distance + 0.1) == 0.0, "chimney range")
	_check(props.chimney_alpha(props.chimney_max_distance * (1.0 - props.visibility_fade * 0.5)) < props.chimney_real_alpha, "chimney fade")
	var previous := 1.0
	var max_jump := 0.0
	var d := props.shrink_start + 1.0
	while d > props.shrink_end - 1.0:
		var s := props.fire_scale(d)
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
	var crecy := Vector2(2191.5, 2986.0)
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
		var mills: bool = effects.get_node("WindmillBodies").visible if effects != null else false
		var chimneys: bool = effects.get_node("Chimneys").visible if effects != null else false
		samples[d] = [float(layer.get("_hamlet_scale")) * ModelLibrary.HAMLET_SCALE, mills, chimneys, layer.get_node("Hamlets").visible]
	print("sz4_prop_scale_test: samples (hamlet, windmills shown, chimneys shown, hamlets visible) %s" % samples)
	var hamlet_base := ModelLibrary.HAMLET_SCALE
	var far: Array = samples[60.0]
	_check(is_equal_approx(far[0], hamlet_base * props.hamlet_ratio), "hamlets at real size far away (VT: 1:1 at every distance)")
	_check(not bool(far[1]) and not bool(far[2]), "windmills and chimney smoke culled beyond their range (VT2)")
	var valley: Array = samples[6.0]
	_check(is_equal_approx(valley[0], hamlet_base * props.hamlet_ratio), "hamlets still real size at the valley tier (%s)" % valley[0])
	_check(bool(valley[1]) and bool((samples[14.0] as Array)[1]), "windmills shown within their range")
	_check(bool(valley[2]), "chimney smoke shown within its range")
	_check(bool((samples[2.0] as Array)[3]), "hamlets kept at the site tier")
	# VT2 : instances à taille réelle (moulins : base du corps ; panaches : hauteur du quad).
	if life != null and life.effects != null:
		var buffers: Dictionary = life.effects.get("_cpu_buffers")
		var bodies: MultiMeshInstance3D = life.effects.get_node("WindmillBodies")
		var body_buffer: PackedFloat32Array = buffers.get(bodies, PackedFloat32Array())
		if _check(body_buffer.size() >= 12, "windmill instances"):
			var basis_x := Vector3(body_buffer[0], body_buffer[4], body_buffer[8]).length()
			_check(is_equal_approx(basis_x, LifeEffects.WINDMILL_SCALE * props.windmill_scale()), "windmill instance at real size (%.5f)" % basis_x)
		var plumes: MultiMeshInstance3D = life.effects.get_node("Chimneys")
		var plume_buffer: PackedFloat32Array = buffers.get(plumes, PackedFloat32Array())
		if _check(plume_buffer.size() >= 16, "chimney instances"):
			var height := Vector3(plume_buffer[1], plume_buffer[5], plume_buffer[9]).length()
			var real := LifeEffects.CHIMNEY_SIZE.y * props.chimney_scale()
			_check(height >= real * 0.79 and height <= real * 1.21, "chimney plume at real size (%.4f)" % height)
		var material: ShaderMaterial = life.effects.get("_chimney_material")
		var applied: Variant = material.get_shader_parameter("prop_scale")
		_check(applied == null or is_equal_approx(float(applied), 1.0), "chimney material not rescaled")
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
