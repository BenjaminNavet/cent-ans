extends SceneTree

## Test headless : le pad macOS (glissement à deux doigts = `InputEventPanGesture`, pincement =
## `InputEventMagnifyGesture`) zoome les caméras de campagne et de bataille comme la molette.
## Usage : godot --headless --path game --script res://tests/trackpad_zoom_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_campaign()
	_test_battle()
	print("trackpad_zoom_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("trackpad_zoom_test: " + message)


func _pan(delta_y: float) -> InputEventPanGesture:
	var event := InputEventPanGesture.new()
	event.delta = Vector2(0.0, delta_y)
	return event


func _pinch(factor: float) -> InputEventMagnifyGesture:
	var event := InputEventMagnifyGesture.new()
	event.factor = factor
	return event


func _test_campaign() -> void:
	var camera := CampaignCamera.new()
	camera.target_distance = 500.0
	camera._unhandled_input(_pan(-1.0))
	_check(camera.target_distance < 500.0, "campagne : deux doigts vers le haut doit rapprocher")
	var near := camera.target_distance
	camera._unhandled_input(_pan(2.0))
	_check(camera.target_distance > near, "campagne : deux doigts vers le bas doit éloigner")
	camera.target_distance = 500.0
	camera._unhandled_input(_pinch(1.25))
	_check(is_equal_approx(camera.target_distance, 400.0), "campagne : pincement écarté doit rapprocher")
	camera._unhandled_input(_pinch(0.5))
	_check(is_equal_approx(camera.target_distance, 800.0), "campagne : pincement serré doit éloigner")
	for i in 200:
		camera._unhandled_input(_pinch(0.5))
	_check(camera.target_distance <= camera.max_distance, "campagne : bornée par max_distance")
	camera.free()


func _test_battle() -> void:
	var camera := BattleCamera.new()
	camera._unhandled_input(_pan(-1.0))
	_check(camera._target_distance < 220.0, "bataille : deux doigts vers le haut doit rapprocher")
	camera._target_distance = 220.0
	camera._unhandled_input(_pinch(2.0))
	_check(is_equal_approx(camera._target_distance, 110.0), "bataille : pincement écarté doit rapprocher")
	for i in 200:
		camera._unhandled_input(_pinch(2.0))
	_check(camera._target_distance >= camera.min_distance, "bataille : bornée par min_distance")
	camera.free()
