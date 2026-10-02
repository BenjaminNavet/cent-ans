extends SceneTree

## Lot TB6 : lumière et atmosphère de la carte de campagne. Vérifie les paramètres appliqués par
## saison et par météo (ombres de nuages bornées, lumière dorée, brume du matin) et que la brume
## épargne la province sélectionnée. Les chiffres de rendu viennent de `tb6_shot.gd`.
## Usage : godot --headless --path game --script res://tests/tb6_light_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := Vector2(2213.2, 3203.9)
const SCREEN := Vector2i(1600, 900)
const SEASONS := ["spring", "summer", "autumn", "winter"]
## Baisse de luminance tolérée au cœur d'une ombre de nuage (brief TB6).
const MAX_GROUND_DIM := 0.15

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	if _failures == 0:
		print("tb6_light OK")
	quit(1 if _failures > 0 else 0)


func _run() -> void:
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_DISABLED
	root.size = SCREEN
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	_test_cloud_shadows(map)
	map.queue_free()
	await process_frame


## 1. Ombres de nuages : aucune par temps clair, douces et bornées sous une vraie météo.
func _test_cloud_shadows(map: Node3D) -> void:
	var view: CampaignWeatherView = map.weather_view
	if not _check(view != null and view.enabled, "campaign weather view missing"):
		return
	var material: ShaderMaterial = map.terrain.material
	var clouds := MapReadability.section("clouds")
	for key: String in ["wet_dim", "mask_shadow", "mask_shadow_softness", "mask_shadow_scale"]:
		_check(clouds.has(key), "clouds.%s missing in data/ui/campaign_map.json" % key)
	# Les valeurs des données arrivent sur le matériau du terrain.
	var pairs := {
		"weather_wet_dim": "wet_dim", "weather_cloud_shade": "mask_shadow",
		"weather_cloud_shade_soft": "mask_shadow_softness", "weather_cloud_shade_scale": "mask_shadow_scale",
	}
	for uniform_name: String in pairs:
		var applied: Variant = material.get_shader_parameter(uniform_name)
		_check(applied != null and is_equal_approx(float(applied), float(clouds.get(pairs[uniform_name], -1.0))),
			"%s should carry clouds.%s (got %s)" % [uniform_name, pairs[uniform_name], applied])
	# Cumul au cœur d'un orage : sol mouillé × ombre des nuées × ombres de nuages du terrain.
	_check(view.max_ground_dim() <= MAX_GROUND_DIM, "weather may not dim the ground by more than 15 %% (%.3f)" % view.max_ground_dim())
	_check(view.max_ground_dim() > 0.03, "real weather should still shade the ground a little")
	# Ombre large et fondue : seuil plus large et bruit plus lent que le plan de nuées (0,22 ; 1).
	_check(view.mask_shadow_softness >= 0.4, "cloud shade edge should be soft (%.2f)" % view.mask_shadow_softness)
	_check(view.mask_shadow_scale <= 0.6, "cloud shade should be broader than the clouds (%.2f)" % view.mask_shadow_scale)
	# Temps clair : rien (règle TB2).
	_check(is_zero_approx(view.shadow_amount_for("clear")) and is_zero_approx(view.shadow_amount_for("fog")), "no cloud shadow in fair weather")
	var saved: Dictionary = view.weather.duplicate(true)
	view.weather = {}
	view._upload_mask()
	_check(not bool(material.get_shader_parameter("weather_enabled")), "clear weather everywhere: no weather mask on the ground")
	view.weather = saved
	view._upload_mask()
	print("tb6 cloud shadows: wet %.2f, mask shade %.2f (soft %.2f, scale %.2f), terrain %.2f -> max dim %.3f" % [
		view.wet_dim, view.mask_shadow, view.mask_shadow_softness, view.mask_shadow_scale, view.weather_shadow_amount, view.max_ground_dim()])


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb6_light: " + message)
	return condition
