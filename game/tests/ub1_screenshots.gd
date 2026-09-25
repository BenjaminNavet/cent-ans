extends SceneTree

## Captures d'UB1 (interface de bataille) qui demandent une interaction : confirmation de la
## « Retraite générale » et journal replié. Usage (avec affichage, pas en headless) :
##   godot --resolution 1440x900 --path game --script res://tests/ub1_screenshots.gd
## Écrit dans `docs/audit/captures/ub1/`.

const OUT_DIR := "../docs/audit/captures/ub1"


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join(OUT_DIR).simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.set("autoplay", true)
	root.add_child(scene)
	for _i in 10:
		await process_frame
	var battle: Object = scene.get("battle")
	for _i in 700:
		battle.call("tick", 0.1)
	scene.set("units", battle.call("get_units"))
	scene.call("_refresh_view", true)
	scene.set("paused", true)
	for _i in 30:
		await process_frame
	var hud: BattleHud = scene.get("hud")
	hud.ask_withdraw_all()
	for _i in 4:
		await process_frame
	await _save(out_dir.path_join("22-retraite-generale-confirmation.png"))
	hud.confirm_panel.visible = false
	hud.toggle_log()
	for _i in 4:
		await process_frame
	await _save(out_dir.path_join("23-journal-replie.png"))
	quit(0)


func _save(path: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("ub1 screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
