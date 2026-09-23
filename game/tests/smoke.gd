extends SceneTree

## Smoke test headless :
##  1. GDExtension : CampaignSim joue 10 tours, vérifie la date ;
##  2. scène de carte sur les fixtures synthétiques (`CENT_ANS_DATA_DIR` → tests/fixtures) :
##     tuiles de terrain, rivières, marqueurs, picking de la province 3 ;
##  3. écran de démarrage : 3 cartes de faction ;
##  4. boucle de campagne (France, via SimFacade, réelle ou factice — imprimé) : sélection de la
##     première armée du joueur, provinces atteignables non vides, ordre de déplacement vers la
##     première atteignable, 4 fins de tour, sauvegarde `user://saves/smoke.json`, rechargement,
##     égalité des dates. Sur les vraies données `data/` si la sim réelle est disponible.
##  5. personnages et dynasties (M4, réelle si elle expose `get_character`, sinon mock dédié sur
##     `data/` — imprimé) : panneau de cour (≥ 1 personnage), `debug_grant_xp` + `learn_skill`
##     sur le dirigeant, `assign_governor` d'un courtisan à `prov_normandie` (ou la première
##     province contrôlée ≠ capitale), `propose_marriage` entre deux candidats valides (ou skip
##     explicite si aucun), 40 fins de tour → au moins une naissance ou une mort au journal.
##  6. technologies (M6, vraie simulation si elle expose `get_tech_tree`, sinon skip imprimé) :
##     panneau des technologies (un nœud par technologie), ordre `research` sur la technologie
##     disponible la moins chère, `get_research` non vide, refus d'une technologie connue,
##     20 fins de tour → au moins une technologie acquise et un événement `technology_researched`.
##  M10 assets : autoload AudioDirector (effets, musiques, persistance des volumes dans
##     `user://settings.cfg`), écus et portraits (`PortraitLoader`), modèles 3D (`ModelLibrary`,
##     marqueur d'armée habillé) et replis quand un fichier manque.
## Usage : godot --headless --path game --script res://tests/smoke.gd
## Code de sortie 0 si tout passe, 1 sinon.

const EXPECTED_LABEL := "Automne 1339"
const EXPECTED_TURN := 10
const PICK_PROVINCE_INDEX := 3
const SMOKE_SAVE := "smoke"

var _failures: int = 0
var _fixtures_dir: String
## Autoloads obtenus dynamiquement : un script `--script` est compilé avant l'enregistrement des
## singletons, donc `SimFacade.x` / `MapPaths.x` (membres d'instance) y sont interdits.
var facade: Node
var paths: Node


func _init() -> void:
	# Doit précéder l'instanciation de l'autoload MapPaths (après _init).
	_fixtures_dir = ProjectSettings.globalize_path("res://tests/fixtures")
	OS.set_environment(MapPaths.ENV_VAR, _fixtures_dir)
	_run_campaign_sim()
	await process_frame
	facade = root.get_node("/root/SimFacade")
	paths = root.get_node("/root/MapPaths")
	await _run_campaign_map()
	await _run_start_menu()
	await _run_campaign_loop()
	_run_city_economy()
	await _run_characters()
	await _run_technologies()
	await _run_diplomacy()
	await _run_assets()  # M10 assets
	quit(1 if _failures > 0 else 0)


func _run_campaign_sim() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		_fail("CampaignSim class not registered: GDExtension not loaded (run core/build.sh)")
		return

	var campaign: Object = ClassDB.instantiate("CampaignSim")
	_check(campaign.get_turn() == -1, "turn before new_campaign should be -1, got %d" % campaign.get_turn())

	if campaign.has_method("get_army_ids"):
		# API M2 : new_campaign(data_dir, player, seed) sur les vraies données.
		var ok: bool = campaign.new_campaign(_project_root().path_join("data"), "fac_france", 1337)
		if not _check(ok, "CampaignSim.new_campaign(data, fac_france, 1337) failed"):
			return
	else:
		campaign.new_campaign(1337)
	_check(campaign.get_turn() == 0, "initial turn should be 0, got %d" % campaign.get_turn())
	_check(campaign.get_date_label() == "Printemps 1337", "initial date should be 'Printemps 1337', got '%s'" % campaign.get_date_label())

	for _i in range(EXPECTED_TURN):
		campaign.end_turn()

	var turn: int = campaign.get_turn()
	var label: String = campaign.get_date_label()
	_check(turn == EXPECTED_TURN, "expected turn %d, got %d" % [EXPECTED_TURN, turn])
	_check(label == EXPECTED_LABEL, "expected '%s', got '%s'" % [EXPECTED_LABEL, label])

	if _failures == 0:
		print("smoke OK: turn %d, %s" % [turn, label])


func _run_campaign_map() -> void:
	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	if scene == null:
		_fail("cannot load campaign_map.tscn")
		return
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame

	if not _check(map.load_ok, "campaign map failed to load: %s" % (map.map_data.load_error if map.map_data else "no data")):
		return
	var data: MapData = map.map_data
	_check(data.height_bpp == 2, "heightmap should be decoded as 16-bit, got bpp %d" % data.height_bpp)
	print("smoke map: heightmap decoder = %s" % data.height_decoder)
	_check(data.size == Vector2i(512, 512), "fixture size should be 512x512, got %s" % data.size)
	_check(data.province_count == 6, "expected 6 provinces, got %d" % data.province_count)
	_check(data.index_of_id("prov_synth_3") == 3, "index_of_id(prov_synth_3) should be 3")
	var terrain: TerrainBuilder = map.terrain
	_check(terrain.chunk_count() == TerrainBuilder.CHUNKS * TerrainBuilder.CHUNKS,
		"expected %d terrain chunks, got %d" % [TerrainBuilder.CHUNKS * TerrainBuilder.CHUNKS, terrain.chunk_count()])
	for chunk in terrain.get_children():
		if chunk is MeshInstance3D and (chunk.mesh == null or chunk.mesh.get_surface_count() == 0):
			_fail("terrain chunk %s has no mesh surface" % chunk.name)
			break
	_check(data.rivers.size() == 2, "expected 2 rivers, got %d" % data.rivers.size())
	_check(not data.coastlines.is_empty(), "coastline missing")
	_check(map.cities.get_child_count() == 6, "expected 6 city markers, got %d" % map.cities.get_child_count())

	# Picking : centroïde de la province 3, en coordonnées monde puis via l'écran.
	var province: Dictionary = data.get_province(PICK_PROVINCE_INDEX)
	if not _check(not province.is_empty(), "province %d missing from provinces.geojson" % PICK_PROVINCE_INDEX):
		return
	var centroid: Vector2 = province["centroid"]
	var picker: ProvincePicker = map.picker
	var direct := picker.province_at_world(centroid.x, centroid.y)
	_check(direct == PICK_PROVINCE_INDEX, "province_at_world at centroid returned %d, expected %d" % [direct, PICK_PROVINCE_INDEX])

	var world := Vector3(centroid.x, data.surface_world_at(centroid.x, centroid.y), centroid.y)
	var rig: CampaignCamera = map.camera_rig
	rig.look_at_point(world, 120.0)
	rig.snap()
	await process_frame
	var camera: Camera3D = map.camera
	var screen := camera.unproject_position(world)
	var picked := picker.pick_screen(screen)
	_check(picked == PICK_PROVINCE_INDEX, "pick_screen at province %d centroid (%s) returned %d" % [PICK_PROVINCE_INDEX, screen, picked])
	var hit := picker.pick_ray_screen(screen)
	if _check(not hit.is_empty(), "pick_ray_screen returned no hit"):
		var error := Vector2(hit["x"], hit["z"]).distance_to(centroid)
		_check(error < 1.5, "ray refinement error %.2f px too large" % error)

	# Sélection de province → panneau visible avec le nom.
	picker.select_index(PICK_PROVINCE_INDEX)
	await process_frame
	_check(map.ui.province_panel.visible, "province panel should be visible after selection")
	_check(map.ui.province_panel.name_label.text == province["name"], "province panel name mismatch: '%s'" % map.ui.province_panel.name_label.text)

	print("smoke map: %s" % JSON.stringify(map.startup_stats))
	if _failures == 0:
		print("smoke OK: map loaded, %d chunks, picked province %d" % [terrain.chunk_count(), picked])
	map.queue_free()
	await process_frame


func _run_start_menu() -> void:
	var scene: PackedScene = load("res://scenes/start_menu.tscn")
	if not _check(scene != null, "cannot load start_menu.tscn"):
		return
	var menu: Control = scene.instantiate()
	root.add_child(menu)
	await process_frame
	_check(menu.card_count() == 3, "start menu should show 3 faction cards, got %d" % menu.card_count())
	_check(menu.selected_faction == "fac_france", "default faction should be fac_france")
	_check(menu.start_button.text.begins_with("Commencer"), "start button label")
	if _failures == 0:
		print("smoke OK: start menu, %d cards" % menu.card_count())
	menu.queue_free()
	await process_frame


func _run_campaign_loop() -> void:
	# Vraies données si la simulation réelle existe (elle a besoin de data/factions…), sinon fixtures.
	var real_data := _project_root().path_join("data")
	var use_real: bool = ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_army_ids") \
		and FileAccess.file_exists(real_data.path_join("map/map.json"))
	facade.set_data_dir(real_data if use_real else _fixtures_dir)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""

	var scene: PackedScene = load("res://scenes/campaign_map.tscn")
	var map: Node3D = scene.instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign scene with SimFacade failed to start"):
		return
	print("smoke campaign: simulation %s, data %s, store %s" % ["REAL" if facade.is_real else "MOCK", paths.data_dir, "loaded" if facade.store_loaded() else "absent"])
	_check(map.player_faction == "fac_france", "player faction should be fac_france, got %s" % map.player_faction)
	_check(map.armies.army_count() > 0, "no army markers on the map")
	_check(map.ui.faction_label.text != "" and map.ui.faction_label.text != "Cent Ans", "top bar faction label not set")

	var army_ids: PackedStringArray = map.player_army_ids()
	if not _check(not army_ids.is_empty(), "player has no army"):
		map.queue_free()
		return
	map.select_army(army_ids[0])
	await process_frame
	_check(map.selected_army == army_ids[0], "army selection failed")
	_check(map.ui.army_panel.visible, "army panel should be visible")
	var reachable: Dictionary = map.reachable
	if not _check(not reachable.is_empty(), "reachable provinces should not be empty for %s" % army_ids[0]):
		map.queue_free()
		return
	var target: String = str(reachable.keys()[0])
	# Aperçu de chemin au survol de la cible.
	var target_index: int = map.map_data.index_of_id(target)
	map._on_province_hovered(target_index)
	_check(map.path_preview.visible, "path preview should be visible when hovering a reachable province")
	var result: Dictionary = map.order_move(army_ids[0], target)
	_check(result.get("ok", false), "move order refused: %s" % result.get("error", "?"))
	var army: Dictionary = map.sim.call("get_army", army_ids[0])
	_check(not army.get("path", []).is_empty(), "army path should be set after the order")

	var date_before: String = map.sim.call("get_date_label")
	for _i in 4:
		map._on_end_turn()
	await process_frame
	var turn: int = map.sim.call("get_turn")
	_check(turn == 4, "expected turn 4 after 4 end turns, got %d" % turn)
	_check(map.sim.call("get_date_label") != date_before, "date should advance after end turns")
	_check(map.ui.log_line_count() > 0, "event log should have entries")

	# Sauvegarde puis rechargement : dates identiques.
	var date_saved: String = map.sim.call("get_date_label")
	_check(facade.save_game(SMOKE_SAVE), "save_game failed")
	_check(FileAccess.file_exists(facade.save_path(SMOKE_SAVE)), "save file missing")
	map._on_end_turn()
	_check(map.sim.call("get_date_label") != date_saved, "date should differ before reload")
	map._on_load(facade.save_path(SMOKE_SAVE))
	await process_frame
	var date_loaded: String = map.sim.call("get_date_label")
	_check(date_loaded == date_saved, "date after load '%s' != saved '%s'" % [date_loaded, date_saved])
	_check(map.ui.date_label.text.begins_with(date_saved), "HUD date label '%s' should start with '%s'" % [map.ui.date_label.text, date_saved])
	var saves: Array = facade.list_saves()
	var found := false
	for save in saves:
		if save["name"] == SMOKE_SAVE:
			found = true
	_check(found, "smoke save not listed by list_saves")

	if _failures == 0:
		print("smoke OK: campaign loop (%s), %d turns, saved and reloaded at %s" % ["real" if facade.is_real else "mock", turn, date_loaded])
	map.queue_free()
	await process_frame


## M3 : construit le bâtiment le moins cher disponible à Paris (marché si absent), passe les
## tours nécessaires, vérifie qu'il apparaît dans `buildings` et que `projected_income` a
## augmenté ; passe l'impôt à Haut et vérifie une nouvelle hausse. Avec la vraie simulation si
## elle expose `get_province_city`/`get_faction_economy` (imprimé), sinon avec le mock sur les
## vraies données (`data/`, qui a les bâtiments/ressources ; pas les fixtures).
func _run_city_economy() -> void:
	const PROVINCE_ID := "prov_ile_de_france"
	const FACTION_ID := "fac_france"
	var data_dir := _project_root().path_join("data")
	var real_capable: bool = ClassDB.class_exists("CampaignSim") \
		and ClassDB.instantiate("CampaignSim").has_method("get_province_city")
	var sim: Object
	if real_capable:
		sim = ClassDB.instantiate("CampaignSim")
	else:
		sim = CampaignSimMock.new()
	if not _check(sim.call("new_campaign", data_dir, FACTION_ID, 1337), "city/economy: new_campaign failed"):
		return
	print("smoke city/economy: simulation %s" % ["REAL" if real_capable else "MOCK"])

	var city: Dictionary = sim.call("get_province_city", PROVINCE_ID)
	if not _check(not city.is_empty(), "get_province_city(%s) should not be empty" % PROVINCE_ID):
		return
	_check(city.has("classes") and (city["classes"] as Dictionary).size() == 4, "get_province_city should expose 4 population classes")

	# Quelques tours d'abord : la population/richesse initiale converge vers son équilibre les
	# premiers tours (les deux moteurs), un bruit qui masquerait l'effet du bâtiment construit
	# juste après le départ.
	for _i in 20:
		sim.call("end_turn")

	var buildable: Array = city.get("buildable", [])
	# Préfère un bâtiment à effet économique direct (trade_income/tax_income/wealth) : plus
	# fiable à mesurer que le moins cher (ex. une caserne n'a aucun effet sur le revenu).
	var choice: Dictionary = _pick_economic_building(buildable)
	if not _check(not choice.is_empty(), "no buildable building available in %s" % PROVINCE_ID):
		return
	print("smoke city/economy: building %s (%d livres, %d tour(s))" % [choice["building"], choice["cost"], choice["turns"]])

	var economy_before: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(not economy_before.is_empty(), "get_faction_economy should not be empty")

	var build_result: Dictionary = sim.call("submit_order", {"type": "build", "province": PROVINCE_ID, "building": choice["building"]})
	_check(build_result.get("ok", false), "build order refused: %s" % build_result.get("error", "?"))

	for _i in int(choice["turns"]):
		sim.call("end_turn")

	var city_after: Dictionary = sim.call("get_province_city", PROVINCE_ID)
	var built_ids: Array = []
	for entry in city_after.get("buildings", []):
		built_ids.append(str(entry.get("id", "")))
	_check(built_ids.has(str(choice["building"])), "%s should be in buildings after %d turn(s), got %s" % [choice["building"], choice["turns"], built_ids])

	# La richesse converge sur plusieurs saisons (§ 1.1) : on laisse une fenêtre de tours après
	# l'achèvement et on retient le meilleur revenu prévisionnel observé plutôt qu'un seul point,
	# pour ne pas dépendre de la vitesse exacte de convergence des deux moteurs.
	var best_after := int(economy_before.get("projected_income", 0))
	for _i in 6:
		sim.call("end_turn")
		var economy: Dictionary = sim.call("get_faction_economy", FACTION_ID)
		best_after = maxi(best_after, int(economy.get("projected_income", 0)))
	_check(best_after > int(economy_before.get("projected_income", 0)),
		"projected_income should increase after construction: %d -> best %d" % [economy_before.get("projected_income", 0), best_after])

	# Impôt : comparaison immédiate (même tour, sans fin de tour entre les deux) pour isoler
	# l'effet du multiplicateur fiscal de la dérive de fond de l'économie.
	var economy_normal: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	var tax_result: Dictionary = sim.call("submit_order", {"type": "set_tax_rate", "rate": "high"})
	_check(tax_result.get("ok", false), "set_tax_rate high refused: %s" % tax_result.get("error", "?"))
	var economy_high_tax: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(int(economy_high_tax.get("projected_income", 0)) > int(economy_normal.get("projected_income", 0)),
		"projected_income should rise with high tax: %d -> %d" % [economy_normal.get("projected_income", 0), economy_high_tax.get("projected_income", 0)])

	if _failures == 0:
		print("smoke OK: city/economy (%s), built %s, projected income %d -> best %d, tax normal->high %d -> %d" % [
			"real" if real_capable else "mock", choice["building"],
			economy_before.get("projected_income", 0), best_after,
			economy_normal.get("projected_income", 0), economy_high_tax.get("projected_income", 0)])


## Bâtiment constructible avec un effet direct sur le revenu (trade_income/tax_income/wealth),
## le moins cher parmi ceux-ci ; à défaut, le constructible disponible le moins cher.
func _pick_economic_building(buildable: Array) -> Dictionary:
	var data_dir := _project_root().path_join("data")
	var best_economic: Dictionary = {}
	var best_any: Dictionary = {}
	for row in buildable:
		if not bool(row.get("available", false)):
			continue
		if best_any.is_empty() or int(row["cost"]) < int(best_any["cost"]):
			best_any = row
		if _has_economic_effect(data_dir, str(row.get("building", ""))):
			if best_economic.is_empty() or int(row["cost"]) < int(best_economic["cost"]):
				best_economic = row
	return best_economic if not best_economic.is_empty() else best_any


func _has_economic_effect(data_dir: String, building_id: String) -> bool:
	var path := data_dir.path_join("buildings").path_join(building_id + ".json")
	if not FileAccess.file_exists(path):
		return false
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		return false
	for effect in (parsed as Dictionary).get("effects", []):
		if ["trade_income", "tax_income", "wealth"].has(str(effect.get("effect", ""))):
			return true
	return false


static func _project_root() -> String:
	return ProjectSettings.globalize_path("res://").path_join("..").simplify_path()


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_fail(message)
	return condition


func _fail(message: String) -> void:
	_failures += 1
	push_error("smoke FAIL: " + message)
	printerr("smoke FAIL: " + message)


## M4 : personnages et dynasties (docs/design/m4-characters-dynasties.md § 5). Avec la vraie
## simulation si elle expose `get_character` (imprimé), sinon un `CampaignSimMock` dédié sur
## `data/` (qui a les personnages ; pas les fixtures).
func _run_characters() -> void:
	const FACTION_ID := "fac_france"
	var data_dir := _project_root().path_join("data")
	var real_capable: bool = ClassDB.class_exists("CampaignSim") \
		and ClassDB.instantiate("CampaignSim").has_method("get_character")
	var sim: Object = ClassDB.instantiate("CampaignSim") if real_capable else CampaignSimMock.new()
	if not _check(sim.call("new_campaign", data_dir, FACTION_ID, 1337), "characters: new_campaign failed"):
		return
	print("smoke characters: simulation %s" % ["REAL" if real_capable else "MOCK"])

	var ids: Array = sim.call("get_faction_characters", FACTION_ID)
	if not _check(not ids.is_empty(), "get_faction_characters(fac_france) should not be empty"):
		return

	# Panneau de cour : au moins 1 personnage listé, via la scène réelle.
	var court_scene: PackedScene = load("res://scenes/ui/court_panel.tscn")
	# Type dynamique (Node) : les classes globales fraîchement ajoutées (CourtPanel) ne sont pas
	# toujours résolues pour un typage statique au moment où `--script` compile ce fichier.
	var court: Node = court_scene.instantiate()
	root.add_child(court)
	await process_frame
	var rows: Array[Dictionary] = []
	for id in ids:
		rows.append(sim.call("get_character", id))
	court.show_court(rows, "France", Color(0.2, 0.3, 0.7))
	_check(court.rows_list.get_child_count() >= 1, "court panel should list at least 1 character")
	court.queue_free()

	# XP + apprentissage d'une compétence de commandement de rang 1 sur le dirigeant.
	var ruler: String = str(ids[0])
	var grant: Dictionary = sim.call("submit_order", {"type": "debug_grant_xp", "character": ruler, "amount": 400})
	_check(grant.get("ok", false), "debug_grant_xp refused: %s" % grant.get("error", "?"))
	var tier1_command := ""
	for node in sim.call("get_skill_tree"):
		if str(node["branch"]) == "command" and int(node["tier"]) == 1:
			tier1_command = str(node["id"])
			break
	if _check(tier1_command != "", "no tier-1 command skill in get_skill_tree()"):
		var learn: Dictionary = sim.call("submit_order", {"type": "learn_skill", "character": ruler, "skill": tier1_command})
		_check(learn.get("ok", false), "learn_skill(%s, %s) refused: %s" % [ruler, tier1_command, learn.get("error", "?")])

	# Gouverneur : un courtisan (pas le dirigeant) nommé à Normandie, sinon la première province
	# contrôlée différente de la capitale.
	var target_province := ""
	for candidate in ["prov_normandie", "prov_picardie", "prov_champagne", "prov_orleanais", "prov_anjou"]:
		var state: Dictionary = sim.call("get_province_state", candidate)
		if str(state.get("owner", "")) == FACTION_ID and candidate != "prov_ile_de_france":
			target_province = candidate
			break
	var courtier := ""
	for id in ids:
		if str(id) != ruler:
			courtier = str(id)
			break
	if _check(target_province != "" and courtier != "", "no province/courtier available for assign_governor"):
		var gov: Dictionary = sim.call("submit_order", {"type": "assign_governor", "character": courtier, "province": target_province})
		_check(gov.get("ok", false), "assign_governor(%s, %s) refused: %s" % [courtier, target_province, gov.get("error", "?")])
		var after: Dictionary = sim.call("get_province_state", target_province)
		_check(str(after.get("governor", "")) == courtier, "province %s should report %s as governor" % [target_province, courtier])

	# Mariage entre deux candidats valides, sinon skip explicite (raison imprimée).
	var married := false
	for id in ids:
		var candidates: Array = sim.call("get_marriage_candidates", id)
		if not candidates.is_empty():
			var spouse: String = str(candidates[0]["id"])
			var marriage: Dictionary = sim.call("submit_order", {"type": "propose_marriage", "character": id, "spouse": spouse})
			_check(marriage.get("ok", false), "propose_marriage(%s, %s) refused: %s" % [id, spouse, marriage.get("error", "?")])
			married = true
			break
	if not married:
		print("smoke characters: propose_marriage skipped, no valid candidates in fac_france at 1337")

	# 40 fins de tour : au moins une naissance ou une mort dans le journal.
	var found_birth_or_death := false
	for _i in 40:
		for event in sim.call("end_turn"):
			var kind: String = str(event.get("kind", ""))
			if kind == "birth" or kind == "death":
				found_birth_or_death = true
	_check(found_birth_or_death, "no birth or death event in 40 end_turn() calls")

	if _failures == 0:
		print("smoke OK: characters (%s), court %d, learn_skill %s, governor %s->%s, marriage %s, birth/death seen" % [
			"real" if real_capable else "mock", rows.size(), tier1_command, courtier, target_province, "yes" if married else "skipped"])


## M6 : technologies (docs/design/m6-technologies.md § 4). Vraie simulation uniquement (le mock
## n'implémente pas la recherche) ; skip explicite sinon.
func _run_technologies() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_tech_tree")):
		print("smoke technologies: skipped, CampaignSim has no get_tech_tree (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "technologies: new_campaign failed"):
		return
	var tree: Array = sim.call("get_tech_tree", FACTION_ID)
	_check(tree.size() >= 30, "get_tech_tree should list >= 30 technologies, got %d" % tree.size())
	var known_before := 0
	var cheapest := ""
	var cheapest_cost := 1 << 30
	for node in tree:
		match str(node.get("state", "")):
			"known":
				known_before += 1
			"available":
				if int(node["effective_cost"]) < cheapest_cost:
					cheapest_cost = int(node["effective_cost"])
					cheapest = str(node["id"])

	# Panneau des technologies (scène réelle) : un bouton par technologie.
	var panel_scene: PackedScene = load("res://scenes/ui/tech_panel.tscn")
	var panel: Node = panel_scene.instantiate()
	root.add_child(panel)
	await process_frame
	panel.show_tree(tree, {}, int(sim.call("get_research_points", FACTION_ID)), "France", Color(0.2, 0.3, 0.7))
	var buttons: int = panel.military_view.buttons.size() + panel.civil_view.buttons.size()
	_check(buttons == tree.size(), "tech panel should show %d nodes, got %d" % [tree.size(), buttons])
	panel.queue_free()

	if not _check(cheapest != "", "no available technology for fac_france"):
		return
	var result: Dictionary = sim.call("submit_order", {"type": "research", "technology": cheapest})
	_check(result.get("ok", false), "research(%s) refused: %s" % [cheapest, result.get("error", "?")])
	var research: Dictionary = sim.call("get_research", FACTION_ID)
	_check(str(research.get("technology", "")) == cheapest, "get_research should report %s, got %s" % [cheapest, research])
	var refused: Dictionary = sim.call("submit_order", {"type": "research", "technology": "tech_masonry"})
	_check(not refused.get("ok", true), "research of a known technology should be refused")

	var researched_events := 0
	for _i in 20:
		for event in sim.call("end_turn"):
			if str(event.get("kind", "")) == "technology_researched" and str(event.get("faction", "")) == FACTION_ID:
				researched_events += 1
	var known_after := 0
	for node in sim.call("get_tech_tree", FACTION_ID):
		if str(node.get("state", "")) == "known":
			known_after += 1
	_check(known_after > known_before, "no technology acquired in 20 turns (%d -> %d)" % [known_before, known_after])
	_check(researched_events >= 1, "no technology_researched event for %s in 20 turns" % FACTION_ID)
	if _failures == 0:
		print("smoke OK: technologies (real), %d nodes, research %s (%d pts, %d/turn), known %d -> %d" % [
			tree.size(), cheapest, cheapest_cost, int(research.get("points_per_turn", 0)), known_before, known_after])


## M5 : diplomatie et religion (docs/design/m5-diplomacy-religion.md § 4). Vraie simulation
## uniquement (le mock n'a pas de diplomatie).
func _run_diplomacy() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_diplomacy")):
		print("smoke diplomacy: skipped, CampaignSim has no get_diplomacy (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "diplomacy: new_campaign failed"):
		return
	var entries: Array = sim.call("get_diplomacy", FACTION_ID)
	_check(entries.size() >= 10, "get_diplomacy should list >= 10 factions, got %d" % entries.size())
	var england: Dictionary = {}
	for entry in entries:
		if str(entry["id"]) == "fac_england":
			england = entry
	_check(str(england.get("status", "")) == "war", "France and England should be at war in 1337")
	_check(not (england.get("attitude_reasons", []) as Array).is_empty(), "attitude reasons expected")

	# Évaluation d'une paix blanche avec l'Angleterre (lue, pas forcément acceptée).
	var verdict: Dictionary = sim.call("evaluate_proposal", {"type": "propose_peace", "target": "fac_england", "provinces": [], "tribute": 0})
	_check(verdict.has("accept") and not (verdict.get("reasons", []) as Array).is_empty(), "evaluate_proposal should give a verdict with reasons")
	var war_verdict: Dictionary = sim.call("evaluate_proposal", {"type": "declare_war", "target": "fac_navarre"})
	_check(not (war_verdict.get("reasons", []) as Array).is_empty(), "declare_war evaluation should list consequences")

	# Embargo puis guerre contre une faction voisine.
	var embargo: Dictionary = sim.call("submit_order", {"type": "set_embargo", "target": "fac_aragon", "active": true})
	_check(embargo.get("ok", false), "set_embargo refused: %s" % embargo.get("error", "?"))
	var war: Dictionary = sim.call("submit_order", {"type": "declare_war", "target": "fac_navarre"})
	_check(war.get("ok", false), "declare_war refused: %s" % war.get("error", "?"))

	# Panneau réel : instanciation et rafraîchissement.
	var panel: Node = (load("res://scripts/ui/diplomacy_panel.gd") as GDScript).new()
	root.add_child(panel)
	await process_frame
	panel.sim = sim
	panel.player_faction = FACTION_ID
	panel.refresh()
	panel.select_faction("fac_england")
	await process_frame
	panel.queue_free()

	# 20 tours : pas d'erreur ; offres et religion lisibles.
	var diplomatic_events := 0
	for _i in 20:
		for event in sim.call("end_turn"):
			var kind := str(event.get("kind", ""))
			if kind in ["war_declared", "peace_signed", "alliance_formed", "alliance_broken", "diplomatic_offer", "embargo", "vassal_rebellion"]:
				diplomatic_events += 1
		for offer in sim.call("get_offers"):
			_check(str(offer.get("text", "")) != "", "offer text expected")
	var religion: Dictionary = sim.call("get_religion_state", FACTION_ID)
	_check(religion.has("papal_favor"), "get_religion_state should report papal favour")
	var province_religion: Dictionary = sim.call("get_province_religion", "prov_ile_de_france")
	_check(province_religion.has("heresy"), "get_province_religion should report heresy")
	if _failures == 0:
		print("smoke OK: diplomacy (real), %d factions, peace verdict %s, embargo + war declared, %d diplomatic events in 20 turns, favour %d" % [
			entries.size(), "accept" if verdict.get("accept", false) else "refuse", diplomatic_events, int(religion.get("papal_favor", 0))])


# --- M10 assets ------------------------------------------------------------------------


func _run_assets() -> void:
	var audio: Node = root.get_node_or_null("/root/AudioDirector")
	if not _check(audio != null, "AudioDirector autoload missing"):
		return
	var missing_sfx: Array = []
	for clip in ["ui_click", "page_turn", "turn_bell", "fanfare", "march_drum", "sword_clash", "arrow_volley", "gallop", "war_horn", "choir"]:
		if not audio.call("has_sfx", clip):
			missing_sfx.append(clip)
	_check(missing_sfx.is_empty(), "missing sound effects: %s" % [missing_sfx])
	for context in ["campaign", "war", "court"]:
		_check(audio.call("has_music", context), "missing music: %s" % context)
	_check(audio.call("play_sfx", "ui_click"), "play_sfx(ui_click) failed")
	_check(not audio.call("play_sfx", "does_not_exist"), "unknown sfx should be ignored")
	audio.call("play_music", "war")
	_check(str(audio.get("current_context")) == "war", "music context should be war")
	_check(str(audio.call("event_sfx", [{"kind": "birth"}, {"kind": "battle"}])) == "sword_clash", "battle should win event sfx priority")

	# Persistance des volumes (valeurs d'origine restaurées ensuite).
	var original: float = audio.get("music_volume")
	audio.call("set_music_volume", 0.35)
	var config := ConfigFile.new()
	_check(config.load("user://settings.cfg") == OK, "settings.cfg not written")
	_check(is_equal_approx(float(config.get_value("audio", "music_volume", -1.0)), 0.35), "music volume not persisted")
	audio.set("music_volume", 1.0)
	audio.call("load_settings")
	_check(is_equal_approx(float(audio.get("music_volume")), 0.35), "music volume not reloaded")
	audio.call("set_music_volume", original)

	# Écus, portraits et replis.
	_check(PortraitLoader.heraldry_texture("fac_france") != null, "fac_france heraldry missing")
	_check(PortraitLoader.portrait_texture("chr_does_not_exist") == null, "unknown portrait should be null")
	var swatch := ColorRect.new()
	root.add_child(swatch)
	var placed := PortraitLoader.overlay_portrait(swatch, "chr_does_not_exist", "fac_england", Vector2(48, 48))
	_check(placed and swatch.get_node_or_null(PortraitLoader.OVERLAY_NAME) != null, "heraldry fallback portrait expected")
	swatch.queue_free()
	var portraits := 0
	var dir := DirAccess.open("res://assets/portraits")
	if dir != null:
		for file_name in dir.get_files():
			if file_name.ends_with(".png") and PortraitLoader.portrait_texture(file_name.get_basename()) != null:
				portraits += 1

	# Modèles 3D.
	var missing_models: Array = []
	for model_name in ["castle", "town", "village", "cathedral", "army", "siege_camp", "ship"]:
		if not ModelLibrary.has_model(model_name):
			missing_models.append(model_name)
	_check(missing_models.is_empty(), "missing models: %s" % [missing_models])
	_check(ModelLibrary.instantiate("does_not_exist") == null, "unknown model should be null")
	_check(ModelLibrary.city_kind("prov_ile_de_france") in ["castle", "town", "village", "cathedral"], "city kind expected")
	var marker: Node3D = (load("res://scenes/map/army_marker.tscn") as PackedScene).instantiate()
	root.add_child(marker)
	await process_frame
	var dressed := ModelLibrary.dress_army_marker(marker, {"stance": "siege"}, Color(0.8, 0.1, 0.1))
	_check(dressed and marker.get_node_or_null(ModelLibrary.MODEL_NODE) != null, "army marker should get a 3D model")
	_check(not (marker.get_node("Banner") as Node3D).visible, "placeholder banner should be hidden")
	_check(marker.get_node_or_null("%s/SiegeCamp" % ModelLibrary.MODEL_NODE) != null, "siege camp expected")
	marker.queue_free()
	await process_frame
	ModelLibrary.clear_cache()
	PortraitLoader.clear_cache()
	audio.call("stop_all")
	await process_frame
	if _failures == 0:
		print("smoke OK: assets, 10 sfx + 3 music, settings persisted, %d portrait(s), 7 models, siege marker dressed" % portraits)
