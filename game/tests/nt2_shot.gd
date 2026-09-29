extends SceneTree

## Capture du lot NT2 : écran « Bataille personnalisée » ouvert depuis le menu principal, avec une
## composition d'exemple (France contre Angleterre). Fenêtre obligatoire (pas de --headless).
## Usage : godot --path game --script res://tests/nt2_shot.gd -- [dossier de sortie]
## (défaut : docs/audit/captures/nt/). N'écrit que le fichier ; l'image n'est pas lue ici.

const VIEW := Vector2i(1600, 900)


func _init() -> void:
	await process_frame
	var args := OS.get_cmdline_user_args()
	var folder: String = args[0] if not args.is_empty() else ProjectSettings.globalize_path("res://").path_join("../docs/audit/captures/nt").simplify_path()
	DirAccess.make_dir_recursive_absolute(folder)
	root.size = VIEW
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "custom_battle/last", JSON.stringify({
			"attacker": {"faction": "fac_france", "budget": 6000, "units": ["unit_crossbowmen", "unit_crossbowmen"]},
			"defender": {"faction": "fac_england", "budget": 6000, "units": ["unit_crossbowmen"]},
			"terrain": "hills", "season": "autumn", "weather": "", "hour": "", "siege": false,
			"fortification": 2, "player_side": "attacker",
		}), false)
	var menu: Control = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	for i in 10:
		await process_frame
	menu.call("open_custom_battle")
	for i in 30:
		await process_frame
	var path := folder.path_join("nt2-custom-battle.png")
	var image := root.get_texture().get_image()
	print("nt2_shot: %s → %s" % [path, error_string(image.save_png(path))])
	quit(0)
