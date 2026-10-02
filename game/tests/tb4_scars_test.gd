extends SceneTree

## Lot TB4 (campagne façon Thrones of Britannia) : conséquences visibles de la guerre et des
## fléaux.
##  1. brûlis posé par-dessus la carte de couleur (SS2) et les matières (HB3), masque de terroir
##     échantillonné à son échelle (carte non carrée) ; les chiffres de rendu viennent de
##     `ss_shot.gd --stats --devastate=` ;
##  2. peste, 3. champ de bataille, 4. engins de siège : contrôles ajoutés avec chaque point.
## Usage : godot --headless --path game --script res://tests/tb4_scars_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")

var _failures := 0


func _init() -> void:
	await process_frame
	_check_burn()
	print("TB4 scars test %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb4_scars_test: " + message)
	return condition


## Point 1 : le brûlis vient après `sg_apply` et `hb_apply` (il n'est plus recouvert), et le
## masque de terroir est lu à l'échelle où `TerroirMask` le peint.
func _check_burn() -> void:
	var code := TERRAIN_SHADER.code
	var burn := code.find("col = terroir_burn(")
	_check(burn >= 0, "terrain shader no longer applies terroir_burn")
	_check(code.find("sg_apply(uv") >= 0 and code.find("sg_apply(uv") < burn, "burn is applied before the colour map (sg_apply)")
	_check(code.find("hb_apply(p") >= 0 and code.find("hb_apply(p") < burn, "burn is applied before the HB materials (hb_apply)")
	_check(code.contains("terroir_at(p / max(map_size.x, map_size.y))"), "terroir mask not sampled at the scale it is painted (non-square map)")
	var uniforms := TERRAIN_SHADER.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))
	for uniform_name in ["burnt_color", "ash_color", "terroir_mask"]:
		_check(uniforms.has(uniform_name), "terrain shader lacks uniform %s" % uniform_name)
	# Même convention côté processeur : un point peint se relit au même endroit.
	var mask := TerroirMask.new()
	var map_size := Vector2(7168.0, 6144.0)
	var at := Vector2(2213.2, 3203.9)
	mask.build([{"kind": "city", "province": "prov_a", "px": at}], [], {"prov_a": {"devastation": 100.0, "population": TerroirMask.REFERENCE_POPULATION}}, null, map_size)
	_check(mask.sample(at).b > 0.9, "burn painted at the settlement: %s" % mask.sample(at))
	var texel := at / maxf(map_size.x, map_size.y) * float(TerroirMask.SIZE)
	_check(mask.image.get_pixel(int(texel.x), int(texel.y)).b > 0.9, "shader lookup (p / max side) hits the burnt texel")
	var stretched := at / map_size * float(TerroirMask.SIZE)
	_check(mask.image.get_pixel(int(stretched.x), int(stretched.y)).b < 0.05, "old lookup (uv) missed the burn: the fix matters")
