extends SceneTree

## Lot CV3-0 : captures de contrôle des 10 défauts de la carte de campagne (annexe A,
## `docs/design/2026-09-27-campagne-vivante.md`). Fenêtre réelle (pas headless, comme SZ5) :
##   godot --path game --script res://tests/cv3_0_shots.gd -- --out=<dossier>
## Options : `--out=` (défaut `user://cv3_0`), `--prefix=`.
## Résolution 640 px de large (contrainte "vérification visuelle" CLAUDE.md).

const PARIS := Vector2(2213.2, 1923.9)
## Entre Paris et Orléans (vue régionale, lot #5).
const ORLEANS_AXIS := Vector2(2185.0, 2010.0)
const WIDTH := 640


func _init() -> void:
	var out_dir := "user://cv3_0"
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
		push_error("cv3_0 shots: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	var terrain: TerrainBuilder = map.get("terrain")
	# Fond d'écran net : masque le HUD 2D pour ne garder que la carte 3D.
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false

	# #1-#3 : vue la plus large depuis Paris (distance max, brouillard de guerre, nuées météo).
	var paris_ground := Vector3(PARIS.x, data.surface_world_at(PARIS.x, PARIS.y), PARIS.y)
	rig.look_at_point(paris_ground, rig.max_distance)
	rig.snap()
	var weather: CampaignWeatherView = map.get("weather_view")
	if weather != null:
		weather.forced = "storm"  # une province au moins en orage, pour juger #3 à large distance.
	for i in 60:
		await process_frame
	if terrain != null:
		terrain.wait_fine_jobs()
	for i in 20:
		await process_frame
	_shot(out_dir, prefix, "01-vue-large-paris-brouillard-nuages")

	# #4 : pluie rapprochée (aiguilles courtes/discrètes).
	if weather != null:
		weather.forced = "rain"
	rig.look_at_point(paris_ground, rig.min_distance_at(paris_ground) * 1.3)
	rig.snap()
	for i in 40:
		await process_frame
	_shot(out_dir, prefix, "04-pluie-rapprochee")
	if weather != null:
		weather.forced = ""

	# #5 : relief régional entre Paris et Orléans.
	var axis_ground := Vector3(ORLEANS_AXIS.x, data.surface_world_at(ORLEANS_AXIS.x, ORLEANS_AXIS.y), ORLEANS_AXIS.y)
	rig.look_at_point(axis_ground, 90.0)
	rig.snap()
	for i in 40:
		await process_frame
	if terrain != null:
		terrain.wait_fine_jobs()
	for i in 20:
		await process_frame
	_shot(out_dir, prefix, "05-relief-regional-paris-orleans")

	# #6 : chemin d'armée (liseré sombre) + #7 (marqueurs/étiquettes) réactivés pour ce plan.
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = true
	if map.movement_ctl != null and map.movement_ctl.path_line != null:
		var points := PackedVector2Array([PARIS, Vector2(PARIS.x + 20.0, PARIS.y + 12.0), Vector2(PARIS.x + 45.0, PARIS.y + 6.0)])
		map.movement_ctl.path_line.setup(data)
		map.movement_ctl.path_line.show_plan(points, 1, PackedInt32Array([1, 2]), rig.distance)
	rig.look_at_point(paris_ground, 60.0)
	rig.snap()
	for i in 40:
		await process_frame
	_shot(out_dir, prefix, "06-07-chemin-et-etiquettes")

	map.queue_free()
	await process_frame
	quit(0)


func _shot(out_dir: String, prefix: String, name: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := out_dir.path_join("%scarte-%s.png" % [prefix, name])
	image.save_png(path)
	print("CV3_0 shot %s" % path)
