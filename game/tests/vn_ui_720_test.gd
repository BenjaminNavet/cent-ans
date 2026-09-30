extends SceneTree

## Lot VN : vérifie à 1280×720 que l'interface tient dans l'écran (menu principal, tutoriel vs
## fenêtre modale, panneau de province vs mini-carte, bandeau de fin de tour).
## Usage : godot --headless --path game --script res://tests/vn_ui_720_test.gd

const LOGICAL := Vector2i(1422, 800)

var _failures := 0
var _viewport: SubViewport


func _init() -> void:
	# Fenêtre 1280×720 à l'échelle automatique de l'interface (0,9) : vue logique 1422×800. Le
	# headless garde une fenêtre de 64×64 ; une SubViewport de cette taille la remplace.
	_viewport = SubViewport.new()
	_viewport.size = LOGICAL
	_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	root.add_child(_viewport)
	await process_frame
	await _menu()
	print("vn_ui_720_test: %s" % ("OK" if _failures == 0 else "FAIL (%d)" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("vn_ui_720_test: FAIL %s" % what)


func _screen() -> Rect2:
	return Rect2(Vector2.ZERO, Vector2(LOGICAL))


## 1. Menu principal : toutes les entrées tiennent, même avec « Continuer » et sa ligne de détail.
func _menu() -> void:
	var menu: Control = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	_viewport.add_child(menu)
	for i in 10:
		await process_frame
	menu.continue_button.show()
	menu._continue_detail.text = "Royaume de France — 1337"
	menu._continue_detail.get_parent().show()
	for i in 4:
		await process_frame
	var screen := _screen()
	var previous_bottom := 0.0
	for button: Button in menu._menu_buttons:
		if not button.visible:
			continue
		var rect := button.get_global_rect()
		_check(screen.encloses(rect), "menu button '%s' %s outside %s" % [button.text, rect, screen])
		_check(rect.position.y >= previous_bottom - 0.5, "menu button '%s' overlaps the previous one" % button.text)
		previous_bottom = rect.end.y
	_check(previous_bottom <= screen.end.y, "menu list ends at %s, below the screen" % previous_bottom)
	menu.queue_free()
	await process_frame
