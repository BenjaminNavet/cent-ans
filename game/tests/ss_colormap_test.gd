extends TestCase

## Lot SS2 (ADR 0142) : carte de couleur du sol. Vérifie que le shader du terrain compile avec
## `satellite_ground.gdshaderinc`, et selon `map.json` :
## - avec `colormap.bc1` : image DXT1 à la taille annoncée, mipmaps, couleurs non grises (le défaut
##   que le chantier corrige) et différentes entre champs, forêt et mer ;
## - sans : `load_colormap` rend null (repli sur l'ancien rendu procédural).
## Usage : godot --headless --path game --script res://tests/ss_colormap_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")


func _init() -> void:
	var uniforms := TERRAIN_SHADER.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))
	for name in ["colormap", "has_colormap", "sg_strength", "sg_near_keep"]:
		if not uniforms.has(name):
			check(false, "SS2: terrain shader lacks uniform %s" % name)
	var map_dir := str(MAP_PATHS.default_data_dir()).path_join("map")
	var image := ReliefLandcover.load_colormap(map_dir)
	if not ReliefLandcover.load_colormap_meta(map_dir):
		if image != null:
			check(false, "SS2: colormap loaded without map.json entry")
		print("SS2 colormap: absent (fallback)")
	elif image == null:
		check(false, "SS2: map.json declares a colormap that does not load")
	else:
		_check_image(image, map_dir)
	finish()


func _check_image(image: Image, map_dir: String) -> void:
	var size := MapData.read_world_size(map_dir) * 2
	if image.get_size() != size or image.get_format() != Image.FORMAT_DXT1 or not image.has_mipmaps():
		check(false, "SS2: colormap %s fmt %d mips %s, expected %s DXT1 with mipmaps" % [image.get_size(), image.get_format(), image.has_mipmaps(), size])
		return
	var rgb := image.duplicate() as Image
	rgb.decompress()
	# Échantillon régulier des terres : saturation moyenne (max - min des canaux).
	var sat_sum := 0.0
	var n := 0
	var distinct := {}
	for y in range(0, rgb.get_height(), 256):
		for x in range(0, rgb.get_width(), 256):
			var c := rgb.get_pixel(x, y)
			sat_sum += maxf(c.r, maxf(c.g, c.b)) - minf(c.r, minf(c.g, c.b))
			n += 1
			distinct[Color8(int(c.r8 / 16) * 16, int(c.g8 / 16) * 16, int(c.b8 / 16) * 16)] = true
	var sat := sat_sum / maxf(n, 1)
	print("SS2 colormap: %s, mean saturation %.3f, %d colour bins" % [image.get_size(), sat, distinct.size()])
	if sat < 0.08:
		check(false, "SS2: colormap too grey (saturation %.3f < 0.08)" % sat)
	if distinct.size() < 24:
		check(false, "SS2: colormap too uniform (%d colour bins)" % distinct.size())
