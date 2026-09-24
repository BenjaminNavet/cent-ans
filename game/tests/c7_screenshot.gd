extends SceneTree

## Captures C7 : fiche d'Édouard III avec sa suite (vignettes), rangée à droite de la Cour
## (repli en une colonne à 1440 px), puis arbre des Valois vers 1351 avec les dates de vie des
## défunts (« 1293–1350 ») et la fiche d'un défunt à côté de l'arbre rétréci.
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/c7_screenshot.gd
## Écrit `docs/img/c7/fiche-suite-edouard-iii.png` et `docs/img/c7/arbre-dates-valois.png`.

const TURNS := 56  # hiver 1350-1351 : Philippe VI est mort (1350)
const VIEW := Vector2i(1440, 900)


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img/c7").simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file")
	root.size = VIEW
	var facade: Node = root.get_node("/root/SimFacade")

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var layer := Control.new()
	layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(layer)
	var court: Node = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	layer.add_child(court)
	var sheet: Node = (load("res://scenes/ui/character_sheet.tscn") as PackedScene).instantiate()
	layer.add_child(sheet)
	await process_frame

	# 1. Angleterre : Édouard III et sa suite, fiche à droite de la liste de la Cour.
	var england: Object = ClassDB.instantiate("CampaignSim")
	england.call("new_campaign", data_dir, "fac_england", 1337)
	facade.set("sim", england)
	var edward := "chr_edward_iii"
	for companion in ["ret_ecuyer", "ret_heraut", "ret_vintenier", "ret_barbier_chirurgien", "ret_menestrel", "ret_banquier_lombard"]:
		print("c7 grant %s: %s" % [companion, england.call("submit_order", {"type": "debug_grant_companion", "character": edward, "companion": companion})])
	court.show_court(_rows(england, "fac_england"), "Angleterre", facade.call("faction_color", "fac_england"))
	_show_sheet(sheet, england, edward)
	await _layout(court, sheet)
	for _i in 8:
		await process_frame
	print("c7 sheet: compact=%s width=%.0f court_right=%.0f" % [sheet.compact, sheet.size.x, court.get_global_rect().end.x])
	_save(out_dir.path_join("fiche-suite-edouard-iii.png"))

	# 2. France vers 1351 : arbre des Valois (dates des défunts) et fiche de Philippe VI.
	var france: Object = ClassDB.instantiate("CampaignSim")
	france.call("new_campaign", data_dir, "fac_france", 1337)
	for _turn in TURNS:
		france.call("end_turn")
	facade.set("sim", france)
	court.sim_source = france
	court.tree_root_id = ""
	court.show_court(_rows(france, "fac_france"), "France", facade.call("faction_color", "fac_france"))
	court.show_tab(1)  # CourtPanel.TAB_TREE
	court.recenter_tree("chr_philippe_vi")
	_show_sheet(sheet, france, "chr_philippe_vi")
	await _layout(court, sheet)
	for _i in 8:
		await process_frame
	var philippe: Dictionary = france.call("get_character", "chr_philippe_vi")
	print("c7 philippe vi: alive=%s %s-%s" % [philippe.get("alive"), philippe.get("birth_year"), philippe.get("death_year")])
	_save(out_dir.path_join("arbre-dates-valois.png"))
	if store != null:
		store.call("reset_discoveries")
	quit(0)


func _rows(sim: Object, faction: String) -> Array[Dictionary]:
	var rows: Array[Dictionary] = []
	for id in sim.call("get_faction_characters", faction):
		rows.append(sim.call("get_character", id))
	return rows


func _show_sheet(sheet: Node, sim: Object, id: String) -> void:
	sheet.show_character(sim.call("get_character", id), sim.call("get_skill_tree"), sim.call("get_learnable", id), [], [], [])


## Même placement que `MapUI.layout_hud` (C7).
func _layout(court: Node, sheet: Node) -> void:
	var width := float(VIEW.x)
	court.set_max_right(width - 600.0 - 32.0)  # CharacterSheet.COMPACT_WIDTH + marges (pas de référence de classe : script compilé avant les autoloads)
	for _i in 3:
		await process_frame
	sheet.fit_beside(court.get_global_rect().end.x, width)
	for _i in 6:
		await process_frame
	# Suite visible en haut de la colonne repliée.
	(sheet.get_node("VBox/Body/Scroll") as ScrollContainer).ensure_control_visible(sheet.retinue_row)


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("c7 screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
