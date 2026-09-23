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
