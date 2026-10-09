extends TestCase

## Lot FA5 — ornements réels de l'interface : le catalogue désigne des fichiers présents, la
## lettrine ne prend une initiale réelle que si le catalogue en liste une (aucune aujourd'hui :
## titres à lettrine dessinée, sans rinceau), les sceaux et les plaques de cuir se posent, et
## `FaUi.enabled = false` rend l'ancien habillage.
##   godot --headless --path game --script res://tests/fa5_ui_test.gd


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	await process_frame
	var catalogue := FaUi.data()
	check(not catalogue.is_empty(), "catalogue lu")
	for kind in ["initials", "ornaments", "seals", "materials"]:
		for entry: Dictionary in catalogue.get(kind, []):
			check(FaUi.texture(kind, str(entry["id"])) != null, "texture %s/%s importée" % [kind, entry["id"]])

	# Revue du lot : initiale réelle et rinceau refusés, désactivés par le catalogue (`initials`
	# vide, `display.title_spray` vide) ; le code reste, les titres gardent la lettrine dessinée.
	var has_initials := not (catalogue.get("initials", []) as Array).is_empty()
	for entry: Dictionary in catalogue.get("initials", []):
		check(not FaUi.initial_for(str(entry["letter"]) + "x").is_empty(), "initiale réelle pour %s" % entry["letter"])
	check(FaUi.initial_for("Zélande").is_empty() or has_initials, "catalogue sans initiale : aucune initiale réelle")
	if not has_initials:
		for title in ["Diplomatie", "Cour — France", "France"]:
			check(FaUi.initial_for(title).is_empty(), "pas d'initiale réelle pour « %s »" % title)
			var lettrine := await _titled(title, false)
			check(lettrine.get("_initial") == null, "lettrine dessinée pour « %s »" % title)
			check(float(lettrine.get("_field")) == float(lettrine.get("_box")), "champ de la lettrine inchangé pour « %s »" % title)
	var spray_id := str((catalogue.get("display", {}) as Dictionary).get("title_spray", ""))
	check(spray_id.is_empty() == (FaUi.ornament(spray_id) == null), "rinceau des titres : présent si et seulement si le catalogue le désigne")
	var fitted := await _titled("Diplomatie", true)
	check(fitted.get("_initial") == null, "pas d'initiale réelle dans un en-tête ajusté")

	var seal := FaUi.seal_rect("chronicle", 34.0)
	check(seal != null and seal.custom_minimum_size.y == 34.0, "sceau de la chronique posé à la hauteur demandée")
	check(FaUi.seal("treaty") != null, "sceau des traités")
	check(FaUi.seal("unknown_role") == null, "rôle de sceau inconnu : rien")
	if seal != null:
		seal.free()

	var button := Button.new()
	FrontEndStyle.style_action_button(button, true)
	check(button.get_theme_stylebox("normal") is StyleBoxTexture, "bouton d'action : plaque de cuir")
	check(button.get_theme_stylebox("normal").get_content_margin(SIDE_LEFT) == 28.0, "marges de contenu conservées")
	button.free()

	FaUi.enabled = false
	check(FaUi.seal_rect("chronicle", 34.0) == null, "désactivé : pas de sceau")
	var flat := Button.new()
	FrontEndStyle.style_action_button(flat, true)
	check(flat.get_theme_stylebox("normal") is StyleBoxFlat, "désactivé : bouton d'action plat")
	flat.free()
	FaUi.enabled = true

	print("fa5_ui_test: %s" % ("OK" if failures == 0 else "%d échec(s)" % failures))
	finish()


## Lettrine posée sur un titre, après une image.
func _titled(text: String, fit: bool) -> Lettrine:
	var label := Label.new()
	label.text = text
	root.add_child(label)
	var lettrine := Lettrine.attach(label, 0.0, fit)
	await process_frame
	return lettrine


