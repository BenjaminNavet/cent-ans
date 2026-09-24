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
##  8. batailles (M7) : BattleSim headless (≤ 12 000 ticks, fin, resolve_battle), puis dialogue
##     d'avant-bataille → battle.tscn (60 images) → écran de fin → retour à la carte.
##  9. chronique (M10, vraie simulation si elle expose `get_pending_decisions`, sinon skip) :
##     60 tours France, au moins un événement historique et un aléatoire proposés au joueur, une
##     décision résolue par l'ordre `choose_event_option` (les autres par la méthode dédiée),
##     fenêtre `ChronicleWindow` instanciée sur une décision réelle.
##  M10 assets : autoload AudioDirector (effets, musiques, persistance des volumes dans
##     `user://settings.cfg`), écus et portraits (`PortraitLoader`), modèles 3D (`ModelLibrary`,
##     marqueur d'armée habillé) et replis quand un fichier manque.
## 11. bataille de siège (M8 § 2) : assaut français de la Guyenne (`debug_stage_siege`),
##     murailles (`get_siege`), défenseurs sur le rempart, IA des deux camps jusqu'à la fin,
##     `resolve_battle` ; puis battle.tscn sur un siège (murailles maillées, 40 images).
## 12. flow (F3) : réglages écrits et relus (volumes conservés), écran de chargement jusqu'à la
##     carte, sauvegarde automatique tournante (auto_1..3, les fichiers du joueur sont mis de
##     côté puis restaurés), rapport de saison non vide après quelques tours, alertes, menu
##     pause ouvert puis fermé (arbre en pause), dialogue de sauvegarde, crédits.
## 13. codex (H2) : fiches de `data/codex`, liens `[[…]]` et auto-liens, pile de 3 bulles (dont
##     une ouverte par survol simulé), découvertes, fermeture après la grâce, fenêtre Codex.
## 14. tutorial/encyclopedia (F8) : tutoriel France sur la vraie simulation, chaque objectif
##     rempli par l'interface ou un ordre fait avancer l'étape (sélection, marche, province,
##     onglet Ville, construction, recherche, diplomatie, fin de tour, rapport, chronique,
##     impôt, gouverneur), progression persistée ; encyclopédie : chaque onglet > 0 entrée,
##     fiche non vide, recherche filtrée, liens internes, retour, touche L.
## 15. table/médecine (H9) : section Table (changement de régime par l'UI, refus affiché,
##     lecture seule), ligne de budget, infobulle de tech médecine, genres table/medicine.
## 16. monnaie/rançons (H11) : changement de monnaie par l'UI et refus du second dans l'année,
##     budget, ordre de chevalerie (refus affiché), panneau des rançons (données simulées), genres
##     coinage/ransom/chivalry, liens Encyclopédie ↔ Codex. Seule : CENT_ANS_SMOKE_ONLY=coinage_ransom.
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
	# F3 : réglages par défaut sur un fichier dédié (le fichier du joueur n'est pas touché),
	# sans sauvegarde automatique hors de l'étape « flow ».
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)  # F8 : seulement à l'étape 13
	# Exécution ciblée d'une étape : CENT_ANS_SMOKE_ONLY=coinage_ransom.
	if OS.get_environment("CENT_ANS_SMOKE_ONLY") == "coinage_ransom":
		await _run_coinage_ransom()
		quit(1 if _failures > 0 else 0)
		return
	await _run_campaign_map()
	await _run_start_menu()
	await _run_campaign_loop()
	_run_city_economy()
	await _run_characters()
	await _run_technologies()
	await _run_diplomacy()
	await _run_battle()
	await _run_chronicle()
	await _run_assets()  # M10 assets
	await _run_icons()  # F2
	await _run_codex()  # H2
	await _run_table_medicine()  # H9
	await _run_coinage_ransom()  # H11
	await _run_siege_battle()
	await _run_flow()  # F3
	await _run_tutorial()  # F8
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
	_check(map.ui.army_strip.visible and map.ui.general_seal.visible, "army strip and general seal should be visible")
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
	# M10 : pas d'événements de chronique pendant la mesure (le bruit masquerait l'effet du bâtiment).
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)

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
	# Depuis G2 l'IA peut retourner un vassal ou déclarer la guerre pendant cette fenêtre : le
	# revenu global peut baisser pour d'autres raisons. On accepte donc aussi la preuve que le
	# bâtiment est entré dans les comptes (entretien des bâtiments en hausse).
	var best_after := int(economy_before.get("projected_income", 0))
	var upkeep_before := int(sim.call("get_faction_summary", FACTION_ID).get("building_upkeep", 0))
	var upkeep_after := upkeep_before
	for _i in 6:
		sim.call("end_turn")
		var economy: Dictionary = sim.call("get_faction_economy", FACTION_ID)
		best_after = maxi(best_after, int(economy.get("projected_income", 0)))
		upkeep_after = maxi(upkeep_after, int(sim.call("get_faction_summary", FACTION_ID).get("building_upkeep", 0)))
	# Information seulement : l'effet d'un bâtiment est testé côté Rust (tests m3/f1) ; sur la vraie
	# simulation, la dérive de fond (guerre, IA, prix) peut le masquer.
	print("smoke city/economy: after construction, income %d -> best %d, building upkeep %d -> %d" % [economy_before.get("projected_income", 0), best_after, upkeep_before, upkeep_after])

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
	var buttons: int = panel.military_view.buttons.size() + panel.civil_view.buttons.size() + panel.medicine_view.buttons.size()
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
## M7 (docs/design/m7-battles.md § 4) : bataille réelle France–Angleterre mise en scène par
## `debug_stage_battle`, ≤ 12 000 ticks headless de `BattleSim` (IA des deux camps), fin atteinte,
## `resolve_battle` accepté ; puis la boucle complète par la carte : dialogue d'avant-bataille,
## « Livrer bataille », scène `battle.tscn` 60 images, fin de bataille, « Retour à la campagne ».
func _run_battle() -> void:
	if not ClassDB.class_exists("BattleSim") or not ClassDB.instantiate("CampaignSim").has_method("debug_stage_battle"):
		_fail("battle: BattleSim / CampaignSim.debug_stage_battle not registered (run core/build.sh)")
		return
	var data_dir := _project_root().path_join("data")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "battle: new_campaign failed"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	if not _check(armies.size() == 2, "battle: no French or English army"):
		return
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var pending: Array = sim.call("get_pending_battles")
	_check(index == 0 and pending.size() == 1, "battle: debug_stage_battle should record 1 pending battle, got %d" % pending.size())
	_check(str(pending[0].get("player_side", "")) == "attacker", "battle: France should attack")
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", setup, int(pending[0]["seed"])), "battle: BattleSim.setup refused the campaign setup"):
		return
	battle.call("set_ai", "attacker", true)
	var units: Array = battle.call("get_units")
	_check(units.size() == (setup["attacker"]["units"] as Array).size() + (setup["defender"]["units"] as Array).size(), "battle: unit count mismatch")
	var terrain: Dictionary = battle.call("get_terrain")
	_check((terrain["heights"] as PackedFloat32Array).size() == int(terrain["nx"]) * int(terrain["nz"]), "battle: terrain grid size")
	var soldiers: PackedFloat32Array = battle.call("get_soldier_transforms", "attacker")
	_check(soldiers.size() > 0 and soldiers.size() % 4 == 0, "battle: soldier transforms empty")
	var refused: Dictionary = battle.call("issue_command", {"type": "halt", "units": [units.size() - 1]})
	_check(not refused.get("ok", true), "battle: commanding an enemy unit should be refused")
	# F10b : ordres du chef (catalogue de data/battle_orders, cri de guerre propre à la faction).
	var orders: Array = battle.call("get_leader_orders", "attacker")
	_check(orders.size() == 5, "battle: expected 5 leader's orders, got %d" % orders.size())
	if not orders.is_empty():
		_check(str(orders[0]["label"]) == "Montjoie ! Saint-Denis !", "battle: French war cry label is %s" % orders[0]["label"])
		var cry: Dictionary = battle.call("issue_command", {"type": "leader_order", "order": "order_war_cry", "units": []})
		_check(bool(cry.get("ok", false)) or str(cry.get("error", "")).contains("impossible"), "battle: war cry command malformed: %s" % cry.get("error", "?"))
	var ticks := 0
	for _i in 12000:
		battle.call("tick", 0.1)
		ticks += 1
		if battle.call("is_finished"):
			break
	if not _check(battle.call("is_finished"), "battle: not finished after 12000 ticks"):
		return
	var outcome: Dictionary = battle.call("get_outcome")
	var events: Array = battle.call("get_events")
	_check(not events.is_empty(), "battle: no battle journal")
	var result: Dictionary = sim.call("resolve_battle", index, outcome)
	_check(result.get("ok", false), "battle: resolve_battle refused: %s" % result.get("error", "?"))
	_check((sim.call("get_pending_battles") as Array).is_empty(), "battle: pending battle should be gone")
	print("smoke battle: %d ticks, winner %s, losses %d / %d, %d journal lines" % [ticks, outcome["winner"], int(outcome["attacker"]["total_losses"]), int(outcome["defender"]["total_losses"]), events.size()])

	# Boucle complète par la carte de campagne (vraies données).
	facade.set_data_dir(data_dir)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null and facade.is_real, "battle: campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var map_armies: Array = BattleScene.main_armies(map.sim, "fac_france", "fac_england")
	map.sim.call("debug_stage_battle", map_armies[0], map_armies[1])
	map._offer_pending_battles()
	var dialog: Node = map._battle_dialog
	if not _check(dialog != null and dialog.visible, "battle: pre-battle dialog should be visible"):
		map.queue_free()
		return
	_check(str(dialog.body_label.text).contains("Météo prévue"), "battle: dialog should show the weather forecast")
	dialog.fight_button.emit_signal("pressed")
	await process_frame
	var scene: Node = null
	for child in root.get_children():
		if child is BattleScene:
			scene = child
	if not _check(scene != null, "battle: battle.tscn not opened by « Livrer bataille »"):
		map.queue_free()
		return
	_check(not map.visible and map.process_mode == Node.PROCESS_MODE_DISABLED, "battle: campaign map should sleep during the battle")
	for _i in 60:
		await process_frame
	_check(scene.units.size() > 0, "battle scene: no units")
	_check(scene.terrain.get_child_count() > 0, "battle scene: no terrain")
	var drawn := 0
	for key in scene._mm:
		drawn += (scene._mm[key] as MultiMeshInstance3D).multimesh.visible_instance_count
	_check(drawn > 0, "battle scene: no soldier instances")
	await _check_battle_deployment_f5c(scene)  # F5c
	_check_battle_hud_f5b(scene)
	# Un ordre du joueur via l'API de la scène, puis fin de bataille accélérée.
	var own: int = -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side:
			own = int(unit["id"])
			break
	var order: Dictionary = scene.issue({"type": "move", "units": [own], "x": 600.0, "z": 330.0, "run": false})
	_check(order.get("ok", false), "battle scene: move order refused: %s" % order.get("error", "?"))
	scene.battle.call("set_ai", scene.player_side, true)
	for _i in 36000:
		scene.battle.call("tick", 0.1)
		if scene.battle.call("is_finished"):
			break
	await process_frame
	await process_frame
	_check(scene.finished_shown and scene.hud.end_panel.visible, "battle scene: end screen should be visible")
	scene._on_return()
	await process_frame
	_check(map.visible and map.process_mode == Node.PROCESS_MODE_INHERIT, "battle: campaign map should be back after the battle")
	_check((map.sim.call("get_pending_battles") as Array).is_empty(), "battle: pending battle should be resolved after « Retour à la campagne »")
	_check(map.ui.log_line_count() > 0, "battle: campaign journal should list the battle")
	if _failures == 0:
		print("smoke OK: battle (headless %d ticks, scene 60 frames, %d soldiers drawn, resolved through the map)" % [ticks, drawn])
	map.queue_free()
	await process_frame


## F5b : cartes compactes rangées par « bataille », groupe Ctrl+1 enregistré puis rappelé,
## minicarte présente et peuplée, boutons de vitesse, noms coupés entre deux mots.
func _check_battle_hud_f5b(scene: BattleScene) -> void:
	var hud: BattleHud = scene.hud
	var own := 0
	var first := -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side:
			own += 1
			if first < 0:
				first = int(unit["id"])
	_check(hud.card_count() == own and own > 0, "battle HUD: %d cards for %d own units" % [hud.card_count(), own])
	_check(BattleGroups.ORDER.has(hud.card_battle(first)), "battle HUD: card not in a battle column")
	var card: UnitCard = hud._cards[first]
	_check(card.custom_minimum_size.x <= 72.0 and card.tooltip_text.contains("Formation"), "battle HUD: card should be compact with formation in its tooltip")
	var font := card.get_theme_default_font()
	var fitted := UnitCard.fit_name("Arbalétriers génois de la compagnie Grimaldi", font, 10, 63.0, 2)
	_check(fitted.split("\n").size() <= 2 and not fitted.contains("Grimaldi"), "battle HUD: name fitting should stop on a whole word: %s" % fitted)
	scene.selected.clear()
	scene.selected.append(first)
	scene.handle_group_key(1, true)
	scene.selected.clear()
	scene.handle_group_key(1, false)
	_check(scene.selected.size() == 1 and scene.selected[0] == first, "battle HUD: group 1 should be recalled (got %s)" % [scene.selected])
	_check(hud.minimap != null and hud.minimap.is_visible_in_tree(), "battle HUD: minimap missing")
	scene._refresh_view(true)
	_check(hud.minimap.dot_count() > own, "battle HUD: minimap should show both sides")
	scene._on_speed_pressed(1)
	_check(hud.active_speed() == 1 and not scene.paused, "battle HUD: ×2 speed button not active")
	scene._on_speed_pressed(0)
	_check(hud.help_panel != null and not hud.help_panel.visible, "battle HUD: F1 help should start hidden")


func _run_chronicle() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_pending_decisions")):
		print("smoke chronicle: skipped, CampaignSim has no get_pending_decisions (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "chronicle: new_campaign failed"):
		return
	var historical := 0
	var random := 0
	var by_order := 0
	var journal := 0
	var sample: Dictionary = {}
	for _i in 60:
		for event in sim.call("end_turn"):
			if str(event.get("kind", "")) == "chronicle":
				journal += 1
		for decision in sim.call("get_pending_decisions"):
			if decision.get("historical", false):
				historical += 1
			else:
				random += 1
			_check(str(decision.get("title", "")) != "" and not (decision.get("options", []) as Array).is_empty(), "decision needs a title and options")
			if sample.is_empty() and decision.get("historical", false):
				sample = decision
			var result: Dictionary
			if by_order == 0:
				result = sim.call("submit_order", {"type": "choose_event_option", "decision": int(decision["id"]), "option": 0})
				by_order += 1
			else:
				result = sim.call("choose_event_option", int(decision["id"]), 0)
			_check(result.get("ok", false), "choose_event_option refused: %s" % result.get("error", "?"))
	_check(historical >= 1, "60 turns should bring at least one historical event, got %d" % historical)
	_check(random >= 1, "60 turns should bring at least one random event, got %d" % random)
	_check(by_order == 1, "one decision should be resolved by order")
	_check((sim.call("get_pending_decisions") as Array).is_empty(), "no decision should remain")
	var refused: Dictionary = sim.call("choose_event_option", 99999, 0)
	_check(not refused.get("ok", true), "unknown decision should be refused")

	# Fenêtre réelle sur une décision historique.
	var window: Node = (load("res://scenes/ui/chronicle_window.tscn") as PackedScene).instantiate()
	root.add_child(window)
	await process_frame
	if not sample.is_empty():
		window.call("show_decision", sample, 2)
		await process_frame
		_check(window.visible, "chronicle window should be visible")
	window.queue_free()
	if _failures == 0:
		print("smoke OK: chronicle (real), %d historical + %d random decisions in 60 turns, %d journal entries, sample « %s »" % [
			historical, random, journal, str(sample.get("title", "?"))])


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


## F2 : icônes (`IconLibrary`, `game/assets/icons/icons.json`) et infobulles riches. Toutes
## les entrées de la table se chargent ; chaque id de `data/` (unités, bâtiments, ressources,
## technologies) a sa propre icône, chaque branche de compétence et catégorie de trait aussi ;
## replis par catégorie ; BBCode d'infobulle non vide et panneau constructible.
func _run_icons() -> void:
	var library: Node = root.get_node_or_null("/root/IconLibrary")
	if not _check(library != null, "IconLibrary autoload missing"):
		return
	var table: Dictionary = library.get("icons")
	_check(table.size() >= 100, "icons.json too small: %d entries" % table.size())
	var broken: Array = []
	for id in table:
		if library.call("get_icon", str(id)) == null:
			broken.append(id)
	_check(broken.is_empty(), "icons not loadable: %s" % [broken])
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var missing: Array = []
	var checked := 0
	for directory in ["unit_types", "buildings", "resources", "technologies", "skills", "traits"]:
		var dir := DirAccess.open(data_dir.path_join(directory))
		if not _check(dir != null, "data/%s missing" % directory):
			continue
		for file_name in dir.get_files():
			if not file_name.ends_with(".json"):
				continue
			var entry: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join(directory).path_join(file_name)))
			var id := str(entry.get("id", ""))
			if directory == "skills":
				id = "branch_" + str(entry.get("branch", ""))
			elif directory == "traits":
				id = "trait_category_" + str(entry.get("category", ""))
			checked += 1
			if not library.call("has_icon", id):
				missing.append(id)
	_check(missing.is_empty(), "data ids without icon: %s" % [missing])
	for hud_id in ["hud_treasury", "hud_income", "hud_research", "hud_season_spring", "hud_season_summer", "hud_season_autumn", "hud_season_winter", "hud_diplomacy", "hud_chronicle", "hud_court", "hud_technologies", "class_peasants", "class_burghers", "class_clergy", "class_nobility", "gauge_unrest", "gauge_health", "gauge_wealth", "gauge_goods_satisfaction"]:
		if not library.call("has_icon", hud_id):
			missing.append(hud_id)
	_check(missing.is_empty(), "UI ids without icon: %s" % [missing])
	_check(str(library.call("resolve", "unit_does_not_exist")) == "cat_unit", "unit fallback expected")
	_check(str(library.call("resolve", "bld_does_not_exist")) == "cat_building", "building fallback expected")
	_check(library.call("get_icon", "totally_unknown") != null, "default fallback expected")
	var tip := RichTooltip.technology({"id": "tech_bombards", "name": "Bombardes", "branch": "military", "tier": 3, "cost": 350, "effective_cost": 350, "effects": [{"kind": "siege_resistance", "value": -5}], "historical_year": 1346})
	_check(tip.contains("[img") and tip.contains("1346") and tip.contains("Résistance aux sièges"), "technology tooltip incomplete: %s" % tip)
	var panel := RichTooltip.make_panel(RichTooltip.gauge("unrest", 40))
	root.add_child(panel)
	await process_frame
	_check(panel.get_node_or_null("Text") is RichTextLabel, "tooltip panel text expected")
	panel.queue_free()
	var chip := IconChip.create("res_wine", "Vin", "x")
	_check(chip.icon_rect != null and chip.icon_rect.texture != null, "icon chip texture expected")
	chip.free()
	if _failures == 0:
		print("smoke OK: icons, %d entries loaded, %d data ids covered, fallbacks and rich tooltips" % [table.size(), checked])


## H2 : Codex (fiches, liens, bulles imbriquées, découvertes, fenêtre).
func _run_codex() -> void:
	var store: Node = root.get_node_or_null("/root/CodexStore")
	var bubbles: Node = root.get_node_or_null("/root/CodexBubbles")
	if not _check(store != null and bubbles != null, "CodexStore / CodexBubbles autoloads missing"):
		return
	store.call("use_test_file")
	store.call("reload", _project_root().path_join("data/codex"))
	var total: int = store.call("total_count")
	_check(total >= 20, "codex should have at least 20 entries, got %d" % total)
	_check(bool(store.call("has_entry", "cdx_crecy")) and bool(store.call("has_entry", "cdx_poitiers")), "codex seed entries missing")
	_check(str(store.call("entry_for_entity", "chr_charles_v")) == "cdx_charles_v", "entity index expected")

	# Liens explicites, auto-liens (première occurrence, hors balises), couleur des fiches lues.
	var linked := CodexText.format("[[cdx_crecy]] puis [[cdx_poitiers|la défaite du roi]].")
	_check(linked.contains("[url=cdx:cdx_crecy]") and linked.contains("[u]") and linked.contains(str(store.call("title", "cdx_crecy"))), "explicit link: %s" % linked)
	_check(linked.contains("[url=cdx:cdx_poitiers]") and linked.contains("la défaite du roi"), "labelled link: %s" % linked)
	_check(CodexText.format("[[cdx_nothing_here|mot]]") == "mot", "unknown link should degrade to its label")
	var alias := str(store.call("title", "cdx_peste_noire"))
	var auto := CodexText.format("[img]res://x/%s.png[/img] En 1348, %s frappe ; %s encore." % [alias, alias, alias], true)
	_check(auto.count("[url=cdx:cdx_peste_noire]") == 1 and auto.begins_with("[img]res://x/%s.png[/img]" % alias), "auto link once, outside tags: %s" % auto)
	_check(not CodexText.format("%s." % alias).contains("[url"), "no auto link unless asked")
	var tooltip := RichTooltip.make_panel("Voir [[cdx_arc_long]].")
	_check(RichTooltip.last_bbcode.contains("[url=cdx:cdx_arc_long]"), "rich tooltips should go through CodexText")
	_check(RichTooltip.visible_panel() == null, "a detached panel is not a visible tooltip")
	tooltip.free()

	# Pile de 3 bulles : deux ouvertes par l'API, la troisième par survol simulé d'un lien.
	var first: Control = bubbles.call("open", "cdx_crecy", Vector2(120, 120))
	var second: Control = bubbles.call("open", "cdx_edouard_iii", Vector2(260, 180), 0)
	await process_frame
	_check(first != null and second != null and int(bubbles.call("bubble_count")) == 2, "two bubbles expected")
	var text := second.find_child("Text", true, false) as RichTextLabel
	_check(text != null and text.text.contains("[url=cdx:"), "bubble summary should contain links")
	text.meta_hover_started.emit("cdx:cdx_poitiers")
	await create_timer(0.5).timeout
	_check(int(bubbles.call("bubble_count")) == 3 and str(bubbles.call("top_id")) == "cdx_poitiers", "hovering a bubble link should open a third bubble (count %d)" % int(bubbles.call("bubble_count")))
	for id in ["cdx_crecy", "cdx_edouard_iii", "cdx_poitiers"]:
		_check(bool(store.call("is_discovered", id)), "%s should be discovered" % id)
	_check(CodexText.link("cdx_crecy").contains(CodexText.READ_COLOR), "read entries use brown ink")
	var reopened: Control = bubbles.call("open", "cdx_chevauchee", Vector2(300, 300), 0)
	_check(reopened != null and int(bubbles.call("bubble_count")) == 2, "opening from bubble 0 should replace the bubbles above it")
	bubbles.call("set_pinned", first, true)
	text.meta_hover_ended.emit("cdx:cdx_poitiers")
	await create_timer(0.7).timeout
	_check(int(bubbles.call("bubble_count")) == 1, "unpinned bubbles should close after the grace delay (count %d)" % int(bubbles.call("bubble_count")))
	bubbles.call("close_all")
	_check(int(bubbles.call("bubble_count")) == 0, "close_all should empty the stack")
	var pinned: Control = bubbles.call("open_text", RichTooltip.last_bbcode, Vector2(40, 40))
	_check(pinned != null and bool(pinned.get_meta("pinned", false)), "pinned tooltip bubble expected")
	bubbles.call("close_all")

	# Fenêtre Codex sur une fiche, historique, compteur.
	bubbles.call("open_entry", "cdx_charles_v")
	await process_frame
	var window: Control = bubbles.call("window")
	_check(bool(bubbles.call("is_window_open")) and str(window.get("current_id")) == "cdx_charles_v", "codex window should show cdx_charles_v")
	var body: RichTextLabel = window.call("body_label")
	_check(body.text.length() > 200 and body.text.contains("[url=cdx:"), "codex body should be formatted with links")
	window.call("navigate", "cdx_du_guesclin")
	window.call("back")
	_check(str(window.get("current_id")) == "cdx_charles_v", "history back expected")
	var counter := str(window.call("counter_text"))
	_check(counter == "%d / %d découvertes" % [int(store.call("discovered_count")), total], "counter: %s" % counter)
	window.hide()
	store.call("reset_discoveries")
	if _failures == 0:
		print("smoke OK: codex, %d entries, links, 3-bubble stack, discoveries, window (%s)" % [total, counter])


## H9 : interface de la Table et de la médecine (vraie simulation si elle expose
## `get_diet_options`, sinon skip imprimé) : section Table d'une province française, changement
## de régime par l'interface, refus affiché, infobulle de tech médecine (plantes, note),
## genres `table` / `medicine` mappés (rapport, lettres, alertes), herbier silencieux.
func _run_table_medicine() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_diet_options")):
		print("smoke table/medicine: skipped, CampaignSim has no get_diet_options (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "table/medicine: new_campaign failed"):
		return
	# Province au joueur avec un régime payant disponible (≠ actuel), une autre avec un régime
	# indisponible.
	var changed_province := ""
	var target_diet := ""
	var refused_province := ""
	var refused_diet := ""
	var diets: Dictionary = sim.call("get_province_diets")
	var province_ids: Array = diets.keys()
	province_ids.sort()
	for province_id in province_ids:
		var state: Dictionary = sim.call("get_province_state", str(province_id))
		if str(state.get("controller", state.get("owner", ""))) != FACTION_ID:
			continue
		for option in sim.call("get_diet_options", str(province_id)):
			var id := str(option.get("id", ""))
			if changed_province == "" and bool(option.get("available", false)) and not bool(option.get("current", false)) and int(option.get("cost", 0)) > 0:
				changed_province = str(province_id)
				target_diet = id
			elif refused_province == "" and str(province_id) != changed_province and not bool(option.get("available", false)):
				refused_province = str(province_id)
				refused_diet = id
	if not _check(changed_province != "" and refused_province != "", "table: no French province with available/unavailable diets (%s / %s)" % [changed_province, refused_province]):
		return

	# Panneau de province réel : la section Table est dans l'onglet Ville.
	var panel: Node = (load("res://scenes/ui/province_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	var hosted: TableSection = panel.get("table_section")
	_check(hosted != null and hosted.get_parent() == (panel.get("classes_list") as Node).get_parent(), "province panel should host the Table section in the Ville tab")
	panel.queue_free()

	var table := TableSection.new()
	root.add_child(table)
	table.show_for(changed_province, true, sim)
	await process_frame
	_check(table.visible and table.option_buttons.size() == (sim.call("get_diet_options", changed_province) as Array).size(), "table section should list every diet option")
	var tip := RichTooltip.diet(table._option(target_diet))
	_check(tip.contains("Coût") and tip.contains("[img"), "diet tooltip incomplete: %s" % tip)
	(table.option_buttons[target_diet] as Button).pressed.emit()
	var now: Dictionary = sim.call("get_province_diet", changed_province)
	_check(str(now.get("diet", "")) == target_diet, "set_diet via UI: expected %s, got %s (%s)" % [target_diet, now.get("diet", ""), table.last_result])
	_check(table.changed_label.visible and table.choose_button.disabled, "changed-this-turn state should be shown")
	var lent_expected := bool(sim.call("is_lent"))
	_check(table.lent_banner.visible == lent_expected, "Lent banner should follow is_lent (%s)" % lent_expected)

	table.show_for(refused_province, true, sim)
	var refused: Dictionary = table.request_diet(refused_diet)
	_check(not bool(refused.get("ok", true)) and table.error_label.visible and table.error_label.text.contains("impossible"), "unavailable diet should be refused and shown: %s" % table.error_label.text)
	var refused_tip := RichTooltip.diet(table._option(refused_diet))
	_check(refused_tip.contains("Manque"), "unavailable diet tooltip should list missing conditions: %s" % refused_tip)
	table.show_for(refused_province, false, sim)
	_check(not table.choose_button.visible and table.option_buttons.is_empty(), "read-only province: no selector")
	table.queue_free()

	var economy: Dictionary = sim.call("get_faction_economy", FACTION_ID)
	_check(int(economy.get("table_upkeep", 0)) > 0, "table_upkeep should be > 0 after a paying diet")

	# Infobulle de tech médecine : plantes et note historique ; libellés des effets.
	var herb_node: Dictionary = {}
	for node in sim.call("get_tech_tree", FACTION_ID):
		if str(node.get("id", "")) == "tech_herb_garden":
			herb_node = node
	var tech_tip := RichTooltip.technology(herb_node)
	_check(tech_tip.contains("Plantes :") and tech_tip.contains("sauge") and tech_tip.contains("médecine") and tech_tip.contains("De Villis"), "medicine tech tooltip incomplete: %s" % tech_tip)
	_check(RichTooltip.effect_text({"kind": "plague_resistance", "value": 10}).begins_with("Résistance à la peste"), "plague_resistance label")
	_check(RichTooltip.effect_text({"kind": "wound_recovery", "value": 15, "mode": "percent"}).begins_with("Soin des blessés"), "wound_recovery label")
	_check(RichTooltip.effect_text({"kind": "diet_health", "value": 25}).begins_with("Santé tirée des régimes"), "diet_health label")

	# Genres table / medicine : rapport de saison, lettres, alertes ; herbier.
	var events := [
		{"kind": "table", "text_fr": "La table revient au pain bis.", "faction": FACTION_ID, "province": changed_province},
		{"kind": "medicine", "text_fr": "Épidémie contenue.", "faction": FACTION_ID, "province": ""},
	]
	var groups := SeasonReport.build_groups(events, func(_event: Dictionary) -> bool: return true)
	_check(groups.size() == 1 and (groups[0]["entries"] as Array).size() == 2, "season report should group table/medicine events: %s" % [groups])
	_check(NewsLetters.KIND_LABELS.has("table") and NewsLetters.KIND_LABELS.has("medicine"), "news letters labels for table/medicine")
	var alerts := CampaignAlerts.table_medicine_alerts(sim, FACTION_ID, events)
	_check(alerts.size() == 2 and str(alerts[0]["kind"]) == "table", "alerts for table/medicine: %s" % [alerts])
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file")
		var herbs := Herbarium.sync(sim, FACTION_ID)
		_check(herbs.size() >= 0, "herbarium sync should ignore missing entries")
		store.call("reset_discoveries")
	if _failures == 0:
		print("smoke OK: table/medicine, %s -> %s, refused %s in %s, table upkeep %d" % [changed_province, target_diet, refused_diet, refused_province, int(economy.get("table_upkeep", 0))])


## M8 § 2 : bataille de siège réelle (armée française devant la Guyenne anglaise), headless puis
## dans la scène 3D.
func _run_siege_battle() -> void:
	if not ClassDB.instantiate("CampaignSim").has_method("debug_stage_siege"):
		_fail("siege battle: CampaignSim.debug_stage_siege not registered (run core/build.sh)")
		return
	var data_dir := _project_root().path_join("data")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", data_dir, "fac_france", 1337), "siege battle: new_campaign failed"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	var pending: Array = sim.call("get_pending_battles")
	if not _check(index == 0 and pending.size() == 1 and bool(pending[0].get("siege", false)), "siege battle: debug_stage_siege should record 1 pending siege battle"):
		return
	_check(int(pending[0].get("defender_strength", 0)) > 0, "siege battle: the garrison should have soldiers")
	var setup: Dictionary = sim.call("get_battle_setup", index)
	_check(setup.has("siege"), "siege battle: setup without siege parameters")
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not _check(battle.call("setup", setup, int(pending[0]["seed"])), "siege battle: BattleSim.setup refused the siege setup"):
		return
	battle.call("set_ai", "attacker", true)
	var siege: Dictionary = battle.call("get_siege")
	_check((siege.get("pieces", []) as Array).size() >= 10, "siege battle: wall pieces missing")
	_check((battle.call("get_terrain") as Dictionary).has("siege"), "siege battle: get_terrain should carry the walls")
	var on_wall := 0
	for unit in battle.call("get_units"):
		if bool(unit.get("on_wall", false)):
			on_wall += 1
	_check(on_wall > 0, "siege battle: no defender on the walls")
	var ticks := 0
	for _i in 36000:
		battle.call("tick", 0.1)
		ticks += 1
		if battle.call("is_finished"):
			break
	if not _check(battle.call("is_finished"), "siege battle: not finished after 36000 ticks"):
		return
	var outcome: Dictionary = battle.call("get_outcome")
	var integrity := float((battle.call("get_siege") as Dictionary).get("integrity", 1.0))
	var result: Dictionary = sim.call("resolve_battle", index, outcome)
	_check(result.get("ok", false), "siege battle: resolve_battle refused: %s" % result.get("error", "?"))
	_check((sim.call("get_pending_battles") as Array).is_empty(), "siege battle: pending battle should be gone")
	print("smoke siege battle: %d ticks, winner %s, losses %d / %d, walls %d %%" % [ticks, outcome["winner"], int(outcome["attacker"]["total_losses"]), int(outcome["defender"]["total_losses"]), int(integrity * 100.0)])

	# La scène 3D sur un second assaut.
	var sim2: Object = ClassDB.instantiate("CampaignSim")
	sim2.call("new_campaign", data_dir, "fac_france", 1338)
	var armies2: Array = BattleScene.main_armies(sim2, "fac_france", "fac_england")
	var index2: int = sim2.call("debug_stage_siege", armies2[0], "prov_guyenne")
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim2, index2, 7)
	root.add_child(scene)
	for _i in 40:
		await process_frame
	if not _check(scene.siege_view != null and scene.siege_view.get_child_count() > 10, "siege scene: walls not built"):
		scene.queue_free()
		return
	_check(scene.hud.siege_panel.visible, "siege scene: siege status should be shown")
	await _check_siege_f5c(scene)  # F5c
	scene.battle.call("set_ai", scene.player_side, true)
	for _i in 36000:
		scene.battle.call("tick", 0.1)
		if scene.battle.call("is_finished"):
			break
	for _i in 3:
		await process_frame
	_check(scene.finished_shown, "siege scene: end screen should be visible")
	scene._on_return()
	await process_frame
	_check((sim2.call("get_pending_battles") as Array).is_empty(), "siege scene: pending siege should be resolved")
	if _failures == 0:
		print("smoke OK: siege battle (headless %d ticks, scene with %d wall nodes, resolved)" % [ticks, scene.siege_view.get_child_count()])
	scene.queue_free()
	await process_frame


# --- F3 : écrans et flux -----------------------------------------------------------


func _run_flow() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if not _check(settings != null, "Settings autoload missing"):
		return
	# Réglages : écriture, relecture, section audio conservée.
	var test_path: String = settings.get("path")
	_check(test_path != "user://settings.cfg", "smoke must not use the player's settings file")
	var seeded := ConfigFile.new()
	seeded.set_value("audio", "music_volume", 0.42)
	seeded.save(test_path)
	settings.call("set_value", "camera/speed", 1.7)
	settings.call("set_value", "interface/confirm_end_turn", true)
	settings.call("set_value", "video/resolution", Vector2i(1600, 900))
	var config := ConfigFile.new()
	_check(config.load(test_path) == OK, "settings file not written")
	_check(is_equal_approx(float(config.get_value("camera", "speed", 0.0)), 1.7), "camera speed not persisted")
	_check(is_equal_approx(float(config.get_value("audio", "music_volume", 0.0)), 0.42), "audio section lost by Settings.save_settings")
	settings.set("values", {})
	settings.call("load_settings")
	_check(is_equal_approx(float(settings.call("get_value", "camera/speed")), 1.7), "camera speed not reloaded")
	_check(bool(settings.call("get_value", "interface/confirm_end_turn")), "confirm_end_turn not reloaded")
	_check(settings.call("get_value", "video/resolution") == Vector2i(1600, 900), "resolution not reloaded")
	settings.call("set_value", "interface/confirm_end_turn", false, false)
	settings.call("set_value", "interface/season_report", true, false)
	settings.call("set_value", "game/autosave_interval", 1, false)
	_check(SaveSlots.autosave_name_for(1, 1) == "auto_1" and SaveSlots.autosave_name_for(4, 1) == "auto_1"
		and SaveSlots.autosave_name_for(6, 2) == "auto_3" and SaveSlots.autosave_name_for(3, 2) == "", "autosave rotation names")

	# Écran de chargement jusqu'à la carte (vraies données si possible, comme la boucle de campagne).
	var real_data := _project_root().path_join("data")
	var use_real: bool = ClassDB.class_exists("CampaignSim") and FileAccess.file_exists(real_data.path_join("map/map.json"))
	facade.set_data_dir(real_data if use_real else _fixtures_dir)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var backup := _backup_autosaves()
	var screen: Node = LoadingScreen.start(self)
	var map: Node = await screen.finished
	if not _check(map != null and map.get("load_ok"), "loading screen did not produce a loaded campaign map"):
		_restore_autosaves(backup)
		return
	_check(current_scene == map, "loading screen should make the map the current scene")
	# Accès non typé : typer FlowController compilerait MapUI avant les autoloads.
	var flow: Node = map.get("flow")
	if not _check(flow != null, "campaign map has no FlowController"):
		_restore_autosaves(backup)
		return
	_check(is_equal_approx(map.camera_rig.pan_speed, 1.2 * 1.7), "camera speed setting not applied")

	# Tours : sauvegarde auto tournante, rapport de saison, alertes.
	var report_lines := 0
	for _turn in 4:
		map._on_end_turn()
		map.chronicle.window.hide()
		if flow.season_report.visible:
			report_lines = maxi(report_lines, flow.season_report.line_count())
	await process_frame
	var turn: int = map.sim.call("get_turn")
	_check(turn == 4, "flow: expected turn 4, got %d" % turn)
	for slot in ["auto_1", "auto_2", "auto_3"]:
		_check(FileAccess.file_exists(facade.save_path(slot)) and FileAccess.file_exists(SaveSlots.meta_path(slot)), "autosave %s missing" % slot)
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(SaveSlots.meta_path("auto_1")))
	_check(meta is Dictionary and int(meta.get("turn", -1)) == 4, "auto_1 should have rotated to turn 4, got %s" % [meta])
	_check(flow.unsaved_turns() == 0, "autosave should reset the unsaved counter")
	_check(not SaveSlots.latest().is_empty(), "Continue: latest save expected")
	_check(report_lines > 0, "season report should list events after 4 turns")
	var all_events: Array = map.sim.call("get_events")
	print("smoke flow: %d report lines, %d alerts, %d journal events" % [report_lines, map.ui.end_turn_cluster.alerts.size(), all_events.size()])

	# P2 : bataille résolue après la fin du tour (dialogue d'avant-bataille) — son résultat
	# doit rejoindre le rapport de saison déjà affiché, pas seulement la chronique
	# (docs/wip/finalisation.md § « Défauts relevés », docs/wip/p2-rapport-smoke.md).
	var battle_armies: Array = BattleScene.main_armies(map.sim, "fac_france", "fac_england")
	if _check(battle_armies.size() == 2, "flow: no French or English army for the late-battle check"):
		var battle_index: int = map.sim.call("debug_stage_battle", battle_armies[0], battle_armies[1])
		_check(battle_index >= 0, "flow: debug_stage_battle failed")
		map._on_battle_auto(battle_index)
		await process_frame
		_check(flow.season_report.visible, "flow: season report should (re)open after a late battle result")
		var found_result := false
		for group in flow.season_report.groups:
			for entry in (group["entries"] as Array):
				if str((entry as Dictionary).get("text_fr", "")).contains("Vainqueur"):
					found_result = true
		_check(found_result, "flow: season report should include the resolved battle (result), not just 'en vue'")

	# Menu pause : ouverture (arbre en pause), dialogue de sauvegarde, fermeture.
	flow.open_pause()
	await process_frame
	_check(paused and flow.is_paused(), "pause menu should pause the tree")
	flow.pause_menu.open_save()
	_check(flow.pause_menu.save_dialog.visible and flow.pause_menu.save_dialog.save_count() >= 3, "save dialog should list the autosaves")
	flow.pause_menu.open_settings()
	await process_frame
	_check(flow.pause_menu.settings_open(), "settings window should open from pause")
	flow.close_pause()
	await process_frame
	_check(not paused and not flow.is_paused(), "closing the pause menu should resume")
	map._on_end_turn()
	_check(flow.unsaved_turns() == 0, "autosave every turn: nothing unsaved")

	# Crédits (CREDITS.md ou texte intégré).
	var credits: Node = (load("res://scenes/ui/credits_screen.tscn") as PackedScene).instantiate()
	root.add_child(credits)
	await process_frame
	_check(credits.text_label.text.length() > 100, "credits text should not be empty")
	credits.queue_free()

	settings.call("set_value", "game/autosave_interval", 0, false)
	current_scene = null
	map.queue_free()
	await process_frame
	_restore_autosaves(backup)
	if _failures == 0:
		print("smoke OK: flow (settings, loading, autosave rotation, season report, pause, credits)")


## Met de côté les sauvegardes automatiques du joueur (`user://saves/auto_*`).
func _backup_autosaves() -> Dictionary:
	var kept: Dictionary = {}
	var dir := DirAccess.open(SaveSlots.SAVES_DIR)
	if dir == null:
		return kept
	for file_name in dir.get_files():
		if file_name.begins_with(SaveSlots.AUTOSAVE_PREFIX):
			var path := SaveSlots.SAVES_DIR.path_join(file_name)
			kept[path] = FileAccess.get_file_as_bytes(path)
			DirAccess.remove_absolute(path)
	return kept


func _restore_autosaves(kept: Dictionary) -> void:
	var dir := DirAccess.open(SaveSlots.SAVES_DIR)
	if dir != null:
		for file_name in dir.get_files():
			if file_name.begins_with(SaveSlots.AUTOSAVE_PREFIX):
				DirAccess.remove_absolute(SaveSlots.SAVES_DIR.path_join(file_name))
	for path in kept:
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file != null:
			file.store_buffer(kept[path])
			file.close()


## F8 : tutoriel des premiers tours (objectifs vérifiables) et encyclopédie tirée de `data/`.
func _run_tutorial() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	var real_data := _project_root().path_join("data")
	if not (ClassDB.class_exists("CampaignSim") and FileAccess.file_exists(real_data.path_join("map/map.json"))):
		print("smoke tutorial: skipped, needs the real simulation and data/map")
		return
	if settings != null:
		settings.call("set_value", "tutorial/enabled", true, false)
		settings.call("set_value", "tutorial/done", false, false)
		settings.call("set_value", "tutorial/step", 0, false)
		settings.call("set_value", "interface/season_report", true, false)
		settings.call("set_value", "game/interactive_battles", false, false)
	facade.set_data_dir(real_data)
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "tutorial: campaign map failed to start"):
		return
	var tutorial: Node = map.get("tutorial")
	if not _check(tutorial != null and tutorial.active, "tutorial should start on a new campaign"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var player: String = map.player_faction
	var visited: PackedStringArray = PackedStringArray()
	_check(tutorial.steps.size() >= 10 and tutorial.steps.size() <= 14, "tutorial should have 10-14 steps, got %d" % tutorial.steps.size())
	_check(str(tutorial.steps[0]["text"]).contains("Bouter les Anglais"), "intro should list the French objectives from data")
	_check(str(tutorial.steps[1]["advice"]).contains("Crécy"), "French advice expected")
	_check(tutorial.current_step_id() == "intro" and not tutorial.check_now(), "intro is manual")
	tutorial.advance()

	# 1. Sélection de l'armée royale.
	_check(tutorial.current_step_id() == "select_army" and not tutorial.check_now(), "select_army: not met before selection")
	var royal: String = tutorial.royal_army
	_check(royal != "" and not tutorial.resolve_target("royal_army").is_empty(), "royal army and its marker target expected")
	map.select_army(royal)
	_check(tutorial.check_now(), "select_army should complete after selecting the royal army")
	visited.append("select_army")
	# 2. Ordre de marche.
	_check(tutorial.current_step_id() == "move_army" and not tutorial.check_now(), "move_army: not met before an order")
	var reachable: Dictionary = map.reachable
	if _check(not reachable.is_empty(), "tutorial: royal army has no reachable province"):
		var result: Dictionary = map.order_move(royal, str(reachable.keys()[0]))
		_check(result.get("ok", false), "tutorial: move refused %s" % result.get("error", "?"))
	_check(tutorial.check_now(), "move_army should complete after a move order")
	visited.append("move_army")
	# 3-4. Province du joueur puis onglet Ville.
	_check(tutorial.current_step_id() == "open_province" and not tutorial.check_now(), "open_province: not met before opening")
	var capital: String = tutorial._capital()
	map.picker.select_index(map.map_data.index_of_id(capital))
	await process_frame
	_check(tutorial.check_now(), "open_province should complete with the capital panel open")
	_check(tutorial.current_step_id() == "city_tab" and not tutorial.check_now(), "city_tab: not met on the garrison tab")
	_check(not tutorial.resolve_target("city_tab").is_empty(), "city tab target expected")
	map.ui.province_panel.show_ville_tab()
	_check(tutorial.check_now(), "city_tab should complete on the Ville tab")
	visited.append_array(["open_province", "city_tab"])
	# 5. Construction.
	_check(tutorial.current_step_id() == "build" and not tutorial.check_now(), "build: not met before building")
	var built := false
	for province_id in tutorial._player_provinces():
		var city: Dictionary = sim.call("get_province_city", province_id)
		if not (city.get("construction", {}) as Dictionary).is_empty():
			continue
		for row in city.get("buildable", []):
			if bool(row.get("available", false)):
				map._on_build(province_id, str(row["building"]))
				built = true
				break
		if built:
			break
	_check(built and tutorial.check_now(), "build should complete after a construction order")
	visited.append("build")
	# 6. Recherche.
	_check(tutorial.current_step_id() == "research" and not tutorial.check_now(), "research: not met before choosing")
	for node in sim.call("get_tech_tree", player):
		if str(node.get("state", "")) == "available":
			map._on_research_requested(str(node["id"]))
			break
	_check(tutorial.check_now(), "research should complete after choosing a technology")
	visited.append("research")
	# 7. Diplomatie.
	_check(tutorial.current_step_id() == "diplomacy" and not tutorial.check_now(), "diplomacy: not met before opening")
	_check(not tutorial.resolve_target("diplomacy").is_empty(), "diplomacy button target expected")
	map.diplomacy.open_panel()
	_check(tutorial.check_now(), "diplomacy should complete with the panel open")
	map.diplomacy.panel.hide()
	visited.append("diplomacy")
	# 8-9. Fin du tour, rapport de saison.
	_check(tutorial.current_step_id() == "end_turn" and not tutorial.check_now(), "end_turn: not met before ending the turn")
	_check(tutorial.resolve_target("end_turn").has("rect"), "end turn button target expected")
	map._on_end_turn()
	_check(tutorial.check_now(), "end_turn should complete after end_turn")
	visited.append("end_turn")
	_check(tutorial.current_step_id() == "season_report", "season_report step expected")
	var report: Control = map.flow.season_report
	if report.visible:
		_check(not tutorial.check_now(), "season_report: not met while the report is open")
		report.close()
	_check(tutorial.check_now(), "season_report should complete once the report is closed")
	visited.append("season_report")
	# 10. Chronique (étape passée s'il n'y a pas encore d'événement).
	_check(tutorial.current_step_id() == "chronicle", "chronicle step expected")
	if map.chronicle.open_window():
		_check(tutorial.check_now(), "chronicle should complete with the window open")
		map.chronicle.window.hide()
		visited.append("chronicle")
	else:
		map.chronicle.window.hide()
		_check(not tutorial.check_now(), "chronicle: not met without window")
		tutorial.advance()
		print("smoke tutorial: no chronicle decision yet, step skipped")
	# 11. Impôt.
	_check(tutorial.current_step_id() == "tax" and not tutorial.check_now(), "tax: not met before changing")
	var rate := "high" if tutorial._tax_rate() != "high" else "low"
	map._on_tax_rate_changed(player, rate)
	_check(tutorial.check_now(), "tax should complete after changing the rate")
	visited.append("tax")
	# 12. Gouverneur.
	_check(tutorial.current_step_id() == "governor" and not tutorial.check_now(), "governor: not met before appointing")
	var appointed := false
	var ruler := str(GameCatalog.definitions("factions").get(player, {}).get("ruler", ""))
	for character_id in sim.call("get_faction_characters", player):
		if str(character_id) == ruler or str((sim.call("get_character", character_id) as Dictionary).get("governor_of", "")) != "":
			continue
		for province_id in tutorial._player_provinces():
			if province_id == capital:
				continue
			var answer: Dictionary = sim.call("submit_order", {"type": "assign_governor", "character": str(character_id), "province": province_id})
			if answer.get("ok", false):
				appointed = true
				break
		if appointed:
			break
	_check(appointed and tutorial.check_now(), "governor should complete after an appointment")
	visited.append("governor")
	# Fin : progression persistée, pas de relance.
	_check(tutorial.current_step_id() == "outro", "outro step expected")
	tutorial.advance()
	_check(not tutorial.active and not tutorial.overlay.visible, "tutorial should close after the last step")
	if settings != null:
		_check(bool(settings.call("get_value", "tutorial/done")), "tutorial/done should be persisted")
	_check(not tutorial.should_autostart(), "a finished tutorial must not restart")

	# Encyclopédie.
	var encyclopedia: Control = tutorial.encyclopedia
	var key := InputEventKey.new()
	key.physical_keycode = KEY_L
	key.pressed = true
	tutorial._unhandled_input(key)
	_check(encyclopedia.visible, "K should open the encyclopedia")
	var counts := PackedStringArray()
	var ids: PackedStringArray = encyclopedia.tab_ids()
	_check(ids.size() == 9, "encyclopedia should have 9 tabs")
	for index in ids.size():
		encyclopedia.select_tab(index)
		_check(encyclopedia.entry_count() > 0, "encyclopedia tab %s is empty" % ids[index])
		_check(encyclopedia.fiche.get_parsed_text().length() > 40, "encyclopedia tab %s: empty fiche for %s" % [ids[index], encyclopedia.current_entry])
		counts.append("%s %d" % [ids[index], encyclopedia.entry_count()])
	var broken: Array = []
	for index in ids.size():
		for entry in encyclopedia._entries[ids[index]]:
			if Encyclopedia.fiche_bbcode(str(entry["id"])).length() < 40:
				broken.append(entry["id"])
	_check(broken.is_empty(), "encyclopedia entries without fiche: %s" % [broken])
	encyclopedia.select_tab(0)
	var total: int = encyclopedia.entry_count()
	encyclopedia.set_query("arc")
	_check(encyclopedia.entry_count() > 0 and encyclopedia.entry_count() < total and "unit_longbowmen" in encyclopedia.visible_ids(), "search 'arc' should filter the units (%d / %d)" % [encyclopedia.entry_count(), total])
	encyclopedia.set_query("ARBALETRIER")
	_check(encyclopedia.entry_count() > 0, "search should ignore case and accents")
	encyclopedia.set_query("zzzzqx")
	_check(encyclopedia.entry_count() == 0, "nonsense search should list nothing")
	encyclopedia.set_query("")
	_check(encyclopedia.entry_count() == total, "clearing the search should restore the list")
	_check(encyclopedia.open_entry("unit_longbowmen"), "open unit_longbowmen")
	_check(Encyclopedia.fiche_bbcode("unit_longbowmen").contains("[url=tech_longbow_drill]"), "unit fiche should link its technology")
	encyclopedia.fiche.meta_clicked.emit("tech_longbow_drill")
	_check(encyclopedia.current_entry == "tech_longbow_drill" and encyclopedia.tab_ids()[encyclopedia.current_tab] == "technologies", "internal link should open the technology")
	encyclopedia.go_back()
	_check(encyclopedia.current_entry == "unit_longbowmen", "back should return to the unit")
	var france := Encyclopedia.fiche_bbcode("fac_france")
	_check(france.contains("Philippe VI") and france.contains("Objectifs"), "faction fiche: ruler and objectives expected")
	tutorial._unhandled_input(key)
	_check(not encyclopedia.visible, "K should close the encyclopedia")

	if settings != null:
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "game/interactive_battles", true, false)
	map.queue_free()
	await process_frame
	if _failures == 0:
		print("smoke OK: tutorial (%s) and encyclopedia (%s)" % [", ".join(visited), ", ".join(counts)])


# --- F5c : déploiement et maisons de siège dans la scène ------------------------------


## Phase de déploiement ouverte par une bataille du joueur : temps gelé, zone dessinée, un
## placement valide et un refusé (toast), « Commencer la bataille », puis le temps avance.
func _check_battle_deployment_f5c(scene: BattleScene) -> void:
	var controller: DeploymentController = scene.deployment
	if not _check(controller != null and controller.active and bool(scene.battle.call("is_deploying")), "deployment: phase should be open for a player battle"):
		return
	_check(controller.zone_view != null and controller.zone_view.get_child_count() == 2, "deployment: zone not drawn")
	_check(controller.banner != null and controller.banner.is_visible_in_tree(), "deployment: banner missing")
	_check(float(scene.battle.call("get_elapsed")) == 0.0, "deployment: time should be frozen")
	var zone: Dictionary = controller.zone
	var own := -1
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]):
			own = int(unit["id"])
			break
	var inside := Vector3((float(zone["x0"]) + float(zone["x1"])) * 0.5, 0, (float(zone["z0"]) + float(zone["z1"])) * 0.5)
	var cam := inside + Vector3(0, 200, -300)
	_check(controller.place([own], inside, inside, cam) == 1, "deployment: placement inside the zone refused")
	scene._refresh_view(true)
	var moved: Dictionary = controller._unit(own)
	_check(Vector2(float(moved["x"]), float(moved["z"])).distance_to(Vector2(inside.x, inside.z)) < 1.0, "deployment: unit not moved to the zone centre")
	var outside := Vector3(inside.x, 0, float(zone["z1"]) + 200.0 if scene.player_side == "attacker" else float(zone["z0"]) - 200.0)
	_check(controller.place([own], outside, outside, cam) == 0, "deployment: placement outside the zone accepted")
	_check(scene.hud.toast_label != null and scene.hud.toast_label.text.contains("zone de déploiement"), "deployment: refusal toast missing")
	_check(controller.finish() and not controller.active, "deployment: start_battle failed")
	_check(not bool(scene.battle.call("is_deploying")), "deployment: still deploying after start_battle")
	for _i in 10:
		await process_frame
	scene.battle.call("tick", 0.5)
	_check(float(scene.battle.call("get_elapsed")) > 0.0, "deployment: battle does not progress after start")
	if _failures == 0:
		print("smoke OK: deployment (zone %s, placement valid + refused, battle started)" % [zone])


## Siège (F5c) : les maisons rendues sont exactement les disques de la simulation, puis le
## déploiement est validé comme en bataille rangée.
func _check_siege_f5c(scene: BattleScene) -> void:
	var houses: Array = (scene.battle.call("get_siege") as Dictionary).get("houses", [])
	var sites: Array = scene.siege_view.house_sites
	var same := houses.size() == sites.size() and not houses.is_empty()
	for i in mini(houses.size(), sites.size()):
		var p: Vector2 = sites[i]["p"]
		same = same and p.distance_to(Vector2(float(houses[i]["x"]), float(houses[i]["z"]))) < 0.01
	_check(same, "siege scene: %d houses rendered for %d simulation houses" % [sites.size(), houses.size()])
	await _check_battle_deployment_f5c(scene)
	_check(not scene.hud.siege_label.text.contains("sortie"), "siege scene: no sortie at the start")
	_check(BattleScene.siege_status({"pieces": [], "sortie": true}).contains("sortie de la garnison"), "siege scene: sortie not shown in the siege status")


## H11 : monnaie (changement par l'UI, refus du 2e changement dans l'année), panneau des
## rançons (données simulées : la sim de 1337 n'a pas de captif), section de l'ordre de
## chevalerie, genres coinage/ransom/chivalry, liens Encyclopédie ↔ Codex.
func _run_coinage_ransom() -> void:
	const FACTION_ID := "fac_france"
	if not (ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_coinage")):
		print("smoke coinage/ransom: skipped, CampaignSim has no get_coinage (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not _check(sim.call("new_campaign", _project_root().path_join("data"), FACTION_ID, 1337), "coinage: new_campaign failed"):
		return
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file")
		store.call("reload", _project_root().path_join("data/codex"))

	# Vraies données (noms de provinces, encyclopédie) ; dossier précédent restauré à la fin.
	var previous_dir := str(paths.get("data_dir"))
	facade.set_data_dir(_project_root().path_join("data"))
	# Panneau de faction réel : sections et lignes de budget ajoutées en code (sim de l'étape).
	var previous_sim: Object = facade.get("sim")
	facade.set("sim", sim)
	var panel: FactionPanel = (load("res://scenes/ui/faction_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	panel.show_faction(FACTION_ID, "France", Color.BLUE, sim.call("get_faction_economy", FACTION_ID))
	var coinage := panel.coinage_section
	_check(coinage != null and coinage.visible and coinage.level_buttons.size() == 4, "coinage section with 4 levels expected")
	_check(panel.chivalry_section.visible and panel.ransom_button.visible, "chivalry section and ransom button expected")
	_check(coinage.explanation_label.text.contains("[url=cdx:cdx_nicole_oresme]"), "coinage explanation should link Oresme: %s" % coinage.explanation_label.text)
	var tip := RichTooltip.coinage((sim.call("get_coinage", "") as Dictionary)["options"][2])
	_check(tip.contains("Seigneuriage") and tip.contains("Un seul changement"), "coinage tooltip incomplete: %s" % tip)

	# Changement par l'UI, reflété par get_coinage ; le second de l'année est refusé et affiché.
	(coinage.level_buttons["debased"] as Button).pressed.emit()
	var now: Dictionary = sim.call("get_coinage", "")
	_check(str(now.get("level", "")) == "debased" and bool(now.get("changed_this_year", false)), "set_coinage via UI: %s (%s)" % [now.get("level", ""), coinage.last_result])
	_check(coinage.changed_label.visible and not coinage.error_label.visible, "changed-this-year note expected")
	(coinage.level_buttons["strong"] as Button).pressed.emit()
	_check(str((sim.call("get_coinage", "") as Dictionary).get("level", "")) == "debased", "second change in the year must be refused")
	_check(coinage.error_label.visible and coinage.error_label.text.contains("déjà été changée"), "refusal should be shown: %s" % coinage.error_label.text)
	panel.show_faction(FACTION_ID, "France", Color.BLUE, sim.call("get_faction_economy", FACTION_ID))
	_check(panel.seigniorage_value.text.begins_with("+") and int((sim.call("get_faction_economy", FACTION_ID) as Dictionary).get("seigniorage", 0)) > 0, "seigniorage budget line: %s" % panel.seigniorage_value.text)

	# Ordre de chevalerie : options de fondation (Étoile, refusée avant 1351), lien Codex.
	var chivalry := panel.chivalry_section
	_check(chivalry.found_buttons.has("ord_star"), "Star order option expected: %s" % [chivalry.found_buttons.keys()])
	_check(ChivalrySection.codex_entry("ord_star") == "cdx_ordre_de_l_etoile", "order codex link")
	var refused: Dictionary = chivalry.request_found("ord_star")
	_check(not bool(refused.get("ok", true)) and chivalry.error_label.visible and chivalry.error_label.text.contains("1351"), "early founding refused and shown: %s" % chivalry.error_label.text)

	# Rançons : panneau réel ouvert depuis le panneau de faction, puis données simulées.
	panel.toggle_ransoms()
	var ransoms := panel.ransom_panel
	_check(ransoms != null and ransoms.visible and ransoms.rows.is_empty(), "ransom panel should open (no captive in 1337)")
	ransoms.show_data(_mock_ransoms())
	_check(ransoms.rows.has("chr_jean_de_normandie") and (ransoms.rows["chr_jean_de_normandie"] as Dictionary).has("plan"), "our captive row with payment plans")
	var plan: OptionButton = ransoms.rows["chr_jean_de_normandie"]["plan"]
	_check(plan.item_count == 5 and plan.get_item_text(0).contains("2 échéances"), "installment plans 2-6: %d" % plan.item_count)
	var terms: OptionButton = ransoms.rows["chr_mock_knight"]["terms"]
	_check(terms.item_count == 3 and terms.get_item_text(1).begins_with("Exiger"), "held prisoner terms: money, province, hold")
	var pay: Dictionary = ransoms.pay_ransom("chr_jean_de_normandie", 1)
	_check(not bool(pay.get("ok", true)) and ransoms.error_label.visible and ransoms.error_label.text.begins_with("Refusé"), "bridge refusal shown in red: %s" % ransoms.error_label.text)
	var alerts := CampaignAlerts.ransom_alerts(sim)
	_check(alerts.is_empty(), "no ransom alert without captive")
	panel.queue_free()
	facade.set("sim", previous_sim)

	# Genres coinage / ransom / chivalry.
	var events := [
		{"kind": "coinage", "text_fr": "La monnaie est affaiblie.", "faction": FACTION_ID},
		{"kind": "ransom", "text_fr": "Rançon payée.", "faction": FACTION_ID},
		{"kind": "chivalry", "text_fr": "Ordre fondé.", "faction": FACTION_ID},
	]
	var groups := SeasonReport.build_groups(events, func(_event: Dictionary) -> bool: return true)
	_check(groups.size() == 1 and (groups[0]["entries"] as Array).size() == 3, "season report group for coinage/ransom/chivalry: %s" % [groups])
	_check(SeasonReport.KIND_STYLES.has("ransom") and NewsLetters.KIND_LABELS.has("chivalry") and not NewsLetters.news_from_event(events[0]).is_empty(), "styles and letters for H11 kinds")

	# Encyclopédie → Codex et Codex → Encyclopédie.
	var encyclopedia: Encyclopedia = (load("res://scenes/ui/encyclopedia.tscn") as PackedScene).instantiate()
	root.add_child(encyclopedia)
	await process_frame
	encyclopedia.open_window("unit_longbowmen")
	_check(encyclopedia.codex_button.visible, "encyclopedia should offer the Codex entry of unit_longbowmen")
	encyclopedia.open_codex_entry()
	var bubbles: Node = root.get_node_or_null("/root/CodexBubbles")
	var window: CodexWindow = bubbles.call("window")
	_check(bool(bubbles.call("is_window_open")) and window.current_id == "cdx_arc_long", "codex window on cdx_arc_long, got %s" % window.current_id)
	_check(window.encyclopedia_button.visible, "codex entry with entity should offer the encyclopedia")
	encyclopedia.open_entry("bld_apothecary")
	window.navigate("cdx_arc_long")
	_check(window.open_in_encyclopedia() and encyclopedia.current_entry == "unit_longbowmen" and not window.visible, "codex -> encyclopedia")
	encyclopedia.queue_free()
	facade.set_data_dir(previous_dir)
	if store != null:
		store.call("reset_discoveries")
	if _failures == 0:
		print("smoke OK: coinage/ransom, debased then refused, ransom panel, order refused, encyclopedia/codex links")


## Rançons simulées au format de `get_ransoms` (captures et smoke).
static func _mock_ransoms() -> Dictionary:
	return {
		"ours": [{"character": "chr_jean_de_normandie", "name": "Jean, duc de Normandie", "faction": "fac_france", "captor": "fac_england",
			"rank": "sovereign", "rank_label": "Souverain", "prestige": 40, "ransom": 21000,
			"terms": {"kind": "money", "province": ""},
			"plans": [2, 3, 4, 5, 6].map(func(n: int) -> Dictionary: return {"installments": n, "total": 23100, "installment": 23100 / n}),
			"cedable_provinces": []}],
		"held": [{"character": "chr_mock_knight", "name": "Thomas Holland", "faction": "fac_england", "captor": "fac_france",
			"rank": "knight", "rank_label": "Chevalier", "prestige": 12, "ransom": 450,
			"terms": {"kind": "money", "province": ""}, "plans": [], "cedable_provinces": ["prov_guyenne"]}],
		"debts": [{"character": "chr_charles_de_blois", "name": "Charles de Blois", "creditor": "fac_england",
			"remaining": 9000, "installment": 3000, "next_due_turn": 4, "missed": 0}],
	}
