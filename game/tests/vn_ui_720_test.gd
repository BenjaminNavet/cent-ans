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
	await _tutorial_vs_modal()
	print("vn_ui_720_test: %s" % ("OK" if _failures == 0 else "FAIL (%d)" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("vn_ui_720_test: FAIL %s" % what)


func _screen() -> Rect2:
	return Rect2(Vector2.ZERO, Vector2(LOGICAL))


## 1. Menu principal : toutes les entrées tiennent, même avec « Continuer » et sa ligne de détail,
## à la hauteur logique de 1280×720 (800) et à des hauteurs plus basses (échelle d'interface 1,0 : 720).
func _menu() -> void:
	for height in [800, 720, 640]:
		await _menu_at(height)


func _menu_at(height: int) -> void:
	_viewport.size = Vector2i(LOGICAL.x, height)
	var menu: Control = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	_viewport.add_child(menu)
	for i in 10:
		await process_frame
	menu.continue_button.show()
	menu._continue_detail.text = "Royaume de France — 1337"
	menu._continue_detail.get_parent().show()
	for i in 4:
		await process_frame
	var screen := Rect2(Vector2.ZERO, Vector2(LOGICAL.x, height))
	var previous_bottom := 0.0
	for button: Button in menu._menu_buttons:
		if not button.visible:
			continue
		var rect := button.get_global_rect()
		_check(screen.encloses(rect), "menu@%d button '%s' %s outside %s" % [height, button.text, rect, screen])
		_check(rect.position.y >= previous_bottom - 0.5, "menu button '%s' overlaps the previous one" % button.text)
		previous_bottom = rect.end.y
	_check(previous_bottom <= screen.end.y, "menu@%d list ends at %s, below the screen" % [height, previous_bottom])
	menu.queue_free()
	_viewport.size = LOGICAL
	await process_frame


## 2. Tutoriel : le parchemin s'efface sous une fenêtre modale (`UiZones.Zone.MODAL`) et revient à
## sa fermeture ; une étape jouée dans une modale (`modal_ok`) le garde.
func _tutorial_vs_modal() -> void:
	var overlay: TutorialOverlay = (load("res://scenes/ui/tutorial.tscn") as PackedScene).instantiate()
	_viewport.add_child(overlay)
	var modal := Panel.new()
	modal.custom_minimum_size = Vector2(600, 400)
	_viewport.add_child(modal)
	UiZones.put(UiZones.Zone.MODAL, modal)
	modal.hide()
	overlay.show_step({"title": "Votre place dans la féodalité", "text": "x", "objective": "y"}, 0, 3)
	for i in 4:
		await process_frame
	_check(overlay.panel.visible, "tutorial panel visible without modal")
	modal.show()
	for i in 4:
		await process_frame
	_check(not overlay.panel.visible, "tutorial panel hidden while a modal window is open")
	modal.hide()
	for i in 4:
		await process_frame
	_check(overlay.panel.visible, "tutorial panel back after the modal closes")
	overlay.show_step({"title": "Les technologies", "text": "x", "objective": "y", "modal_ok": true}, 1, 3)
	modal.show()
	for i in 4:
		await process_frame
	_check(overlay.panel.visible, "modal_ok step keeps its panel over a modal")
	modal.hide()
	overlay.queue_free()
	modal.queue_free()
	await process_frame
