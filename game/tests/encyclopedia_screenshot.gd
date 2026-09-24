extends SceneTree

## Capture d'une fiche de l'encyclopédie avec sa miniature.
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/encyclopedia_screenshot.gd [-- <identifiant>]
## Écrit `docs/img/encyclopedia-illustration.png`.


func _init() -> void:
	await process_frame
	var entry_id := "unit_bombard"
	var args := OS.get_cmdline_user_args()
	if not args.is_empty():
		entry_id = args[0]
	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var encyclopedia: Control = (load("res://scenes/ui/encyclopedia.tscn") as PackedScene).instantiate()
	root.add_child(encyclopedia)
	await process_frame
	encyclopedia.call("open_window", entry_id)
	for _i in 8:
		await process_frame
	var path := ProjectSettings.globalize_path("res://").path_join("../docs/img/encyclopedia-illustration.png").simplify_path()
	var error := root.get_texture().get_image().save_png(path)
	print("encyclopedia screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
	quit(0)
