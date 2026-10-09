extends TestCase

## Lot FK2 : assets de la carte vivante. Vérifie les accessoires `assets/models/folk/` (chaque
## modèle du manifeste se charge, porte un maillage, rôle et créneaux lisibles), les clips de
## travail `scythe` / `carry` / `plough` du rig fin (table des clips sous `MAX_CLIPS`), les
## figurines civiles `villager_0..3` (maillages, styles `folk` / `folk_carry` / `militia`) et les
## jeux de clips des états de travail.
## Usage : godot --headless --path game --script res://tests/fk2_assets_test.gd [-- --coarse-figures]

const FOLK_DIR := "res://assets/models/folk/"
const PROPS := ["merchant_cart", "dead_cart", "stone_cart", "market_stall", "pyre", "pitchfork", "torch", "sheep", "cow", "ox", "horse", "procession_cross", "procession_banner", "scaffold", "plough"]
const WORK_CLIPS := ["scythe", "carry", "plough"]
const VILLAGERS := 4


func _mesh_count(node: Node) -> int:
	var count := 1 if node is MeshInstance3D and (node as MeshInstance3D).mesh != null else 0
	for child in node.get_children():
		count += _mesh_count(child)
	return count


func _init() -> void:
	# Accessoires.
	var text := FileAccess.get_file_as_string(FOLK_DIR + "manifest.json")
	var parsed = JSON.parse_string(text) if text != "" else null
	check(parsed is Dictionary, "manifeste des accessoires illisible")
	var models: Dictionary = (parsed as Dictionary).get("models", {}) if parsed is Dictionary else {}
	for name in PROPS:
		check(models.has(name), "accessoire %s absent du manifeste" % name)
		var entry: Dictionary = models.get(name, {})
		var scene := load(FOLK_DIR + str(entry.get("file", name + ".glb"))) as PackedScene
		check(scene != null, "accessoire %s : glb non importé" % name)
		if scene != null:
			var node := scene.instantiate()
			check(_mesh_count(node) >= 1, "accessoire %s sans maillage" % name)
			node.free()
		check(int(entry.get("tris", 0)) > 0 and int(entry.get("tris", 0)) < 4000, "accessoire %s : %d triangles" % [name, int(entry.get("tris", 0))])
		check(str(entry.get("role", "")) != "", "accessoire %s sans rôle" % name)
	check((models.get("dead_cart", {}).get("slots", {}) as Dictionary).has("porter_l"), "charrette des morts sans créneau de porteur")
	# Clips de travail et figurines civiles.
	var fine := BattleSkinned.fine_enabled()
	for v in VILLAGERS:
		if not BattleSkinned.has_figure("villager", v):
			check(not fine, "figurine villager_%d absente" % v)
			continue
		check(BattleSkinned.mesh("villager", v, 1) != null, "villager_%d : LOD1 illisible" % v)
		var clips: Dictionary = BattleSkinned.rig("villager", v).get("clips", {})
		check(clips.size() <= BattleSkinned.MAX_CLIPS, "villager_%d : %d clips > MAX_CLIPS" % [v, clips.size()])
		for c in WORK_CLIPS:
			check(not fine or clips.has(c), "villager_%d : clip %s absent" % [v, c])
	if fine:
		check(BattleSkinned.style_of("villager", 0) == "folk", "style de villager_0")
		check(BattleSkinned.style_of("villager", 2) == "militia", "style de villager_2")
		check(BattleSkinned.style_of("villager", 3) == "folk_carry", "style de villager_3")
		for state in WORK_CLIPS:
			var names: Array = BattleSkinned.state_config("villager", 1, state, false)["names"]
			check(names == [state], "état %s : jeu %s" % [state, names])
		check((BattleSkinned.state_config("villager", 3, "marching", false)["names"] as Array) == ["carry"], "porteur en marche sans carry")
		check(is_equal_approx(BattleSkinned.clip_seconds("villager", 1, "scythe"), 41.0 / 24.0), "durée de scythe")
	finish()
