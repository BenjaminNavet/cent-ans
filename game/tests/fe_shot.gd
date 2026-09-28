extends SceneTree

## Lot FE6 (féodalité, spec FE § 6) : quatre vues à 1280×720 dans `docs/img/fe6/` (JPEG qualité 85) —
## choix de faction sur la carte (fiche de Foix-Béarn au survol), filtre « Féodalité » (hachures
## des grands vassaux, écu parti de la Guyenne), panneau « Arbre féodal » ouvert sur la France avec
## un vassal sélectionné, confirmation de guerre contre Albret avec « Qui peut entrer en guerre ».
## Chaque vue tourne dans son propre processus Godot fenêtré (le rendu headless ne produit pas
## d'image), comme `p2c_shot.gd` ; les mises en scène sont `start_menu.gd --menu-stage=faction_map`
## et `campaign_map.gd --stage=feudal_*`.
## Usage : godot --path game --script res://tests/fe_shot.gd -- [--out=<dossier>] [--only=<n,n>]

const RESOLUTION := "1280x720"
const MAP := "res://scenes/campaign_map.tscn"
const MENU := "res://scenes/start_menu.tscn"

## [numéro, nom, scène, options après `--`].
const VIEWS := [
	[1, "choix-faction-carte", MENU, ["--menu-stage=faction_map"]],
	[2, "filtre-feodalite", MAP, ["--stage=feudal_map"]],
	[3, "arbre-feodal", MAP, ["--stage=feudal_tree"]],
	[4, "qui-peut-entrer-en-guerre", MAP, ["--stage=feudal_war"]],
]


func _init() -> void:
	var out := "docs/img/fe6"
	var only := PackedInt32Array()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			for part in arg.trim_prefix("--only=").split(",", false):
				only.append(int(part))
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	DirAccess.make_dir_recursive_absolute(out_dir)
	var failed := 0
	for entry in VIEWS:
		if not only.is_empty() and not only.has(int(entry[0])):
			continue
		var png := out_dir.path_join("%02d-%s.png" % [entry[0], entry[1]])
		DirAccess.remove_absolute(png.get_basename() + ".jpg")
		DirAccess.remove_absolute(png)
		var args := PackedStringArray(["--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION, str(entry[2]), "--"])
		args.append_array(PackedStringArray(entry[3]))
		args.append("--screenshot=" + png)
		var started := Time.get_ticks_msec()
		var output := []
		var code := OS.execute(OS.get_executable_path(), args, output, true)
		var ok := code == 0 and FileAccess.file_exists(png) and _to_jpg(png)
		if ok:
			print("FE_SHOT %s OK (%.1f s)" % [png.get_basename() + ".jpg", (Time.get_ticks_msec() - started) / 1000.0])
		else:
			failed += 1
			print("FE_SHOT %s FAIL (code %d)\n%s" % [png, code, "".join(output).right(1500)])
	print("FE_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


func _to_jpg(png: String) -> bool:
	var image := Image.load_from_file(png)
	if image == null or image.is_empty():
		return false
	var err := image.save_jpg(png.get_basename() + ".jpg", 0.85)
	DirAccess.remove_absolute(png)
	return err == OK
