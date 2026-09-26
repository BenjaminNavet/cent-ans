extends SceneTree

## Capture DA7c : fiche d'Édouard III (3 traits de départ), pastilles à l'encre propres à
## chaque trait au lieu de l'icône de catégorie générique. Usage (avec affichage) :
##   godot --path game --script res://tests/da7c_traits_shot.gd
## Écrit `docs/img/da7c/fiche_traits.png`.


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img/da7c").simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	root.size = Vector2i(900, 900)

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var layer := Control.new()
	layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(layer)

	var england: Object = ClassDB.instantiate("CampaignSim")
	england.call("new_campaign", data_dir, "fac_england", 1337)
	var edward := "chr_edward_iii"
	var character: Dictionary = england.call("get_character", edward)
	var skill_tree: Array = england.call("get_skill_tree")
	var learnable: Array = england.call("get_learnable", edward)

	var sheet: Node = (load("res://scenes/ui/character_sheet.tscn") as PackedScene).instantiate()
	layer.add_child(sheet)
	await process_frame
	sheet.show_character(character, skill_tree, learnable, [], [], [])
	for _i in 8:
		await process_frame
	print("da7c traits: %s" % [character.get("traits", [])])
	var image := root.get_texture().get_image()
	var error := image.save_png(out_dir.path_join("fiche_traits.png"))
	print("da7c screenshot: %s" % ("ok" if error == OK else "error %d" % error))
	quit(0)
