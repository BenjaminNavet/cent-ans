extends TestCase

## Lot FA7 : herbe des batailles en vrais brins. Vérifie que le catalogue
## `data/art/battle_grass.json` est lu, que l'atlas `grass_tufts.png` est compressé avec mipmaps
## et tient dans le budget mémoire, que `BattleVegetation` câble le shader (cases, tirages cumulés,
## parcelles) et bâtit le maillage décrit par les données.
## Usage : godot --headless --path game --script res://tests/fa7_grass_test.gd

const MEMORY_BUDGET := 6 * 1024 * 1024


func _init() -> void:
	var fa := BattleVegetation.fa_catalogue()
	check(not fa.is_empty(), "catalogue lu")
	if fa.is_empty():
		failures += 1
		finish()
		return
	var atlas: Dictionary = fa["atlas"]
	var variants: Array = fa["variants"]
	check(variants.size() >= 4 and variants.size() <= BattleVegetation.FA_MAX_VARIANTS, "4 à %d touffes" % BattleVegetation.FA_MAX_VARIANTS)
	check(variants.size() <= int(atlas["columns"]) * int(atlas["rows"]), "une case par touffe")

	var texture: Texture2D = load(BattleVegetation.FA_TEXTURE_PATH)
	check(texture != null, "atlas chargé")
	check(texture.get_width() == int(atlas["columns"]) * int(atlas["cell_width"]) and texture.get_height() == int(atlas["rows"]) * int(atlas["cell_height"]), "taille de l'atlas")
	var image := texture.get_image()
	check(image.has_mipmaps(), "mipmaps")
	check(image.is_compressed(), "compression GPU (%s)" % image.get_format())
	print("FA7 mémoire de l'atlas : %.2f Mo" % (image.get_data().size() / 1048576.0))
	check(image.get_data().size() <= MEMORY_BUDGET, "budget mémoire de 6 Mo")

	var render: Dictionary = fa["render"]
	var mesh := BattleVegetation._clump_mesh_fa(render["card"])
	var cards := int(render["card"]["count"])
	check(mesh.surface_get_array_len(0) == cards * 6, "%d cartes de 6 sommets" % cards)
	var top := 0.0
	for v in mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]:
		top = maxf(top, v.y)
	check(is_equal_approx(top, 1.0), "cartes d'un mètre (hauteur fixée par la case)")

	var mat := ShaderMaterial.new()
	mat.shader = BattleVegetation.GRASS_SHADER
	BattleVegetation._apply_fa(mat, fa)
	check(is_equal_approx(float(mat.get_shader_parameter("fa_on")), 1.0), "shader : fa_on")
	check(int(mat.get_shader_parameter("fa_count")) == variants.size(), "shader : nombre de cases")
	for key in ["fa_cum_open", "fa_cum_clump"]:
		var cumulative: PackedFloat32Array = mat.get_shader_parameter(key)
		var rising := cumulative.size() == BattleVegetation.FA_MAX_VARIANTS
		for i in range(1, cumulative.size()):
			rising = rising and cumulative[i] >= cumulative[i - 1]
		check(rising and is_equal_approx(cumulative[variants.size() - 1], 1.0), "shader : %s cumulé jusqu'à 1" % key)
	var heights: PackedFloat32Array = mat.get_shader_parameter("fa_height")
	check(is_equal_approx(heights[0], float(variants[0]["height_m"])), "shader : hauteurs des cases")
	var wheat: Vector2i = mat.get_shader_parameter("fa_wheat")
	check(str(variants[wheat.x]["name"]) == str(render["fields"]["wheat_variants"][0]), "shader : cases du blé")
	check(is_equal_approx(float(mat.get_shader_parameter("tex_lum")), float(render["tex_lum"])), "shader : luminance de l'atlas")
	check(BattleVegetation.fa_grass_enabled(), "actif sans --no-fa-grass")

	finish()
