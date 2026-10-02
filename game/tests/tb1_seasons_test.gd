extends SceneTree

## Lot TB1 (campagne façon Thrones of Britannia) : saisons visibles à tous les zooms.
## Squelette : les contrôles arrivent avec chaque point du lot (delta saisonnier par-dessus la carte
## de couleur, mer selon la saison, étalonnage par saison lu dans `data/ui/`, neige sur les toits
## des villes 1:1).
## Usage : godot --headless --path game --script res://tests/tb1_seasons_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")


func _init() -> void:
	var ok := true
	ok = _check_ground_delta() and ok
	ok = _check_sea() and ok
	ok = _check_grade() and ok
	ok = _check_roofs() and ok
	if ok:
		print("TB1 seasons test OK")
	quit(0 if ok else 1)


## Point 1 : delta saisonnier appliqué par-dessus la carte de couleur (SS2) et les matières (HB3).
## Les réglages existent, et l'écart de saison est bien passé aux deux crochets : il n'est plus
## effacé par la carte (les chiffres de rendu viennent de `ss_shot.gd --stats --season=`).
func _check_ground_delta() -> bool:
	var ok := true
	var uniforms := _uniforms(TERRAIN_SHADER)
	for uniform_name in ["sg_season_strength", "hb_season_strength", "hb_season_gain", "summer_gold", "winter_south", "winter_cover"]:
		if not uniforms.has(uniform_name):
			push_error("TB1: terrain shader lacks uniform %s" % uniform_name)
			ok = false
	var code := TERRAIN_SHADER.code
	for hook in ["sg_apply(uv, footprint, snow, land_f, season_k, col)", "k_grass, k_forest,"]:
		if not code.contains(hook):
			push_error("TB1: terrain shader no longer passes the seasonal delta (%s)" % hook)
			ok = false
	for path in ["res://shaders/satellite_ground.gdshaderinc", "res://shaders/hb_ground.gdshaderinc"]:
		var include := FileAccess.get_file_as_string(path)
		if not include.contains("season_k") and not include.contains("k_forest"):
			push_error("TB1: %s ignores the seasonal delta" % path)
			ok = false
	return ok


func _uniforms(shader: Shader) -> Array:
	return shader.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))


## Point 2 : mer selon la saison.
func _check_sea() -> bool:
	return true


## Point 3 : étalonnage par saison (`data/ui/`).
func _check_grade() -> bool:
	return true


## Point 4 : neige sur les toits des villes 1:1.
func _check_roofs() -> bool:
	return true
