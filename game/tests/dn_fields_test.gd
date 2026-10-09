extends SceneTree

## Test headless du lot DN-CHAMPS (ADR 0220) : champs en modèles générés.
##  1. données : table lue, glb présents (si le paquet de modèles est installé) ;
##  2. `FieldPlan` : déterministe, cultures dominantes par région cohérentes avec les tables du sol
##     (vignes au sud, blé au nord), aucune parcelle en forêt dense ;
##  3. `FieldLayer` : MultiMesh par modèle, budget d'instances, coût de reconstruction.
## Usage : godot --headless --path game --script res://tests/dn_fields_test.gd

var _failures := 0


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("dn_fields_test: " + message)
	return condition


func _init() -> void:
	await process_frame
	await _run()


func _run() -> void:
	var data_dir := OutbuildingLayer.data_dir()
	var config := FieldLayer._read_json(data_dir.path_join(FieldLayer.CONFIG_FILE))
	if not _check(not config.is_empty(), "dn_fields.json not read"):
		_finish()
		return
	var map_data := MapData.load_from_dir(data_dir.path_join("map"))
	_check(map_data.load_error == "", "map load failed: %s" % map_data.load_error)
	var plan := _make_plan(config, map_data, data_dir)
	_check(plan.is_ready(), "plan not ready (splat or biomes missing)")
	_check_plan(plan, config, map_data)
	await _check_layer(map_data)
	_finish()


func _finish() -> void:
	print("dn_fields_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _make_plan(config: Dictionary, map_data: MapData, data_dir: String) -> FieldPlan:
	var plan := FieldPlan.new()
	plan.setup(config, map_data, FieldLayer._read_json(data_dir.path_join(HbGround.MIX_FILE)), FieldLayer._read_json(data_dir.path_join(HbGround.AGRI_FILE)),
		HbGround._load_biomes(data_dir.path_join(HbGround.BIOMES_FILE)), HbGround._load_biomes(data_dir.path_join(HbGround.AGRI_MASK_FILE)), PackedVector2Array())
	return plan


func _check_plan(plan: FieldPlan, config: Dictionary, map_data: MapData) -> void:
	# Tirage entier stable et réparti.
	_check(is_equal_approx(FieldPlan.hash01(3, 4, 5), FieldPlan.hash01(3, 4, 5)), "hash not stable")
	var mean := 0.0
	for i in 2000:
		mean += FieldPlan.hash01(i, i * 7, 13) / 2000.0
	_check(absf(mean - 0.5) < 0.05, "hash mean %.3f" % mean)
	# Cultures dominantes sur toute la carte (pas de 8 px).
	var tally := {}
	var step := 8
	for y in range(0, map_data.size.y, step):
		for x in range(0, map_data.size.x, step):
			var px := Vector2(x, y)
			if not map_data.is_land_px(x, y):
				continue
			var material := plan.dominant_material(px)
			tally[material] = int(tally.get(material, 0)) + 1
	print("dn_fields_test: dominant materials ", tally)
	for needed in ["wheat_ripe", "vineyard_rows", "olive_grove"]:
		_check(int(tally.get(needed, 0)) > 0, "no region dominated by %s" % needed)
	# Parcelles : forêt dense exclue, déterminisme, tailles cohérentes.
	var farmed := 0
	var forested := 0
	var checked := Vector2i.ZERO
	var splat := map_data.splat_image
	for gy in range(1840, 2025, 1):
		for gx in range(1200, 1400, 1):
			var id := Vector2i(gx, gy)
			var site := plan.parcel_site(id)
			if site.x >= map_data.size.x or site.y >= map_data.size.y:
				continue
			var found := plan.parcel(id)
			var sx := clampi(int(site.x / map_data.size.x * splat.get_width()), 0, splat.get_width() - 1)
			var sy := clampi(int(site.y / map_data.size.y * splat.get_height()), 0, splat.get_height() - 1)
			var forest := splat.get_pixel(sx, sy).b
			if found.is_empty():
				continue
			farmed += 1
			if forest > float(config["plan"]["forest_max"]) + 0.01:
				forested += 1
			checked.x += 1
	_check(forested == 0, "%d farmed parcels in dense forest" % forested)
	_check(farmed > 100, "few farmed parcels (%d)" % farmed)
	var a := plan.plan_cell(Vector2i(1048, 1550))
	var b := plan.plan_cell(Vector2i(1048, 1550))
	var same := a.size() == b.size()
	for id: String in a:
		same = same and b.has(id) and (a[id] as PackedFloat32Array) == (b[id] as PackedFloat32Array)
	_check(same, "plan_cell not deterministic")
	# Une instance par parcelle cultivée, à l'échelle de la parcelle.
	var cell_instances := 0
	var scale_ok := true
	var footprint := plan.footprint_m()
	var parcel_m := float(config["plan"]["parcel_px"]) * map_data.meters_per_px
	for cy in range(1540, 1560):
		for cx in range(1040, 1060):
			var cell_plan := plan.plan_cell(Vector2i(cx, cy))
			for id: String in cell_plan:
				var buffer: PackedFloat32Array = cell_plan[id]
				for n in buffer.size() / FieldPlan.STRIDE:
					cell_instances += 1
					var size_m := buffer[n * FieldPlan.STRIDE + 3]
					scale_ok = scale_ok and absf(size_m / footprint - 1.0) <= float(config["plan"]["size_jitter"]) + 0.001 and size_m < parcel_m
	_check(cell_instances > 0, "no parcel instance in sample area")
	_check(scale_ok, "instance size is not the parcel footprint")


func _check_layer(map_data: MapData) -> void:
	var data_dir := OutbuildingLayer.data_dir()
	var data := SettlementData.load_from(data_dir, data_dir.path_join("map"))
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.current = true
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, ZoomTiers.load_default())
	var fields := layer.fields
	if not _check(fields != null and fields.enabled, "no field layer"):
		return
	fields.force_active = true
	var t_warm := Time.get_ticks_msec()
	var missing := 0
	for material: String in fields.config["crops"]:
		for entry: Dictionary in fields.config["crops"][material]["variants"]:
			if not fields.has_model(entry["id"]):
				missing += 1
	print("dn_fields_test: model warm-up %d ms, %d missing" % [Time.get_ticks_msec() - t_warm, missing])
	if missing > 0:
		push_warning("dn_fields_test: %d parcel models not ingested yet (fal generation pending)" % missing)
	var spots := {"beauce": Vector2(2045.0, 3150.0), "bourgogne": Vector2(2377.0, 3340.0), "provence": Vector2(2351.0, 3857.0), "bordelais": Vector2(1790.0, 3718.0)}
	for name_spot: String in spots:
		var focus: Vector2 = spots[name_spot]
		camera.look_at_from_position(Vector3(focus.x, 8.0, focus.y + 8.0), Vector3(focus.x, 0.0, focus.y), Vector3.UP)
		for rig in [10.0, 30.0]:
			fields.update_view(rig)
			fields.flush(focus)
			print("dn_fields_test: %s rig %.0f : %d instances, %d nodes, plan %.1f ms (max cell %.1f), build %.1f ms, models %s" % [name_spot, rig,
				fields.instance_count(), fields.node_count(), fields.stats["plan_ms"], fields.stats["plan_ms_max"], fields.stats["build_ms"], str(fields.shown_models())])
			_check(fields.instance_count() <= int(config_cap(fields)), "instance cap exceeded")
			_check(float(fields.stats["build_ms"]) < 150.0, "rebuild too slow %.1f" % fields.stats["build_ms"])
	fields.force_active = false
	fields.update_view(200.0)
	_check(not fields.visible, "layer visible beyond view range")


func config_cap(fields: FieldLayer) -> int:
	return int(fields.config["render"]["max_instances"])
