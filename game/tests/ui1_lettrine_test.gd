extends TestCase

## Lot UI1 — lettrines de titres : la lettrine se pose sur le label sans toucher à son texte,
## suit ses changements, et un second `attach` ne la duplique pas.
##   godot --headless --path game --script res://tests/ui1_lettrine_test.gd


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	var panel: Control = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	var title: Label = panel.get_node("%TitleLabel")
	var lettrine := title.get_node_or_null("Lettrine") as Lettrine
	check(lettrine != null, "lettrine posée sur le titre de la cour")
	check(Lettrine.attach(title) == lettrine, "attach idempotent")
	check(title.get_children().filter(func(c: Node) -> bool: return c is Lettrine).size() == 1, "une seule lettrine")
	var rows: Array[Dictionary] = []
	panel.call("show_court", rows, "Angleterre", Color(0.6, 0.1, 0.1))
	await process_frame
	check(title.text == "Cour — Angleterre", "texte du label intact : %s" % title.text)
	check(lettrine.get("_text") == title.text, "la lettrine suit le texte")
	check(title.self_modulate.a == 0.0, "label d'origine transparent")
	check(title.get_combined_minimum_size().y >= 30.0, "hauteur du champ réservée")
	title.text = "« entre guillemets »"
	await process_frame
	check(not lettrine.call("_has_initial"), "pas d'initiale sur un titre non alphabétique")
	panel.queue_free()
	await process_frame
	await _check_accented_titles()
	await _check_diplomacy_panel()
	finish()


## CV3-0 (#10) : un titre à initiale accentuée (« Île-de-France ») doit réserver la même place
## qu'un titre ordinaire, dans une mise en page réaliste (HBoxContainer + bouton, comme
## `province_panel.tscn`) : pas de recouvrement du texte par le champ de la lettrine.
func _check_accented_titles() -> void:
	for text in ["Île-de-France", "Éperon", "Diplomatie"]:
		var box := HBoxContainer.new()
		root.add_child(box)
		var label := Label.new()
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		label.add_theme_font_size_override("font_size", 24)
		label.text = text
		box.add_child(label)
		var close := Button.new()
		close.text = "×"
		box.add_child(close)
		var lettrine := Lettrine.attach(label)
		for f in 3:
			await process_frame
		check(bool(lettrine.call("_has_initial")), "initiale détectée sur « %s »" % text)
		var rest_width: float = float(label.size.x) - (float(lettrine.get("_box")) + 1.0)
		check(rest_width > 0.0, "place réservée pour la lettrine de « %s » (largeur label %s)" % [text, label.size.x])
		box.queue_free()
		await process_frame


## CV3-0 (#10) : le titre de l'écran de diplomatie n'est plus tronqué à la main (« iplomatie »,
## l'ancien `DropCap` local) et passe par le kit partagé comme les autres panneaux.
func _check_diplomacy_panel() -> void:
	var panel: Control = DiplomacyPanel.new()
	root.add_child(panel)
	for f in 3:
		await process_frame
	var found := false
	for label in _all_labels(panel):
		if label.text == "Diplomatie":
			found = true
			check(label.get_node_or_null("Lettrine") != null, "lettrine posée sur le titre « Diplomatie »")
		check(label.text != "iplomatie", "le titre ne doit plus être tronqué à la main")
	check(found, "le titre « Diplomatie » (texte complet) est présent")
	panel.queue_free()
	await process_frame


func _all_labels(node: Node) -> Array[Label]:
	var result: Array[Label] = []
	if node is Label:
		result.append(node)
	for child in node.get_children():
		result.append_array(_all_labels(child))
	return result


