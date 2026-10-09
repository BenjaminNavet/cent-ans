extends TestCase

## Lot ME1 (mer vivante) : réglages `data/fx/sea_life.json`, estrans (Mont-Saint-Michel, Wadden,
## Wash), shader de mer compilé avec la mer vivante active, saturation bornée, sillage de flotte.
## Usage : godot --headless --path game --script res://tests/me1_sea_life_test.gd

const WATER_SHADER := preload("res://shaders/water.gdshader")
const MAP_SIZE := Vector2i(7168, 6144)


func _init() -> void:
	check(not SeaLife.spec().is_empty(), "data/fx/sea_life.json lisible")
	check(SeaLife.tide_zone_at(Vector2(1814, 3180)).get("id", "") == "mont_saint_michel", "zone du Mont-Saint-Michel")
	check(SeaLife.tide_zone_at(Vector2(2700, 2550)).get("id", "") == "wadden", "zone des Wadden")
	check(SeaLife.tide_zone_at(Vector2(2080, 2550)).get("id", "") == "wash", "zone du Wash")
	check(SeaLife.tide_zone_at(Vector2(1479, 3620)).is_empty(), "pas d'estran en plein Atlantique")
	var image := SeaLife.tide_texture(MAP_SIZE).get_image()
	var msm := image.get_pixelv(Vector2i(1814 / 16, 3180 / 16)).r
	check(msm > 0.5 and image.get_pixelv(Vector2i(1479 / 16, 3620 / 16)).r == 0.0, "texture des estrans (R = %.2f)" % msm)
	var material := ShaderMaterial.new()
	material.shader = WATER_SHADER
	check(SeaLife.apply(material, MAP_SIZE), "SeaLife.apply active la mer vivante")
	check(is_equal_approx(float(material.get_shader_parameter("sea_sat_max")), 0.4), "saturation maximale 0,40 (bible D5)")
	var names: Array = WATER_SHADER.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))
	for uniform_name in ["sea_life_on", "tide_zones", "shallows_amount", "crest_amount", "surf_min_screen_px"]:
		check(names.has(uniform_name), "uniforme %s dans water.gdshader" % uniform_name)
	var wake := SeaLife.make_wake(1.0)
	check(wake != null and wake.mesh != null and wake.mesh.get_surface_count() == 1, "sillage construit")
	if wake != null:
		var aabb := wake.mesh.get_aabb()
		check(aabb.position.x < -30.0 and aabb.size.z > 14.0, "sillage derrière la poupe (+X = avant)")
		wake.free()
	finish()
