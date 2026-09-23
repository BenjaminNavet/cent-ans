extends SceneTree

## Vérification headless de GameDataStore (GDExtension `core/crates/godot-bridge`).
## Usage : godot --headless --path game --script <chemin absolu>/core/checks/data_store_check.gd
## Le dossier `data/` est résolu relativement au projet Godot (`res://../data`).
## Code de sortie 0 si tout passe, 1 sinon.

var _failures: int = 0


func _init() -> void:
	_run()
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	if not ClassDB.class_exists("GameDataStore"):
		_fail("GameDataStore class not registered: GDExtension not loaded (run core/build.sh)")
		return

	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var store: GameDataStore = GameDataStore.new()
	_check(not store.is_loaded(), "store should not be loaded before load()")
	_check(store.load(data_dir), "load(%s) should return true" % data_dir)
	if _failures > 0:
		return

	var warnings := store.get_warnings()
	print("data store: %d warnings" % warnings.size())
	for warning in warnings:
		print("  warning: " + warning)

	var province_ids := store.get_province_ids()
	_check(province_ids.has("prov_normandie"), "province ids should contain prov_normandie")

	var normandie := store.get_province("prov_normandie")
	_check(normandie.get("owner", "") == "fac_france", "prov_normandie owner should be fac_france, got %s" % str(normandie.get("owner")))
	_check(normandie.get("display_name", "") == "Normandie (Rouen)", "display_name should be Normandie (Rouen)")
	_check(normandie.get("local_name", "") == "Normendie", "local_name should be Normendie")
	_check(normandie.get("owner_display_name", "") == "France", "owner_display_name should be France")
	_check(normandie.get("capital", "") == "Rouen", "capital should be Rouen")
	_check(normandie.get("terrain", "") == "plains", "terrain should be plains")
	_check(normandie.get("coastal", false) == true, "coastal should be true")
	_check(normandie.get("port", false) == true, "port should be true")
	var population: Dictionary = normandie.get("population", {})
	_check(population.get("peasants", 0) == 419120, "peasants should be 419120")
	_check(normandie.get("population_total", 0) == 520000, "population_total should be 520000")
	_check(store.get_province("prov_atlantis").is_empty(), "unknown province should give an empty dictionary")

	var faction_ids := store.get_faction_ids()
	_check(faction_ids.has("fac_france") and faction_ids.has("fac_england"), "faction ids should contain France and England")
	var france := store.get_faction("fac_france")
	_check(france.get("name", "") == "Royaume de France", "faction name should be Royaume de France")
	var color: Color = france.get("color", Color.MAGENTA)
	_check(color.is_equal_approx(Color("#1F3A93")), "France color should be #1F3A93, got %s" % str(color))
	_check(str(france.get("blazon", "")).begins_with("D'azur"), "France blazon should start with D'azur")

	var character_ids := store.get_character_ids()
	_check(character_ids.has("chr_philippe_vi"), "character ids should contain chr_philippe_vi")
	var philippe := store.get_character("chr_philippe_vi")
	_check(philippe.get("faction", "") == "fac_france", "Philippe VI faction should be fac_france")
	var birth: Dictionary = philippe.get("birth", {})
	_check(birth.get("year", 0) == 1293 and birth.get("uncertain", false) == true, "Philippe VI birth should be 1293 (uncertain)")
	var death: Dictionary = philippe.get("death", {})
	_check(death.get("value", "") == "1350-08-22", "Philippe VI death should be 1350-08-22")
	var skills: Dictionary = philippe.get("skills", {})
	_check(skills.get("command", -1) == 5, "Philippe VI command should be 5")
	var titles: PackedStringArray = philippe.get("titles", PackedStringArray())
	_check(titles.has("Roi de France"), "Philippe VI titles should contain Roi de France")

	if _failures == 0:
		print("data store OK: %d provinces, %d factions, %d characters" % [province_ids.size(), faction_ids.size(), character_ids.size()])


func _check(condition: bool, message: String) -> void:
	if not condition:
		_fail(message)


func _fail(message: String) -> void:
	_failures += 1
	push_error("data store FAIL: " + message)
	printerr("data store FAIL: " + message)
