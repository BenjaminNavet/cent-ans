extends SceneTree

## Galerie du README : refait les six images de `docs/img/readme/` (JPEG 1600 px, qualité 86).
## Chaque vue tourne dans son propre processus Godot fenêtré (le rendu headless ne produit pas
## d'image), avec les options de capture déjà en place : bataille (`--screenshot`, `--closeup`,
## `--historical`), carte (`--stage`, `--focus`), villes (`readme_shots.gd`), menu (`mm1_capture.gd`).
## Usage : godot --path game --script res://tests/readme_gallery.gd -- [--out=<dossier>] [--only=<nom,nom>]
## `--out` relatif : relatif à la racine du dépôt (défaut `docs/img/readme`).

const RESOLUTION := "1600x900"
const WIDTH := 1600
const BATTLE := "res://scenes/battle/battle.tscn"
const MAP := "res://scenes/campaign_map.tscn"

## Nord de la France et Manche (px carte), distance de la vue régionale de `po3_shot.gd`.
const CAMPAIGN_FOCUS := "--focus=2180,3050,491"


func _init() -> void:
	var out := "docs/img/readme"
	var only := PackedStringArray()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			only = arg.trim_prefix("--only=").split(",", false)
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	DirAccess.make_dir_recursive_absolute(out_dir)
	# [nom, scène ou script, options après `--` (`%s` = fichier écrit par le processus), fichier écrit].
	var views := [
		# Mêlée engagée depuis une demi-minute (premier choc vers 142 s), sans interface.
		["bataille", BATTLE, ["--closeup", "--closeup-distance=14", "--shot-at=170", "--no-hud", "--screenshot=%s"], "bataille.png"],
		["bataille_poitiers", BATTLE, ["--historical=poitiers", "--screenshot=%s"], "bataille_poitiers.png"],
		# `--focus` après `--screenshot` : la mise en scène de capture recadre sinon la caméra.
		["campagne", MAP, ["--stage=map", "--season=summer", "--screenshot=%s", CAMPAIGN_FOCUS], "campagne.png"],
		["paris", "--script=res://tests/readme_shots.gd", ["--only=paris", "--out=" + out_dir], "paris.jpg"],
		["londres", "--script=res://tests/readme_shots.gd", ["--only=londres", "--out=" + out_dir], "londres.jpg"],
		["menu", "--script=res://tests/mm1_capture.gd", ["--scene=menu", "--wait=6", "--out=%s"], "menu.png"],
	]
	var failed := 0
	for view: Array in views:
		var name: String = view[0]
		if not only.is_empty() and not only.has(name):
			continue
		var written := out_dir.path_join(view[3])
		var args := PackedStringArray(["--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION])
		var target: String = view[1]
		if target.begins_with("--script="):
			args.append_array(PackedStringArray(["--script", target.trim_prefix("--script=")]))
		else:
			args.append(target)
		args.append("--")
		for option: String in view[2]:
			args.append(option % written if option.contains("%s") else option)
		# Le vrai curseur, s'il tombe sur la fenêtre, ouvre une infobulle de province : coin de l'écran.
		if OS.get_name() == "macOS":
			OS.execute("osascript", ["-l", "JavaScript", "-e", "ObjC.import('CoreGraphics'); $.CGWarpMouseCursorPosition({x: 2, y: 2})"])
		var before := FileAccess.get_modified_time(written)
		var started := Time.get_ticks_msec()
		var output := []
		var code := OS.execute(OS.get_executable_path(), args, output, true)
		var ok := code == 0 and FileAccess.get_modified_time(written) > before and _to_jpg(written)
		if ok:
			print("README_SHOT %s OK (%.1f s)" % [name, (Time.get_ticks_msec() - started) / 1000.0])
		else:
			failed += 1
			print("README_SHOT %s FAIL (code %d)\n%s" % [name, code, "".join(output).right(1500)])
	print("README_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


## Ramène l'image à `WIDTH` px de large et l'écrit en JPEG (le PNG intermédiaire est supprimé).
func _to_jpg(path: String) -> bool:
	var image := Image.load_from_file(path)
	if image == null or image.is_empty():
		return false
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_jpg(path.get_basename() + ".jpg", 0.86)
	if path.get_extension() == "png":
		DirAccess.remove_absolute(path)
	return err == OK
