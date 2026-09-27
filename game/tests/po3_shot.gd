extends SceneTree

## Lot PO3 (carte de campagne) : trois vues de la carte autour de Paris à 1280×720 — large,
## régionale, rapprochée — écrites dans `<out>/po3-<vue>-<saison>.jpg` (défaut `docs/img/po/po3/`).
## Chaque vue tourne dans son propre processus Godot fenêtré (le rendu headless ne produit pas
## d'image) avec les options de capture de la carte (`--focus`, `--season`, `--screenshot`).
## Usage :
##   godot --path game --script res://tests/po3_shot.gd -- [--out=<dossier>] [--season=summer,autumn]
## `--out` relatif : relatif à la racine du dépôt. Les PNG sont convertis en JPEG (qualité 85).

const RESOLUTION := "1280x720"
const MAP := "res://scenes/campaign_map.tscn"
## Paris (px carte, cf. `pb1_bench.gd`).
const PARIS := Vector2(2213, 1924)

## [nom, distance caméra].
const VIEWS := [
	["large", 1250.0],
	["regionale", 491.0],
	["rapprochee", 150.0],
]


func _init() -> void:
	var out := "docs/img/po/po3"
	var seasons := PackedStringArray(["summer"])
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--season="):
			seasons = arg.trim_prefix("--season=").split(",", false)
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	DirAccess.make_dir_recursive_absolute(out_dir)
	var failed := 0
	for season in seasons:
		for entry in VIEWS:
			var png := out_dir.path_join("po3-%s-%s.png" % [entry[0], season])
			DirAccess.remove_absolute(png)
			var focus := "--focus=%d,%d,%d" % [int(PARIS.x), int(PARIS.y), int(entry[1])]
			var args := PackedStringArray([
				"--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION, MAP,
				"--", focus, "--season=" + season, "--screenshot=" + png,
			])
			var started := Time.get_ticks_msec()
			var output := []
			var code := OS.execute(OS.get_executable_path(), args, output, true)
			var ok := code == 0 and FileAccess.file_exists(png) and _to_jpg(png)
			if ok:
				print("PO3_SHOT %s OK (%.1f s)" % [png.get_basename() + ".jpg", (Time.get_ticks_msec() - started) / 1000.0])
			else:
				failed += 1
				print("PO3_SHOT %s FAIL (code %d)\n%s" % [png, code, "".join(output).right(1500)])
	print("PO3_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


func _to_jpg(png: String) -> bool:
	var image := Image.load_from_file(png)
	if image == null or image.is_empty():
		return false
	var err := image.save_jpg(png.get_basename() + ".jpg", 0.85)
	DirAccess.remove_absolute(png)
	return err == OK
