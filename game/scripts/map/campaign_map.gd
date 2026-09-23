extends Node3D

## Scène principale de la carte de campagne : charge `data/map/`, assemble terrain,
## mer, rivières, côte, villes, caméra, picking et UI. Aucune règle de jeu ici.
##
## Options de ligne de commande (après `--`) :
##   --screenshot=<chemin.png>  capture la vue après quelques frames puis quitte.
##   --focus=<x>,<y>,<distance>  place la caméra (coordonnées carte) au démarrage.
## Touches de debug : F12 = capture dans docs/img/, F2 = bascule du pan par bords.

const SCREENSHOT_DELAY_FRAMES := 40

@onready var terrain: TerrainBuilder = $Terrain
@onready var sea: Sea = $Sea
@onready var rivers: RiversRenderer = $Rivers
@onready var coast: CoastRenderer = $Coast
@onready var cities: CityMarkers = $Cities
@onready var camera_rig: CampaignCamera = $CameraRig
@onready var camera: Camera3D = $CameraRig/Camera3D
@onready var picker: ProvincePicker = $Picker
@onready var ui: MapUI = $UI

var map_data: MapData
var load_ok: bool = false
var campaign: Object = null  # CampaignSim (GDExtension), null si non chargée
var hovered_index: int = 0
var selected_index: int = 0
var startup_stats: Dictionary = {}

var _screenshot_path: String = ""
var _screenshot_countdown: int = -1


func _ready() -> void:
	var t0 := Time.get_ticks_msec()
	var map_dir: String = MapPaths.map_dir()
	map_data = MapData.load_from_dir(map_dir)
	if map_data.load_error != "":
		ui.set_date("Carte introuvable : %s" % map_dir)
		push_error("CampaignMap: cannot load map from %s (%s)" % [map_dir, map_data.load_error])
		return
	var t1 := Time.get_ticks_msec()
	_configure_lod()
	terrain.build(map_data)
	var t2 := Time.get_ticks_msec()
	sea.setup(map_data.size)
	rivers.minor_max_distance = maxf(map_data.size.x, map_data.size.y) * 0.35
	rivers.build(map_data)
	coast.build(map_data)
	cities.label_max_distance = maxf(map_data.size.x, map_data.size.y) * 0.35
	cities.build(map_data)
	var t3 := Time.get_ticks_msec()

	var bounds := Rect2(Vector2.ZERO, Vector2(map_data.size))
	camera_rig.setup(bounds, maxf(map_data.size.x, map_data.size.y) * 0.55)
	picker.setup(camera, map_data)
	picker.province_hovered.connect(_on_province_hovered)
	picker.province_selected.connect(_on_province_selected)
	ui.end_turn_pressed.connect(_on_end_turn)
	_setup_campaign()
	load_ok = true
	startup_stats = {
		"load_ms": t1 - t0,
		"terrain_ms": t2 - t1,
		"decor_ms": t3 - t2,
		"total_ms": Time.get_ticks_msec() - t0,
		"data": map_data.timings,
		"terrain": terrain.build_stats,
	}
	print("CampaignMap: %s" % JSON.stringify(startup_stats))
	_parse_cmdline()


## Pas de sommets proportionnels à la taille de carte (4096 → 4/8, 512 → 1/2).
func _configure_lod() -> void:
	var scale := float(maxi(map_data.size.x, map_data.size.y)) / 4096.0
	terrain.near_step = clampi(int(round(4.0 * scale)), 1, 4)
	terrain.far_step = clampi(int(round(8.0 * scale)), 2, 8)
	terrain.near_distance = maxf(terrain.chunk_px_for(map_data.size) * 2.0, 150.0)


func _setup_campaign() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		push_warning("CampaignMap: CampaignSim not available (run core/build.sh)")
		ui.set_date("Printemps 1337 (simulation absente)")
		return
	campaign = ClassDB.instantiate("CampaignSim")
	campaign.call("new_campaign", 1337)
	_refresh_date()


func _refresh_date() -> void:
	if campaign == null:
		return
	ui.set_date("%s — tour %d" % [campaign.call("get_date_label"), campaign.call("get_turn")])


func _on_end_turn() -> void:
	if campaign == null:
		return
	campaign.call("end_turn")
	_refresh_date()


func _on_province_hovered(index: int) -> void:
	hovered_index = index
	terrain.set_highlight(hovered_index, selected_index)
	ui.set_hovered(map_data.get_province(index))


func _on_province_selected(index: int) -> void:
	selected_index = index
	terrain.set_highlight(hovered_index, selected_index)
	ui.show_province(map_data.get_province(index))


func _process(_delta: float) -> void:
	if not load_ok:
		return
	terrain.update_lod(camera.global_position)
	cities.update_visibility(camera_rig.distance)
	rivers.update_visibility(camera_rig.distance)
	if _screenshot_countdown > 0:
		_screenshot_countdown -= 1
		if _screenshot_countdown == 0:
			_take_screenshot(_screenshot_path, true)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("map_screenshot"):
		var path := MapPaths.project_root().path_join("docs/img/godot-map-%d.png" % Time.get_unix_time_from_system())
		_take_screenshot(path, false)
	elif event.is_action_pressed("map_toggle_edge_pan"):
		camera_rig.edge_pan_enabled = not camera_rig.edge_pan_enabled


func _parse_cmdline() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			_screenshot_countdown = SCREENSHOT_DELAY_FRAMES
			camera_rig.edge_pan_enabled = false
			# Sélection d'une province pour la capture : montre surbrillance + panneau.
			if map_data.province_count >= 3:
				picker.select_index(3)
		elif arg.begins_with("--focus="):
			var parts := arg.trim_prefix("--focus=").split(",")
			if parts.size() == 3:
				var x := float(parts[0])
				var y := float(parts[1])
				camera_rig.look_at_point(Vector3(x, map_data.surface_world_at(x, y), y), float(parts[2]))
				camera_rig.snap()


func _take_screenshot(path: String, quit_after: bool) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("CampaignMap: screenshot %s (%s)" % [path, error_string(err)])
	if quit_after:
		get_tree().quit(0 if err == OK else 1)
