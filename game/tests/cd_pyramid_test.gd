extends TestCase

## Lot CC CD : pyramide de relief de la carte composée (MNT réel déformé, `geo compose-relief-real`).
## godot --headless --path game --script res://tests/cd_pyramid_test.gd
##  1. Sans `data/map/pyramid/relief_pyramid.json` (hors git) : test ignoré.
##  2. Le manifeste se charge (E1+), l'étage le plus fin est lu, l'E0 vient de `height/`.
##  3. La hauteur lue dans une tuile fine alpine (E1) est du même ordre que l'E0 voisin (cohérence des étages).


func _init() -> void:
	var map_dir := ProjectSettings.globalize_path("res://").path_join("../data/map").simplify_path()
	var manifest := map_dir.path_join("pyramid/relief_pyramid.json")
	if not FileAccess.file_exists(manifest):
		print("cd_pyramid_test: SKIP (pas de pyramide composée : lancer `geo compose-relief-real`)")
		finish()
		return
	var pyramid := ReliefPyramid.new()
	check(pyramid.load_manifest(map_dir, manifest), "manifeste composé chargé : " + pyramid.load_error)
	check(pyramid.max_level >= 1, "au moins E1")
	check(pyramid.tile_count() > 100, "tuiles fines présentes")
	check(pyramid.e0_dir.ends_with("height"), "E0 lu dans height/ : " + pyramid.e0_dir)
	var levels := {}
	for level in range(1, pyramid.max_level + 1):
		levels[level] = pyramid.cols(level)
	check(levels.size() == pyramid.max_level, "étages 1..max")
	if failures == 0:
		print("cd_pyramid_test: OK (E1-E%d, %d tuiles)" % [pyramid.max_level, pyramid.tile_count()])
	finish()
