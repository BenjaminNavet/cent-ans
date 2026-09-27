extends SceneTree

## Lot PO (polish) : planche de la tranche verticale, 10 vues à 1280×720, écrites dans
## `<out>/NN-<vue>.png` (défaut `docs/img/po/avant/`). Chaque vue tourne dans son propre processus
## Godot fenêtré (le rendu headless ne produit pas d'image) en réutilisant les options de capture
## des scènes (`--screenshot`, `--stage=…`, `--deploy-shot`, `--result-shot`). La rencontre n'a
## pas d'option de scène : ce script la met en scène lui-même (`--view=encounter`).
## Usage :
##   godot --path game --script res://tests/po_shot.gd -- [--out=<dossier>] [--only=<n,n>]
## `--out` relatif : relatif à la racine du dépôt. PO6 : `--out=docs/img/po/apres`.
## Chaque PNG est converti en JPEG (qualité 85) puis supprimé : la planche pèse ≈ 4 Mo.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const RESOLUTION := "1280x720"
const MAP := "res://scenes/campaign_map.tscn"
const BATTLE := "res://scenes/battle/battle.tscn"
const TIMEOUT_S := 240.0

## [numéro, nom, scène (vide = menu principal), options après `--`].
const VIEWS := [
	[1, "titre", "", []],
	[2, "faction", "", ["--menu-stage=faction"]],
	[3, "carte-large", MAP, ["--stage=next_hint"]],
	[4, "carte-regionale", MAP, ["--stage=province"]],
	[5, "armee-chemin", MAP, ["--stage=movement"]],
	[6, "fin-de-tour", MAP, ["--stage=turn_banner"]],
	[7, "rencontre", "", []],  # mise en scène par ce script (`--view=encounter`)
	[8, "bataille-deploiement", BATTLE, ["--deploy-shot"]],
	[9, "melee", BATTLE, []],
	[10, "resultat", BATTLE, ["--result-shot"]],
]


func _init() -> void:
	var out := "docs/img/po/avant"
	var only := PackedInt32Array()
	var view := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			for part in arg.trim_prefix("--only=").split(",", false):
				only.append(int(part))
		elif arg.begins_with("--view="):
			view = arg.trim_prefix("--view=")
	if view == "encounter":
		_encounter.call_deferred()
		return
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	DirAccess.make_dir_recursive_absolute(out_dir)
	var failed := 0
	for entry in VIEWS:
		if not only.is_empty() and not only.has(int(entry[0])):
			continue
		var png := out_dir.path_join("%02d-%s.png" % [entry[0], entry[1]])
		DirAccess.remove_absolute(png.get_basename() + ".jpg")
		var args := PackedStringArray(["--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION])
		if str(entry[1]) == "rencontre":
			args.append_array(["--script", "res://tests/po_shot.gd", "--", "--view=encounter", "--out=" + png])
		else:
			if str(entry[2]) != "":
				args.append(str(entry[2]))
			args.append("--")
			args.append_array(PackedStringArray(entry[3]))
			args.append("--screenshot=" + png)
		DirAccess.remove_absolute(png)
		var started := Time.get_ticks_msec()
		var output := []
		var code := OS.execute(OS.get_executable_path(), args, output, true)
		var ok := code == 0 and FileAccess.file_exists(png)
		if ok:
			ok = _to_jpg(png)
		if not ok:
			failed += 1
			var text := "".join(output)
			print("PO_SHOT %s FAIL (code %d)\n%s" % [png, code, text.right(1500)])
		else:
			print("PO_SHOT %s OK (%.1f s)" % [png.get_basename() + ".jpg", (Time.get_ticks_msec() - started) / 1000.0])
	print("PO_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


func _to_jpg(png: String) -> bool:
	var image := Image.load_from_file(png)
	if image == null or image.is_empty():
		return false
	var err := image.save_jpg(png.get_basename() + ".jpg", 0.85)
	DirAccess.remove_absolute(png)
	return err == OK


## Vue 7 : fenêtre de rencontre sur la carte (même mise en scène que `cv3_4_ui_shot.gd`).
func _encounter() -> void:
	var png := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			png = arg.trim_prefix("--out=")
	create_timer(TIMEOUT_S).timeout.connect(func() -> void:
		print("po_shot: timeout")
		quit(1))
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "interface/season_report", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load(MAP) as PackedScene).instantiate()
	root.add_child(map)
	current_scene = map
	for i in 20:
		await process_frame
	var sim: Object = map.sim
	var army_id: String = map.player_army_ids()[0]
	map.select_army(army_id)
	var army: Dictionary = sim.call("get_army", army_id)
	var start: Vector2 = army["position"]
	var spot := start
	var rect: Rect2 = map.movement_ctl.bubble.covered_rect()
	var radius := maxf(rect.size.x, rect.size.y) * 0.5
	for i in 24:
		var point := start + Vector2.RIGHT.rotated(TAU * float(i) / 24.0) * radius * 0.25
		var plan: Dictionary = sim.call("find_path_points", army_id, point.x, point.y)
		if plan.get("ok", false) and bool(plan.get("reachable_this_turn", false)):
			spot = point
			break
	var site := int(sim.call("debug_place_encounter", "enc_marchands_lombards", spot.x, spot.y))
	map.refresh_all()
	var middle: Vector3 = (map.armies.world_position_of(army_id) + map.armies.world_at_pixel(spot)) * 0.5
	map.camera_rig.look_at_point(middle, 70.0)
	map.camera_rig.snap()
	for i in 60:
		await process_frame
	map.encounters.site_clicked(site)
	for i in 40:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	var err := image.save_png(png)
	print("po_shot: %s (%s)" % [png, error_string(err)])
	quit(0 if err == OK else 1)
