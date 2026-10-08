extends SceneTree

## Test headless du lot PO5 (ADR 0097) : `SceneFader.go` aboutit à la bonne scène (instantané
## en headless), voile `cover` / `reveal`, ressenti des caméras lu dans `data/ui/camera_feel.json`,
## glissement de focus qui aboutit à la cible une fois la durée écoulée (carte et bataille), sans
## saut à la première image, inertie du déplacement, cartouche de saison et onde d'ordre.
## Usage : godot --headless --path game --script res://tests/po5_motion_test.gd

const START_MENU := "res://scenes/start_menu.tscn"
const FEEL_FILE := "res://../data/ui/camera_feel.json"
const STEP := 1.0 / 60.0

var _failures := 0


func _init() -> void:
	await process_frame
	_test_camera_feel()
	_test_campaign_glide()
	_test_campaign_inertia()
	_test_battle_glide()
	_test_season_banner()
	_test_order_ripple()
	await _test_scene_fader()
	print("po5_motion_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("po5_motion_test: " + message)
	return condition


func _test_camera_feel() -> void:
	CameraFeel.reload()
	var path := ProjectSettings.globalize_path(FEEL_FILE).simplify_path()
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not _check(parsed is Dictionary, "camera_feel.json unreadable at %s" % path):
		return
	var glide := CameraFeel.get_value("campaign", "focus_glide_s")
	_check(CameraFeel.loaded_from_data(), "camera feel should come from data/, not the fallback")
	_check(is_equal_approx(glide, float(parsed["campaign"]["focus_glide_s"])), "campaign glide %f should match the data file" % glide)
	_check(is_equal_approx(CameraFeel.get_value("battle", "zoom_damping"), float(parsed["battle"]["zoom_damping"])), "battle zoom damping should match the data file")
	_check(glide > 0.0, "campaign focus glide should be a real glide")


func _test_campaign_glide() -> void:
	var rig := CampaignCamera.new()
	rig.bounds = Rect2(Vector2.ZERO, Vector2(MapData.default_world_size()))
	rig.target_focus = Vector3(100.0, 0.0, 100.0)
	rig.target_distance = 500.0
	rig.snap()
	var goal := Vector3(900.0, 0.0, 700.0)
	rig.look_at_point(goal, 300.0)
	_check(rig.is_gliding(), "look_at_point should start a glide")
	rig._process(STEP)
	var first_step := rig.focus.distance_to(Vector3(100.0, 0.0, 100.0))
	_check(first_step < goal.distance_to(Vector3(100.0, 0.0, 100.0)) * 0.1, "no jump on the first frame of a focus glide (moved %.1f)" % first_step)
	var duration := CameraFeel.get_value("campaign", "focus_glide_s")
	var elapsed := STEP
	while elapsed < duration + STEP:
		rig._process(STEP)
		elapsed += STEP
	_check(not rig.is_gliding(), "glide should be over once its duration has elapsed")
	_check(rig.focus.distance_to(goal) < 0.01, "focus should reach the target after the glide (gap %.3f)" % rig.focus.distance_to(goal))
	_check(absf(rig.distance - 300.0) < 0.01, "distance should reach the requested zoom after the glide (%.2f)" % rig.distance)
	rig.free()


func _test_campaign_inertia() -> void:
	var rig := CampaignCamera.new()
	rig.edge_pan_enabled = false
	rig.target_focus = Vector3(2000.0, 0.0, 2000.0)
	rig.target_distance = 500.0
	rig.snap()
	rig.pan_velocity = Vector3(300.0, 0.0, 0.0)
	var speed := rig.pan_velocity.length()
	rig._process(STEP)
	_check(rig.target_focus.x > 2000.0, "released pan should keep drifting (inertia)")
	_check(rig.pan_velocity.length() < speed, "inertia should decay")
	for _i in 180:
		rig._process(STEP)
	_check(rig.pan_velocity == Vector3.ZERO, "inertia should die out within 3 s (%s)" % rig.pan_velocity)
	# Zoom lissé : la distance rejoint la cible sans saut.
	rig.target_distance = 250.0
	rig._process(STEP)
	_check(rig.distance < 500.0 and rig.distance > 300.0, "zoom should be smoothed toward the target (%.1f)" % rig.distance)
	rig.free()


func _test_battle_glide() -> void:
	var rig := BattleCamera.new()
	rig.glide_in_headless = true
	rig.look_at_point(Vector3(100.0, 0.0, 100.0), 200.0, 0.0)
	var goal := Vector3(500.0, 0.0, 300.0)
	rig.glide_to(goal, 150.0, 0.5)
	_check(rig.is_gliding(), "battle glide_to should glide")
	rig._process(STEP)
	_check(rig.target.distance_to(goal) > 100.0, "battle glide should not jump on its first frame")
	var elapsed := STEP
	while elapsed < CameraFeel.get_value("battle", "focus_glide_s") + STEP:
		rig._process(STEP)
		elapsed += STEP
	_check(not rig.is_gliding(), "battle glide should end after its duration")
	_check(Vector2(rig.target.x, rig.target.z).distance_to(Vector2(goal.x, goal.z)) < 0.01, "battle target should reach the goal")
	_check(absf(rig.yaw - 0.5) < 1e-3 and absf(rig.distance - 150.0) < 0.01, "battle yaw and distance should reach the goal")
	# Sans `glide_in_headless`, coupe franche en headless (smoke, captures).
	rig.glide_in_headless = false
	rig.glide_to(Vector3(50.0, 0.0, 50.0), 150.0, 0.5)
	_check(not rig.is_gliding() and is_equal_approx(rig.target.x, 50.0), "headless battle glide should be instant")
	rig.free()


func _test_season_banner() -> void:
	var banner := SeasonBanner.new()
	root.add_child(banner)
	banner.announce("Printemps 1338")
	_check(banner.last_text == "Printemps 1338", "season banner should hold the announced season")
	_check(not banner.visible, "season banner stays hidden in headless")
	banner.free()


func _test_order_ripple() -> void:
	_check(OrderRipple.spawn(root, null, Vector3.ZERO) == null, "no ripple without camera / in headless")
	var ripple := OrderRipple.new()
	root.add_child(ripple)
	_check(not ripple.advance(OrderRipple.DURATION * 0.5), "ripple still running mid-way")
	_check(ripple.advance(OrderRipple.DURATION), "ripple should finish after its duration")
	ripple.free()


func _test_scene_fader() -> void:
	SceneFader.cover()
	_check(SceneFader.is_covered(), "cover should veil the view (instant in headless)")
	SceneFader.reveal()
	_check(not SceneFader.is_covered(), "reveal should lift the veil")
	var error: Error = await SceneFader.go(START_MENU)
	_check(error == OK, "SceneFader.go should accept the scene change (%s)" % error)
	var fader := SceneFader.instance()
	_check(fader.last_path == START_MENU, "SceneFader.go should remember the requested scene")
	for _i in 3:
		await process_frame
	var scene := current_scene
	_check(scene != null and scene.scene_file_path == START_MENU, "SceneFader.go should end on %s (got %s)" % [START_MENU, scene.scene_file_path if scene != null else "null"])
	_check(not fader.busy, "SceneFader should not stay busy in headless")
	_check(fader.get_parent() == root, "the fader layer should outlive scene changes (child of root)")
