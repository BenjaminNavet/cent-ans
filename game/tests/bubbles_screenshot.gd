extends SceneTree

## Capture B1 (bulles partout) : une chaîne de 3 bulles façon Baldur's Gate 3 — une infobulle
## riche d'unité verrouillée par T, une bulle fille ouverte par survol d'un mot-clé et verrouillée,
## puis une petite-fille non épinglée (pied « T : maintenir ouverte »).
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/bubbles_screenshot.gd
## Écrit `docs/img/bulles-bg3.png`.


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
	for _i in 2:
		await process_frame

	# 1. Infobulle riche d'unité verrouillée (T) : bulle épinglée, titre lié à sa fiche.
	var tip := RichTooltip.unit("unit_longbowmen")
	var first: PanelContainer = bubbles.call("open_text", CodexText.format(tip, true), Vector2(60, 60), RichTooltip.title_entry(CodexText.format(tip, true)))
	await process_frame
	# 2. Survol d'un mot-clé de la bulle épinglée → bulle fille, verrouillée par T.
	var second := await _hover_link(bubbles, first, Vector2(420, 200))
	if second != null:
		bubbles.call("pin_current")
		# 3. Survol d'un mot-clé de la fille → petite-fille, encore ouverte à la lecture.
		await _hover_link(bubbles, second, Vector2(780, 340))
	for _i in 6:
		await process_frame
	_save(out_dir.path_join("bulles-bg3.png"))
	store.call("reset_discoveries")
	quit(0)


## Survole le premier lien du Codex de `bubble` (souris placée à `at`) et renvoie la bulle fille.
func _hover_link(bubbles: Node, bubble: PanelContainer, at: Vector2) -> PanelContainer:
	var label := bubble.find_child("Text", true, false) as RichTextLabel
	var found := RegEx.create_from_string("\\[url=(cdx:[^\\]]+)\\]").search(label.text)
	if found == null:
		return null
	root.warp_mouse(at)
	label.meta_hover_started.emit(found.get_string(1))
	await create_timer(0.5).timeout
	return bubbles.get("bubbles")[-1]


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("bubbles screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
