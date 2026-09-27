extends SceneTree

## Lot PO4 (bataille) : trois vues à 1280×720 dans `docs/img/po/po4/` (JPEG qualité 85) :
## matin (déploiement, `--hour=morning`), soir (déploiement, `--hour=dusk`) et mêlée (heure par
## défaut). Chaque vue tourne dans son propre processus Godot fenêtré (le rendu headless ne
## produit pas d'image), comme `po_shot.gd`.
## Usage : godot --path game --script res://tests/po4_shot.gd -- [--out=<dossier>] [--only=<n,n>]

const RESOLUTION := "1280x720"
const BATTLE := "res://scenes/battle/battle.tscn"

## [numéro, nom, options après `--`].
const VIEWS := [
	[1, "matin", ["--deploy-shot", "--hour=morning"]],
	[2, "soir", ["--deploy-shot", "--hour=dusk"]],
	[3, "melee", []],
]


func _init() -> void:
	var out := "docs/img/po/po4"
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
		var args := PackedStringArray(["--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION, BATTLE, "--"])
		args.append_array(PackedStringArray(entry[2]))
		args.append("--screenshot=" + png)
		var started := Time.get_ticks_msec()
		var output := []
		var code := OS.execute(OS.get_executable_path(), args, output, true)
		var ok := code == 0 and FileAccess.file_exists(png) and _to_jpg(png)
		if ok:
			print("PO4_SHOT %s OK (%.1f s)" % [png.get_basename() + ".jpg", (Time.get_ticks_msec() - started) / 1000.0])
		else:
			failed += 1
			print("PO4_SHOT %s FAIL (code %d)\n%s" % [png, code, "".join(output).right(1500)])
	print("PO4_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


func _to_jpg(png: String) -> bool:
	var image := Image.load_from_file(png)
	if image == null or image.is_empty():
		return false
	var err := image.save_jpg(png.get_basename() + ".jpg", 0.85)
	DirAccess.remove_absolute(png)
	return err == OK
