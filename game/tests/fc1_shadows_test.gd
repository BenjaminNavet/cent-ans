extends SceneTree

## FC1 : ombres des maquettes de colonies et des moulins réglées par le préréglage de qualité.
## Usage : godot --headless --path game --script res://tests/fc1_shadows_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	_test_preset_values()
	_test_settlement_layer()
	_test_windmills()
	RenderQuality.override_level = ""
	print("fc1_shadows_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _test_preset_values() -> void:
	var expected := {"low": 0.0, "medium": 150.0, "high": 250.0, "ultra": 500.0, "legacy": 500.0}
	for level: String in expected:
		_check(is_equal_approx(float(RenderQuality.PRESETS[level]["model_shadow_distance"]), expected[level]), "%s model_shadow_distance" % level)
	# Legacy : même seuil que `ZoomTiers.model_shadow_distance` (comportement d'avant FC1).
	_check(is_equal_approx(float(RenderQuality.PRESETS["legacy"]["model_shadow_distance"]), ZoomTiers.new().model_shadow_distance), "legacy keeps the zoom tier threshold")


func _test_settlement_layer() -> void:
	var layer := SettlementLayer.new()
	layer.tiers = ZoomTiers.new()
	root.add_child(layer)
	layer.add_to_group(RenderQuality.CLIENT_GROUP)
	var part := MeshInstance3D.new()
	part.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
	layer.add_child(part)
	layer._shadow_geometries.append(part)
	RenderQuality.override_level = "low"
	RenderQuality.apply_clients(self)
	_check(is_zero_approx(layer.model_shadow_limit()), "low preset reaches the settlement layer")
	layer._update_model_shadows(5.0, true)
	_check(part.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "low: no settlement shadows even up close")
	RenderQuality.override_level = "high"
	RenderQuality.apply_clients(self)
	_check(is_equal_approx(layer.model_shadow_limit(), 250.0), "high preset reaches the settlement layer")
	layer._update_model_shadows(100.0)
	_check(part.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON, "high: shadows at d = 100")
	layer._update_model_shadows(300.0)
	_check(part.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_OFF, "high: no shadows at d = 300")
	layer.free()


func _test_windmills() -> void:
	var effects := LifeEffects.new()
	root.add_child(effects)
	RenderQuality.override_level = "high"
	effects.setup(null, null)
	var bodies := effects.get_node("WindmillBodies") as MultiMeshInstance3D
	var sails := effects.get_node("WindmillSails") as MultiMeshInstance3D
	effects._update_mill_shadows(60.0)
	_check(bodies.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON and sails.cast_shadow == GeometryInstance3D.SHADOW_CASTING_SETTING_ON, "high: windmill shadows at d = 60")
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
