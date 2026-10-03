extends SceneTree

## Lot LR-18 : capture de la crue (nappe translucide) et des fuyards, scène `flood` forcée sur
## une colonie riveraine. Usage (avec affichage) :
##   godot --path game --resolution 1280x720 --script res://tests/lr18_flood_shot.gd -- \
##     --out=<png> [--kind=flood|devastation] [--settlement=<id>] [--distance=1.2]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out := ""
	var kind := "flood"
	var settlement := "set_paris"
	var distance := 1.2
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--kind="):
			kind = arg.trim_prefix("--kind=")
		elif arg.begins_with("--settlement="):
			settlement = arg.trim_prefix("--settlement=")
		elif arg.begins_with("--distance="):
			distance = float(arg.trim_prefix("--distance="))
	await process_frame
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	var data := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 30, 0)
	world.add_child(sun)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var pool := FolkPool.new()
	world.add_child(pool)
	pool.setup(map_data, terrain, 600)
	pool.warm_all()
	var scenes := FolkScenes.new()
	scenes.setup(map_data, data)
	pool.register(scenes)
	var entry: Dictionary = data.get_settlement(settlement)
	var focus: Vector2 = entry["px"]
	scenes.forced = [{"province": str(entry["province"]), "kind": kind, "settlement": settlement, "intensity": 1.0}]
	pool.refresh(null)
	for _i in 4:
		pool.update_view(focus, distance, 1.0)
		await process_frame
	var camera := Camera3D.new()
	world.add_child(camera)
	var ground := terrain.surface_height_at(focus.x, focus.y)
	camera.look_at_from_position(Vector3(focus.x + distance * 6.0, ground + distance * 30.0, focus.y + distance * 40.0), Vector3(focus.x, ground, focus.y))
	camera.current = true
	for _i in 40:
		pool.update_view(focus, distance, 1.0)
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	print("lr18_flood_shot: %s figures=%d props=%d (%s)" % [out, pool.figure_count(), pool.prop_count(), error_string(image.save_png(out))])
	quit(0)
