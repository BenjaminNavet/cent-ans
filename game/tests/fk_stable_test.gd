extends TestCase

## Test headless du lot CV-B (ADR 0354) : taille monde fixe et aucun pop de la carte vivante.
##  1. taille fixe : l'échelle des figurines ne dépend pas de la distance ; un homme ≥ 1,2 unité ;
##  2. semis ancré sur le monde : on place, on déplace le point visé d'un petit pas, on zoome de
##     15 %, on replace de force : toute instance commune garde position, trajectoire (données
##     d'instance) et aléa ; aucune instance sous le seuil ne disparaît dans la zone intérieure,
##     aucune n'apparaît ;
##  3. éclaircie : le seuil d'aléa décroît avec la distance (même instances effacées) ;
##  4. fondus : uniforms de zone (point visé courant), de seuil et de portée posés à chaque image ;
##     au-delà de la portée tout est vidé (déjà fondu avant).
## Usage : godot --headless --path game --script res://tests/fk_stable_test.gd

const D := 60.0


func _init() -> void:
	await process_frame
	await _run()
	FolkModels.clear_cache()
	finish()


func _view(pool: FolkPool, focus: Vector2, distance: float) -> void:
	pool.update_view(focus, distance, 1.0)
	pool.update_view(focus, distance, 1.0)


## Instances indexées par groupe et origine (au 1/1000 d'unité).
func _index(records: Array) -> Dictionary:
	var out := {}
	for record in records:
		var origin: Vector2 = record["origin"]
		out["%s|%d|%d" % [record["group"], roundi(origin.x * 1000.0), roundi(origin.y * 1000.0)]] = record
	return out


func _run() -> void:
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var mask := TerroirMask.new()
	mask.build(data.settlements, data.hamlets, {}, VegetationFields.landuse(map_data), Vector2(map_data.size))
	var paris: Vector2 = data.get_settlement("set_paris")["px"]

	var pool := FolkPool.new()
	world.add_child(pool)
	pool.setup(map_data, terrain, 4000)
	pool.set_maquette(true)
	pool.warm_all()
	var routine := FolkRoutine.new()
	routine.setup(map_data, data)
	routine.terroir = mask
	routine.season = "summer"
	routine.cities = PackedVector2Array([paris])
	pool.register(routine)
	pool.refresh(null)

	# 1. Taille fixe.
	var folk_scale := TownMaquetteData.prop("folk_scale", 1.0)
	var man_units := folk_scale / map_data.meters_per_px * 1.8
	check(man_units >= 1.2, "a man is %.2f units tall (>= 1.2)" % man_units)
	var scales := {}
	for distance in [5.0, 30.0, 90.0, 200.0]:
		_view(pool, paris, distance)
		scales[distance] = pool.current_scale()
	check(is_equal_approx(scales[5.0], scales[200.0]) and is_equal_approx(scales[30.0], scales[90.0]), "scale independent of the camera distance: %s" % [scales])
	check(is_equal_approx(scales[5.0], folk_scale / map_data.meters_per_px), "scale = folk_scale: %.5f" % scales[5.0])

	# 2. Semis ancré : petit déplacement + zoom de 15 %.
	var focus_a := paris + Vector2(40.0, 25.0)
	_view(pool, focus_a, D)
	var placements := int(pool.stats["placements"])
	var records_a := pool.instance_records()
	var radius_a := pool.active_radius(D)
	var keep_a := pool.keep_top(D)
	check(records_a.size() > 20, "enough instances to compare: %d" % records_a.size())
	var focus_b := focus_a + Vector2(radius_a * 0.1, -radius_a * 0.07)
	var distance_b := D * 1.15
	_view(pool, focus_b, distance_b)
	pool.invalidate()
	_view(pool, focus_b, distance_b)
	check(int(pool.stats["placements"]) > placements, "a new placement happened")
	var records_b := pool.instance_records()
	var radius_b := pool.active_radius(distance_b)
	var keep_b := pool.keep_top(distance_b)
	var index_a := _index(records_a)
	var index_b := _index(records_b)
	var inner := minf(radius_a, radius_b) * pool.edge_inner * 0.95
	var keep := minf(keep_a, keep_b) * 0.98
	var compared := 0
	var missing := 0
	var changed := 0
	for key in index_a:
		var record: Dictionary = index_a[key]
		var origin: Vector2 = record["origin"]
		if origin.distance_to(focus_a) > inner or origin.distance_to(focus_b) > inner or float(record["h"]) > keep:
			continue
		compared += 1
		if not index_b.has(key):
			missing += 1
			continue
		var other: Dictionary = index_b[key]
		if absf(float(other["h"]) - float(record["h"])) > 1e-3 or not (other["custom"] as Color).is_equal_approx(record["custom"]):
			changed += 1
	print("fk_stable_test: %d → %d instances, %d compared, ceiling %.2f, keep %.3f/%.3f" % [records_a.size(), records_b.size(), compared, pool.ceiling(), keep_a, keep_b])
	check(compared > 10, "instances compared in the inner zone: %d" % compared)
	check(missing == 0, "no instance vanishes in the inner zone: %d of %d" % [missing, compared])
	check(changed == 0, "common instances keep trajectory, phase and hazard: %d changed" % changed)
	var appeared := 0
	for key in index_b:
		var record: Dictionary = index_b[key]
		var origin: Vector2 = record["origin"]
		if origin.distance_to(focus_a) <= inner and origin.distance_to(focus_b) <= inner and float(record["h"]) <= keep and not index_a.has(key):
			appeared += 1
	check(appeared == 0, "no instance appears in the inner zone: %d" % appeared)

	# 2b. Attelage : chaque cheval en marche partage longueur, vitesse et phase d'un accessoire
	# (charrette) : ils bouclent ensemble (sinon le cheval repart derrière la charrette).
	var horses := 0
	var unhitched := 0
	for record in records_b:
		var custom: Color = record["custom"]
		if not str(record["group"]).contains("horse") or custom.r <= 0.0:
			continue
		horses += 1
		var hitched := false
		for other in records_b:
			var c: Color = other["custom"]
			if other["group"] != record["group"] and absf(c.r - custom.r) < 1e-3 and absf(c.g - custom.g) < 1e-4 and absf(c.b - custom.b) < 1e-3:
				hitched = true
				break
		unhitched += 0 if hitched else 1
	print("fk_stable_test: %d walking horses, %d unhitched" % [horses, unhitched])
	check(horses > 0 and unhitched == 0, "every walking horse loops with its cart: %d of %d unhitched" % [unhitched, horses])

	# 3. Éclaircie : mêmes instances effacées (h croissant), seuil décroissant avec la distance.
	check(pool.keep_for(30.0) >= pool.keep_for(60.0) and pool.keep_for(60.0) > pool.keep_for(200.0), "keep decreases with distance")
	var near_ceiling := pool.ceiling_for(40.0)
	var far_ceiling := pool.ceiling_for(120.0)
	check(far_ceiling < near_ceiling, "ceiling %.2f < %.2f" % [far_ceiling, near_ceiling])
	_view(pool, focus_a, 120.0)
	var hazards_far := 0.0
	var count_far := 0
	for record in pool.instance_records():
		hazards_far = maxf(hazards_far, float(record["h"]))
		count_far += 1
	check(count_far > 0 and hazards_far < far_ceiling + 1e-3, "far placement only holds h < ceiling: max %.3f / %.3f (%d)" % [hazards_far, far_ceiling, count_far])
	var far_set := _index(pool.instance_records())
	_view(pool, focus_a, 40.0)
	var near_set := _index(pool.instance_records())
	var lost := 0
	for key in far_set:
		var record: Dictionary = far_set[key]
		var origin: Vector2 = record["origin"]
		# CC : les tronçons de la carte composée sont longs (médiane 10 px, p95 100 px) et choisis par leur
		# milieu : l'origine d'un voyageur peut être à ~20 px de plus que le disque, d'où cette marge.
		if not near_set.has(key) and origin.distance_to(focus_a) < pool.active_radius(40.0) * 0.9 - 20.0:
			lost += 1
	check(lost == 0, "zooming in keeps every instance already shown far away: %d lost" % lost)

	# 4. Fondus : uniforms posés chaque image.
	_view(pool, focus_b, D)
	var checked := 0
	for child in pool.get_children():
		var mmi := child as MultiMeshInstance3D
		if mmi == null or mmi.multimesh.instance_count == 0:
			continue
		var material: Material = mmi.material_override
		if material == null and mmi.multimesh.mesh != null:
			material = mmi.multimesh.mesh.surface_get_material(0)
		if not material is ShaderMaterial:
			continue
		var zone: Vector4 = (material as ShaderMaterial).get_shader_parameter("fk_zone")
		var thin: Vector4 = (material as ShaderMaterial).get_shader_parameter("fk_thin")
		check(is_equal_approx(zone.x, focus_b.x) and is_equal_approx(zone.y, focus_b.y), "%s: fade zone follows the aimed point" % mmi.name)
		check(zone.z < zone.w and absf(zone.w - pool.active_radius(D)) < 1e-3, "%s: fade ring inside the view radius" % mmi.name)
		check(absf(thin.x - pool.keep_top(D)) < 1e-4 and is_equal_approx(thin.z, 1.0), "%s: thinning threshold and range" % mmi.name)
		checked += 1
	check(checked > 0, "fade uniforms checked on %d groups" % checked)
	# Portée : fondu avant le vidage.
	_view(pool, focus_b, pool.figure_max_distance * 0.95)
	var edge_group := false
	for child in pool.get_children():
		var mmi := child as MultiMeshInstance3D
		if mmi != null and mmi.multimesh.instance_count > 0:
			var material: Material = mmi.material_override
			if material is ShaderMaterial:
				edge_group = true
				var thin: Vector4 = (material as ShaderMaterial).get_shader_parameter("fk_thin")
				check(thin.z < 1.0 and thin.z > 0.0, "range fade under way before the cutoff: %.2f" % thin.z)
				break
	check(edge_group or pool.figure_count() == 0, "range fade checked")
	_view(pool, focus_b, pool.figure_max_distance + 5.0)
	check(pool.figure_count() == 0 and not pool.visible, "emptied beyond the range")

	world.queue_free()
	await process_frame
