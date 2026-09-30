extends SceneTree

## VT-E (ADR 0138) : calque du lointain des villes (`TownFarLayer`), sans pyramide ni terrain.
##  1. génération complète image par image (fils de travail + construction budgétée) : temps réel,
##     temps principal max par image ;
##  2. tuiles F1/F2 : nombre, triangles, portées de visibilité, ombres ;
##  3. masque qui suit des `built_ids()` simulés (ville ordinaire et ville v2), vidé ensuite ;
##  4. calque masqué en vue stratégique ;
##  5. préréglage bas : portée F1 réduite.
## Usage : godot --headless --path game --script res://tests/tf_far_layer_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## Temps principal max par image toléré (ms) : budget 2 ms + une tuile (≤ ~1 ms) ; marge machine chargée.
const FRAME_MAX_MS := 8.0


## Calque 1:1 simulé : `version` et `built_ids()`, comme `TownLayer` / `LandmarkCityLayer`.
class FakeSource:
	extends RefCounted
	var version := 0
	var ids: Array[String] = []

	func built_ids() -> Array[String]:
		return ids

	func set_ids(p_ids: Array[String]) -> void:
		ids = p_ids
		version += 1


var _failures := 0


func _init() -> void:
	await process_frame
	var data := TownData.load_from(MAP_PATHS.default_data_dir().path_join("map"))
	var ids: Array = data.towns.keys()
	for city: Dictionary in LandmarkV2Library.all():
		if not ids.has(str(city["settlement"])):
			ids.append(str(city["settlement"]))
	var towns := FakeSource.new()
	var cities := FakeSource.new()
	var tiers := ZoomTiers.load_default()
	var layer := TownFarLayer.new()
	root.add_child(layer)
	var cam := Camera3D.new()
	root.add_child(cam)
	cam.position = Vector3(2000.0, 50.0, 2000.0)
	var t0 := Time.get_ticks_usec()
	layer.setup(null, null, tiers, ids, [towns, cities], data)
	var setup_ms := (Time.get_ticks_usec() - t0) / 1000.0
	var frames := 0
	var max_usec := 0
	while not layer.is_complete() and frames < 5000:
		var tf := Time.get_ticks_usec()
		layer.update_view(100.0)
		max_usec = maxi(max_usec, Time.get_ticks_usec() - tf)
		frames += 1
		await process_frame
	var total_ms := (Time.get_ticks_usec() - t0) / 1000.0
	var s := layer.stats
	print("tf_far_layer_test: setup %.1f ms ; génération %.0f ms réels (%.0f ms cumulés sur les fils) ; tout construit en %d images, %.0f ms ; image principale max %.2f ms, tuile max %.2f ms" % [setup_ms, float(s.get("gen_ms", -1)), float(s.get("gen_cpu_ms", -1)), frames, total_ms, max_usec / 1000.0, float(s.get("build_tile_max_ms", -1))])
	print("tf_far_layer_test: %d villes ; F1 %d tuiles, %d tri ; F2 %d tuiles, %d tri ; %d sommets ; ≈ %.1f Mo" % [int(s.get("towns", 0)), int(s.get("tiles_f1", 0)), int(s.get("triangles_f1", 0)), int(s.get("tiles_f2", 0)), int(s.get("triangles_f2", 0)), int(s.get("vertices", 0)), float(s.get("vram_mb", 0))])
	_check(layer.is_complete(), "generation completes")
	_check(max_usec / 1000.0 <= FRAME_MAX_MS, "main thread max per frame %.2f ms > %.1f" % [max_usec / 1000.0, FRAME_MAX_MS])
	var n := int(s.get("towns", 0))
	_check(n >= 2000, "towns with a far mesh: %d" % n)
	_check(int(s.get("tiles_f1", 0)) > 50 and int(s.get("tiles_f2", 0)) > 5, "tile counts")
	_check(int(s.get("tiles_f1", 0)) + int(s.get("tiles_f2", 0)) == layer.get_child_count(), "one node per tile")
	var tri1 := int(s.get("triangles_f1", 0))
	var tri2 := int(s.get("triangles_f2", 0))
	_check(tri1 > 200 * n and tri1 < 450 * n, "F1 triangles %d for %d towns" % [tri1, n])
	_check(tri2 > 40 * n and tri2 < 80 * n, "F2 triangles %d for %d towns" % [tri2, n])
	_test_ranges(layer, tiers)
	_test_mask(layer, towns, cities, data)
	_test_strategic(layer, tiers)
	_test_quality(layer)
	layer.queue_free()
	cam.queue_free()
	await process_frame
	print("tf_far_layer_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(cond: bool, msg: String) -> void:
	if not cond:
		_failures += 1
		push_error("tf_far_layer_test: " + msg)


func _test_ranges(layer: TownFarLayer, tiers: ZoomTiers) -> void:
	var f1: MeshInstance3D = layer.tile_nodes("f1")[0]
	var f2: MeshInstance3D = layer.tile_nodes("f2")[0]
	var r := layer.f1_range_current()
	_check(is_equal_approx(f1.visibility_range_end, r) and f1.visibility_range_begin == 0.0, "F1 range end %.0f" % f1.visibility_range_end)
	_check(is_equal_approx(f2.visibility_range_begin, r) and is_equal_approx(f2.visibility_range_end, tiers.model_range), "F2 range %.0f-%.0f" % [f2.visibility_range_begin, f2.visibility_range_end])
	_check(f1.visibility_range_fade_mode == GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF and f2.visibility_range_fade_mode == GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF, "fade self")
	_check(is_equal_approx(f1.visibility_range_end_margin, f2.visibility_range_begin_margin), "matching cross-fade margins")
	_check(f1.custom_aabb.size.y > 0.0 and f1.custom_aabb.size.x > 0.0, "F1 custom AABB")
	_check(f2.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "F2 never casts shadows")
	layer.update_view(20.0)
	var high := bool(layer.quality.get(RenderQuality.current(), {}).get("shadows", false))
	_check((f1.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON) == high, "F1 shadows under rig 60 (quality %s)" % RenderQuality.current())
	layer.update_view(100.0)
	_check(f1.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "F1 shadows off above rig 60")


func _test_mask(layer: TownFarLayer, towns: FakeSource, cities: FakeSource, data: TownData) -> void:
	var town_id := ""
	for id: String in data.towns:
		if LandmarkV2Library.for_settlement(id).is_empty():
			town_id = id
			break
	var city_id := str(LandmarkV2Library.all()[0]["settlement"])
	var ti := layer.index_of(town_id)
	var ci := layer.index_of(city_id)
	_check(ti >= 0 and ti < data.towns.size(), "town index in file order (%d)" % ti)
	_check(ci >= data.towns.size(), "v2 city index after the file towns (%d)" % ci)
	_check(layer.sink_distance() == 0.0, "no sink without built towns")
	towns.set_ids([town_id] as Array[String])
	cities.set_ids([city_id] as Array[String])
	layer.update_view(10.0)
	_check(layer.mask.is_built(ti) and layer.mask.is_built(ci), "mask follows built_ids")
	var profile := TownRenderProfile.load_default()
	var block := float(profile.factors(RenderQuality.current()).get("block", 1.0))
	_check(is_equal_approx(layer.sink_distance(), profile.block_range * block * 0.95), "sink distance = block range x 0.95 (%.2f)" % layer.sink_distance())
	_check(layer.sink_distance() < profile.block_range * block, "far sinks inside the block range only")
	var uploads := layer.mask.uploads
	layer.update_view(10.0)
	_check(layer.mask.uploads == uploads, "unchanged sources: no mask upload")
	towns.set_ids([] as Array[String])
	layer.update_view(10.0)
	_check(not layer.mask.is_built(ti) and layer.mask.is_built(ci), "town unmarked")
	cities.set_ids([] as Array[String])
	layer.update_view(30.0)
	_check(not layer.mask.is_built(ci) and layer.sink_distance() == 0.0, "mask cleared when the 1:1 layers are inactive")


func _test_strategic(layer: TownFarLayer, tiers: ZoomTiers) -> void:
	layer.update_view(tiers.strategic_threshold + tiers.strategic_fade * 2.0)
	_check(not layer.visible, "hidden in the strategic view")
	layer.update_view(500.0)
	_check(layer.visible, "shown in the 3D view")


func _test_quality(layer: TownFarLayer) -> void:
	var saved := RenderQuality.override_level
	RenderQuality.override_level = "low"
	layer.apply_render_quality({})
	var f1: MeshInstance3D = layer.tile_nodes("f1")[0]
	_check(is_equal_approx(f1.visibility_range_end, 150.0), "low preset: F1 range 150 (%.0f)" % f1.visibility_range_end)
	RenderQuality.override_level = saved
	layer.apply_render_quality({})
