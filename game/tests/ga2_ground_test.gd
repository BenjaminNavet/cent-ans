extends SceneTree

## Lot GA2 : mémoire du `Texture2DArray` du sol (13 couches, albédo 2k, normale 1k, BC7/ASTC +
## mipmaps) et vérification que les couches ajoutées par les données (`data/fx/
## battle_ground_layers.json`) sont bien câblées (rôles résolus, uniformes posés sur le
## matériau). Modèle : `game/tests/fg3_maps_test.gd`.
## Usage : godot --headless --path game --script res://tests/ga2_ground_test.gd


func _init() -> void:
	var ok := true
	var layers := BattleTerrain.ground_layers()
	print("GA2 couches de données : %d" % layers.size())
	if layers.size() != 13:
		print("GA2: 13 couches attendues (9 historiques + 4 GA2), trouvé %d" % layers.size())
		ok = false
	for role in ["flowering_meadow", "trodden_grass", "stubble", "fresh_plough"]:
		var idx := BattleTerrain.ground_role_index(role)
		print("GA2 rôle %s -> index %d" % [role, idx])
		if idx < 0:
			print("GA2: rôle %s absent des données" % role)
			ok = false

	await process_frame
	var world := Node3D.new()
	root.add_child(world)
	var terrain := BattleTerrain.new()
	world.add_child(terrain)
	var nx := 61
	var nz := 41
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	terrain.build({"nx": nx, "nz": nz, "resolution": 10.0, "heights": heights, "terrain": "plains", "season": "summer", "ground": "dry", "woodland": 0.0}, "clear")

	var total := 0
	for key in ["albedo_array", "normal_array"]:
		var tex: Variant = terrain.ground_material.get_shader_parameter(key)
		if tex == null or not (tex is TextureLayered):
			print("GA2 map %s: absente ou pas un Texture2DArray" % key)
			ok = false
			continue
		var arr := tex as TextureLayered
		var fmt := arr.get_format()
		var w := arr.get_width()
		var h := arr.get_height()
		var count := arr.get_layers()
		# Octets par texel selon le format VRAM réel (`compress/channel_pack=0` de l'import choisit
		# DXT1/BC1 pour ces couches sans alpha, 0,5 o/texel — pas BC7/BPTC comme la spec l'anticipait ;
		# mesuré ici plutôt que supposé, cf. note ADR 0105).
		var per_texel := 4.0
		if fmt == Image.FORMAT_BPTC_RGBA or fmt == Image.FORMAT_ASTC_4x4 or fmt == Image.FORMAT_DXT3 or fmt == Image.FORMAT_DXT5 or fmt == Image.FORMAT_RGTC_RG:
			per_texel = 1.0
		elif fmt == Image.FORMAT_DXT1 or fmt == Image.FORMAT_RGTC_R:
			per_texel = 0.5
		var bytes := int(w * h * count * per_texel * 4.0 / 3.0)
		total += bytes
		print("GA2 map %s: %dx%d x%d, format %d, %.2f Mo (mipmaps compris)" % [key, w, h, count, fmt, bytes / 1048576.0])
		if count != 13:
			print("GA2: %s attend 13 couches, trouvé %d" % [key, count])
			ok = false
	print("GA2 mémoire du sol : %.2f Mo" % (total / 1048576.0))
	if total > 120 * 1048576:
		print("GA2: mémoire du sol > 120 Mo (plafond de la spec)")
		ok = false

	var layer_count: Variant = terrain.ground_material.get_shader_parameter("layer_count")
	print("GA2 layer_count (uniforme) : %s" % layer_count)
	if int(layer_count) != 13:
		ok = false
	for role in ["flowering_meadow", "trodden_grass", "stubble", "fresh_plough"]:
		var uniform_name := "idx_%s" % role
		var idx: Variant = terrain.ground_material.get_shader_parameter(uniform_name)
		print("GA2 uniforme %s : %s" % [uniform_name, idx])
		if int(idx) < 0:
			ok = false

	print("GA2 OK" if ok else "GA2 ECHEC")
	quit(0 if ok else 1)
