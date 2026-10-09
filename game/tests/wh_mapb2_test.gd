extends TestCase

## Lot WH mapb2 : rotation de caméra à la souris + suivi d'armée, pings et clic droit de la
## minicarte, panneau Commerce.
## Usage : godot --headless --path game --script res://tests/wh_mapb2_test.gd


func _init() -> void:
	await process_frame
	await _test_camera()
	_test_feel_data()
	_test_minimap()
	_test_pings()
	_test_trade_panel()
	_test_hotkeys()
	finish()


func _mouse_button(index: int, pressed: bool, alt := false, shift := false) -> InputEventMouseButton:
	var e := InputEventMouseButton.new()
	e.button_index = index
	e.pressed = pressed
	e.alt_pressed = alt
	e.shift_pressed = shift
	return e


func _motion(rel: Vector2) -> InputEventMouseMotion:
	var m := InputEventMouseMotion.new()
	m.relative = rel
	return m


func _test_camera() -> void:
	var rig: CampaignCamera = load("res://scenes/map/campaign_camera.tscn").instantiate() if ResourceLoader.exists("res://scenes/map/campaign_camera.tscn") else null
	if rig == null:
		rig = CampaignCamera.new()
		var cam := Camera3D.new()
		cam.name = "Camera3D"
		rig.add_child(cam)
	root.add_child(rig)
	await process_frame
	rig.edge_pan_enabled = false  # Souris hors fenêtre en headless = bord d'écran
	rig.setup(Rect2(0, 0, 1000, 1000), 500.0)
	var focus_before := rig.target_focus
	var yaw_before := rig.target_yaw
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, true, true))
	rig._unhandled_input(_motion(Vector2(40, 12)))
	check(not is_equal_approx(rig.target_yaw, yaw_before), "Alt + middle drag rotates yaw")
	check(rig.target_focus == focus_before, "rotation leaves the pan untouched")
	var expected := yaw_before - deg_to_rad(40.0 * CameraFeel.get_value("campaign", "rotate_mouse_deg_per_px"))
	check(is_equal_approx(rig.target_yaw, expected), "yaw = -relative.x * rotate_mouse_deg_per_px")
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, false))
	# Maj marche aussi ; sans modificateur c'est le glisser (pan), plus la rotation.
	var yaw_shift := rig.target_yaw
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, true, false, true))
	rig._unhandled_input(_motion(Vector2(-20, 0)))
	check(rig.target_yaw > yaw_shift, "Shift + middle drag rotates too")
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, false))
	var yaw_plain := rig.target_yaw
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, true))
	rig._unhandled_input(_motion(Vector2(30, 0)))
	check(is_equal_approx(rig.target_yaw, yaw_plain) and rig.target_focus != focus_before, "plain middle drag pans, no rotation")
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, false))
	# Suivi : la cible mobile est suivie, un glisser manuel libère.
	var moving := {"pos": Vector3(200, 0, 300)}
	rig.start_follow(func() -> Variant: return moving["pos"])
	rig._process(0.016)
	check(rig.is_following() and is_equal_approx(rig.target_focus.x, 200.0) and is_equal_approx(rig.target_focus.z, 300.0), "follow locks target_focus")
	moving["pos"] = Vector3(250, 0, 320)
	rig._process(0.016)
	check(is_equal_approx(rig.target_focus.x, 250.0), "follow tracks the moving target")
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, true, true))
	rig._unhandled_input(_motion(Vector2(50, 0)))
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, false))
	check(rig.is_following(), "rotation does not release the follow")
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, true))
	rig._unhandled_input(_motion(Vector2(40, 0)))
	rig._unhandled_input(_mouse_button(MOUSE_BUTTON_MIDDLE, false))
	check(not rig.is_following(), "manual drag releases the follow")
	rig.start_follow(func() -> Variant: return moving["pos"])
	rig.look_at_point(Vector3(10, 0, 10))
	check(not rig.is_following(), "recentring releases the follow")
	rig.start_follow(func() -> Variant: return null)
	rig._process(0.016)
	check(not rig.is_following(), "follow ends when the target vanishes")
	rig.queue_free()


func _test_feel_data() -> void:
	check(CameraFeel.loaded_from_data(), "camera_feel.json loaded")
	for key in ["rotate_mouse_deg_per_px", "follow_release_px", "minimap_ping_s", "minimap_ping_radius_px"]:
		check(CameraFeel.get_value("campaign", key) > 0.0, "camera_feel campaign.%s > 0" % key)


func _test_minimap() -> void:
	var minimap := CampaignMinimap.new()
	var got: Array = []
	minimap.ordered.connect(func(p: Vector2) -> void: got.append(p))
	root.add_child(minimap)
	minimap.order_at(Vector2(10, 10))
	check(got.size() == 1, "right click emits ordered")
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.pressed = true
	event.position = Vector2(5, 5)
	minimap._on_overlay_input(event)
	check(got.size() == 2, "right-click overlay input emits ordered")
	var lefts: Array = []
	minimap.clicked.connect(func(p: Vector2) -> void: lefts.append(p))
	minimap._on_overlay_input(event)
	check(lefts.is_empty(), "right click is not a recentre")
	minimap.ping(Vector2(100, 100))
	check(minimap.ping_count() == 1, "ping registered")
	minimap._pings[0]["start"] = Time.get_ticks_msec() - 100000
	minimap._process(0.1)
	check(minimap.ping_count() == 0, "ping expires")
	minimap.queue_free()


func _test_trade_panel() -> void:
	var panel := TradePanel.new()
	root.add_child(panel)
	var overview := {"income": 30, "routes": [
		{"id": "a", "from_hub_name": "Bruges", "to_hub_name": "Londres", "mode": "sea", "cut": false, "mine": true, "total_value": 30, "my_value": 30, "agreement": false},
		{"id": "b", "from_hub_name": "Paris", "to_hub_name": "Lyon", "mode": "land", "cut": true, "cut_reason": "guerre", "mine": true, "total_value": 0, "my_value": 0},
		{"id": "c", "from_hub_name": "Gênes", "to_hub_name": "Venise", "mode": "sea", "cut": false, "mine": false, "total_value": 99, "my_value": 0},
	]}
	panel.mine_only = true
	panel.show_data(overview)
	check(panel.rows.size() == 2, "my routes filter hides foreign routes (%d)" % panel.rows.size())
	check(String(panel.rows[1]["text"]).contains("coupée (guerre)"), "cut reason shown: %s" % panel.rows[1]["text"])
	check(String(panel.rows[0]["text"]).contains("mer") and String(panel.rows[0]["text"]).contains("30"), "mode and value shown")
	panel.set_mine_only(false)
	check(panel.rows.size() == 3, "filter off shows all routes")
	check(panel.summary_label.text.contains("30"), "summary shows the income")
	# Pont réel (si la bibliothèque est construite) : la somme des parts = revenu.
	if ClassDB.class_exists("CampaignSim"):
		var sim: Object = ClassDB.instantiate("CampaignSim")
		check(sim.has_method("get_trade_overview"), "bridge has get_trade_overview")
	panel.queue_free()


func _test_hotkeys() -> void:
	for action in ["army_follow", "campaign_commerce"]:
		check(CampaignHotkeys.ACTIONS.has(action), "%s routed" % action)
		check(InputMap.has_action(action), "InputMap has %s" % action)
		check(ShortcutSheet.action_keys(action) != "", "%s has a key label" % action)


## Faux « sim » : deux armées ennemies, l'une visible, l'autre cachée par le brouillard.
class FakeSim:
	extends RefCounted

	func get_army(id: String) -> Dictionary:
		return {"faction": "fac_england", "position": Vector2(100, 100) if id == "seen" else Vector2(300, 300), "location_province": "p"}


class FakeMap:
	extends Node
	var sim: Object = FakeSim.new()
	var map_data: MapData = null
	var player_faction := "fac_france"


func _test_pings() -> void:
	# Chargé à l'exécution : un `extends MinimapController` compilerait MapUI avant l'autoload IconLibrary.
	var controller: Node = load("res://scripts/map/minimap_controller.gd").new()
	controller.map = FakeMap.new()
	controller.fog_active = true  # brouillard par cellule : seule « seen » est en vue
	controller.fog_by_cell = true
	controller.visible_armies = {"seen": true}
	controller.minimap = CampaignMinimap.new()
	var seen := {"id": "army:seen", "kind": "enemy_army", "army_id": "seen", "province_id": "p"}
	var hidden := {"id": "army:hidden", "kind": "enemy_army", "army_id": "hidden", "province_id": "p"}
	check(controller.on_alerts([seen]).is_empty(), "first series is silent (load / start)")
	check(controller.on_alerts([seen]).is_empty(), "known alert does not ping again")
	var pinged: Array = controller.on_alerts([seen, hidden, {"id": "debt", "kind": "debt"}])
	check(pinged.is_empty(), "hidden army and non-map alerts never ping")
	controller.on_alerts([])
	pinged = controller.on_alerts([seen])
	check(pinged.size() == 1 and pinged[0] == Vector2(100, 100), "a new visible enemy army pings its position: %s" % [pinged])
	check(controller.minimap.ping_count() == 1, "minimap received the ping")
	controller.minimap.free()
	controller.map.free()
	controller.free()
