extends SceneTree

## Lot P2e (chantier PO, phase 2) : planche des menus secondaires migrés, 1280×720, écrite dans
## `<out>/NN-<vue>.png` (défaut `docs/img/po/p2e/`). Comme `po_shot.gd` : chaque vue tourne dans
## son propre processus Godot fenêtré (le rendu headless ne produit pas d'image). Les vues qui ont
## une option `--menu-stage=` (`start_menu.gd`) l'utilisent ; les autres (rejeux, pause,
## sauvegarder) sont mises en scène par ce script (`--view=…`).
## Usage :
##   godot --path game --script res://tests/p2e_shot.gd -- [--out=<dossier>] [--only=<n,n>]

const RESOLUTION := "1280x720"
const PAUSE_MENU_SCENE := "res://scenes/ui/pause_menu.tscn"
const SAVE_DIALOG_SCENE := "res://scenes/ui/save_load_dialog.tscn"
const TIMEOUT_S := 60.0

## [numéro, nom, vue ("" : `start_menu.gd` avec `--menu-stage=`, sinon mise en scène ici),
## options après `--` pour `start_menu.gd`].
const VIEWS := [
	[1, "demos-bataille", "", ["--menu-stage=demos"]],
	[2, "batailles-historiques", "", ["--menu-stage=historical"]],
	[3, "rejeux", "replays", []],
	[4, "reglages", "", ["--menu-stage=settings"]],
	[5, "pause", "pause", []],
	[6, "credits", "", ["--menu-stage=credits"]],
	[7, "sauvegarder", "save", []],
]


func _init() -> void:
	var out := "docs/img/po/p2e"
	var only := PackedInt32Array()
	var view := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			for part in arg.trim_prefix("--only=").split(",", false):
				only.append(int(part))
		elif arg.begins_with("--view="):
			view = arg.trim_prefix("--view=")
	if view != "":
		_stage.call_deferred(view)
		return
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	DirAccess.make_dir_recursive_absolute(out_dir)
	var failed := 0
	for entry in VIEWS:
		if not only.is_empty() and not only.has(int(entry[0])):
			continue
		var png := out_dir.path_join("%02d-%s.png" % [entry[0], entry[1]])
		DirAccess.remove_absolute(png.get_basename() + ".jpg")
		DirAccess.remove_absolute(png)
		var args := PackedStringArray(["--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION])
		if str(entry[2]) != "":
			args.append_array(["--script", "res://tests/p2e_shot.gd", "--", "--view=" + str(entry[2]), "--out=" + png])
		else:
			args.append("res://scenes/start_menu.tscn")
			args.append("--")
			args.append_array(PackedStringArray(entry[3]))
			args.append("--screenshot=" + png)
		var started := Time.get_ticks_msec()
		var output := []
		var code := OS.execute(OS.get_executable_path(), args, output, true)
		var ok := code == 0 and FileAccess.file_exists(png)
		if ok:
			ok = _to_jpg(png)
		if not ok:
			failed += 1
			var text := "".join(output)
			print("P2E_SHOT %s FAIL (code %d)\n%s" % [png, code, text.right(1500)])
		else:
			print("P2E_SHOT %s OK (%.1f s)" % [png.get_basename() + ".jpg", (Time.get_ticks_msec() - started) / 1000.0])
	print("P2E_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


func _to_jpg(png: String) -> bool:
	var image := Image.load_from_file(png)
	if image == null or image.is_empty():
		return false
	var err := image.save_jpg(png.get_basename() + ".jpg", 0.85)
	DirAccess.remove_absolute(png)
	return err == OK


## Mise en scène des vues sans option `start_menu.gd` : rejeux, pause, boîte de sauvegarde.
func _stage(view: String) -> void:
	var png := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			png = arg.trim_prefix("--out=")
	create_timer(TIMEOUT_S).timeout.connect(func() -> void:
		print("p2e_shot: timeout")
		quit(1))
	await process_frame
	await process_frame
	match view:
		"replays":
			ReplaysMenu.dir_override = ProjectSettings.globalize_path("user://p2e_shot_replays")
			root.add_child(ReplaysMenu.new())
		"pause":
			var facade: Node = root.get_node_or_null("/root/SimFacade")
			if facade != null:
				facade.set("pending_faction", "fac_france")
			var pause: Node = (load(PAUSE_MENU_SCENE) as PackedScene).instantiate()
			pause.unsaved_turns = 2
			root.add_child(pause)
		"save":
			var dialog: Node = (load(SAVE_DIALOG_SCENE) as PackedScene).instantiate()
			root.add_child(dialog)
			await process_frame
			dialog.open_save("partie_p2e")
	for i in 12:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	var err := image.save_png(png)
	print("p2e_shot: %s (%s)" % [png, error_string(err)])
	quit(0 if err == OK else 1)
