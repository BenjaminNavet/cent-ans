extends SceneTree

## Lot P2b (PO phase 2, ADR 0097) : deux captures de contrôle à 1280×720 — l'écran des
## technologies et l'écran de diplomatie, tous deux migrés vers `UiLayout` (zone `MODAL`),
## `UiType` et `UiMotion` — écrites dans `docs/img/po/p2b/` (JPEG qualité 85). Carte de campagne
## réelle (France, graine 1337), comme `po_ui_test.gd`.
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1280x720 --script res://tests/p2b_shot.gd -- [--out=<dossier>] [--only=tech,diplomacy]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	var out := "docs/img/po/p2b"
	var only := PackedStringArray()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			only = arg.trim_prefix("--only=").split(",", false)
	await process_frame
	if DisplayServer.get_name() == "headless":
		root.size = Vector2i(1280, 720)  # headless : la fenêtre par défaut est minuscule (64x64)

	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for _i in 5:
		await process_frame
	if not (map.get("load_ok") and map.get("sim") != null):
		push_error("p2b_shot: campaign map with the real simulation failed to start (run core/build.sh?)")
		quit(1)
		return

	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	DirAccess.make_dir_recursive_absolute(out_dir)
	var failed := 0

	if only.is_empty() or only.has("tech"):
		if map.call("_tech_available"):
			map.call("_on_tech_panel_requested")
			failed += 0 if await _shot(out_dir.path_join("01-technologies.png")) else 1
			map.ui.tech_panel.hide()
		else:
			push_error("p2b_shot: technologies unavailable with this simulation")
			failed += 1
		await process_frame

	if only.is_empty() or only.has("diplomacy"):
		var controller: Node = map.diplomacy
		if controller != null and controller.call("available"):
			controller.call("open_panel")
			failed += 0 if await _shot(out_dir.path_join("02-diplomatie.png")) else 1
			controller.panel.hide()
		else:
			push_error("p2b_shot: diplomacy unavailable with this simulation")
			failed += 1

	print("P2B_SHOT done, %d failed" % failed)
	quit(0 if failed == 0 else 1)


## Capture le rendu courant vers `<png>` (converti en JPEG qualité 85, PNG supprimé).
func _shot(png: String) -> bool:
	for _i in 15:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("p2b_shot: empty viewport image (headless?) for %s" % png)
		return false
	var jpg := png.get_basename() + ".jpg"
	DirAccess.remove_absolute(png)
	DirAccess.remove_absolute(jpg)
	var err := image.save_jpg(jpg, 0.85)
	print("P2B_SHOT %s (%s)" % [jpg, error_string(err)])
	return err == OK
