extends SceneTree

## Captures du Codex (H2) : une pile de 3 bulles au-dessus de la chronique, puis la fenêtre.
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/codex_screenshot.gd
## Écrit `docs/img/codex-bubbles.png` et `docs/img/codex-window.png`.


func _init() -> void:
	await process_frame
	var store: Node = root.get_node("/root/CodexStore")
	var bubbles: Node = root.get_node("/root/CodexBubbles")
	store.call("use_test_file")
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img").simplify_path()

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var event: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(
		ProjectSettings.globalize_path("res://").path_join("../data/events/evt_crecy.json").simplify_path()))
	var chronicle: Node = (load("res://scenes/ui/chronicle_window.tscn") as PackedScene).instantiate()
	root.add_child(chronicle)
	await process_frame
	chronicle.call("show_decision", {
		"id": 1, "historical": true, "title": str(event.get("title", "")), "text": str(event.get("text", "")),
		"province_name": "Ponthieu", "expires_in": 2,
		"options": [{"index": 0, "text": "Charger", "effects_text": ""}, {"index": 1, "text": "Attendre", "effects_text": ""}],
	}, 1)
	for _i in 4:
		await process_frame
	bubbles.call("open", "cdx_crecy", Vector2(360, 330))
	await process_frame
	bubbles.call("open", "cdx_arc_long", Vector2(640, 400), 0)
	await process_frame
	bubbles.call("open", "cdx_chevauchee", Vector2(900, 470), 1)
	for _i in 6:
		await process_frame
	_save(out_dir.path_join("codex-bubbles.png"))

	bubbles.call("close_all")
	chronicle.queue_free()
	bubbles.call("open_entry", "cdx_du_guesclin")
	for _i in 6:
		await process_frame
	var window: Control = bubbles.call("window")
	var body: RichTextLabel = window.call("body_label")
	bubbles.call("open", "cdx_charles_v", body.get_global_rect().position + Vector2(220, 60))
	for _i in 6:
		await process_frame
	_save(out_dir.path_join("codex-window.png"))
	store.call("reset_discoveries")
	quit(0)


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("codex screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
