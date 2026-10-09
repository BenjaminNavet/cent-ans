extends TestCase

## Lot OMR-R2 : `ReliefFloor.compute` lit la heightmap 16 bits en valeurs brutes (minimum et maximum
## convertis une fois par cellule) : grilles identiques au chemin générique (`generic_path`) sur la
## vraie carte ; temps des deux chemins.
## Usage : godot --headless --path game --script res://tests/r2_relief_floor_test.gd



func _init() -> void:
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if map_data.load_error != "":
		print("R2 fond: carte illisible (%s)" % map_data.load_error)
		failures += 1
		finish()
		return
	var profile := ReliefExaggerationProfile.load_default()
	ReliefFloor.generic_path = false
	var fast := ReliefFloor.compute(map_data, profile)
	ReliefFloor.generic_path = true
	var generic := ReliefFloor.compute(map_data, profile)
	ReliefFloor.generic_path = false
	check(map_data.height_bpp == 2 and map_data.height_little_endian, "R2 fond: heightmap non 16 bits little-endian, chemin rapide non exercé")
	for key in ["data", "base", "squash", "amplitude"]:
		if fast[key] != generic[key]:
			check(false, "R2 fond: grille %s différente" % key)
	finish()
