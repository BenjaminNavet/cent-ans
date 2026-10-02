extends SceneTree

## Lot TB4 (campagne façon Thrones of Britannia) : conséquences visibles de la guerre et des
## fléaux.
##  1. brûlis posé par-dessus la carte de couleur (SS2) et les matières (HB3), masque de terroir
##     échantillonné à son échelle (carte non carrée) ; les chiffres de rendu viennent de
##     `ss_shot.gd --stats --devastate=` ;
##  2. peste : fosses, portes marquées, charrette des morts selon l'intensité, au palier proche,
##     sans pictogramme ; retirées quand la peste cesse ;
##  3. champ de bataille : tertre, corbeaux, débris à l'endroit de la bataille, corbeaux puis
##     débris partis, puis plus rien après `battlefield.turns` tours (`data/ui/war_scars.json`) ;
##  4. siège : engins du camp selon l'avancement (`get_assault_odds().engines`).
## Usage : godot --headless --path game --script res://tests/tb4_scars_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


## Simulation factice : tour, événements, armées et engins de siège réglés par le test.
class FakeSim:
	extends RefCounted

	var turn := 1
	var events: Array = []
	var pending: Array = []
	var armies: Dictionary = {}
	var engines: Dictionary = {}
	var odds_calls := 0

	func get_turn() -> int:
		return turn

	func get_events() -> Array:
		return events

	func get_pending_events() -> Array:
		return pending

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

var _failures := 0


func _init() -> void:
	await process_frame
	_check_burn()
	var data: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(MAP_PATHS.default_data_dir().path_join(WarScars.DATA_PATH)))
	_check(not WarScars.settings().is_empty(), "war_scars.json read")
	_check_plague(data["plague"])
	_check_battlefield(data["battlefield"])
	WarScarMeshes.clear_cache()
	FolkModels.clear_cache()
	print("TB4 scars test %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb4_scars_test: " + message)
	return condition


## Point 1 : le brûlis vient après `sg_apply` et `hb_apply` (il n'est plus recouvert), et le
## masque de terroir est lu à l'échelle où `TerroirMask` le peint.
func _check_burn() -> void:
	var code := TERRAIN_SHADER.code
	var burn := code.find("col = terroir_burn(")
	_check(burn >= 0, "terrain shader no longer applies terroir_burn")
	_check(code.find("sg_apply(uv") >= 0 and code.find("sg_apply(uv") < burn, "burn is applied before the colour map (sg_apply)")
	_check(code.find("hb_apply(p") >= 0 and code.find("hb_apply(p") < burn, "burn is applied before the HB materials (hb_apply)")
	_check(code.contains("terroir_at(p / max(map_size.x, map_size.y))"), "terroir mask not sampled at the scale it is painted (non-square map)")
	var uniforms := TERRAIN_SHADER.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))
	for uniform_name in ["burnt_color", "ash_color", "terroir_mask"]:
		_check(uniforms.has(uniform_name), "terrain shader lacks uniform %s" % uniform_name)
	# Même convention côté processeur : un point peint se relit au même endroit.
	var mask := TerroirMask.new()
	var map_size := Vector2(7168.0, 6144.0)
	var at := Vector2(2213.2, 3203.9)
	mask.build([{"kind": "city", "province": "prov_a", "px": at}], [], {"prov_a": {"devastation": 100.0, "population": TerroirMask.REFERENCE_POPULATION}}, null, map_size)
	_check(mask.sample(at).b > 0.9, "burn painted at the settlement: %s" % mask.sample(at))
	var texel := at / maxf(map_size.x, map_size.y) * float(TerroirMask.SIZE)
	_check(mask.image.get_pixel(int(texel.x), int(texel.y)).b > 0.9, "shader lookup (p / max side) hits the burnt texel")
	var stretched := at / map_size * float(TerroirMask.SIZE)
	_check(mask.image.get_pixel(int(stretched.x), int(stretched.y)).b < 0.05, "old lookup (uv) missed the burn: the fix matters")


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
	var site := {"settlement": "set_test", "center": anchor, "out": Vector2.RIGHT, "footprint": 0.5, "intensity": 1.0, "seed": 42}
	scars.set_plague_sites([site])
	var node := scars.plague_node("set_test")
	if not _check(node != null, "plague site built"):
		return
	var pits: Array = block["pits"]
	var doors: Array = block["doors"]
	_check(scars.plague_count("set_test", "Pit_") == int(pits[1]), "pits at full intensity: %d" % scars.plague_count("set_test", "Pit_"))
	_check(scars.plague_count("set_test", "DeadCart") == (1 if block["cart"] else 0), "dead cart by the pits")
	_check(not node.visible, "plague hidden before the first view")
	# Palier proche : visible ; les portes attendent le plan de la ville 1:1.
	scars.update_view(float(block["max_distance"]) * 0.5, 1.0)
	_check(node.visible, "plague shown near")
	_check(scars.plague_count("set_test", "Door_") == 0, "no door before the town plan is loaded")
	plan_ready[0] = true
	scars._door_timer = 0.0
	scars.update_view(float(block["max_distance"]) * 0.5, 1.0)
	_check(scars.plague_count("set_test", "Door_") == int(doors[1]), "marked doors at full intensity: %d" % scars.plague_count("set_test", "Door_"))
	# Chaque porte tient à une façade : à une demi-profondeur de maison (+ 0,15 m) de son axe.
	for child in node.get_children():
		if not str(child.name).begins_with("Door_"):
			continue
		var local := (Vector2((child as Node3D).position.x, (child as Node3D).position.z) - anchor) * 719.0
		_check(absf(absf(local.y) - 25.85) < 0.2 and local.length() <= float(block["door_search_m"]) + 5.0, "door on a street front within the search radius: %s" % local)
	# Fosses hors de la colonie, du côté de la scène.
	var pit := node.get_node("Pit_0") as Node3D
	_check(pit.position.x > anchor.x + 0.5, "pits beyond the settlement edge")
	_check(_only_meshes(node), "plague shown by 3D props only (no interface sign)")
	_check(int(scars.stats.get("doors", 0)) == int(doors[1]) and int(scars.stats.get("pits", 0)) == int(pits[1]), "stats: %s" % scars.stats)
	# Vue moyenne et parchemin : rien.
	scars.update_view(float(block["max_distance"]) * 2.0, 1.0)
	_check(not node.visible, "plague hidden beyond max_distance")
	scars.update_view(float(block["max_distance"]) * 0.5, 0.0)
	_check(not node.visible, "plague hidden on the parchment view")
	# Peste faible : moins de fosses ; peste finie : plus rien.
	site["intensity"] = 0.0
	scars.set_plague_sites([site])
	_check(scars.plague_count("set_test", "Pit_") == int(pits[0]), "pits at low intensity")
	scars.set_plague_sites([])
	_check(scars.plague_node("set_test") == null and int(scars.stats.get("plague_sites", -1)) == 0, "plague props removed when the plague ends")
	scars.free()


## Point 3 : bataille → champ marqué à l'endroit de l'armée, qui s'efface par étapes puis
## disparaît après `turns` tours.
func _check_battlefield(block: Dictionary) -> void:
	var armies := FakeArmies.new()
	root.add_child(armies)
	var scars := _new_scars(armies)
	var sim := FakeSim.new()
	var turns := int(block["turns"])
	_check(WarScars.battlefield_turns() == turns and turns >= 2, "battlefield duration read from data: %d" % WarScars.battlefield_turns())
	armies.positions["army_1"] = Vector3(50.0, 3.0, 60.0)
	sim.events = [
		{"kind": "income", "province": "prov_a", "army": "", "text_fr": "Recettes."},
		{"kind": "battle", "province": "prov_a", "army": "army_1", "text_fr": "Bataille de A."},
		{"kind": "battle", "province": "prov_a", "army": "army_1", "text_fr": "L'ost gagne en renom."},
	]
	scars.refresh(sim)
	_check(scars.battlefield_keys() == ["prov_a"], "one marked field per battle province: %s" % [scars.battlefield_keys()])
	var at := scars.battlefield_at("prov_a")
	_check(at.distance_to(Vector2(50.0, 60.0)) <= WarScars.FIELD_OFFSET + 0.01, "field at the place of the battle: %s" % at)
	var node := scars.battlefield_node("prov_a")
	if not _check(node != null, "battlefield node"):
		return
	_check(node.get_node_or_null("Mound") != null, "mound")
	_check(_count(node, "Crow_") == int(block["crows"]), "crows: %d" % _count(node, "Crow_"))
	_check(_count(node, "Debris_") == int(block["debris"]), "debris: %d" % _count(node, "Debris_"))
	_check(_only_meshes(node), "battlefield shown by 3D props only")
	# Vue : taille des armées, caché sous le brouillard de guerre et sur le parchemin.
	scars.update_view(100.0, 1.0)
	_check(node.visible, "battlefield shown in the normal view")
	var mound := node.get_node("Mound") as Node3D
	_check(is_equal_approx(mound.scale.x, ArmyMarkers.scale_for_distance(100.0)), "battlefield follows the army scale: %s" % mound.scale)
	_check(Vector2(mound.position.x, mound.position.z).distance_to(at) < 0.001, "mound at the field")
	var crow := node.get_node("Crow_0") as Node3D
	var crow_before := crow.position
	scars._time += 1.0
	scars.update_view(100.0, 1.0)
	_check(crow.position.distance_to(crow_before) > 1e-4 and crow.position.y > mound.position.y, "crows circle above the mound")
	armies.hidden_provinces["prov_a"] = true
	scars.update_view(100.0, 1.0)
	_check(not node.visible, "battlefield hidden under the fog of war")
	armies.hidden_provinces.clear()
	scars.update_view(100.0, 0.0)
	_check(not node.visible, "battlefield hidden on the parchment view")
	scars.update_view(float(block["max_distance"]) * 1.5, 1.0)
	_check(not node.visible, "battlefield hidden beyond max_distance")
	# Les tours passent sans nouvelle bataille : corbeaux, débris, puis tertre s'en vont.
	sim.events = []
	for age in range(1, turns + 1):
		sim.turn += 1
		scars.refresh(sim)
		node = scars.battlefield_node("prov_a")
		if age >= turns:
			_check(node == null and scars.battlefield_keys().is_empty(), "field gone after %d turns" % turns)
			break
		if not _check(node != null and node.get_node_or_null("Mound") != null, "mound still there at age %d" % age):
			break
		_check((_count(node, "Crow_") > 0) == (age < int(block["crow_turns"])), "crows at age %d: %d" % [age, _count(node, "Crow_")])
		_check((_count(node, "Debris_") > 0) == (age < int(block["debris_turns"])), "debris at age %d: %d" % [age, _count(node, "Debris_")])
	# Bataille de la main du joueur (événement en attente, sans armée) : marquée tout de suite à
	# la position lue du pont ; reprise au journal du tour suivant, elle ne repart pas de zéro.
	sim.armies["army_2"] = {"position": Vector2(10.0, 20.0)}
	sim.pending = [{"kind": "battle", "province": "", "army": "army_2", "text_fr": "Escarmouche."}]
	scars.refresh(sim)
	_check(scars.battlefield_keys() == ["army:army_2"], "pending battle marked at once: %s" % [scars.battlefield_keys()])
	_check(scars.battlefield_at("army:army_2").distance_to(Vector2(10.0, 20.0)) <= WarScars.FIELD_OFFSET + 0.01, "pending battle at the army position")
	var marked_turn := sim.turn
	sim.turn += 1
	sim.events = sim.pending
	sim.pending = []
	scars.refresh(sim)
	_check(int(scars._fields["army:army_2"]["turn"]) == marked_turn, "journal copy of a known battle does not restart the mark")
	# Sans position connue : rien.
	sim.events = [{"kind": "battle", "province": "prov_unknown", "army": "", "text_fr": "?"}]
	sim.turn += 1
	scars.refresh(sim)
	_check(not scars.battlefield_keys().has("prov_unknown"), "no mark without a known place")
	# Partie chargée : oubli.
	scars.reset()
	_check(scars.battlefield_keys().is_empty(), "reset forgets the marked fields")
	scars.free()
	armies.free()


func _count(node: Node, prefix: String) -> int:
	var count := 0
	for child in node.get_children():
		if str(child.name).begins_with(prefix):
			count += 1
	return count


## Point 4 : engins du camp selon l'avancement du siège.
func _check_siege(block: Dictionary) -> void:
	var almost := int(block["almost_ready_turns"])
	_check(WarScars.engine_stages([], almost).is_empty(), "no engine without a siege")
	var building := [{"kind": "ladders", "ready": false, "turns_left": almost + 2}, {"kind": "ram", "ready": false, "turns_left": almost + 6}]
	_check(WarScars.engine_stages(building, almost) == [{"kind": "ladders", "stage": "frame"}], "first engine under construction only: %s" % [WarScars.engine_stages(building, almost)])
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
	_check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_frame"]), "turn 1: ladders being built: %s" % scars.engine_names("army_1"))
	var engines_root := marker.figures.get_node_or_null(WarScars.ENGINES_NODE) as Node3D
	_check(engines_root != null, "engines stand in the besiegers' camp (army figures)")
	# Même tour, même état : pas de relecture du pont à chaque rafraîchissement.
	var calls := sim.odds_calls
	scars.refresh(sim)
	_check(sim.odds_calls == calls, "siege engines read once per turn")
	# Les tours passent : échelles prêtes, bélier en charpente, puis presque prêt, puis prêt.
	sim.turn += 1
	sim.engines["army_1"] = [{"kind": "ladders", "ready": true, "turns_left": 0}, {"kind": "ram", "ready": false, "turns_left": almost + 2}, {"kind": "tower", "ready": false, "turns_left": 9}]
	scars.refresh(sim)
	_check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_ready", "Engine_ram_frame"]), "turn 2: %s" % scars.engine_names("army_1"))
	sim.turn += 1
	sim.engines["army_1"] = [{"kind": "ladders", "ready": true, "turns_left": 0}, {"kind": "ram", "ready": false, "turns_left": almost}, {"kind": "tower", "ready": false, "turns_left": 7}]
	scars.refresh(sim)
	_check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_ready", "Engine_ram_almost"]), "turn 3: %s" % scars.engine_names("army_1"))
	engines_root = marker.figures.get_node_or_null(WarScars.ENGINES_NODE) as Node3D
	var ram := engines_root.get_node_or_null("Engine_ram_almost") as Node3D if engines_root != null else null
	_check(ram != null and ram.get_node_or_null("Scaffold") != null and ram.get_node_or_null("Model") != null, "almost ready: model under its scaffold")
	sim.turn += 1
	sim.engines["army_1"] = [{"kind": "ladders", "ready": true, "turns_left": 0}, {"kind": "ram", "ready": true, "turns_left": 0}, {"kind": "tower", "ready": true, "turns_left": 0}]
	scars.refresh(sim)
	_check(scars.engine_names("army_1") == PackedStringArray(["Engine_ladders_ready", "Engine_ram_ready", "Engine_tower_ready"]), "all engines ready: %s" % scars.engine_names("army_1"))
	engines_root = marker.figures.get_node_or_null(WarScars.ENGINES_NODE) as Node3D
	var configs: Dictionary = block["engines"]
	for kind in ["ram", "tower"]:
		var engine := engines_root.get_node_or_null("Engine_%s_ready" % kind) as Node3D
		var model := engine.get_node_or_null("Model") as Node3D if engine != null else null
		if not _check(model != null and model.get_node_or_null("../Scaffold") == null, "%s ready: model alone" % kind):
			continue
		if model.has_meta("dims"):  # maquette du kit (repli procédural sinon)
			var dims: Vector3 = model.get_meta("dims")
			_check(is_equal_approx(maxf(dims.x, maxf(dims.y, dims.z)), float(configs[kind]["size"])), "%s model sized from data: %s" % [kind, dims])
		var slot: Array = configs[kind]["slot"]
		_check(engine.position.is_equal_approx(Vector3(float(slot[0]), 0.0, float(slot[1]))), "%s at its camp slot" % kind)
	for geometry in engines_root.find_children("*", "GeometryInstance3D", true, false):
		_check((geometry as GeometryInstance3D).layers == 2, "engine geometry on the army layer")
		break
	scars.update_view(float(block["max_distance"]) * 2.0, 1.0)
	_check(not engines_root.visible, "engines hidden beyond max_distance")
	scars.update_view(100.0, 1.0)
	_check(engines_root.visible, "engines shown with the camp")
	# Marqueur reconstruit par `ArmyMarkers` (armée changée) : engins reposés sur le nouveau.
	var rebuilt := FakeMarker.new()
	rebuilt.figures = Node3D.new()
	rebuilt.add_child(rebuilt.figures)
	armies.add_child(rebuilt)
	armies.markers["army_1"] = rebuilt
	marker.free()
	scars.refresh(sim)
	_check(rebuilt.figures.get_node_or_null(WarScars.ENGINES_NODE) != null and scars.engine_names("army_1").size() == 3, "engines follow a rebuilt marker")
	# Siège levé : plus d'engins.
	rebuilt.status = ""
	scars.refresh(sim)
	_check(scars.engine_names("army_1").is_empty() and rebuilt.figures.get_node_or_null(WarScars.ENGINES_NODE) == null, "engines removed when the siege ends")
	scars.free()
	armies.free()
