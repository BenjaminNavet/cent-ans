extends TestCase

## Lot OM3 (ADR 0116) : la steppe et le désert réutilisent le sol de plaine avec leur teinte
## (`terrain_tints` de `data/fx/battle_ground_layers.json`) ; libellés français des terrains.
## Usage : godot --headless --path game --script res://tests/om3_terrain_test.gd


func _tint_of(key: String) -> Color:
	var world := Node3D.new()
	root.add_child(world)
	var terrain := BattleTerrain.new()
	world.add_child(terrain)
	var nx := 31
	var nz := 21
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	terrain.build({"nx": nx, "nz": nz, "resolution": 10.0, "heights": heights, "terrain": key, "season": "spring", "ground": "dry", "woodland": 0.0}, "clear")
	var tint: Variant = terrain.ground_material.get_shader_parameter("grass_tint")
	world.queue_free()
	return tint if tint is Color else Color(0.9, 1.0, 0.8)


func _init() -> void:
	if BattleTerrain.terrain_tint("plains") != Color(1, 1, 1):
		check(false, "OM3: la plaine ne doit pas être teintée")
	for key in ["steppe", "desert"]:
		if BattleTerrain.terrain_tint(key) == Color(1, 1, 1):
			check(false, "OM3: teinte absente pour %s" % key)
	await process_frame
	var plains := _tint_of("plains")
	var desert := _tint_of("desert")
	print("OM3 herbe plaine %s, désert %s" % [plains, desert])
	if not (desert.r > plains.r and desert.b < plains.b):
		check(false, "OM3: le désert doit tirer vers le sable")
	var panel_labels: Dictionary = load("res://scripts/map/province_panel.gd").TERRAIN_LABELS
	var tooltip_labels: Dictionary = load("res://scripts/ui/rich_tooltip.gd").TERRAIN_LABELS
	if panel_labels.get("desert") != "Désert" or panel_labels.get("steppe") != "Steppe" or tooltip_labels.get("desert") != "désert":
		check(false, "OM3: libellés français manquants")
	finish()
