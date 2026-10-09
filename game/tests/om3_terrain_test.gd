extends TestCase

## Lot OM3 (ADR 0116), repris après ADR 0244 : la steppe et le désert ont leur propre biome de sol
## (paquets TX), plus de teinte Poly Haven ; libellés français des terrains.
## Usage : godot --headless --path game --script res://tests/om3_terrain_test.gd


func _init() -> void:
	# Steppe et désert pointent vers des biomes de sol propres (donc des paquets TX distincts).
	var by_terrain: Dictionary = BattleGroundTextures.spec().get("terrain_biomes", {})
	if int(by_terrain.get("steppe", 0)) == int(by_terrain.get("plains", 0)) or int(by_terrain.get("desert", 0)) == int(by_terrain.get("plains", 0)):
		check(false, "OM3: steppe et désert doivent avoir leur propre biome de sol")
	var probe := BattleTerrain.new()
	if probe.has_method("terrain_tint"):
		check(false, "OM3: la teinte Poly Haven doit avoir disparu")
	probe.free()
	await process_frame
	var panel_labels: Dictionary = load("res://scripts/map/province_panel.gd").TERRAIN_LABELS
	var tooltip_labels: Dictionary = load("res://scripts/ui/rich_tooltip.gd").TERRAIN_LABELS
	if panel_labels.get("desert") != "Désert" or panel_labels.get("steppe") != "Steppe" or tooltip_labels.get("desert") != "désert":
		check(false, "OM3: libellés français manquants")
	finish()
