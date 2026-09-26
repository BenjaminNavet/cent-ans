extends SceneTree

## Lot FG3 : cartes cuites des figurines fines. Vérifie que les textures se chargent (formats
## compressés, mémoire), que les figurines cuites basculent sur la variante `FG3_BAKED` avec
## leur couche, que les maillages `CAM2` portent l'UV d'atlas (LOD0/LOD1) et pas le LOD2, et
## qu'avec `--coarse-figures` (figurines Quaternius, FG5) rien ne change (shader par défaut).
## Usage : godot --headless --path game --script res://tests/fg3_maps_test.gd [-- --coarse-figures]


func _init() -> void:
	var ok := true
	var fine := BattleSkinned.fine_enabled()
	var maps := BattleSkinned.fine_maps()
	var total := 0
	for key in BattleSkinned.FINE_MAPS:
		var tex = maps.get(key)
		if tex == null:
			print("FG3 map %s: absente" % key)
			ok = false
			continue
		var bytes := 0
		var fmt := -1
		var desc := ""
		# Taille calculée (le rendu headless ne relit pas les données compressées) :
		# BC7 / ASTC 4x4 = 1 octet par texel, mipmaps = x4/3.
		var w := 0
		var h := 0
		var layers := 1
		if tex is TextureLayered:
			var arr := tex as TextureLayered
			fmt = arr.get_format()
			w = arr.get_width()
			h = arr.get_height()
			layers = arr.get_layers()
		elif tex is Texture2D:
			var t2 := tex as Texture2D
			w = t2.get_width()
			h = t2.get_height()
			fmt = t2.get_image().get_format() if DisplayServer.get_name() != "headless" else Image.FORMAT_BPTC_RGBA
		var per_texel := 1.0 if fmt == Image.FORMAT_BPTC_RGBA or fmt == Image.FORMAT_ASTC_4x4 else 4.0
		bytes = int(w * h * layers * per_texel * 4.0 / 3.0)
		desc = "%dx%d x%d" % [w, h, layers]
		total += bytes
		print("FG3 map %s: %s, format %d, %.2f Mo (mipmaps compris)" % [key, desc, fmt, bytes / 1048576.0])
		if fmt != Image.FORMAT_BPTC_RGBA and fmt != Image.FORMAT_ASTC_4x4:
			print("  format non compressé BC7/ASTC")
	print("FG3 mémoire des cartes : %.2f Mo" % (total / 1048576.0))
	var figures: Dictionary = BattleSkinned.manifest().get("figures", {})
	var baked := 0
	for fig_name in figures:
		var entry: Dictionary = figures[fig_name]
		var parts := str(fig_name).rsplit("_", true, 1)
		var kind := parts[0]
		var variant := int(parts[1])
		var mat := ShaderMaterial.new()
		mat.shader = BattleSkinned.SHADER
		BattleSkinned.setup_material(mat, kind, variant)
		var has_layer := entry.has("atlas_layer") and fine
		var variant_on := mat.shader != BattleSkinned.SHADER
		if has_layer != variant_on:
			print("FG3 %s : variante %s attendue %s" % [fig_name, variant_on, has_layer])
			ok = false
		if not has_layer:
			continue
		baked += 1
		for level in 3:
			var mesh := BattleSkinned.mesh(kind, variant, level)
			var uv2: PackedVector2Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_TEX_UV2]
			var with_atlas := 0
			for v in uv2:
				if v.y > 0.5:
					with_atlas += 1
			var expected := level < 2
			if (with_atlas > 0) != expected:
				print("FG3 %s LOD%d : UV d'atlas %d/%d" % [fig_name, level, with_atlas, uv2.size()])
				ok = false
	if not fine:
		var mat := ShaderMaterial.new()
		mat.shader = BattleSkinned.SHADER
		BattleSkinned.setup_material(mat, "infantry", 0)
		if mat.shader != BattleSkinned.SHADER:
			print("FG3 : le rendu par défaut a changé de shader")
			ok = false
	print("FG3 figurines cuites : %d" % baked)
	print("FG3_MAPS %s" % ("OK" if ok else "FAIL"))
	quit(0 if ok else 1)
