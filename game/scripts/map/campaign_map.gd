extends Node3D

## Scène de la carte de campagne : charge `data/map/`, assemble terrain, mer, rivières,
## côte, villes, armées, caméra, picking et HUD, et relie l'interface à la simulation
## (`SimFacade.sim` : `CampaignSim` Rust ou mock). Aucune règle de jeu ici : la scène
## affiche l'état, soumet des ordres et rafraîchit après chaque réponse.
##
## Options de ligne de commande (après `--`) :
##   --screenshot=<chemin.png>  capture la vue après quelques frames puis quitte
##                              (sélectionne la première armée du joueur + aperçu de chemin).
##   --focus=<x>,<y>,<distance>  place la caméra (coordonnées carte) au démarrage.
## Touches de debug : F12 = capture dans docs/img/, F2 = bascule du pan par bords.

const SCREENSHOT_DELAY_FRAMES := 40
const START_MENU_SCENE := "res://scenes/start_menu.tscn"

@onready var terrain: TerrainBuilder = $Terrain
@onready var sea: Sea = $Sea
@onready var rivers: RiversRenderer = $Rivers
@onready var coast: CoastRenderer = $Coast
@onready var cities: CityMarkers = $Cities
@onready var armies: ArmyMarkers = $Armies
@onready var path_preview: PathPreview = $PathPreview
@onready var camera_rig: CampaignCamera = $CameraRig
@onready var camera: Camera3D = $CameraRig/Camera3D
@onready var picker: ProvincePicker = $Picker
@onready var ui: MapUI = $UI

var map_data: MapData
var load_ok: bool = false
var sim: Object = null  # SimFacade.sim (CampaignSim ou CampaignSimMock), null si échec
var player_faction: String = ""
var hovered_index: int = 0
var selected_index: int = 0
var selected_army: String = ""
## Provinces atteignables ce tour par l'armée sélectionnée : id → coût.
var reachable: Dictionary = {}
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
	var map_extent := maxf(map_data.size.x, map_data.size.y)
	rivers.minor_max_distance = map_extent * 0.35
	rivers.build(map_data)
	coast.build(map_data)
	# Étiquettes visibles quand peu de provinces sont à l'écran : seuil ∝ 1/√(nombre de provinces).
	cities.label_max_distance = map_extent * 0.35 * sqrt(20.0 / maxf(map_data.province_count, 1.0))
	cities.build(map_data)
	var t3 := Time.get_ticks_msec()

	var bounds := Rect2(Vector2.ZERO, Vector2(map_data.size))
	camera_rig.setup(bounds, maxf(map_data.size.x, map_data.size.y) * 0.55)
	picker.setup(camera, map_data)
	picker.province_hovered.connect(_on_province_hovered)
	picker.province_selected.connect(_on_province_selected)
	picker.province_right_clicked.connect(_on_province_right_clicked)
	picker.click_interceptor = _try_select_army
	armies.setup(map_data, camera)
	path_preview.setup(map_data)
	_connect_ui()
	_setup_campaign()
	load_ok = true
	startup_stats = {
		"load_ms": t1 - t0,
		"terrain_ms": t2 - t1,
		"decor_ms": t3 - t2,
		"total_ms": Time.get_ticks_msec() - t0,
		"data": map_data.timings,
		"terrain": terrain.build_stats,
		"heightmap_decoder": map_data.height_decoder,
	}
	print("CampaignMap: %s" % JSON.stringify(startup_stats))
	_parse_cmdline()


## Pas de sommets proportionnels à la taille de carte (4096 → 4/8, 512 → 1/2).
func _configure_lod() -> void:
	var scale := float(maxi(map_data.size.x, map_data.size.y)) / 4096.0
	terrain.near_step = clampi(int(round(4.0 * scale)), 1, 4)
	terrain.far_step = clampi(int(round(8.0 * scale)), 2, 8)
	terrain.near_distance = maxf(terrain.chunk_px_for(map_data.size) * 2.0, 150.0)


func _connect_ui() -> void:
	ui.end_turn_pressed.connect(_on_end_turn)
	ui.save_requested.connect(_on_save)
	ui.load_requested.connect(_on_load)
	ui.main_menu_requested.connect(func() -> void: get_tree().change_scene_to_file(START_MENU_SCENE))
	ui.quit_requested.connect(func() -> void: get_tree().quit())
	ui.recruit_requested.connect(_on_recruit)
	ui.create_army_requested.connect(_on_create_army)
	ui.stance_changed.connect(_on_stance_changed)
	ui.army_panel_closed.connect(func() -> void: deselect_army())
	ui.province_panel_closed.connect(func() -> void:
		selected_index = 0
		terrain.set_highlight(hovered_index, 0))


# --- Campagne ------------------------------------------------------------------------


func _setup_campaign() -> void:
	var ok: bool
	if SimFacade.pending_load_path != "":
		ok = SimFacade.load_game(SimFacade.pending_load_path)
		SimFacade.pending_load_path = ""
	else:
		ok = SimFacade.new_campaign(SimFacade.pending_faction, SimFacade.pending_seed)
	sim = SimFacade.sim
	if not ok:
		sim = null
		ui.set_date("Simulation indisponible")
		ui.show_toast("Impossible de démarrer la campagne.", true)
		push_error("CampaignMap: campaign could not start")
		return
	player_faction = str(sim.call("get_player_faction"))
	ui.clear_log()
	ui.add_events(sim.call("get_events"), "%s (simulation %s)" % [sim.call("get_date_label"), SimFacade.engine_label()])
	refresh_all()
	_focus_first_player_army()


## Rafraîchit couleurs, marqueurs, barre supérieure et panneaux après tout changement d'état.
func refresh_all() -> void:
	if sim == null:
		return
	_refresh_owner_colors()
	armies.refresh(sim, SimFacade.faction_color, player_faction)
	_refresh_top_bar()
	if selected_army != "":
		if armies.has_army(selected_army):
			select_army(selected_army)
		else:
			deselect_army()
	if selected_index > 0:
		_show_province_panel(selected_index)


func _refresh_top_bar() -> void:
	ui.set_faction(SimFacade.faction_short_name(player_faction), SimFacade.faction_color(player_faction))
	ui.set_date("%s — tour %d" % [sim.call("get_date_label"), sim.call("get_turn")])
	var summary: Dictionary = sim.call("get_faction_summary", player_faction)
	ui.set_treasury(int(summary.get("treasury", 0)), int(summary.get("income", 0)))


## Couleur de chaque province = couleur héraldique du propriétaire courant (simulation),
## initialisée par `GameDataStore.get_province_owner_colors` ; palette de repli sans store.
func _refresh_owner_colors() -> void:
	var ids := PackedStringArray()
	ids.resize(map_data.province_count)
	for index in range(1, map_data.province_count + 1):
		ids[index - 1] = str(map_data.get_province(index).get("id", ""))
	var colors := PackedColorArray()
	if SimFacade.store_loaded():
		colors = SimFacade.store.call("get_province_owner_colors", ids)
	if colors.size() != ids.size():
		colors.resize(ids.size())
		colors.fill(Color(0, 0, 0, 0))
	var fallback_by_owner: Dictionary = {}
	for i in ids.size():
		var state: Dictionary = sim.call("get_province_state", ids[i])
		var owner: String = str(state.get("owner", map_data.get_province(i + 1).get("owner", "")))
		if owner == "":
			colors[i] = Color(0, 0, 0, 0)
		elif SimFacade.store_loaded():
			colors[i] = SimFacade.faction_color(owner)
		else:
			if not fallback_by_owner.has(owner):
				fallback_by_owner[owner] = TerrainBuilder.FALLBACK_PALETTE[fallback_by_owner.size() % TerrainBuilder.FALLBACK_PALETTE.size()]
			colors[i] = fallback_by_owner[owner]
		colors[i].a = 1.0 if owner != "" else 0.0
	terrain.set_province_colors(colors)


func _focus_first_player_army() -> void:
	var ids := player_army_ids()
	if ids.is_empty():
		return
	var world := armies.world_position_of(ids[0])
	camera_rig.look_at_point(world, maxf(map_data.size.x, map_data.size.y) * 0.12)
	camera_rig.snap()


func player_army_ids() -> PackedStringArray:
	var result := PackedStringArray()
	if sim == null:
		return result
	for army_id in sim.call("get_army_ids"):
		if str(sim.call("get_army", army_id).get("faction", "")) == player_faction:
			result.append(army_id)
	return result


# --- Sélection ---------------------------------------------------------------------


## Intercepteur de clic gauche du picker : vrai si une armée a été cliquée.
func _try_select_army(screen_position: Vector2) -> bool:
	var army_id := armies.pick_screen(screen_position)
	if army_id == "":
		return false
	select_army(army_id)
	return true


func select_army(army_id: String) -> void:
	if sim == null:
		return
	var army: Dictionary = sim.call("get_army", army_id)
	if army.is_empty():
		deselect_army()
		return
	selected_army = army_id
	armies.set_selected(army_id)
	var faction: String = str(army.get("faction", ""))
	var is_player := faction == player_faction
	reachable = sim.call("get_reachable", army_id) if is_player else {}
	_apply_reachable_mask(PackedInt32Array())
	ui.show_army(army_id, army, SimFacade.faction_short_name(faction), SimFacade.faction_color(faction), is_player, province_name_of)
	ui.hide_province()
	selected_index = 0
	terrain.set_highlight(hovered_index, 0)
	# Le chemin en cours (ordre déjà donné) est prévisualisé.
	var path: Array = army.get("path", [])
	if not path.is_empty():
		var ids := PackedStringArray([str(army.get("location", ""))])
		for step in path:
			ids.append(str(step))
		path_preview.show_path(ids, camera_rig.distance)
	else:
		path_preview.hide_path()


func deselect_army() -> void:
	selected_army = ""
	reachable = {}
	armies.set_selected("")
	terrain.set_reachable(PackedInt32Array(), PackedInt32Array())
	path_preview.hide_path()
	ui.hide_army()


func _apply_reachable_mask(path_indices: PackedInt32Array) -> void:
	var indices := PackedInt32Array()
	for province_id in reachable:
		var index := map_data.index_of_id(str(province_id))
		if index > 0:
			indices.append(index)
	terrain.set_reachable(indices, path_indices)


func province_name_of(province_id: String) -> String:
	if SimFacade.store_loaded():
		var info: Dictionary = SimFacade.store.call("get_province", province_id)
		if not info.is_empty():
			return str(info.get("display_name", province_id))
	var province := map_data.get_province(map_data.index_of_id(province_id))
	return str(province.get("name", province_id))


## Entrée MapData enrichie des champs `GameDataStore.get_province` (noms d'affichage).
func province_info(index: int) -> Dictionary:
	var province := map_data.get_province(index).duplicate()
	if province.is_empty():
		return province
	if SimFacade.store_loaded():
		var info: Dictionary = SimFacade.store.call("get_province", str(province["id"]))
		for key in info:
			if key != "centroid" and key != "capital_px" and key != "neighbors":
				province[key] = info[key]
	return province


func _on_province_hovered(index: int) -> void:
	hovered_index = index
	terrain.set_highlight(hovered_index, selected_index)
	var province := province_info(index)
	if selected_army == "" or index == 0 or sim == null:
		ui.set_hovered(province)
		if selected_army != "":
			_apply_reachable_mask(PackedInt32Array())
			path_preview.hide_path()
		return
	_preview_path(str(province["id"]), str(province.get("display_name", province.get("name", ""))))


## Aperçu du chemin de l'armée sélectionnée vers `target_id` : ruban, masque, coût.
func _preview_path(target_id: String, target_name: String) -> void:
	var army: Dictionary = sim.call("get_army", selected_army)
	var path: PackedStringArray = sim.call("find_path", selected_army, target_id)
	var path_indices := PackedInt32Array()
	for step in path:
		path_indices.append(map_data.index_of_id(step))
	_apply_reachable_mask(path_indices)
	if path.is_empty():
		path_preview.hide_path()
		ui.set_hover_path(target_name, 0, 0, false)
		return
	var ids := PackedStringArray([str(army.get("location", ""))])
	ids.append_array(path)
	path_preview.show_path(ids, camera_rig.distance)
	ui.set_hover_path(target_name, path.size(), int(reachable.get(target_id, 0)), reachable.has(target_id))


func _on_province_selected(index: int) -> void:
	if selected_army != "":
		deselect_army()
	selected_index = index
	terrain.set_highlight(hovered_index, selected_index)
	if index == 0:
		ui.hide_province()
		return
	_show_province_panel(index)


func _show_province_panel(index: int) -> void:
	var province := province_info(index)
	if province.is_empty() or sim == null:
		ui.show_province(province)
		return
	var province_id: String = str(province["id"])
	var state: Dictionary = sim.call("get_province_state", province_id)
	var is_player_owner := str(state.get("owner", "")) == player_faction and str(state.get("controller", state.get("owner", ""))) == player_faction
	var recruitable: Array = sim.call("get_recruitable", province_id) if is_player_owner else []
	ui.show_province(province, state, recruitable, is_player_owner, SimFacade.faction_short_name)


## Clic droit : ordre de déplacement de l'armée sélectionnée vers la province visée.
func _on_province_right_clicked(index: int) -> void:
	if selected_army == "" or index == 0 or sim == null:
		return
	var target_id: String = str(map_data.get_province(index).get("id", ""))
	var result := order_move(selected_army, target_id)
	if not result["ok"]:
		ui.show_toast(str(result.get("error", "Ordre refusé")), true)


## Soumet un ordre `move_army` via `find_path` ; renvoie la réponse de la simulation.
func order_move(army_id: String, target_id: String) -> Dictionary:
	var path: PackedStringArray = sim.call("find_path", army_id, target_id)
	if path.is_empty():
		return {"ok": false, "error": "Aucun chemin vers %s." % province_name_of(target_id)}
	var order := {"type": "move_army", "army": army_id, "path": Array(path)}
	var result: Dictionary = sim.call("submit_order", order)
	if result.get("ok", false):
		refresh_all()
	return result


func _on_stance_changed(army_id: String, stance: String) -> void:
	_submit({"type": "set_stance", "army": army_id, "stance": stance}, "Posture modifiée.")


func _on_recruit(province_id: String, unit_type: String) -> void:
	_submit({"type": "recruit", "province": province_id, "unit_type": unit_type}, "Recrutement lancé : l'unité rejoindra la garnison au prochain tour.")


func _on_create_army(province_id: String, unit_indices: Array) -> void:
	var result := _submit({"type": "create_army", "province": province_id, "units_from_garrison": unit_indices}, "Armée formée.")
	if result.get("ok", false) and result.has("army"):
		select_army(str(result["army"]))


func _submit(order: Dictionary, success_text: String) -> Dictionary:
	if sim == null:
		return {"ok": false, "error": "Simulation absente"}
	var result: Dictionary = sim.call("submit_order", order)
	if result.get("ok", false):
		ui.show_toast(success_text)
		refresh_all()
	else:
		ui.show_toast(str(result.get("error", "Ordre refusé")), true)
	return result


func _on_end_turn() -> void:
	if sim == null or ui.is_dialog_open():
		return
	var events: Array = sim.call("end_turn")
	ui.add_events(events, str(sim.call("get_date_label")))
	refresh_all()
	for event in events:
		if str(event.get("kind", "")) == "battle":
			ui.show_toast(str(event.get("text_fr", "Bataille")))
			break


# --- Sauvegarde ----------------------------------------------------------------------


func _on_save(save_name: String) -> void:
	if sim == null:
		return
	if SimFacade.save_game(save_name):
		ui.show_toast("Partie sauvegardée : %s" % save_name)
	else:
		ui.show_toast("Échec de la sauvegarde.", true)


func _on_load(path: String) -> void:
	if not SimFacade.load_game(path):
		ui.show_toast("Impossible de charger cette sauvegarde.", true)
		return
	sim = SimFacade.sim
	player_faction = str(sim.call("get_player_faction"))
	deselect_army()
	selected_index = 0
	ui.hide_province()
	ui.clear_log()
	ui.add_events(sim.call("get_events"), "%s (partie chargée)" % sim.call("get_date_label"))
	refresh_all()
	ui.show_toast("Partie chargée.")


# --- Boucle et debug -----------------------------------------------------------------


func _process(_delta: float) -> void:
	if not load_ok:
		return
	terrain.update_lod(camera.global_position)
	cities.update_visibility(camera_rig.distance)
	rivers.update_visibility(camera_rig.distance)
	armies.update_scale(camera_rig.distance)
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
	elif event.is_action_pressed("ui_cancel") and selected_army != "":
		deselect_army()


func _parse_cmdline() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			_screenshot_countdown = SCREENSHOT_DELAY_FRAMES
			camera_rig.edge_pan_enabled = false
			_stage_screenshot()
		elif arg.begins_with("--focus="):
			var parts := arg.trim_prefix("--focus=").split(",")
			if parts.size() == 3:
				var x := float(parts[0])
				var y := float(parts[1])
				camera_rig.look_at_point(Vector3(x, map_data.surface_world_at(x, y), y), float(parts[2]))
				camera_rig.snap()


## Mise en scène pour la capture : armée du joueur sélectionnée, aperçu de chemin vers la
## province atteignable la plus lointaine, caméra cadrée sur l'armée.
func _stage_screenshot() -> void:
	var ids := player_army_ids()
	if ids.is_empty():
		if map_data.province_count >= 3:
			picker.select_index(3)
		return
	select_army(ids[0])
	var world := armies.world_position_of(ids[0])
	camera_rig.look_at_point(world, maxf(map_data.size.x, map_data.size.y) * 0.09)
	camera_rig.snap()
	var best := ""
	var best_cost := -1
	for province_id in reachable:
		if int(reachable[province_id]) > best_cost:
			best_cost = int(reachable[province_id])
			best = str(province_id)
	if best != "":
		var index := map_data.index_of_id(best)
		hovered_index = index
		terrain.set_highlight(index, 0)
		_preview_path(best, province_name_of(best))


func _take_screenshot(path: String, quit_after: bool) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("CampaignMap: screenshot %s (%s)" % [path, error_string(err)])
	if quit_after:
		get_tree().quit(0 if err == OK else 1)
