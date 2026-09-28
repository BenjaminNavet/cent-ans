extends SceneTree

## Test headless du lot UX1 « Carte lisible » : placement des plaques d'armée hors des noms de
## ville (rectangles synthétiques, stabilité, plaques entre elles, grille), légende de la carte
## (données, sections par mode, échantillons), boutons de la minicarte au thème parchemin.
## Usage : godot --headless --path game --script res://tests/ux1_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_placer_avoids_city_name()
	_test_placer_is_stable()
	_test_placer_separates_plates()
	_test_placer_scales()
	await _test_legend()
	await _test_minimap_buttons()
	if _failures == 0:
		print("ux1 OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("ux1: " + message)
	return condition


func _overlaps_any(rect: Rect2, others: Array) -> bool:
	for other in others:
		if (other as Rect2).intersects(rect):
			return true
	return false


## Une plaque posée sur « Paris » s'en écarte ; une plaque libre ne bouge pas.
func _test_placer_avoids_city_name() -> void:
	var placer := LabelPlacer.new()
	var paris := Rect2(600, 330, 60, 24)
	var plate := Rect2(610, 318, 50, 20)  # recouvre le haut du nom
	var free_plate := Rect2(100, 100, 50, 20)
	var offsets := placer.place([{"id": "a", "rect": plate}, {"id": "b", "rect": free_plate}], [paris])
	var moved := Rect2(plate.position + offsets["a"], plate.size)
	_check(not moved.intersects(paris), "plate a still covers the city name: %s" % moved)
	_check(offsets["a"] == LabelPlacer.candidate_offsets(plate.size)[1], "first fallback is straight above: %s" % offsets["a"])
	_check(offsets["b"] == Vector2.ZERO, "a free plate keeps its place")
	_check(placer.choice_of("b") == 0 and placer.choice_of("a") == 1, "choices recorded")


## Hystérésis : une plaque déplacée ne revient pas à sa base tant que le nom est tout proche ;
## elle y revient quand la voie est largement libre ; pas d'aller-retour sur deux images.
func _test_placer_is_stable() -> void:
	var placer := LabelPlacer.new()
	var plate := Rect2(610, 318, 50, 20)
	var paris := Rect2(600, 330, 60, 24)
	placer.place([{"id": "a", "rect": plate}], [paris])
	_check(placer.choice_of("a") == 1, "moved above")
	# Le nom descend de 10 px : la base serait tout juste libre (écart < hystérésis).
	var near := Rect2(600, 341, 60, 24)
	_check(not plate.intersects(near), "setup: base is barely free")
	var offsets := placer.place([{"id": "a", "rect": plate}], [near])
	_check(placer.choice_of("a") == 1 and offsets["a"] != Vector2.ZERO, "stays moved within the hysteresis margin")
	offsets = placer.place([{"id": "a", "rect": plate}], [near])
	_check(placer.choice_of("a") == 1, "no flicker on the next frame")
	# Nom loin : retour à la base.
	offsets = placer.place([{"id": "a", "rect": plate}], [Rect2(600, 400, 60, 24)])
	_check(placer.choice_of("a") == 0 and offsets["a"] == Vector2.ZERO, "returns home when clearly free")
	# Une armée disparue est oubliée.
	placer.place([], [])
	_check(placer.choice_of("a") == -1, "stale choices dropped")


## Deux plaques au même endroit : la première (prioritaire) reste, la seconde s'écarte.
func _test_placer_separates_plates() -> void:
	var placer := LabelPlacer.new()
	var first := Rect2(800, 150, 52, 20)
	var second := Rect2(806, 156, 52, 20)
	var offsets := placer.place([{"id": "big", "rect": first}, {"id": "small", "rect": second}], [])
	_check(offsets["big"] == Vector2.ZERO, "priority plate keeps its base")
	var moved := Rect2(second.position + offsets["small"], second.size)
	_check(not moved.intersects(first), "second plate no longer overlaps the first: %s" % moved)
	# Encerclée de toutes parts : on garde le candidat qui recouvre le moins, sans erreur.
	var crowd: Array = []
	for y in range(-3, 4):
		for x in range(-3, 4):
			crowd.append(Rect2(700 + x * 60, 300 + y * 24, 58, 22))
	offsets = placer.place([{"id": "trapped", "rect": Rect2(700, 300, 50, 20)}], crowd)
	_check(offsets.has("trapped"), "trapped plate still placed")


## Des dizaines d'armées et une centaine de noms : tous les cas libres trouvent une place libre.
func _test_placer_scales() -> void:
	var placer := LabelPlacer.new()
	var names: Array = []
	for i in 120:
		names.append(Rect2((i % 12) * 130.0, floorf(i / 12.0) * 90.0, 70, 22))
	var items: Array = []
	for i in 40:
		# Chaque plaque tombe sur un nom (au-dessus de sa moitié haute).
		var target: Rect2 = names[i * 3]
		items.append({"id": "army_%d" % i, "rect": Rect2(target.position + Vector2(8, -10), Vector2(50, 20))})
	var start := Time.get_ticks_usec()
	var offsets := placer.place(items, names)
	var elapsed := Time.get_ticks_usec() - start
	var placed: Array = []
	var clear := 0
	for item in items:
		var rect := Rect2((item["rect"] as Rect2).position + offsets[item["id"]], (item["rect"] as Rect2).size)
		if not _overlaps_any(rect, names) and not _overlaps_any(rect, placed):
			clear += 1
		placed.append(rect)
	_check(clear == items.size(), "all 40 plates clear of names and of each other (%d)" % clear)
	_check(elapsed < 50000, "placement of 40 plates is cheap (%d us)" % elapsed)


func _test_legend() -> void:
	var data := MapLegend.data()
	_check(not data.is_empty(), "map_legend.json loaded")
	_check(str(data.get("title", "")) != "", "legend has a title")
	var political := MapLegend.sections_for("political")
	var diplomacy := MapLegend.sections_for("diplomacy")
	var ids_political: Array = political.map(func(s: Dictionary) -> String: return str(s["id"]))
	var ids_diplomacy: Array = diplomacy.map(func(s: Dictionary) -> String: return str(s["id"]))
	_check(ids_political.has("mode_political") and not ids_political.has("mode_diplomacy"), "political sections: %s" % [ids_political])
	_check(ids_diplomacy.has("mode_diplomacy") and not ids_diplomacy.has("mode_political"), "diplomacy sections: %s" % [ids_diplomacy])
	for common in ["fog", "settlements", "armies", "agents"]:
		_check(ids_political.has(common) and ids_diplomacy.has(common), "section %s in every mode" % common)
	var legend := MapLegend.new()
	root.add_child(legend)
	var context := {"player_faction": "fac_france", "player_color": Color(0.2, 0.3, 0.7), "factions": [["fac_england", Color(0.8, 0.2, 0.2), "Angleterre"]]}
	legend.build("political", context)
	await process_frame
	var labels := legend.entry_labels()
	_check(labels.has("Cité") and labels.has("E : espion") and labels.has("Terres voilées"), "legend lists settlements, agents and fog: %s" % [labels])
	_check(labels.has("Territoire occupé"), "legend explains occupied borders: %s" % [labels])  # RS-E : FR1 hachures
	var political_count := legend.entry_count()
	legend.set_mode("diplomacy")
	await process_frame
	_check(legend.entry_labels().has("Guerre") and not legend.entry_labels().has("Aplats et liserés"), "diplomacy legend shows relations")
	_check(legend.entry_count() > political_count, "diplomacy legend has more colour entries")
	# Chaque échantillon a quelque chose à montrer.
	var samples := legend.find_children("Sample_*", "", true, false)
	_check(samples.size() == legend.entry_count(), "one sample per entry (%d/%d)" % [samples.size(), legend.entry_count()])
	for sample in samples:
		_check((sample as Control).get_child_count() == 1, "sample %s has content" % sample.name)
	var plates := legend.find_children("Plate_legend", "PanelContainer", true, false)
	_check(plates.size() >= 3, "army plates drawn with the real plate builder (%d)" % plates.size())
	_check(LegendSample.catalog().atlas != null and LegendSample.catalog().cell_of("city") >= 0, "settlement markers atlas (DA3)")
	legend.close_legend()
	_check(not legend.visible, "close hides the legend")
	legend.queue_free()


func _test_minimap_buttons() -> void:
	var minimap := CampaignMinimap.new()
	root.add_child(minimap)
	await process_frame
	_check(minimap.theme != null and minimap.theme.resource_path.ends_with("parchment_theme.tres"), "minimap uses the parchment theme")
	_check(minimap.legend_button != null and minimap.legend_button.toggle_mode, "legend button present")
	var toggled := [false]
	minimap.legend_toggled.connect(func(pressed: bool) -> void: toggled[0] = pressed)
	minimap.legend_button.button_pressed = true
	_check(toggled[0], "legend button emits legend_toggled")
	minimap.queue_free()
