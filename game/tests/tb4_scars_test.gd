extends TestCase

## Lot TB4 (campagne façon Thrones of Britannia) : conséquences visibles de la guerre et des
## fléaux.
##  1. brûlis posé par-dessus la carte de couleur (SS2) et les matières (HB3), masque de terroir
##     échantillonné à son échelle (carte non carrée) ;
##  2. peste : fosses, portes marquées, charrette des morts selon l'intensité, au palier proche,
##     sans pictogramme ; retirées quand la peste cesse ;
##  3. champ de bataille : tertre, corbeaux, débris à l'endroit de la bataille, corbeaux puis
##     débris partis, puis plus rien après `battlefield.turns` tours (`data/ui/war_scars.json`) ;
##  3 bis. historique du cœur (`get_battle_history`, vraie simulation) : bataille, sauvegarde,
##     rechargement dans un rendu neuf : la marque revient au même endroit avec son âge, puis
##     disparaît à l'échéance ; une sauvegarde sans historique se charge (aucune marque) ;
##  4. siège : engins du camp selon l'avancement (`get_assault_odds().engines`).
## Usage : godot --headless --path game --script res://tests/tb4_scars_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")


## Simulation factice : tour, événements, armées et engins de siège réglés par le test.
class FakeSim:
	extends RefCounted

	var turn := 1
	var history: Array = []
	var armies: Dictionary = {}
	var engines: Dictionary = {}
	var odds_calls := 0

	func get_turn() -> int:
		return turn

	func get_battle_history() -> Array:
		return history

	func get_army(id: String) -> Dictionary:
		return armies.get(id, {})

	func get_assault_odds(id: String) -> Dictionary:
		odds_calls += 1
		return {"available": engines.has(id), "engines": engines.get(id, [])}


class FakeMarker:
	extends Node3D

	var status := "siege"
	var figures: Node3D = null


## Remplace `ArmyMarkers` : marqueurs, positions et provinces sous le brouillard.
class FakeArmies:
	extends Node

	var markers: Dictionary = {}
	var positions: Dictionary = {}
	var hidden_provinces: Dictionary = {}

	func marker_ids() -> Array:
		return markers.keys()

	func marker_of(id: String) -> Node3D:
		return markers.get(id)

	func world_position_of(id: String) -> Vector3:
		return positions.get(id, Vector3.ZERO)


func _init() -> void:
	await process_frame
	_check_burn()
	var data: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(MAP_PATHS.default_data_dir().path_join(WarScars.DATA_PATH)))
	check(not WarScars.settings().is_empty(), "war_scars.json read")
	_check_plague(data["plague"])
	_check_battlefield(data["battlefield"])
	_check_history(data["battlefield"])
	_check_siege(data["siege"])
	WarScarMeshes.clear_cache()
	FolkModels.clear_cache()
	finish()


## Point 1 : le brûlis vient après `sg_apply` et `hb_apply` (il n'est plus recouvert), et le
## masque de terroir est lu à l'échelle où `TerroirMask` le peint.
func _check_burn() -> void:
	var code := TERRAIN_SHADER.code
	var burn := code.find("col = terroir_burn(")
	check(burn >= 0, "terrain shader no longer applies terroir_burn")
	check(code.find("sg_apply(uv") >= 0 and code.find("sg_apply(uv") < burn, "burn is applied before the colour map (sg_apply)")
	check(code.find("hb_apply(p") >= 0 and code.find("hb_apply(p") < burn, "burn is applied before the HB materials (hb_apply)")
	check(code.contains("terroir_at(p / max(map_size.x, map_size.y))"), "terroir mask not sampled at the scale it is painted (non-square map)")
	var uniforms := TERRAIN_SHADER.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))
	check(code.contains("col, burn_parcel);") and code.contains("burn_parcel, 1.0 / max(map_size.x, map_size.y))"), "burn does not use the HB parcels")
	for uniform_name in ["burnt_color", "ash_color", "terroir_mask", "burn_active", "burn_charred_share", "burn_singed_share", "burn_singe_tint", "burn_ground_keep"]:
		check(uniforms.has(uniform_name), "terrain shader lacks uniform %s" % uniform_name)
	# Même convention côté processeur : un point peint se relit au même endroit.
	var mask := TerroirMask.new()
	var map_size := Vector2(7168.0, 6144.0)
	var at := Vector2(2213.2, 3203.9)
	mask.build([{"kind": "city", "province": "prov_a", "px": at}], [], {"prov_a": {"devastation": 100.0, "population": TerroirMask.REFERENCE_POPULATION}}, null, map_size)
	check(mask.sample(at).b > 0.9, "burn painted at the settlement: %s" % mask.sample(at))
	var texel := at / maxf(map_size.x, map_size.y) * float(TerroirMask.SIZE)
	check(mask.image.get_pixel(int(texel.x), int(texel.y)).b > 0.9, "shader lookup (p / max side) hits the burnt texel")
	var stretched := at / map_size * float(TerroirMask.SIZE)
	check(mask.image.get_pixel(int(stretched.x), int(stretched.y)).b < 0.05, "old lookup (uv) missed the burn: the fix matters")


func _new_scars(armies: Object) -> WarScars:
	var scars := WarScars.new()
	root.add_child(scars)
	scars.setup(null, null, armies)
	return scars


## Vrai si la branche ne contient que de la géométrie 3D (aucun signe d'interface : ni
## étiquette, ni sprite, ni contrôle).
func _only_meshes(node: Node) -> bool:
	for child in node.find_children("*", "", true, false):
		if child is Label3D or child is SpriteBase3D or child is CanvasItem or child is CanvasLayer:
			return false
	return true


## Point 2 : colonie pestiférée → fosses et charrette au bord, portes marquées sur les maisons
## du plan, nombres selon l'intensité (données), visibles au palier proche seulement.
func _check_plague(block: Dictionary) -> void:
	var scars := _new_scars(null)
	var anchor := Vector2(100.0, 200.0)
	var houses := {"x": PackedFloat32Array(), "y": PackedFloat32Array(), "yaw": PackedFloat32Array(), "depth": PackedFloat32Array()}
	for i in 40:
		houses["x"].append(-190.0 + 10.0 * i)
		houses["y"].append(30.0 if i % 2 == 0 else -30.0)
		# Rue le long de y = 0 : la maison du côté n = (0, ±1) a `yaw = atan2(n.x, -n.y)`
		# (`TownPlan._add_house`), sa façade regarde la rue.
		houses["yaw"].append(PI if i % 2 == 0 else 0.0)
		houses["depth"].append(8.0)
	var plan_ready := [false]
	scars.plan_provider = func(_id: String) -> Dictionary:
		return {"plan": {"houses": houses}, "anchor": anchor, "meters_per_unit": 719.0} if plan_ready[0] else {}
	var road := PackedVector2Array([anchor, anchor + Vector2(0.0, -2.0), anchor + Vector2(1.0, -12.0)])
	var site := {"settlement": "set_test", "center": anchor, "out": Vector2.RIGHT, "footprint": 0.5, "intensity": 1.0, "seed": 42, "road": road}
	scars.set_plague_sites([site])
	var node := scars.plague_node("set_test")
	if not check(node != null, "plague site built"):
		return
	var pits: Array = block["pits"]
	var doors: Array = block["doors"]
	check(scars.plague_count("set_test", "Pit_") == int(pits[1]), "pits at full intensity: %d" % scars.plague_count("set_test", "Pit_"))
	check(scars.plague_count("set_test", "DeadCart") == (1 if block["cart"] else 0), "dead cart by the pits")
	check(not node.visible, "plague hidden before the first view")
	# Palier proche : visible ; les portes attendent le plan de la ville 1:1.
	scars.update_view(float(block["max_distance"]) * 0.5, 1.0)
	check(node.visible, "plague shown near")
	check(scars.plague_count("set_test", "Door_") == 0, "no door before the town plan is loaded")
	plan_ready[0] = true
	scars._door_timer = 0.0
	scars.update_view(float(block["max_distance"]) * 0.5, 1.0)
	check(scars.plague_count("set_test", "Door_") == int(doors[1]), "marked doors at full intensity: %d" % scars.plague_count("set_test", "Door_"))
	# Chaque porte tient à une façade : à une demi-profondeur de maison (+ 0,15 m) de son axe.
	for child in node.get_children():
		if not str(child.name).begins_with("Door_"):
			continue
		var local := (Vector2((child as Node3D).position.x, (child as Node3D).position.z) - anchor) * 719.0
		check(absf(absf(local.y) - 25.85) < 0.2 and local.length() <= float(block["door_search_m"]) + 5.0, "door on a street front within the search radius: %s" % local)
	# Fosses hors de la colonie, du côté de la scène.
	var pit := node.get_node("Pit_0") as Node3D
	check(pit.position.x > anchor.x + 0.5, "pits beyond the settlement edge")
	check(_only_meshes(node), "plague shown by 3D props only (no interface sign)")
	check(int(scars.stats.get("doors", 0)) == int(doors[1]) and int(scars.stats.get("pits", 0)) == int(pits[1]), "stats: %s" % scars.stats)
	_check_plague_screen(scars, node, block)
	_check_door_cross()
	# Vue lointaine et parchemin : rien.
	scars.update_view(float(block["max_distance"]) * 2.0, 1.0)
	check(not node.visible, "plague hidden beyond max_distance")
	scars.update_view(float(block["max_distance"]) * 0.5, 0.0)
	check(not node.visible, "plague hidden on the parchment view")
	# Peste faible : moins de fosses ; peste finie : plus rien.
	site["intensity"] = 0.0
	scars.set_plague_sites([site])
	check(scars.plague_count("set_test", "Pit_") == int(pits[0]), "pits at low intensity")
	scars.set_plague_sites([])
	check(scars.plague_node("set_test") == null and int(scars.stats.get("plague_sites", -1)) == 0, "plague props removed when the plague ends")
	scars.free()


## Point 3 : bataille → champ marqué à l'endroit de l'armée, qui s'efface par étapes puis
## disparaît après `turns` tours.
func _check_battlefield(block: Dictionary) -> void:
	var armies := FakeArmies.new()
	root.add_child(armies)
	var scars := _new_scars(armies)
	var sim := FakeSim.new()
	var turns := int(block["turns"])
	check(WarScars.battlefield_turns() == turns and turns >= 2, "battlefield duration read from data: %d" % WarScars.battlefield_turns())
	sim.history = [{"province": "prov_a", "turn": sim.turn, "position": Vector2(50.0, 60.0), "attacker_losses": 3, "defender_losses": 5}]
	scars.refresh(sim)
	check(scars.battlefield_keys() == ["prov_a"], "one marked field per battle province: %s" % [scars.battlefield_keys()])
	var at := scars.battlefield_at("prov_a")
	check(at.distance_to(Vector2(50.0, 60.0)) <= WarScars.FIELD_OFFSET + 0.01, "field at the place of the battle: %s" % at)
	var node := scars.battlefield_node("prov_a")
	if not check(node != null, "battlefield node"):
		return
	check(node.get_node_or_null("Mound") != null, "mound")
	check(_count(node, "Crow_") == int(block["crows"]), "crows: %d" % _count(node, "Crow_"))
	check(_count(node, "Debris_") == int(block["debris"]), "debris: %d" % _count(node, "Debris_"))
	check(_only_meshes(node), "battlefield shown by 3D props only")
	# Vue : taille des armées, caché sous le brouillard de guerre et sur le parchemin.
	scars.update_view(100.0, 1.0)
	check(node.visible, "battlefield shown in the normal view")
	var mound := node.get_node("Mound") as Node3D
	check(is_equal_approx(mound.scale.x, ArmyMarkers.scale_for_distance(100.0)), "battlefield follows the army scale: %s" % mound.scale)
	check(Vector2(mound.position.x, mound.position.z).distance_to(at) < 0.001, "mound at the field")
	var crow := node.get_node("Crow_0") as Node3D
	var crow_before := crow.position
	scars._time += 1.0
	scars.update_view(100.0, 1.0)
	check(crow.position.distance_to(crow_before) > 1e-4 and crow.position.y > mound.position.y, "crows circle above the mound")
	armies.hidden_provinces["prov_a"] = true
	scars.update_view(100.0, 1.0)
	check(not node.visible, "battlefield hidden under the fog of war")
	armies.hidden_provinces.clear()
	scars.update_view(100.0, 0.0)
	check(not node.visible, "battlefield hidden on the parchment view")
	scars.update_view(float(block["max_distance"]) * 1.5, 1.0)
	check(not node.visible, "battlefield hidden beyond max_distance")
	# Les tours passent sans nouvelle bataille : corbeaux, débris, puis tertre s'en vont.
	for age in range(1, turns + 1):
		sim.turn += 1
		scars.refresh(sim)
		node = scars.battlefield_node("prov_a")
		if age >= turns:
			check(node == null and scars.battlefield_keys().is_empty(), "field gone after %d turns" % turns)
			break
		if not check(node != null and node.get_node_or_null("Mound") != null, "mound still there at age %d" % age):
			break
		check((_count(node, "Crow_") > 0) == (age < int(block["crow_turns"])), "crows at age %d: %d" % [age, _count(node, "Crow_")])
		check((_count(node, "Debris_") > 0) == (age < int(block["debris_turns"])), "debris at age %d: %d" % [age, _count(node, "Debris_")])
	# Partie chargée : oubli.
	scars.reset()
	check(scars.battlefield_keys().is_empty(), "reset forgets the marked fields")
	scars.free()
	armies.free()


## Point 3 bis : les marques viennent de l'historique des batailles du cœur et survivent au
## rechargement. Vraie simulation (France, graine 1337) : bataille rangée auto-résolue au tour 0,
## un tour, sauvegarde, rechargement dans une autre simulation et un rendu neuf.
func _check_history(block: Dictionary) -> void:
	if not check(ClassDB.class_exists("CampaignSim"), "CampaignSim not registered (run core/build.sh)"):
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not check(sim.has_method("get_battle_history"), "bridge lacks get_battle_history (run core/build.sh)"):
		return
	var data_dir := MAP_PATHS.default_data_dir()
	if not check(sim.new_campaign(data_dir, "fac_france", 1337), "new_campaign failed"):
		return
	check((sim.get_battle_history() as Array).is_empty(), "no battle in the history at the start")
	var rules: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("rules/battle_history.json")))
	var turns := int(block["turns"])
	check(turns <= int(rules["max_age_turns"]), "the core keeps battles at least as long as the map marks them (%d ≤ %d)" % [turns, int(rules["max_age_turns"])])
	# Une armée française attaque une armée anglaise amenée à elle ; résolution automatique.
	var french := ""
	var english := ""
	for id in sim.get_army_ids():
		var faction := str(sim.get_army(id).get("faction", ""))
		if faction == "fac_france" and french == "":
			french = id
		elif faction == "fac_england" and english == "":
			english = id
	if not check(french != "" and english != "", "a French and an English army at the start"):
		return
	var index: int = sim.debug_stage_battle(french, english)
	if not check(index >= 0, "debug_stage_battle failed"):
		return
	var fought_turn: int = sim.get_turn()
	check(not (sim.auto_resolve_battle(index) as Array).is_empty(), "battle auto-resolved")
	var history: Array = sim.get_battle_history()
	if not check(history.size() == 1, "one battle in the history: %s" % [history]):
		return
	var record: Dictionary = history[0]
	var province := str(record.get("province", ""))
	check(int(record.get("turn", -1)) == fought_turn and int(record.get("age", -1)) == 0, "record of this turn: %s" % record)
	check(province != "" and record.get("position") is Vector2 and str(record.get("kind", "")) == "field", "record place and kind: %s" % record)
	check(str(record.get("attacker", "")) == "fac_france" and str(record.get("defender", "")) == "fac_england"
		and str(record.get("winner", "")) in ["fac_france", "fac_england"], "record sides: %s" % record)
	check(int(record.get("attacker_strength", 0)) > 0 and int(record.get("attacker_losses", 0)) + int(record.get("defender_losses", 0)) > 0, "record strengths and losses: %s" % record)
	var place: Vector2 = record["position"]
	# La marque est posée tout de suite (bataille de la main du joueur), au lieu de l'historique.
	var armies := FakeArmies.new()
	root.add_child(armies)
	var scars := _new_scars(armies)
	scars.refresh(sim)
	check(scars.battlefield_keys() == [province], "field marked from the history: %s" % [scars.battlefield_keys()])
	var at := scars.battlefield_at(province)
	check(at.distance_to(place) <= WarScars.FIELD_OFFSET + 0.01, "field at the recorded place: %s vs %s" % [at, place])
	check(_count(scars.battlefield_node(province), "Crow_") == int(block["crows"]), "fresh field: crows")
	# Un tour passe, puis sauvegarde.
	sim.end_turn()
	scars.refresh(sim)
	var saved: String = sim.save_to_string()
	check(saved.contains("\"battle_history\""), "history written in the save")
	scars.free()
	# Rechargement : autre simulation, rendu neuf (aucune mémoire des tours passés).
	var loaded: Object = ClassDB.instantiate("CampaignSim")
	if not check(loaded.load_from_string(saved), "save with a battle history loads"):
		armies.free()
		return
	scars = _new_scars(armies)
	scars.refresh(loaded)
	var last := _last_battle_turn(loaded, province)
	var age: int = int(loaded.get_turn()) - last
	check(last >= fought_turn and age <= 1, "history kept across the reload: last battle of %s at turn %d" % [province, last])
	if last == fought_turn:
		check(scars.battlefield_at(province).is_equal_approx(at), "reloaded field at the same place: %s vs %s" % [scars.battlefield_at(province), at])
	if check(scars.battlefield_keys().has(province), "field still marked after the reload: %s" % [scars.battlefield_keys()]):
		check(int(scars._fields[province]["turn"]) == last, "reloaded field keeps the turn of the battle (age %d)" % age)
		_check_stage(scars.battlefield_node(province), block, age)
	# Les tours passent : la marque suit l'âge de la bataille puis part à l'échéance.
	var expired := false
	for _i in turns + 2:
		loaded.end_turn()
		scars.refresh(loaded)
		last = _last_battle_turn(loaded, province)
		age = int(loaded.get_turn()) - last
		if last < 0 or age >= turns:
			check(not scars.battlefield_keys().has(province), "field gone %d turns after its last battle" % turns)
			expired = last == fought_turn or last < 0
			if expired:
				check(int(loaded.get_turn()) - fought_turn >= turns, "field not dropped early")
			break
		if check(scars.battlefield_keys().has(province), "field still marked at age %d" % age):
			check(int(scars._fields[province]["turn"]) == last, "field turn follows the history at age %d" % age)
			_check_stage(scars.battlefield_node(province), block, age)
	print("tb4_scars_test: history: battle at turn %d in %s, mark gone at turn %d (%s)" % [fought_turn, province, int(loaded.get_turn()),
		"its own deadline" if expired else "after later battles there"])
	# Toutes les marques viennent de batailles encore dans l'historique, d'âge inférieur à `turns`.
	for key in scars.battlefield_keys():
		var key_age: int = int(loaded.get_turn()) - _last_battle_turn(loaded, str(key))
		check(key_age >= 0 and key_age < turns, "mark %s backed by a recent battle (age %d)" % [key, key_age])
	# Sauvegarde d'avant l'historique (clé absente) : elle se charge, aucune marque.
	# La clé est ôtée dans le texte (les entiers de la sauvegarde restent tels quels).
	var start := saved.find(",\"battle_history\":[")
	var end := _matching_bracket(saved, saved.find("[", start)) if start >= 0 else -1
	if check(start >= 0 and end > start, "battle_history key found in the save text"):
		var old: Object = ClassDB.instantiate("CampaignSim")
		if check(old.load_from_string(saved.substr(0, start) + saved.substr(end + 1)), "a save without battle_history loads"):
			check((old.get_battle_history() as Array).is_empty(), "older save: empty history")
			check(int(old.get_turn()) == fought_turn + 1, "older save: same game otherwise")
			scars.reset()
			scars.refresh(old)
			check(scars.battlefield_keys().is_empty(), "older save: no mark, no error")
	scars.free()
	armies.free()


## Position du crochet fermant qui répond au crochet ouvrant en `open` (hors chaînes : les
## enregistrements de l'historique n'ont ni crochet ni guillemet échappé dans leurs textes).
func _matching_bracket(text: String, open: int) -> int:
	var depth := 0
	for i in range(open, text.length()):
		var c := text[i]
		if c == "[":
			depth += 1
		elif c == "]":
			depth -= 1
			if depth == 0:
				return i
	return -1


## Tour de la dernière bataille avec des morts de `province` dans l'historique du pont (-1 sans).
func _last_battle_turn(sim: Object, province: String) -> int:
	var last := -1
	for record in sim.get_battle_history():
		if str(record.get("province", "")) == province and int(record.get("attacker_losses", 0)) + int(record.get("defender_losses", 0)) > 0:
			last = maxi(last, int(record.get("turn", -1)))
	return last


## Tertre présent, corbeaux et débris selon l'âge (`crow_turns`, `debris_turns`).
func _check_stage(node: Node3D, block: Dictionary, age: int) -> void:
	if not check(node != null and node.get_node_or_null("Mound") != null, "mound at age %d" % age):
		return
	check((_count(node, "Crow_") > 0) == (age < int(block["crow_turns"])), "crows at age %d: %d" % [age, _count(node, "Crow_")])
	check((_count(node, "Debris_") > 0) == (age < int(block["debris_turns"])), "debris at age %d: %d" % [age, _count(node, "Debris_")])


func _count(node: Node, prefix: String) -> int:
	var count := 0
	for child in node.get_children():
		if str(child.name).begins_with(prefix):
			count += 1
	return count


## Point 4 : engins du camp selon l'avancement du siège.
func _check_siege(block: Dictionary) -> void:
	var almost := int(block["almost_ready_turns"])
	check(WarScars.engine_stages([], almost).is_empty(), "no engine without a siege")
	var building := [{"kind": "ladders", "ready": false, "turns_left": almost + 2}, {"kind": "ram", "ready": false, "turns_left": almost + 6}]
	check(WarScars.engine_stages(building, almost) == [{"kind": "ladders", "stage": "frame"}], "first engine under construction only: %s" % [WarScars.engine_stages(building, almost)])
	var armies := FakeArmies.new()
	root.add_child(armies)
	var marker := FakeMarker.new()
	marker.figures = Node3D.new()
	marker.add_child(marker.figures)
	armies.add_child(marker)
	armies.markers["army_1"] = marker
	var scars := _new_scars(armies)
	var sim := FakeSim.new()
	sim.engines["army_1"] = building
	scars.refresh(sim)
	check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_frame"]), "turn 1: ladders being built: %s" % scars.engine_names("army_1"))
	var engines_root := marker.figures.get_node_or_null(WarScars.ENGINES_NODE) as Node3D
	check(engines_root != null, "engines stand in the besiegers' camp (army figures)")
	# Même tour, même état : pas de relecture du pont à chaque rafraîchissement.
	var calls := sim.odds_calls
	scars.refresh(sim)
	check(sim.odds_calls == calls, "siege engines read once per turn")
	# Les tours passent : échelles prêtes, bélier en charpente, puis presque prêt, puis prêt.
	sim.turn += 1
	sim.engines["army_1"] = [{"kind": "ladders", "ready": true, "turns_left": 0}, {"kind": "ram", "ready": false, "turns_left": almost + 2}, {"kind": "tower", "ready": false, "turns_left": 9}]
	scars.refresh(sim)
	check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_ready", "Engine_ram_frame"]), "turn 2: %s" % scars.engine_names("army_1"))
	sim.turn += 1
	sim.engines["army_1"] = [{"kind": "ladders", "ready": true, "turns_left": 0}, {"kind": "ram", "ready": false, "turns_left": almost}, {"kind": "tower", "ready": false, "turns_left": 7}]
	scars.refresh(sim)
	check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_ready", "Engine_ram_almost"]), "turn 3: %s" % scars.engine_names("army_1"))
	engines_root = marker.figures.get_node_or_null(WarScars.ENGINES_NODE) as Node3D
	var ram := engines_root.get_node_or_null("Engine_ram_almost") as Node3D if engines_root != null else null
	check(ram != null and ram.get_node_or_null("Scaffold") != null and ram.get_node_or_null("Model") != null, "almost ready: model under its scaffold")
	sim.turn += 1
	sim.engines["army_1"] = [{"kind": "ladders", "ready": true, "turns_left": 0}, {"kind": "ram", "ready": true, "turns_left": 0}, {"kind": "tower", "ready": true, "turns_left": 0}]
	scars.refresh(sim)
	check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_ready", "Engine_ram_ready", "Engine_tower_ready"]), "all engines ready: %s" % scars.engine_names("army_1"))
	engines_root = marker.figures.get_node_or_null(WarScars.ENGINES_NODE) as Node3D
	var configs: Dictionary = block["engines"]
	for kind in ["ram", "tower"]:
		var engine := engines_root.get_node_or_null("Engine_%s_ready" % kind) as Node3D
		var model := engine.get_node_or_null("Model") as Node3D if engine != null else null
		if not check(model != null and model.get_node_or_null("../Scaffold") == null, "%s ready: model alone" % kind):
			continue
		if model.has_meta("dims"):  # maquette du kit (repli procédural sinon)
			var dims: Vector3 = model.get_meta("dims")
			check(is_equal_approx(maxf(dims.x, maxf(dims.y, dims.z)), float(configs[kind]["size"])), "%s model sized from data: %s" % [kind, dims])
		var slot: Array = configs[kind]["slot"]
		check(engine.position.is_equal_approx(Vector3(float(slot[0]), 0.0, float(slot[1]))), "%s at its camp slot" % kind)
	for geometry in engines_root.find_children("*", "GeometryInstance3D", true, false):
		check((geometry as GeometryInstance3D).layers == 2, "engine geometry on the army layer")
		break
	scars.update_view(float(block["max_distance"]) * 2.0, 1.0)
	check(not engines_root.visible, "engines hidden beyond max_distance")
	scars.update_view(100.0, 1.0)
	check(engines_root.visible, "engines shown with the camp")
	# Marqueur reconstruit par `ArmyMarkers` (armée changée) : engins reposés sur le nouveau.
	var rebuilt := FakeMarker.new()
	rebuilt.figures = Node3D.new()
	rebuilt.add_child(rebuilt.figures)
	armies.add_child(rebuilt)
	armies.markers["army_1"] = rebuilt
	marker.free()
	scars.refresh(sim)
	check(rebuilt.figures.get_node_or_null(WarScars.ENGINES_NODE) != null and scars.engine_names("army_1").size() == 3, "engines follow a rebuilt marker")
	# Siège levé : plus d'engins.
	rebuilt.status = ""
	scars.refresh(sim)
	check(scars.engine_names("army_1").is_empty() and rebuilt.figures.get_node_or_null(WarScars.ENGINES_NODE) == null, "engines removed when the siege ends")
	scars.free()
	armies.free()


## Retouche du 02/10 : aux distances de jeu du palier proche (15 à 40), fosses et charrette sont
## dans le champ d'une caméra posée sur la colonie et font au moins `MIN_PX` à l'écran (900p,
## champ de 55° de la carte de campagne) ; la charrette est sur la route.
const MIN_PX := 12.0
## LR-14 : la charrette (3 m) est une figurine, plus petite que les fosses : seuil propre.
const CART_MIN_PX := 6.0
const VIEW_HEIGHT := 900.0
const CAMERA_FOV := 55.0


func _check_plague_screen(scars: WarScars, node: Node3D, block: Dictionary) -> void:
	var site_center := Vector3(100.0, 0.0, 200.0)
	var camera := Camera3D.new()
	camera.fov = CAMERA_FOV
	root.add_child(camera)
	for distance in [15.0, 20.0, 40.0]:
		if distance > float(block["max_distance"]):
			continue
		scars.update_view(distance, 1.0)
		# Caméra de la carte : visée sur la colonie, tangage de 30° (`pitch_near_deg`).
		var pitch := deg_to_rad(30.0)
		camera.global_position = site_center + Vector3(0.0, sin(pitch), cos(pitch)) * distance
		camera.look_at(site_center, Vector3.UP)
		var smallest := INF
		for child in node.get_children():
			var prop := child as MeshInstance3D
			if str(prop.name).begins_with("Door_"):
				continue
			# Longueur projetée du grand axe au sol (raccourci compris : les fosses du test sont
			# en enfilade, le pire cas).
			var box := prop.mesh.get_aabb()
			var axis := Vector3.RIGHT if box.size.x >= box.size.z else Vector3.BACK
			var half := maxf(box.size.x, box.size.z) * 0.5
			var px := _screen(camera, prop.global_transform * (box.get_center() - axis * half)).distance_to(_screen(camera, prop.global_transform * (box.get_center() + axis * half)))
			smallest = minf(smallest, px)
			var wanted := CART_MIN_PX if prop.name == &"DeadCart" else MIN_PX
			check(px >= wanted, "%s is %.1f px at distance %.0f (≥ %.0f wanted)" % [prop.name, px, distance, wanted])
			check(camera.is_position_in_frustum(prop.global_position), "%s in view at distance %.0f" % [prop.name, distance])
			check(Vector2(prop.position.x - site_center.x, prop.position.z - site_center.z).length() < distance * 0.45, "%s stays close to the town at distance %.0f" % [prop.name, distance])
		print("tb4_scars_test: plague props at distance %.0f: smallest %.1f px, factor %.0f" % [distance, smallest, WarScars.plague_factor(distance)])
	# Charrette sur la route (tracé qui part vers -Z), pas dans les fosses.
	var cart := node.get_node_or_null("DeadCart") as Node3D
	if cart != null:
		check(absf(cart.position.x - site_center.x) < 0.5 and cart.position.z < site_center.z - 0.5, "dead cart on the road out of town: %s" % cart.position)
	# De très près : échelle réelle (pas de fosse géante dans la ville 1:1).
	scars.update_view(0.2, 1.0)
	check(is_equal_approx((node.get_node("Pit_0") as Node3D).scale.x, 1.0 / 719.0), "pits at real scale up close")
	camera.free()
	scars.update_view(float(block["max_distance"]) * 0.5, 1.0)


## Point monde → pixels d'un écran de `VIEW_HEIGHT` de haut (projection de la caméra, sans
## dépendre de la fenêtre du test).
func _screen(camera: Camera3D, point: Vector3) -> Vector2:
	var local := camera.global_transform.affine_inverse() * point
	return Vector2(local.x, local.y) / maxf(-local.z, 1e-6) / tan(deg_to_rad(CAMERA_FOV * 0.5)) * VIEW_HEIGHT * 0.5


## Croix des portes marquées : au moins 6 px à la distance minimale de la caméra de jeu (dernier
## étage de `CloseCameraProfile.level_min_distance`), à l'échelle réelle.
func _check_door_cross() -> void:
	var profile := CloseCameraProfile.load_default()
	var floor_distance: float = profile.level_min_distance[profile.level_min_distance.size() - 1]
	var px := WarScarMeshes.DOOR_CROSS_M / 719.0 / (2.0 * floor_distance * tan(deg_to_rad(CAMERA_FOV * 0.5))) * VIEW_HEIGHT
	print("tb4_scars_test: door cross %.1f px at the camera floor %.2f (ordinary towns)" % [px, floor_distance])
	check(px >= 6.0, "door cross is %.1f px at the closest camera distance (≥ 6 wanted)" % px)
	var landmark_px := WarScarMeshes.DOOR_CROSS_M / 719.0 / (2.0 * profile.landmark_min_distance * tan(deg_to_rad(CAMERA_FOV * 0.5))) * VIEW_HEIGHT
	check(landmark_px < 6.0, "landmark cities: crosses unreadable (%.1f px), hence no marked door there" % landmark_px)
