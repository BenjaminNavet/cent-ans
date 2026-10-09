extends TestCase

## Test headless du chantier RC (ADR 0141) : affichage des fleuves de la carte de campagne.
##  1. `river_display.json` est appliqué (largeurs écran, distance des rivières mineures, liseré) ;
##  2. `RiverLabels` : étiquettes posées sur les données réelles, noms français
##     (`river_names.json`), aucune dans les zones personnalisées, portée croissante avec
##     l'importance, au plus `max_labels`.
## Usage : godot --headless --path game --script res://tests/rc3_river_labels_test.gd


func _init() -> void:
	await process_frame
	_run()
	finish()


func _run() -> void:
	var data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if not check(data.load_error == "", "map data: %s" % data.load_error):
		return
	var rivers := RiversRenderer.new()
	root.add_child(rivers)
	rivers.build(data)
	var extent := maxf(data.size.x, data.size.y)
	var display := rivers.display
	check(not display.is_empty(), "river_display.json loaded")
	check(is_equal_approx(rivers.major_min_px, float(display["major_min_px"])), "major_min_px from data")
	check(is_equal_approx(rivers.minor_max_distance, float(display["minor_max_distance"]) * extent), "minor distance from data")
	var water := rivers.material_override as ShaderMaterial
	check(water != null and is_equal_approx(float(water.get_shader_parameter("bank_ink_strength")), float(display["bank_ink_strength"])), "bank ink on the major rivers")
	if not check(rivers.labels != null, "labels built"):
		return
	var placed := rivers.labels.placed()
	var cfg: Dictionary = display["labels"]
	check(placed.size() > 200, "many river labels (%d)" % placed.size())
	check(placed.size() <= int(cfg["max_labels"]), "at most max_labels")
	var texts := {}
	for entry in placed:
		texts[entry["text"]] = true
		check(not rivers.in_custom_zone(entry["px"]), "no label inside a custom zone (%s)" % entry["text"])
	for name in ["Loire", "Seine", "Garonne", "Rhin", "Meuse", "Danube"]:
		check(texts.has(name), "label %s" % name)
	for source in ["Maas", "Rhine", "Rhein", "Donau"]:
		check(not texts.has(source), "source name %s translated" % source)
	# Portée : un grand fleuve se lit de plus loin qu'un affluent.
	var reach := {}
	for label: Label3D in rivers.labels.get_children():
		reach[int(label.get_meta("importance", 0))] = label.visibility_range_end
	if reach.has(6) and reach.has(1):
		check(float(reach[6]) > float(reach[1]), "importance 6 visible farther than importance 1")
	print("rc3_river_labels_test: %d labels, %d names, stats %s" % [placed.size(), texts.size(), JSON.stringify(rivers.stats)])
	rivers.queue_free()
