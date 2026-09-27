extends SceneTree

## Captures du lot CV3-4 (non headless, 640 px de large) dans `docs/img/cv3/` :
## `ui-postures.png` (sceau et boutons de posture, pastille sur l'étendard, site de rencontre),
## `ui-rencontre.png` (fenêtre de choix), `ui-resultat.png` (écran de fin, bandeau de classe).
## Usage : godot --path game --script res://tests/cv3_4_ui_shot.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const OUT_DIR := "../docs/img/cv3/"
const WIDTH := 640


func _init() -> void:
	await process_frame
	root.size = Vector2i(1280, 800)
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "interface/season_report", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 20:
		await process_frame
	var sim: Object = map.sim
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)
	var army_id: String = map.player_army_ids()[0]
	map.call("_on_stance_changed", army_id, "raid")
	map.select_army(army_id)
	var army: Dictionary = sim.call("get_army", army_id)
	var start: Vector2 = army["position"]
	var spot := start
	var rect: Rect2 = map.movement_ctl.bubble.covered_rect()
	var radius := maxf(rect.size.x, rect.size.y) * 0.5
	for i in 24:
		var point := start + Vector2.RIGHT.rotated(TAU * float(i) / 24.0) * radius * 0.25
		var plan: Dictionary = sim.call("find_path_points", army_id, point.x, point.y)
		if plan.get("ok", false) and bool(plan.get("reachable_this_turn", false)):
			spot = point
			break
	var site := int(sim.call("debug_place_encounter", "enc_marchands_lombards", spot.x, spot.y))
	map.refresh_all()
	var middle: Vector3 = (map.armies.world_position_of(army_id) + map.armies.world_at_pixel(spot)) * 0.5
	map.camera_rig.look_at_point(middle, 70.0)
	map.camera_rig.snap()
	for i in 60:
		await process_frame
	_save("ui-postures.png")

	map.encounters.site_clicked(site)
	for i in 30:
		await process_frame
	_save("ui-rencontre.png")

	map.encounters.window.hide()
	var screen := BattleResultScreen.new()
	root.add_child(screen)
	var sides := {
		"attacker": {"name": "Royaume de France", "faction": "fac_france", "color": Color(0.16, 0.26, 0.62)},
		"defender": {"name": "Royaume d'Angleterre", "faction": "fac_england", "color": Color(0.7, 0.12, 0.1)},
	}
	var outcome := {"winner": "attacker", "duration": 1260.0, "attacker": {"total_losses": 310}, "defender": {"total_losses": 2400}}
	var aftermath := {"campaign_outcome": {"attacker_class": "heroic", "attacker_label": "Victoire héroïque", "defender_class": "disaster", "defender_label": "Désastre"}}
	screen.show_result("Bataille de Saint-Denis", "attacker", sides, [], outcome, aftermath)
	for i in 20:
		await process_frame
	_save("ui-resultat.png")
	quit(0)


func _save(file: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	image.resize(WIDTH, int(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path("res://" + OUT_DIR))
	var path := ProjectSettings.globalize_path("res://" + OUT_DIR + file)
	image.save_png(path)
	print("cv3_4_ui_shot: ", path)
