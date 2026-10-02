extends SceneTree

## Test headless du lot TB2 (désencombrement de la carte de campagne, plan
## `docs/design/2026-10-02-campagne-tob.md` § 3) : un seul signe par ville et par palier de zoom,
## étiquettes entières dans l'écran, noms de région en vue moyenne, frontières discrètes hors
## sélection, brouillard de guerre en voile de parchemin, nuages réservés à la météo réelle.
## Usage : godot --headless --path game --script res://tests/tb2_declutter_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	if _failures == 0:
		print("tb2_declutter OK")
	quit(1 if _failures > 0 else 0)


func _run() -> void:
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
	_test_fog(map)
	map.queue_free()
	await process_frame


## 1. Brouillard de guerre : voile de parchemin sombre et désaturé, plus de nappe blanche.
func _test_fog(map: Node3D) -> void:
	var material: ShaderMaterial = map.terrain.material
	var fog := MapReadability.section("fog_of_war")
	_check(not fog.is_empty(), "fog_of_war block missing in data/ui/campaign_map.json")
	var veil: Color = material.get_shader_parameter("fog_veil_color")
	_check(veil.r > veil.g and veil.g > veil.b, "fog veil should be a warm parchment tone, got %s" % veil)
	_check(veil.get_luminance() < 0.75, "fog veil should not be a white wash (luminance %.2f)" % veil.get_luminance())
	var dim := float(material.get_shader_parameter("fog_dim"))
	_check(dim >= 0.55 and dim < 0.9, "fogged ground should be dimmed but readable (dim %.2f)" % dim)
	_check(float(material.get_shader_parameter("fog_desaturation")) >= 0.4, "fogged ground should be desaturated")
	# La brume résiduelle ne doit plus couvrir le relief (elle montait à 0,66 avant TB2). Le
	# palier de zoom peut encore la réduire (`_apply_close_tiers`), jamais l'augmenter.
	_check(float(material.get_shader_parameter("fog_mist_max")) <= 0.15, "residual mist must stay thin")
	_check(float(material.get_shader_parameter("fog_mist_glow")) <= 0.05, "mist must not glow (it washed the ground white)")
	print("tb2 fog: veil %s dim %.2f desaturation %.2f mist_max %.2f" % [veil.to_html(false), dim, float(material.get_shader_parameter("fog_desaturation")), float(material.get_shader_parameter("fog_mist_max"))])


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb2_declutter: " + message)
	return condition
