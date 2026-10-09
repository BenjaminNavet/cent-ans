extends TestCase

## Test headless de la pile des panneaux (audit A3, lot U1) : exclusivité des panneaux centraux,
## compagnons, mise de côté puis retour des panneaux ancrés, Échap sur le panneau du dessus,
## modal prioritaire.
## Usage : godot --headless --path game --script res://tests/ui_panel_stack_test.gd


func _init() -> void:
	await process_frame
	var layer := CanvasLayer.new()
	root.add_child(layer)
	var stack := PanelStack.new()
	var province := _panel(layer, "Province")
	var faction := _panel(layer, "Faction")
	var court := _panel(layer, "Court")
	var tech := _panel(layer, "Tech")
	var sheet := _panel(layer, "Sheet")
	var pause := _panel(layer, "Pause")
	stack.register(province, PanelStack.Kind.DOCKED)
	stack.register(faction, PanelStack.Kind.CENTRAL)
	stack.register(court, PanelStack.Kind.CENTRAL)
	stack.register(tech, PanelStack.Kind.CENTRAL)
	stack.register(sheet, PanelStack.Kind.COMPANION, [court])
	stack.register(pause, PanelStack.Kind.MODAL)
	var closed_count := [0]
	province.connect("closed", func() -> void: closed_count[0] += 1)

	province.show()
	check(stack.top() == province, "province on top")
	court.show()
	check(not province.visible and stack.is_suspended(province), "central suspends the docked panel")
	# Rafraîchissement de la province sous la Cour : elle reste de côté.
	province.show()
	check(not province.visible, "docked refresh stays hidden under a central panel")
	sheet.show()
	check(court.visible and sheet.visible, "companion keeps its host")
	tech.show()
	check(not court.visible and not sheet.visible and tech.visible, "centrals are exclusive, companion closes")
	faction.show()
	check(not tech.visible and faction.visible, "second central closes the first")
	# Échap : ferme le panneau du dessus, la province revient.
	check(stack.close_top(), "escape closes the top panel")
	check(not faction.visible and province.visible, "docked panel restored after the last central")
	check(stack.close_top() and not province.visible and closed_count[0] == 1, "escape then closes the province (closed emitted)")
	check(not stack.close_top(), "nothing left to close")
	# Nouvelle province pendant qu'un central est ouvert : `reveal` ferme les centraux.
	province.show()
	court.show()
	stack.reveal(province)
	check(not court.visible, "reveal closes central panels")
	province.show()
	check(province.visible, "revealed province is shown")
	# Désélection pendant la mise de côté : la province ne revient pas.
	tech.show()
	stack.forget_suspended(province)
	stack.close(tech)
	check(not province.visible, "forgotten docked panel is not restored")
	# Modal : Échap lui appartient.
	court.show()
	pause.show()
	check(not stack.close_top() and court.visible, "modal owns escape")
	pause.hide()
	check(stack.close_top() and not court.visible, "escape works again after the modal")
	# Compagnon seul (fiche ouverte depuis la carte) : ferme un central qui n'est pas son hôte.
	faction.show()
	sheet.show()
	check(not faction.visible and sheet.visible, "companion closes an unrelated central")
	# Fermer l'hôte ferme le compagnon.
	court.show()
	sheet.show()
	court.hide()
	check(not sheet.visible, "companion closes with its host")
	finish()


func _panel(parent: Node, panel_name: String) -> PanelContainer:
	var panel := PanelContainer.new()
	panel.name = panel_name
	panel.add_user_signal("closed")
	panel.hide()
	parent.add_child(panel)
	return panel


