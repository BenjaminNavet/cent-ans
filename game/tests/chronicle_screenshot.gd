extends SceneTree

## Capture de la fenêtre de chronique avec sa miniature d'événement.
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/chronicle_screenshot.gd [-- <evt_id>]
## Écrit `docs/img/chronicle-miniature.png`.


func _init() -> void:
	await process_frame
	var event_id := "evt_appel_gascon"
	var args := OS.get_cmdline_user_args()
	if not args.is_empty():
		event_id = args[0]
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var event: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(
		repo.path_join("data/events/%s.json" % event_id)))
	var chronicle: Node = (load("res://scenes/ui/chronicle_window.tscn") as PackedScene).instantiate()
	root.add_child(chronicle)
	await process_frame
	var options: Array = []
	for index in (event.get("options", []) as Array).size():
		options.append({"index": index, "text": str(event["options"][index].get("text", "")), "effects_text": ""})
	chronicle.call("show_decision", {
		"id": 1, "event": event_id, "historical": event.get("kind", "") != "random",
		"title": str(event.get("title", "")), "text": str(event.get("text", "")),
		"province_name": "", "expires_in": 2, "options": options,
	}, 1)
	for _i in 6:
		await process_frame
	var path := repo.path_join("docs/img/chronicle-miniature.png")
	var error := root.get_texture().get_image().save_png(path)
	print("chronicle screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
	quit(0)
