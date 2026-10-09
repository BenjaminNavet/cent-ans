extends TestCase

## Test headless du lot HB7 (ADR 0143, lisibilité des cours d'eau en vue 3D).
##  1. Largeur écran minimale : un fleuve majeur (importance 6) fait au moins `MIN_MAJOR_PX`
##     pixels à toutes les distances de la vue 3D ; un petit affluent reste un filet plus fin.
##  2. Relais sans saut : le palier fin (ordre de Strahler 9-10) a la même largeur minimale que
##     le palier moyen (importance 6), à 25 % près ; mêmes couleurs d'eau.
##  3. Garde-fou : les shaders prennent `abs(PROJECTION_MATRIX[1][1])` (le terme est négatif
##     sous Metal/Vulkan : la largeur minimale était inopérante).
##  4. L'eau est bleu-vert (bleu nettement au-dessus du rouge).
## Usage : godot --headless --path game --script res://tests/hb7_river_width_test.gd

const MIN_MAJOR_PX := 6.0
const MAX_BROOK_PX := 2.5
## Distances caméra de la vue 3D (unités monde).
const DISTANCES := [20.0, 80.0, 250.0, 600.0, 1200.0]
const FOV_DEG := 55.0
const VIEWPORT_H := 675.0


func _init() -> void:
	await process_frame
	_run()
	finish()


## Largeur écran (px) d'un ruban moyen, comme `river_water.gdshader`.
static func _mid_px(material: ShaderMaterial, width: float, importance: int, distance: float) -> float:
	var wpp := distance * 2.0 * tan(deg_to_rad(FOV_DEG) * 0.5) / VIEWPORT_H
	var lo: Vector4 = material.get_shader_parameter("importance_px_lo")
	var hi: Vector3 = material.get_shader_parameter("importance_px_hi")
	var scale := lo[importance] if importance < 4 else hi[importance - 4]
	var min_px := float(material.get_shader_parameter("min_px")) * scale
	return maxf(width / wpp, min_px)


static func _fine_min_px(material: ShaderMaterial, order: int) -> float:
	var lo: Vector4 = material.get_shader_parameter("order_px_lo")
	var hi: Vector4 = material.get_shader_parameter("order_px_hi")
	var i := clampi(order, 3, 10) - 3
	return float(material.get_shader_parameter("min_px")) * (lo[i] if i < 4 else hi[i - 4])


func _run() -> void:
	for path in ["res://shaders/river_water.gdshader", "res://shaders/river_fine.gdshader"]:
		var code := FileAccess.get_file_as_string(path)
		check(code.contains("abs(PROJECTION_MATRIX[1][1])"), "%s: abs() on the projection term" % path)
	var data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if not check(data.load_error == "", "map data: %s" % data.load_error):
		return
	var rivers := RiversRenderer.new()
	root.add_child(rivers)
	rivers.build(data)
	var major := rivers.material_override as ShaderMaterial
	if not check(major != null, "major river material"):
		return
	var seine_width := 0.0
	for river in rivers.rivers:
		if str(river["name"]).begins_with("Seine") and int(river["importance"]) == 6:
			seine_width = maxf(seine_width, RiversRenderer._max(river["widths"]))
	check(seine_width > 0.0, "Seine (importance 6) in rivers_render.json")
	for distance: float in DISTANCES:
		var px := _mid_px(major, 0.0, 6, distance)
		check(px >= MIN_MAJOR_PX, "major river %.1f px at distance %d (>= %.0f)" % [px, int(distance), MIN_MAJOR_PX])
	check(_mid_px(major, 0.0, 6, 600.0) > _mid_px(major, 0.0, 3, 600.0), "width grows with importance")
	var minor := (rivers.get_node("MinorRivers") as MeshInstance3D).material_override as ShaderMaterial
	check(_mid_px(minor, 0.0, 0, 600.0) <= MAX_BROOK_PX, "small rivers stay thin threads")
	# Palier fin : mêmes réglages que le palier moyen (relais au seuil « près »).
	var fine := ShaderMaterial.new()
	fine.shader = preload("res://shaders/river_fine.gdshader")
	rivers.apply_fine_display(fine)
	var mid6 := _mid_px(major, 0.0, 6, 150.0)
	var fine9 := _fine_min_px(fine, 9)
	check(absf(fine9 - mid6) <= 0.25 * mid6, "fine order 9 (%.1f px) matches mid importance 6 (%.1f px)" % [fine9, mid6])
	check(_fine_min_px(fine, 3) <= MAX_BROOK_PX, "fine order 3 is a thread (%.1f px)" % _fine_min_px(fine, 3))
	for key in ["shallow_color", "deep_color"]:
		var c_mid: Color = major.get_shader_parameter(key)
		var c_fine: Color = fine.get_shader_parameter(key)
		check(c_mid.is_equal_approx(c_fine), "%s shared by both tiers" % key)
		check(c_mid.b > c_mid.r + 0.2, "%s is blue-green water (%s)" % [key, c_mid])
	print("hb7_river_width_test: major px %s, fine order 9 %.1f px, Seine max width %.2f" % [
		JSON.stringify(DISTANCES.map(func(d: float) -> float: return snappedf(_mid_px(major, seine_width, 6, d), 0.1))), fine9, seine_width])
	rivers.queue_free()
