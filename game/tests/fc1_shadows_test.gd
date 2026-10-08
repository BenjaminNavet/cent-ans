extends SceneTree

## FC1 : ombres des moulins (les maquettes de colonies n'existent plus, ADR 0138) réglées par le préréglage de qualité.
## Usage : godot --headless --path game --script res://tests/fc1_shadows_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	_test_preset_values()
	_test_windmills()
	RenderQuality.override_level = ""
	print("fc1_shadows_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _test_preset_values() -> void:
	var expected := {"low": 0.0, "medium": 150.0, "high": 250.0, "ultra": 500.0, "legacy": 500.0}
	for level: String in expected:
		_check(is_equal_approx(float(RenderQuality.presets()[level]["model_shadow_distance"]), expected[level]), "%s model_shadow_distance" % level)
	# Legacy : seuil d'avant FC1 (l'ancien `ZoomTiers.model_shadow_distance`, VT-G : retiré).
	_check(is_equal_approx(float(RenderQuality.presets()["legacy"]["model_shadow_distance"]), 500.0), "legacy keeps the zoom tier threshold")


func _test_windmills() -> void:
	var effects := LifeEffects.new()
	root.add_child(effects)
	RenderQuality.override_level = "high"
	effects.setup(null, null)
	var bodies := effects.get_node("WindmillBodies") as MultiMeshInstance3D
	var sails := effects.get_node("WindmillSails") as MultiMeshInstance3D
	effects._update_mill_shadows(60.0)
	_check(bodies.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON and sails.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "high: windmill body shadows at d = 60, never the sails (A6-L10)")
	effects._update_mill_shadows(200.0)
	_check(bodies.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF and sails.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "high: no windmill shadows beyond veg_shadow_distance")
	RenderQuality.override_level = "low"
	RenderQuality.apply_clients(self)
	effects._update_mill_shadows(10.0)
	_check(bodies.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "low: no windmill shadows")
	effects.free()


func _check(condition: bool, label: String) -> void:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
