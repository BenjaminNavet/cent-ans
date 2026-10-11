extends TestCase

## RG1 : les fenêtres centrales / compagnons se redimensionnent à la souris (poignée bas droite),
## bornées par leur minimum initial et l'écran ; modaux et ancrés n'en ont pas.
## `godot --headless --path game --script res://tests/rg1_resize_test.gd`


func _make_panel(name_: String) -> PanelContainer:
	var panel := PanelContainer.new()
	panel.name = name_
	panel.custom_minimum_size = Vector2(200, 100)
	panel.position = Vector2(20, 30)
	panel.add_child(Label.new())
	root.add_child(panel)
	return panel


func _press(grip: Control, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = pressed
	grip._gui_input(event)


func _motion(grip: Control, relative: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.relative = relative
	grip._gui_input(event)


func _init() -> void:
	await process_frame
	WindowResizeGrip.forget_all()
	root.size = Vector2i(1000, 700)  # le viewport headless est minuscule par défaut
	await process_frame
	var stack := PanelStack.new()
	var central := _make_panel("RgCentral")
	var modal := _make_panel("RgModal")
	var docked := _make_panel("RgDocked")
	stack.register(central, PanelStack.Kind.CENTRAL)
	stack.register(modal, PanelStack.Kind.MODAL)
	stack.register(docked, PanelStack.Kind.DOCKED)
	await process_frame
	check(central.get_node_or_null("ResizeGrip") != null, "central panel has a grip")
	check(modal.get_node_or_null("ResizeGrip") == null, "modal has no grip")
	check(docked.get_node_or_null("ResizeGrip") == null, "docked has no grip")
	var grip := central.get_node("ResizeGrip") as Control
	check(grip.mouse_default_cursor_shape == Control.CURSOR_FDIAGSIZE, "diagonal resize cursor")
	var before := central.size
	_press(grip, true)
	_motion(grip, Vector2(120, 80))
	await process_frame
	check(central.size.is_equal_approx(before + Vector2(120, 80)), "grew by the drag: %s" % str(central.size))
	check(central.has_meta(WindowResizeGrip.USER_SIZED), "user-sized flag set")
	_motion(grip, Vector2(-5000, -5000))
	await process_frame
	check(central.size.is_equal_approx(Vector2(200, 100)), "clamped to the initial minimum: %s" % str(central.size))
	_motion(grip, Vector2(99999, 99999))
	await process_frame
	var view := central.get_viewport_rect().size
	check(central.get_global_rect().end.x <= view.x and central.get_global_rect().end.y <= view.y, "clamped to the screen: %s" % str(central.size))
	_press(grip, false)
	var final_size := central.size
	await process_frame
	check(central.size.is_equal_approx(final_size), "size kept after a layout pass")
	# Mémorisation de session : un panneau du même nom, recréé, retrouve sa taille.
	_press(grip, true)
	_motion(grip, Vector2(-99999, -99999))
	_motion(grip, Vector2(150, 90))
	_press(grip, false)
	var chosen := central.size
	central.free()
	var again := _make_panel("RgCentral")
	stack.register(again, PanelStack.Kind.COMPANION)
	await process_frame
	await process_frame
	check(again.size.is_equal_approx(chosen), "size remembered for the session: %s vs %s" % [str(again.size), str(chosen)])
	print("rg1_resize_test: %s" % ("OK" if failures == 0 else "%d failure(s)" % failures))
	finish()
