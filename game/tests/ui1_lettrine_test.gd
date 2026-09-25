extends SceneTree

## Lot UI1 — lettrines de titres : la lettrine se pose sur le label sans toucher à son texte,
## suit ses changements, et un second `attach` ne la duplique pas.
##   godot --headless --path game --script res://tests/ui1_lettrine_test.gd

var _failures := 0


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	var panel: Control = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	var title: Label = panel.get_node("%TitleLabel")
	var lettrine := title.get_node_or_null("Lettrine") as Lettrine
	_check(lettrine != null, "lettrine posée sur le titre de la cour")
	_check(Lettrine.attach(title) == lettrine, "attach idempotent")
	_check(title.get_children().filter(func(c: Node) -> bool: return c is Lettrine).size() == 1, "une seule lettrine")
	var rows: Array[Dictionary] = []
	panel.call("show_court", rows, "Angleterre", Color(0.6, 0.1, 0.1))
	await process_frame
	_check(title.text == "Cour — Angleterre", "texte du label intact : %s" % title.text)
	_check(lettrine.get("_text") == title.text, "la lettrine suit le texte")
	_check(title.self_modulate.a == 0.0, "label d'origine transparent")
	_check(title.get_combined_minimum_size().y >= 30.0, "hauteur du champ réservée")
	title.text = "« entre guillemets »"
	await process_frame
	_check(not lettrine.call("_has_initial"), "pas d'initiale sur un titre non alphabétique")
	print("ui1_lettrine_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("ÉCHEC : " + message)
