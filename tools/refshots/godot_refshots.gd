extends SceneTree

## Lot refshots : captures de référence côté Godot pour tools/compare_views.sh.
## Usage (fenêtré, par tools/godot_bg.sh, jamais en direct) :
##   tools/godot_bg.sh --path <main>/game --resolution 1280x800 --script <ce fichier> -- \
##       --views-file=<views.txt> --out=<dossier> --group=campaign|battle|siege [--only=<nom>]
## Chaque vue du groupe donne <out>/<nom>.png. L'interface reste visible (didacticiel et
## briefing coupés) ; réglages en fichier de test (ceux du joueur ne sont pas touchés).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _out := ""
var _group := ""
var _only := ""


func _init() -> void:
	_out = CmdArgs.value("--out", "")
	_group = CmdArgs.value("--group", "campaign")
	_only = CmdArgs.value("--only", "")
	var views := _read_views(CmdArgs.value("--views-file", ""))
	if _out == "" or views.is_empty():
		push_error("godot_refshots: --out and --views-file required")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(_out)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
		settings.call("set_value", "interface/campaign_briefing", false, false)
	match _group:
		"campaign":
			await _campaign(views)
		"battle":
			await _battle(views)
		"siege":
			for v: PackedStringArray in views:
				await _siege(v)
				break  # une scène de siège par lancement
	quit(0)


func _read_views(path: String) -> Array:
	var out: Array = []
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return out
	while not file.eof_reached():
		var line := file.get_line().strip_edges()
		if line == "" or line.begins_with("#"):
			continue
		var parts := line.split(" ", false)
		if parts.size() < 2 or parts[1] != _group:
			continue
		if _only != "" and parts[0] != _only:
			continue
		out.append(parts)
	return out


func _frames(count: int) -> void:
	for _i in count:
		await process_frame


func _shot(name: String) -> void:
	await RenderingServer.frame_post_draw
	var path := _out.path_join(name + ".png")
	var err := root.get_texture().get_image().save_png(path)
	print("godot_refshots: %s (%s)" % [path, error_string(err)])


func _campaign(views: Array) -> void:
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _frames(60)
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.get("map_data")
	for v: PackedStringArray in views:
		var px := FaunaLayer.lonlat_to_px(float(v[2]), float(v[3]), data)
		rig.look_at_point(Vector3(px.x, data.surface_world_at(px.x, px.y), px.y), float(v[4]))
		rig.snap()
		# Laisser charger relief, végétation, villes (streaming) avant la capture.
		await _frames(240)
		await _shot(v[0])
		print("godot_refshots: %s px=%s dist=%.0f" % [v[0], px, rig.distance])


func _battle(views: Array) -> void:
	# Composition par défaut de l'écran « Bataille personnalisée » (France contre Angleterre,
	# plaine en été, budgets par défaut), comme `default_custom_battle` du client Bevy.
	var screen: Control = (load("res://scripts/ui/custom_battle_screen.gd") as GDScript).new()
	root.add_child(screen)
	await _frames(5)
	var config: Dictionary = screen.call("battle_config")
	screen.queue_free()
	config["seed"] = 1337
	BattleScene.custom_config = config
	var scene: Node = await _open_battle()
	var rig: BattleCamera = scene.camera_rig
	var yaw := rig.yaw
	for v: PackedStringArray in views:
		if v[2] == "low":
			var units: Array = scene.battle.call("get_units")
			for unit: Dictionary in units:
				if int(unit["id"]) == 0:
					rig.look_at_point(Vector3(float(unit["x"]), 0, float(unit["z"])), float(v[3]), yaw)
		await _frames(150)
		await _shot(v[0])


func _siege(v: PackedStringArray) -> void:
	for demo: Dictionary in BattleDemosMenu.load_demos():
		if str(demo["id"]) == v[2]:
			BattleScene.demo_args = BattleDemosMenu.args_for(demo)
	await _open_battle()
	await _frames(150)
	await _shot(v[0])


## Ouvre la scène de bataille, saute le déploiement et met en pause (soldats immobiles).
func _open_battle() -> Node:
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	root.add_child(scene)
	await _frames(30)
	if scene.deployment != null and scene.deployment.active:
		scene.deployment.finish()
	await _frames(10)
	scene.paused = true
	scene.camera_rig.edge_pan_enabled = false
	return scene
