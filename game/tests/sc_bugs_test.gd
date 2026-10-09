extends SceneTree

## Test headless des trois bugs du lot SC « bugs » :
##  1. le pont expose `supports_order` (lu par `HudController.split_supported`) ;
##  2. clé unique "peace_summons" pour la sommation de paix (cœur, contrôleur féodal, panneau) ;
##  3. `OutbuildingLayer` : une vue demandée avant/pendant le setup est mémorisée puis servie,
##     et le warm-up ne reste pas en suspens.
## Usage : godot --headless --path game --script res://tests/sc_bugs_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("sc_bugs_test: " + message)
		print("FAIL: " + message)


func _init() -> void:
	await process_frame
	_test_split_supported()
	_test_summons_key()
	await _test_outbuilding_view()
	print("sc_bugs_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _test_split_supported() -> void:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	_check(sim != null, "CampaignSim missing")
	if sim == null:
		return
	_check(sim.has_method("supports_order"), "bridge lacks supports_order")
	_check(sim.has_method("supports_order") and bool(sim.call("supports_order", "split_army")), "split_army should be supported")
	_check(sim.has_method("supports_order") and not bool(sim.call("supports_order", "no_such_order")), "unknown order should not be supported")


func _test_summons_key() -> void:
	for path in ["res://scripts/map/feudal_controller.gd", "res://scripts/ui/diplomacy/diplomacy_offers_section.gd"]:
		var source := FileAccess.get_file_as_string(path)
		_check(source.contains("\"peace_summons\""), "%s should use the core key peace_summons" % path)
		_check(not source.contains("== \"summons\""), "%s still matches the old key summons" % path)


func _test_outbuilding_view() -> void:
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	var data := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var layer := OutbuildingLayer.new()
	world.add_child(layer)
	layer.update_view(10.0)  # avant le setup : mémorisée
	layer.setup(null, map_data, terrain, data)
	layer.update_view(10.0)  # dans la même image que le setup : ne doit pas bloquer
	for _i in 600:
		if layer.visible and layer.get("_warm_task") == -1:
			break
		await process_frame
	_check(layer.get("_warm_task") == -1, "warm-up should be integrated")
	_check(layer.visible, "view requested during warm-up should be served after it")
	_check(not (layer.get("_meshes") as Dictionary).is_empty(), "meshes should be warmed")
	layer.setup(null, map_data, terrain, data)  # second setup : pas de tâche orpheline
	layer.update_view(10.0)
	for _i in 600:
		if layer.get("_warm_task") == -1:
			break
		await process_frame
	_check(layer.get("_warm_task") == -1, "second setup should not leave a warm-up pending")
	await process_frame
	world.queue_free()
