extends SceneTree

## TX T3 : bataille de plaine vue de près (herbe du biome, écorce des arbres, matières des maisons).
## Usage : tools/godot_bg.sh --path game --resolution 1280x720 \
##   --script res://tests/tx_battle_grass_shot.gd -- --out=<fichier.png> --battle-biome=4 [--distance=14]
## Le biome de l'herbe vient de `--battle-biome` (4 = steppe, 3 = sec, 6 = alpin, 5 = arctique).


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tx_battle_grass.png")
	var distance := CmdArgs.number("--distance", 14.0)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
	BattleScene.custom_config = {
		"attacker": {"faction": "fac_france", "budget": 6000, "units": ["unit_crossbowmen", "unit_crossbowmen"]},
		"defender": {"faction": "fac_england", "budget": 6000, "units": ["unit_crossbowmen"]},
		"terrain": "plains", "season": "summer", "weather": "clear", "hour": "morning",
		"siege": false, "fortification": 0, "player_side": "", "seed": 7,
	}
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	root.add_child(scene)
	for _i in 30:
		await process_frame
	var rig: BattleCamera = scene.get("camera_rig")
	var terrain: BattleTerrain = scene.get("terrain")
	var center := Vector3(terrain.FIELD_W * 0.5, terrain.height_at(terrain.FIELD_W * 0.5, terrain.FIELD_D * 0.5), terrain.FIELD_D * 0.5)
	rig.look_at_point(center, distance, 0.0)
	for _i in 90:
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(out)
	print("tx_battle_grass_shot: ", out, " biome=", BattleGrassGroups.biome_for(""), " group=", BattleGrassGroups.group_of(BattleGrassGroups.biome_for("")))
	scene.queue_free()
	await process_frame
	quit(0)
