extends SceneTree

## Lot NV2 : finitions navales. Vérifie (sans fenêtre) :
## 1. le bandeau des navires tient dans l'écran en 1280×720 et 1920×1080 (cartes compactes sur
##    deux lignes), et défile au-delà (flèches) ;
## 2. les coques et les châteaux portent les matériaux procéduraux (bordé à clin, peinture) ;
## 3. en campagne, la traversée vers Calais se livre dans le pas de Calais (Manche) et les navires
##    portent des noms historiques (plus de « Nef n°1 »).
##   godot --headless --path game --script res://tests/nv2_naval_test.gd

var _failures: int = 0


func _init() -> void:
	await process_frame
	await _run_hud()
	await _run_materials()
	_run_campaign()
	print("NV2 naval test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("NV2: " + message)
	else:
		print("  ok  " + message)


func _data_dir() -> String:
	return ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()


func _fake_ships(count: int) -> Array:
	var ships := []
	for i in count:
		ships.append({
			"id": i, "side": "attacker", "name": "la Trinité de Winchelsea %d" % i, "class_name": "Cogue",
			"status": "afloat", "soldiers": 70.0, "sailors": 24.0, "hull": 100.0, "hull_max": 100.0,
			"fire": 0.0, "order": "hold", "chain": -1, "flagship": i == 0,
		})
	return ships


func _run_hud() -> void:
	for case in [[Vector2i(1280, 720), 13], [Vector2i(1920, 1080), 13], [Vector2i(1920, 1080), 8], [Vector2i(1280, 720), 30]]:
		var screen: Vector2i = case[0]
		var count: int = case[1]
		root.size = screen
		var hud := NavalHud.new()
		root.add_child(hud)
		await process_frame
		hud.update_cards(_fake_ships(count), [])
		await process_frame
		var layout := hud.card_layout()
		var columns := int(layout["columns"])
		var width := float(layout["card_width"])
		var used := columns * (width + 5.0) - 5.0
		var label := "%d×%d, %d navires" % [screen.x, screen.y, count]
		if count <= 8 and screen.x >= 1920:
			_check(not bool(layout["compact"]) and columns == count, "%s : cartes pleines sur une ligne" % label)
		elif count <= 14:
			_check(bool(layout["compact"]) and columns == ceili(count / 2.0), "%s : cartes compactes sur deux lignes (%d colonnes)" % [label, columns])
			_check(used <= float(layout["strip_width"]) + 0.5 and not bool(layout["scroll"]), "%s : le bandeau tient dans l'écran (%d px sur %d)" % [label, int(used), int(layout["strip_width"])])
		else:
			_check(bool(layout["scroll"]), "%s : le bandeau défile (flèches)" % label)
		# Le bas du bandeau reste dans l'écran.
		var strip := hud.get_node("Root/Bottom") as Control
		var visible := hud.get_viewport().get_visible_rect()
		_check(strip.get_global_rect().end.y <= visible.end.y + 0.5 and strip.get_global_rect().end.x <= visible.end.x + 0.5, "%s : bandeau dans l'écran (%s dans %s)" % [label, strip.get_global_rect().end, visible.end])
		hud.queue_free()
		await process_frame
	root.size = Vector2i(1280, 720)


func _run_materials() -> void:
	var scene: NavalScene = load("res://scenes/naval/naval_battle.tscn").instantiate()
	scene.scenario_id = "sluys"
	root.add_child(scene)
	await process_frame
	var hulls := 0
	var paints := 0
	for view in scene.views.values():
		for child in (view as Node).find_children("*", "MeshInstance3D", true, false):
			var mesh := (child as MeshInstance3D).mesh
			if mesh == null:
				continue
			for surface in mesh.get_surface_count():
				var material := mesh.surface_get_material(surface) as ShaderMaterial
				if material == null or material.shader == null:
					continue
				var path := material.shader.resource_path
				if path.ends_with("naval_hull.gdshader"):
					hulls += 1
				elif path.ends_with("naval_paint.gdshader"):
					paints += 1
	_check(hulls >= 26, "coques en bordé à clin (%d surfaces)" % hulls)
	_check(paints >= 26, "châteaux et pavois peints (%d surfaces)" % paints)
	scene.queue_free()
	await process_frame


func _run_campaign() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_check(false, "CampaignSim class")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	_check(bool(sim.call("new_campaign", _data_dir(), "fac_england", 1337)), "campaign as England")
	var armies := BattleScene.main_armies(sim, "fac_england", "fac_france")
	if armies.is_empty():
		_check(false, "main English army")
		return
	var index: int = sim.call("debug_stage_naval", armies[0], "set_calais", "fac_france")
	_check(index >= 0, "French squadron intercepts the crossing to Calais")
	if index < 0:
		return
	var setup: Dictionary = sim.call("get_naval_battle_setup", index)
	_check(str(setup.get("sea_zone", "")) == "sea_channel", "crossing to Calais in the Channel (%s)" % setup.get("sea_zone", ""))
	_check(str(setup.get("place_name", "")) == "le pas de Calais", "place: %s" % setup.get("place_name", ""))
	var names: Array = []
	for side in ["attacker", "defender"]:
		for ship in (setup[side] as Dictionary).get("ships", []):
			names.append(str(ship.get("name", "")))
	var numbered := names.filter(func(n: String) -> bool: return n.contains("n°"))
	_check(not names.is_empty() and numbered.is_empty(), "historical ship names: %s" % ", ".join(names.slice(0, 6)))
