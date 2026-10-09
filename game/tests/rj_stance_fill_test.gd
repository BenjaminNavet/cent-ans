extends TestCase

## Test headless du lot RJ-d (ADR 0175, lavis par position diplomatique) :
##  1. fonctions pures : couleur par catégorie (or nous, rouge ennemi, vert ami, gris autres),
##     couleurs reprises des frontières (stance_cues.json), opacités du JSON, provinces sans
##     contrôleur transparentes, contrôleur (et non propriétaire) pris en compte ;
##  2. atténuation de près et coupure hors mode politique ou par l'option du joueur ;
##  3. vraie carte, France jouée : nos provinces en or, la Guyenne anglaise (en guerre) en rouge,
##     texture posée sur le matériau du terrain, crochet présent dans le shader.
## Usage : godot --headless --path game --script res://tests/rj_stance_fill_test.gd

const PLAYER := "fac_france"


func _init() -> void:
	await process_frame
	_pure()
	await _real_map()
	finish()


func _same(a: Color, b: Color) -> bool:
	return absf(a.r - b.r) < 0.02 and absf(a.g - b.g) < 0.02 and absf(a.b - b.b) < 0.02


func _pure() -> void:
	var fill := {
		"colors": {"other": "#808080"},
		"alpha": {"self": 0.2, "enemy": 0.25, "friend": 0.15, "other": 0.05},
		"zoom": {"near_distance": 50.0, "far_distance": 250.0, "near_scale": 0.4},
		"modes": {"political": 1.0},
	}
	var cues := {
		"categories": {"self": "self", "war": "enemy", "ally": "friend", "vassal": "friend"},
		"border": {"self": "#d9a833", "enemy": "#d21f17", "friend": "#3a9a42"},
	}
	var gold := StanceFill.fill_color("self", fill, cues)
	check(_same(gold, Color.html("#d9a833")) and is_equal_approx(gold.a, 0.2), "self fill is the border gold at alpha 0.2: %s" % gold)
	var red := StanceFill.fill_color("enemy", fill, cues)
	check(_same(red, Color.html("#d21f17")) and is_equal_approx(red.a, 0.25), "enemy fill is the border red: %s" % red)
	var green := StanceFill.fill_color("friend", fill, cues)
	check(_same(green, Color.html("#3a9a42")) and is_equal_approx(green.a, 0.15), "friend fill is the border green: %s" % green)
	var grey := StanceFill.fill_color("other", fill, cues)
	check(_same(grey, Color.html("#808080")) and is_equal_approx(grey.a, 0.05), "other fill is the light grey: %s" % grey)
	check(grey.s < 0.05, "neutral fill carries no hue (never reads as a stance)")
	check(red.a > grey.a and gold.a > grey.a, "self and enemy weigh more than neutrals")
	var no_alpha := fill.duplicate(true)
	no_alpha["alpha"]["other"] = 0.0
	check(StanceFill.fill_color("other", no_alpha, cues).a == 0.0, "zero alpha means no fill")

	var controllers := PackedStringArray(["fac_france", "fac_england", "fac_scotland", "fac_flanders", "", "fac_avignon"])
	var stances := {"fac_england": "war", "fac_scotland": "ally", "fac_flanders": "neutral", "fac_avignon": "vassal"}
	var colors := StanceFill.colors_for(controllers, PLAYER, stances, fill, cues)
	check(colors.size() == controllers.size(), "one colour per province")
	check(colors[0] == gold, "our province is gold")
	check(colors[1] == red, "a province held by an enemy at war is red")
	check(colors[2] == green, "an ally's province is green")
	check(colors[3] == grey, "a neutral's province is light grey")
	check(colors[4].a == 0.0, "a province without controller is left bare")
	check(colors[5] == green, "a vassal counts as a friend")

	# Zoom et option : StanceFill hors carte (pas de matériau), réglages fournis.
	var node := StanceFill.new()
	node.tuning = fill
	check(is_equal_approx(node.zoom_factor(10.0), 0.4), "close up: near_scale")
	check(is_equal_approx(node.zoom_factor(400.0), 1.0), "far: full alpha")
	check(node.zoom_factor(150.0) > 0.4 and node.zoom_factor(150.0) < 1.0, "smooth fade in between")
	node.update_view(400.0)
	check(is_equal_approx(node.effective_alpha, 1.0), "political mode far: alpha 1")
	node.set_enabled(false)
	check(node.effective_alpha == 0.0, "player option off: no fill")
	node.set_enabled(true)
	node.mode = "religion"
	node.set_enabled(true)
	check(node.effective_alpha == 0.0, "other map modes paint their own colours: no fill")
	node.free()


func _real_map() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = PLAYER
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	var fill: StanceFill = map.get("stance_fill")
	if not check(fill != null, "campaign map has a StanceFill"):
		map.queue_free()
		return
	var material: ShaderMaterial = map.terrain.material
	var has_hook := false
	for entry in material.shader.get_shader_uniform_list():
		if str(entry["name"]) == "sf_alpha":
			has_hook = true
	check(has_hook, "terrain shader carries the sf_fill hook")
	check(material.get_shader_parameter("sf_colors") is Texture2D, "fill texture set on the terrain material")
	var cues := StanceCues.tuning()
	var colors := fill.province_colors()
	var data: MapData = map.map_data
	var paris := data.index_of_id("prov_ile_de_france") if data.has_method("index_of_id") else -1
	var guyenne := data.index_of_id("prov_guyenne") if data.has_method("index_of_id") else -1
	if check(paris > 0 and guyenne > 0, "province indices found"):
		check(_same(colors[paris - 1], Color.html(str(cues["border"]["self"]))), "Île-de-France is gold for France: %s" % colors[paris - 1])
		check(_same(colors[guyenne - 1], Color.html(str(cues["border"]["enemy"]))), "English Guyenne is red for France: %s" % colors[guyenne - 1])
	map.queue_free()
	await process_frame
