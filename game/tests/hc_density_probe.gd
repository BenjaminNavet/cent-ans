extends SceneTree

## Sonde du lot HC1 (réglage, pas un test) : densité des arbres généralisés par milieu autour d'un
## point, sans lire d'image. Sème les tuiles vues à `--rig` (défaut 90) autour de `--at=x,z`
## (défaut Paris) et imprime, sur un disque de `--radius` px (défaut 120) : part de forêt du
## masque, arbres par px² en forêt (masque > 0,5), en lisière (0,1-0,5), hors forêt (< 0,1), part
## des arbres hors forêt à moins de 2 px d'un fleuve (ripisylves), temps de semis.
## Usage : godot --headless --path game --script res://tests/hc_density_probe.gd --
##   [--at=2213,3204] [--rig=90] [--radius=120] [--prop=<champ de MapPropScale>=<valeur>]…

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const STRIDE := VegetationTileJob.FLOATS_PER_INSTANCE


func _init() -> void:
	var at := Vector2(2213.0, 3204.0)
	var rig := 90.0
	var radius := 120.0
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--at="):
			var xz := arg.trim_prefix("--at=").split(",")
			at = Vector2(float(xz[0]), float(xz[1]))
		elif arg.begins_with("--rig="):
			rig = float(arg.trim_prefix("--rig="))
		elif arg.begins_with("--radius="):
			radius = float(arg.trim_prefix("--radius="))
		elif arg.begins_with("--prop="):
			var kv := arg.trim_prefix("--prop=").split("=")
			MapPropScale.shared().set(kv[0], float(kv[1]))
	await process_frame
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	var settlements := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, settlements, ZoomTiers.load_default())
	var roads: Array = []
	for road: Dictionary in settlements.roads:
		if road["main"]:
			roads.append(road["points"])
	var vegetation := Vegetation.new()
	vegetation.camera_rig_path = NodePath("")
	world.add_child(vegetation)
	vegetation.set_process(false)
	vegetation.quality_max_distance = -1.0
	vegetation.extra_exclusions = layer.vegetation_exclusions()
	vegetation.clearance_roads = roads
	vegetation.bind_terrain(terrain)
	vegetation.build(map_data)
	var focus := Vector3(at.x, map_data.surface_world_at(at.x, at.y), at.y)
	var camera := focus + Vector3(0.0, 0.56 * rig, 0.83 * rig)
	var guard := 0
	while guard < 3000:
		guard += 1
		vegetation.update_view(camera, rig, focus)
		if vegetation.pending_jobs() == 0 and guard > 3:
			break
		await process_frame
	var noise := VegetationMask.make_noise(1337)
	# Aires par milieu (grille de 2 px sur le disque).
	var area := [0.0, 0.0, 0.0]
	var y := at.y - radius
	while y < at.y + radius:
		var x := at.x - radius
		while x < at.x + radius:
			if Vector2(x, y).distance_to(at) < radius and map_data.height_world_at(x, y) > 0.0:
				area[_class(float(vegetation.mask.sample(x, y, noise)["forest"]))] += 4.0
			x += 2.0
		y += 2.0
	var trees := [0, 0, 0]
	var riparian := 0
	for entry: Dictionary in (vegetation.get("_tiles") as Dictionary).values():
		for buffer: PackedFloat32Array in entry["buffers"]:
			var k := 0
			while k < buffer.size():
				var p := Vector2(buffer[k + 3], buffer[k + 11])
				if p.distance_to(at) < radius:
					var c := _class(float(vegetation.mask.sample(p.x, p.y, noise)["forest"]))
					trees[c] += 1
					if c == 2 and map_data.river_sd_at(p.x, p.y) < 2.0:
						riparian += 1
				k += STRIDE
	var total_area: float = area[0] + area[1] + area[2]
	print("HC probe at (%.0f, %.0f) rig %.0f radius %.0f : forest %.1f %% edge %.1f %% open %.1f %% of land" % [at.x, at.y, rig, radius,
		100.0 * area[0] / total_area, 100.0 * area[1] / total_area, 100.0 * area[2] / total_area])
	print("HC probe trees/px2 : forest %.3f (%d), edge %.3f (%d), open %.4f (%d, %d riparian) ; census %s ; build ms max %.0f, native %.0f, clearance %.0f, cleared %d" % [
		trees[0] / maxf(area[0], 1.0), trees[0], trees[1] / maxf(area[1], 1.0), trees[1], trees[2] / maxf(area[2], 1.0), trees[2], riparian,
		JSON.stringify(vegetation.visible_census()), float(vegetation.stats["build_ms_max"]), float(vegetation.stats.get("native_ms_max", 0.0)),
		float(vegetation.stats.get("clearance_ms_max", 0.0)), int(vegetation.stats.get("cleared", 0))])
	vegetation.clear()
	world.queue_free()
	await process_frame
	quit(0)


## 0 forêt (masque > 0,5), 1 lisière (0,1-0,5), 2 hors forêt.
static func _class(forest: float) -> int:
	return 0 if forest > 0.5 else (1 if forest > 0.1 else 2)
