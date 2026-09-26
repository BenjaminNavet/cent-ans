extends SceneTree

## Lot SZ5 (suite de ZG7c, défaut S7) : captures avant/après de la pluie de la carte de campagne
## aux paliers site / vallée / lointain, pour vérifier que les gouttes restent crédibles à toute
## distance. Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/sz5_pluie_shots.gd -- --map-weather=rain --out=<dossier>
## Options : `--out=`, `--prefix=`, `--map-weather=rain|snow|clear` (forcé partout, cf. CM2/ZG7c).

const PLACES := [
	["paris", Vector2(2213.2, 1923.9)],
	["val_de_loire", Vector2(2052.0, 2127.3)],
]
## Distances des paliers (unités) ; « site » = distance minimale au point (+ 5 %).
const TIERS := {"site": -1.0, "vallee": 6.0, "lointain": 60.0}


func _init() -> void:
	var out_dir := "user://sz5"
	var prefix := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("sz5 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var weather: CampaignWeatherView = map.get("weather_view")
	for place in PLACES:
		var point: Vector2 = place[1]
		for tier: String in TIERS.keys():
			var focus := Vector3(point.x, data.surface_world_at(point.x, point.y), point.y)
			var distance: float = TIERS[tier]
			if distance < 0.0:
				distance = rig.min_distance_at(focus) * 1.05
			rig.look_at_point(focus, distance)
			rig.snap()
			for i in 40:
				await process_frame
			if terrain != null:
				terrain.wait_fine_jobs()
			for i in 30:
				await process_frame
			for layer in root.find_children("*", "CanvasLayer", true, false):
				(layer as CanvasLayer).visible = false
			for i in 3:
				await process_frame
			if terrain != null and terrain.material != null:
				terrain.material.set_shader_parameter("fog_enabled", false)
				for i in 2:
					await process_frame
			var image := root.get_viewport().get_texture().get_image()
			if image.get_width() > 960:
				image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
			var path := out_dir.path_join("%s%s_%s.jpg" % [prefix, place[0], tier])
			image.save_jpg(path, 0.82)
			var kind := weather.weather_at(point) if weather != null else "?"
			print("SZ5 shot %s d=%.3f weather=%s" % [path, distance, kind])
	map.queue_free()
	await process_frame
	quit(0)
