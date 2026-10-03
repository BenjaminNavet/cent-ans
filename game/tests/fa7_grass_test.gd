extends SceneTree

## Lot FA7 : herbe des batailles en vrais brins. Vérifie que le catalogue
## `data/art/battle_grass.json` est lu, que l'atlas `grass_tufts.png` est compressé avec mipmaps
## et tient dans le budget mémoire, que `BattleVegetation` câble le shader (cases, tirages cumulés,
## parcelles) et bâtit le maillage décrit par les données.
## Usage : godot --headless --path game --script res://tests/fa7_grass_test.gd

const MEMORY_BUDGET := 6 * 1024 * 1024

var _ok := true


func _check(condition: bool, label: String) -> void:
	print("FA7 %s : %s" % [label, "ok" if condition else "ECHEC"])
	_ok = _ok and condition


func _init() -> void:
	var fa := BattleVegetation.fa_catalogue()
	_check(not fa.is_empty(), "catalogue lu")
	if fa.is_empty():
		quit(1)
		return
	var atlas: Dictionary = fa["atlas"]
	var variants: Array = fa["variants"]
	_check(variants.size() >= 4 and variants.size() <= BattleVegetation.FA_MAX_VARIANTS, "4 à %d touffes" % BattleVegetation.FA_MAX_VARIANTS)
	_check(variants.size() <= int(atlas["columns"]) * int(atlas["rows"]), "une case par touffe")

	var texture: Texture2D = load(BattleVegetation.FA_TEXTURE_PATH)
	_check(texture != null, "atlas chargé")
	_check(texture.get_width() == int(atlas["columns"]) * int(atlas["cell_width"]) and texture.get_height() == int(atlas["rows"]) * int(atlas["cell_height"]), "taille de l'atlas")
	var image := texture.get_image()
	_check(image.has_mipmaps(), "mipmaps")
	_check(image.is_compressed(), "compression GPU (%s)" % image.get_format())
	print("FA7 mémoire de l'atlas : %.2f Mo" % (image.get_data().size() / 1048576.0))
	_check(image.get_data().size() <= MEMORY_BUDGET, "budget mémoire de 6 Mo")

	var render: Dictionary = fa["render"]
	var mesh := BattleVegetation._clump_mesh_fa(render["card"])
	var cards := int(render["card"]["count"])
	_check(mesh.surface_get_array_len(0) == cards * 6, "%d cartes de 6 sommets" % cards)
	var top := 0.0
	for v in mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]:
		top = maxf(top, v.y)
	_check(is_equal_approx(top, 1.0), "cartes d'un mètre (hauteur fixée par la case)")

	var mat := ShaderMaterial.new()
	mat.shader = BattleVegetation.GRASS_SHADER
	BattleVegetation._apply_fa(mat, fa)
	_check(is_equal_approx(float(mat.get_shader_parameter("fa_on")), 1.0), "shader : fa_on")
	_check(int(mat.get_shader_parameter("fa_count")) == variants.size(), "shader : nombre de cases")
	for key in ["fa_cum_open", "fa_cum_clump"]:
		var cumulative: PackedFloat32Array = mat.get_shader_parameter(key)
		var rising := cumulative.size() == BattleVegetation.FA_MAX_VARIANTS
		for i in range(1, cumulative.size()):
			rising = rising and cumulative[i] >= cumulative[i - 1]
		_check(rising and is_equal_approx(cumulative[variants.size() - 1], 1.0), "shader : %s cumulé jusqu'à 1" % key)
	var heights: PackedFloat32Array = mat.get_shader_parameter("fa_height")
	_check(is_equal_approx(heights[0], float(variants[0]["height_m"])), "shader : hauteurs des cases")
	var wheat: Vector2i = mat.get_shader_parameter("fa_wheat")
	_check(str(variants[wheat.x]["name"]) == str(render["fields"]["wheat_variants"][0]), "shader : cases du blé")
	_check(is_equal_approx(float(mat.get_shader_parameter("tex_lum")), float(render["tex_lum"])), "shader : luminance de l'atlas")
	_check(BattleVegetation.fa_grass_enabled(), "actif sans --no-fa-grass")

	print("FA7 OK" if _ok else "FA7 ECHEC")
	quit(0 if _ok else 1)
