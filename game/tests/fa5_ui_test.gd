extends SceneTree

## Lot FA5 — ornements réels de l'interface : le catalogue désigne des fichiers présents, la
## lettrine prend l'initiale réelle quand elle existe (et seulement hors en-tête ajusté), les
## sceaux et les plaques de cuir se posent, et `FaUi.enabled = false` rend l'ancien habillage.
##   godot --headless --path game --script res://tests/fa5_ui_test.gd

var _failures := 0


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	await process_frame
	var catalogue := FaUi.data()
	_check(not catalogue.is_empty(), "catalogue lu")
	for kind in ["initials", "ornaments", "seals", "materials"]:
		for entry: Dictionary in catalogue.get(kind, []):
			_check(FaUi.texture(kind, str(entry["id"])) != null, "texture %s/%s importée" % [kind, entry["id"]])

	_check(not FaUi.initial_for("Diplomatie").is_empty(), "initiale réelle pour un titre en D")
	_check(not FaUi.initial_for("dauphiné").is_empty(), "initiale réelle trouvée en minuscule")
	_check(FaUi.initial_for("Zélande").is_empty(), "pas d'initiale réelle pour une lettre absente")

	var drawn := await _titled("Zélande", false)
	var real := await _titled("Diplomatie", false)
	var fitted := await _titled("Diplomatie", true)
	_check(real.get("_initial") != null, "la lettrine prend l'initiale réelle")
	_check(drawn.get("_initial") == null, "la lettrine reste dessinée sans initiale réelle")
	_check(fitted.get("_initial") == null, "pas d'initiale réelle dans un en-tête ajusté")
	_check(float(real.get("_field")) > float(drawn.get("_field")), "l'initiale réelle est agrandie")

	var seal := FaUi.seal_rect("chronicle", 34.0)
	_check(seal != null and seal.custom_minimum_size.y == 34.0, "sceau de la chronique posé à la hauteur demandée")
	_check(FaUi.seal("treaty") != null, "sceau des traités")
	_check(FaUi.seal("unknown_role") == null, "rôle de sceau inconnu : rien")
	if seal != null:
		seal.free()

	var button := Button.new()
	FrontEndStyle.style_action_button(button, true)
	_check(button.get_theme_stylebox("normal") is StyleBoxTexture, "bouton d'action : plaque de cuir")
	_check(button.get_theme_stylebox("normal").get_content_margin(SIDE_LEFT) == 28.0, "marges de contenu conservées")
	button.free()

	FaUi.enabled = false
	_check(FaUi.initial_for("Diplomatie").is_empty(), "désactivé : pas d'initiale réelle")
	_check(FaUi.seal_rect("chronicle", 34.0) == null, "désactivé : pas de sceau")
	var flat := Button.new()
	FrontEndStyle.style_action_button(flat, true)
	_check(flat.get_theme_stylebox("normal") is StyleBoxFlat, "désactivé : bouton d'action plat")
	flat.free()
	FaUi.enabled = true

	print("fa5_ui_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(0 if _failures == 0 else 1)


## Lettrine posée sur un titre, après une image.
func _titled(text: String, fit: bool) -> Lettrine:
	var label := Label.new()
	label.text = text
	root.add_child(label)
	var lettrine := Lettrine.attach(label, 0.0, fit)
	await process_frame
	return lettrine


func _check(condition: bool, what: String) -> void:
	if not condition:
		_failures += 1
		print("FAIL: %s" % what)
