extends Node3D

## Scène de la carte de campagne : charge `data/map/`, assemble terrain, mer, rivières,
## côte, villes, armées, caméra, picking et HUD, et relie l'interface à la simulation
## (`SimFacade.sim` : `CampaignSim` Rust ou mock). Aucune règle de jeu ici : la scène
## affiche l'état, soumet des ordres et rafraîchit après chaque réponse.
##
## Options de ligne de commande (après `--`) :
##   --screenshot=<chemin.png>  capture la vue après quelques frames puis quitte
##                              (sélectionne la première armée du joueur + aperçu de chemin).
##   --stage=map                avec --screenshot : carte seule (aucune sélection, aucun panneau).
##   --stage=province           avec --screenshot : sélectionne plutôt la capitale du joueur
##                              et ouvre le panneau de recrutement.
##   --stage=city               capitale du joueur, panneau de province sur l'onglet Ville.
##   --stage=faction            panneau de faction (trésor, revenus, impôts, biens).
##   --stage=tech               panneau des technologies (une recherche lancée, M6) ;
##   --stage=tech_civil         idem sur l'onglet Civil.
##   --stage=battle             bataille France–Angleterre mise en scène, dialogue d'avant-bataille (M7).
##   --stage=tooltips           recrutement de la capitale + infobulles riches figées (F2).
##   --stage=tutorial|encyclopedia  étape du tutoriel / fiche d'encyclopédie (F8).
##   --focus=<x>,<y>,<distance>  place la caméra (coordonnées carte) au démarrage.
##   --select-settlement=<id>    sélectionne une colonie (surbrillance, lot C6).
##   --stage=agents             C6 : espion, héraut et prédicateur recrutés, espion sélectionné ;
##   --stage=agents_registry    idem, registre des agents (G) ouvert.
##   --stage=settlement|settlement_orders  panneau d'une ville du joueur / armée, colonies
##                              atteignables et chemin sur le graphe (lot C5).
##   --fps-probe                 imprime les FPS moyens après la mise en place (lot C6).
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
@onready var construction_markers: ConstructionMarkers = $ConstructionMarkers

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
var unrest_mode: bool = false
var _faction_panel_id: String = ""
var _court_open: bool = false
var _open_character_id: String = ""
var diplomacy: DiplomacyController = null  # M5
var sieges: SiegeController = null  # M8
var victory: VictoryController = null  # M10
var help: HelpController = null  # M10
var _tech_open: bool = false  # M6
var chronicle: ChronicleController = null  # M10
var hud: HudController = null  # F10b : bandeau d'ost, sceau, cloche et alertes, lettres
var flow: FlowController = null  # F3 : pause, réglages, sauvegardes, rapport, alertes
var tutorial: TutorialController = null  # F8 : tutoriel, encyclopédie (K)
## Lot C6 : paliers de zoom, colonies, hameaux et routes.
var zoom_tiers: ZoomTiers = null
var settlement_data: SettlementData = null
var settlement_layer: SettlementLayer = null
var roads: RoadRenderer = null
var _fps_probe_frames: int = -1
var _fps_probe_start: int = 0
var _fps_probe_gpu_ms: float = 0.0
var _fps_probe_cpu_ms: float = 0.0
## Temps cumulés (µs) : LOD du terrain, couches C6 (colonies, routes).
var _fps_probe_map_us: Vector2 = Vector2.ZERO
var minimap_ctl: MinimapController = null  # C1 : minicarte, brouillard de guerre
var settlements_ctl: SettlementController = null  # C5 : panneau de colonie, ordres par colonie
var agents_ctl: AgentController = null  # C6 (agents) : espions, hérauts, prédicateurs

var _screenshot_path: String = ""
var _screenshot_countdown: int = -1
var _screenshot_stage: String = "army"


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
	cities.labels_only = true  # C6 : noms de provinces (palier loin), colonies à part
	cities.build(map_data)
	_setup_settlements()
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
	settlements_ctl = SettlementController.new()  # C5
	add_child(settlements_ctl)
	settlements_ctl.setup(self)
	agents_ctl = AgentController.new()  # C6 agents (après C5 : chaîne ses intercepteurs de clic)
	add_child(agents_ctl)
	agents_ctl.setup(self)
	minimap_ctl = MinimapController.new()  # C1
	minimap_ctl.name = "MinimapController"
	add_child(minimap_ctl)
	minimap_ctl.setup(self)
	diplomacy = DiplomacyController.new()
	add_child(diplomacy)
	diplomacy.setup(self)
	sieges = SiegeController.new()
	add_child(sieges)
	sieges.setup(self)
	victory = VictoryController.new()
	add_child(victory)
	# M10 : chronique (fenêtre de décision, bouton de la barre).
	chronicle = ChronicleController.new()
	add_child(chronicle)
	chronicle.setup(self)
	_setup_campaign()
	victory.setup(self)
	help = HelpController.new()
	add_child(help)
	help.setup(self)
	# U1 : objectifs et aide dans la pile des panneaux (exclusifs, Échap).
	ui.register_panel(victory.panel, PanelStack.Kind.CENTRAL)
	ui.register_panel(victory.end_dialog, PanelStack.Kind.MODAL)
	ui.register_panel(help.panel, PanelStack.Kind.CENTRAL)
	flow = FlowController.new()  # F3
	add_child(flow)
	flow.setup(self)
	hud = HudController.new()  # F10b
	hud.name = "HudController"
	add_child(hud)
	hud.setup(self)
	tutorial = TutorialController.new()  # F8
	add_child(tutorial)
	tutorial.setup(self)
	var audio_director := get_node_or_null("/root/AudioDirector")  # M10 assets
	if audio_director != null:
		audio_director.attach_campaign(self)
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


## Lot C6 : colonies (icônes, maquettes, étiquettes), hameaux et routes, paliers de zoom.
func _setup_settlements() -> void:
	zoom_tiers = ZoomTiers.load_default()
	settlement_data = SettlementData.load_from(MapPaths.data_dir, MapPaths.map_dir())
	roads = RoadRenderer.new()
	roads.name = "Roads"
	add_child(roads)
	roads.build(map_data, settlement_data, terrain)
	settlement_layer = SettlementLayer.new()
	settlement_layer.name = "Settlements"
	add_child(settlement_layer)
	settlement_layer.setup(map_data, terrain, settlement_data, zoom_tiers)
	settlement_layer.settlement_selected.connect(_on_settlement_selected)
	armies.settlement_position = settlement_layer.world_position_of  # C4
	var vegetation := get_node_or_null("Vegetation")
	if vegetation != null:
		vegetation.set("extra_exclusions", settlement_layer.vegetation_exclusions())


## Lot C6 : sélection d'une colonie (le panneau viendra au lot C5).
func _on_settlement_selected(settlement_id: String) -> void:
	if settlements_ctl != null and settlements_ctl.available():
		return  # C5 : le panneau de colonie s'ouvre
	var entry := settlement_data.get_settlement(settlement_id) if settlement_data != null else {}
	if not entry.is_empty():
		ui.show_toast("%s (%s)" % [entry.get("name", settlement_id), province_name_of(str(entry.get("province", "")))])


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
	ui.main_menu_requested.connect(func() -> void:
		if flow != null:
			flow.request_exit("main_menu")
		else:
			get_tree().change_scene_to_file(START_MENU_SCENE))
	ui.quit_requested.connect(func() -> void:
		if flow != null:
			flow.request_exit("quit")
		else:
			get_tree().quit())
	ui.recruit_requested.connect(_on_recruit)
	ui.create_army_requested.connect(_on_create_army)
	ui.build_requested.connect(_on_build)
	ui.cancel_build_requested.connect(_on_cancel_build)
	ui.tax_rate_changed.connect(_on_tax_rate_changed)
	ui.faction_panel_requested.connect(_on_faction_panel_requested)
	ui.court_panel_requested.connect(_on_court_panel_requested)
	ui.province_court_requested.connect(_on_province_court_requested)
	ui.character_selected.connect(_on_character_selected)
	ui.governor_requested.connect(_on_governor_requested)
	ui.general_requested.connect(_on_general_requested)
	ui.marriage_requested.connect(_on_marriage_requested)
	ui.learn_skill_requested.connect(_on_learn_skill_requested)
	ui.stance_changed.connect(_on_stance_changed)
	ui.tech_panel_requested.connect(_on_tech_panel_requested)  # M6
	ui.research_requested.connect(_on_research_requested)  # M6
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
	ui.journal_player_faction = player_faction
	ui.journal_faction_name = SimFacade.faction_short_name
	ui.clear_log()
	ui.add_events(sim.call("get_events"), "%s (simulation %s)" % [sim.call("get_date_label"), SimFacade.engine_label()])
	refresh_all()
	_focus_first_player_army()


## Rafraîchit couleurs, marqueurs, barre supérieure et panneaux après tout changement d'état.
func refresh_all() -> void:
	if sim == null:
		return
	_refresh_owner_colors()
	if minimap_ctl != null:  # C1 : brouillard avant les marqueurs d'armée
		minimap_ctl.refresh_fog()
	armies.refresh(sim, SimFacade.faction_color, player_faction)
	if settlement_layer != null:  # C6
		settlement_layer.refresh(sim, SimFacade.faction_color)
	if minimap_ctl != null:
		minimap_ctl.refresh()
	_refresh_top_bar()
	_refresh_construction_markers()
	if settlements_ctl != null:  # C5
		settlements_ctl.refresh()
	if agents_ctl != null:  # C6 agents
		agents_ctl.refresh()
	if unrest_mode:
		_refresh_unrest_colors()
	if selected_army != "":
		if armies.has_army(selected_army):
			select_army(selected_army)
		else:
			deselect_army()
	if selected_index > 0:
		_show_province_panel(selected_index)
	if ui.faction_panel_visible() and _faction_panel_id != "":
		_show_faction_panel(_faction_panel_id)
	# U1 : un panneau fermé (×, Échap, exclusivité) ne se rouvre pas au rafraîchissement.
	_court_open = _court_open and ui.court_panel_visible()
	if _court_open:
		_show_court_panel()
	if _open_character_id != "" and ui.character_sheet.visible:
		_show_character_sheet(_open_character_id)
	else:
		_open_character_id = ""
	if diplomacy != null:
		diplomacy.refresh()
	if chronicle != null:  # M10
		chronicle.refresh()
	_refresh_research()  # M6
	_tech_open = _tech_open and ui.tech_panel_visible()
	if _tech_open:
		_show_tech_panel()
	if flow != null:  # F3
		flow.refresh()
	if hud != null:  # F10b
		hud.refresh()


func _city_available() -> bool:
	return sim != null and sim.has_method("get_province_city")


func _economy_available() -> bool:
	return sim != null and sim.has_method("get_faction_economy")


func _characters_available() -> bool:
	return sim != null and sim.has_method("get_character") and sim.has_method("get_faction_characters")


func _refresh_top_bar() -> void:
	ui.set_faction(SimFacade.faction_short_name(player_faction), SimFacade.faction_color(player_faction))
	PortraitLoader.overlay_heraldry(ui.faction_swatch, player_faction, Vector2(22, 26))  # M10 assets
	# Le moteur compte les tours à partir de 0 ; le joueur commence au tour 1 (audit A3 C2).
	ui.set_date("%s — tour %d" % [sim.call("get_date_label"), int(sim.call("get_turn")) + 1])
	var summary: Dictionary = sim.call("get_faction_summary", player_faction)
	var economy: Dictionary = {}
	if _economy_available():
		economy = sim.call("get_faction_economy", player_faction)
	ui.set_treasury(int(summary.get("treasury", 0)), int(summary.get("income", 0)), economy)


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
	if minimap_ctl != null:  # C1
		minimap_ctl.set_province_colors(colors)


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
		# C6 : clic sur une icône ou une maquette de colonie.
		var settlement_id := settlement_layer.pick_screen(screen_position) if settlement_layer != null else ""
		if settlement_id == "":
			return false
		settlement_layer.select(settlement_id)
		return true
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
	if agents_ctl != null:  # C6 agents : une seule sélection à la fois
		agents_ctl.deselect()
	var faction: String = str(army.get("faction", ""))
	var is_player := faction == player_faction
	reachable = sim.call("get_reachable", army_id) if is_player else {}
	_apply_reachable_mask(PackedInt32Array())
	if settlements_ctl != null:  # C5 : colonies atteignables
		settlements_ctl.on_army_selected(army_id, is_player)
	if hud != null:  # F10b : bandeau d'ost et sceau du chef
		hud.show_army(army_id, army, is_player)
	if sieges != null:
		sieges.on_army_shown(army_id, is_player)
	ui.hide_province()
	selected_index = 0
	terrain.set_highlight(hovered_index, 0)
	# Le chemin en cours (ordre déjà donné) est prévisualisé.
	if settlements_ctl != null and settlements_ctl.show_current_path(army):
		return
	var path: Array = army.get("path_provinces", army.get("path", []))
	if not path.is_empty():
		var ids := PackedStringArray([str(army.get("location_province", army.get("location", "")))])
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
	if settlements_ctl != null:  # C5
		settlements_ctl.on_army_deselected()
	path_preview.hide_path()
	ui.hide_army()


func _apply_reachable_mask(path_indices: PackedInt32Array) -> void:
	var indices := PackedInt32Array()
	if selected_army != "" and sim != null:
		# La province de départ compte comme atteignable (pas assombrie).
		var selected: Dictionary = sim.call("get_army", selected_army)
		indices.append(map_data.index_of_id(str(selected.get("location_province", selected.get("location", "")))))
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
	if settlements_ctl != null and settlements_ctl.preview_hover(target_id, target_name):
		return  # C5 : chemin sur le graphe des colonies
	var army: Dictionary = sim.call("get_army", selected_army)
	# C4 : le chemin réel passe par des colonies ; l'aperçu reste par province.
	var path: PackedStringArray = sim.call("find_path_provinces", selected_army, target_id) if sim.has_method("find_path_provinces") else sim.call("find_path", selected_army, target_id)
	var path_indices := PackedInt32Array()
	for step in path:
		path_indices.append(map_data.index_of_id(step))
	_apply_reachable_mask(path_indices)
	if path.is_empty():
		path_preview.hide_path()
		ui.set_hover_path(target_name, 0, 0, false)
		return
	var ids := PackedStringArray([str(army.get("location_province", army.get("location", "")))])
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
	var city: Dictionary = sim.call("get_province_city", province_id) if _city_available() else {}
	ui.show_province(province, state, recruitable, is_player_owner, SimFacade.faction_short_name, city)


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


func _on_build(province_id: String, building_id: String) -> void:
	_submit({"type": "build", "province": province_id, "building": building_id}, "Construction lancée.")


func _on_cancel_build(province_id: String) -> void:
	_submit({"type": "cancel_build", "province": province_id}, "Construction annulée (moitié du coût remboursée).")


func _on_tax_rate_changed(faction_id: String, rate: String) -> void:
	var result := _submit({"type": "set_tax_rate", "rate": rate}, "Taux d'imposition modifié.")
	if result.get("ok", false):
		_show_faction_panel(faction_id)


func _on_faction_panel_requested() -> void:
	_show_faction_panel(player_faction)


func _show_faction_panel(faction_id: String) -> void:
	_faction_panel_id = faction_id
	var economy: Dictionary = sim.call("get_faction_economy", faction_id) if _economy_available() else {}
	ui.show_faction(faction_id, SimFacade.faction_short_name(faction_id), SimFacade.faction_color(faction_id), economy)


# --- Personnages et dynasties (M4) --------------------------------------------------------


func _on_court_panel_requested() -> void:
	if not _characters_available():
		ui.show_toast("Cour indisponible avec cette simulation.", true)
		return
	_court_open = true
	_show_court_panel()


func _show_court_panel(preset_filter: int = -1) -> void:
	var rows: Array[Dictionary] = []
	for id in sim.call("get_faction_characters", player_faction):
		var character: Dictionary = sim.call("get_character", id)
		if not character.is_empty():
			rows.append(character)
	ui.show_court(rows, SimFacade.faction_short_name(player_faction), SimFacade.faction_color(player_faction), preset_filter)


## Bouton « Cour » du panneau de province : ouvre la cour filtrée sur les gouverneurs.
func _on_province_court_requested() -> void:
	if not _characters_available():
		ui.show_toast("Cour indisponible avec cette simulation.", true)
		return
	_court_open = true
	_show_court_panel(CourtPanel.FILTER_GOVERNOR)


func _on_character_selected(character_id: String) -> void:
	if not _characters_available():
		return
	_open_character_id = character_id
	_show_character_sheet(character_id)


func _show_character_sheet(character_id: String) -> void:
	var character: Dictionary = sim.call("get_character", character_id)
	if character.is_empty():
		_open_character_id = ""
		ui.hide_character()
		return
	var skill_tree: Array = sim.call("get_skill_tree") if sim.has_method("get_skill_tree") else []
	var learnable: Array = sim.call("get_learnable", character_id) if sim.has_method("get_learnable") else []
	var candidates: Array = sim.call("get_marriage_candidates", character_id) if sim.has_method("get_marriage_candidates") else []
	ui.show_character(character, skill_tree, learnable, _governable_provinces(str(character.get("faction", ""))), _commandable_armies(character), candidates)


## Provinces contrôlées par la faction du personnage : `[{id, name}]`.
func _governable_provinces(faction_id: String) -> Array:
	var result: Array = []
	for index in range(1, map_data.province_count + 1):
		var province := map_data.get_province(index)
		var province_id: String = str(province.get("id", ""))
		var state: Dictionary = sim.call("get_province_state", province_id)
		if str(state.get("owner", "")) == faction_id and str(state.get("controller", state.get("owner", ""))) == faction_id:
			result.append({"id": province_id, "name": province_name_of(province_id)})
	return result


## Armées de la faction du personnage dans sa province actuelle : `[{id, name}]`.
func _commandable_armies(character: Dictionary) -> Array:
	var result: Array = []
	var faction_id: String = str(character.get("faction", ""))
	var location: String = str(character.get("location", ""))
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if str(army.get("faction", "")) == faction_id and str(army.get("location_province", army.get("location", ""))) == location:
			var general_name: String = str(army.get("general_name", ""))
			result.append({"id": army_id, "name": "Armée%s" % (" (général : %s)" % general_name if general_name != "" else "")})
	return result


func _on_governor_requested(character_id: String, province_id: String) -> void:
	_submit_character({"type": "assign_governor", "character": character_id, "province": province_id}, "Gouverneur nommé.")


func _on_general_requested(character_id: String, army_id: String) -> void:
	_submit_character({"type": "assign_general", "character": character_id, "army": army_id}, "Commandement confié.")


func _on_marriage_requested(character_id: String, spouse_id: String) -> void:
	_submit_character({"type": "propose_marriage", "character": character_id, "spouse": spouse_id}, "Mariage célébré.")


func _on_learn_skill_requested(character_id: String, skill_id: String) -> void:
	_submit_character({"type": "learn_skill", "character": character_id, "skill": skill_id}, "Compétence apprise.")


func _submit_character(order: Dictionary, success_text: String) -> Dictionary:
	var result := _submit(order, success_text)
	if result.get("ok", false):
		var character_id: String = str(order.get("character", ""))
		if character_id != "":
			_open_character_id = character_id
			_show_character_sheet(character_id)
		if _court_open:
			_show_court_panel()
	return result


# --- Technologies (M6) --------------------------------------------------------------------


func _tech_available() -> bool:
	return sim != null and sim.has_method("get_tech_tree")


func _refresh_research() -> void:
	if not _tech_available():
		ui.research_box.hide()
		return
	ui.research_box.show()
	ui.set_research_progress(sim.call("get_research", player_faction), int(sim.call("get_research_points", player_faction)))


func _on_tech_panel_requested() -> void:
	if not _tech_available():
		ui.show_toast("Technologies indisponibles avec cette simulation.", true)
		return
	_tech_open = true
	_show_tech_panel()


func _show_tech_panel() -> void:
	ui.show_tech_tree(sim.call("get_tech_tree", player_faction), sim.call("get_research", player_faction),
		int(sim.call("get_research_points", player_faction)),
		SimFacade.faction_short_name(player_faction), SimFacade.faction_color(player_faction))


func _on_research_requested(technology_id: String) -> void:
	_submit({"type": "research", "technology": technology_id}, "Recherche lancée.")


## Mise en scène « technologies » : première technologie militaire disponible lancée, deux
## tours joués pour montrer la progression, panneau ouvert.
func _stage_screenshot_tech() -> void:
	_focus_capital()
	ui.hide_province()
	if not _tech_available():
		return
	for node in sim.call("get_tech_tree", player_faction):
		if str(node.get("state", "")) == "available" and str(node.get("branch", "")) == "military":
			sim.call("submit_order", {"type": "research", "technology": str(node["id"])})
			break
	sim.call("end_turn")
	sim.call("end_turn")
	ui.add_events(sim.call("get_events"), str(sim.call("get_date_label")))
	selected_index = 0
	ui.hide_province()
	_on_tech_panel_requested()
	refresh_all()


## Mode d'affichage « mécontentement » (touche M) : teinte les provinces vert → rouge par
## mécontentement moyen pondéré au lieu de la couleur de faction. Sans `get_province_city`,
## le mode ne fait rien (bascule ignorée, notification).
func _toggle_unrest_mode() -> void:
	if not _city_available():
		ui.show_toast("Données de mécontentement indisponibles avec cette simulation.", true)
		return
	unrest_mode = not unrest_mode
	if unrest_mode:
		_refresh_unrest_colors()
	else:
		_refresh_owner_colors()


## Couleur par province = vert (0 mécontentement) → rouge (100), moyenne pondérée par classe.
func _refresh_unrest_colors() -> void:
	var colors := PackedColorArray()
	colors.resize(map_data.province_count)
	for index in range(1, map_data.province_count + 1):
		var id := str(map_data.get_province(index).get("id", ""))
		var city: Dictionary = sim.call("get_province_city", id) if id != "" else {}
		var classes: Dictionary = city.get("classes", {})
		var total := 0.0
		var weighted := 0.0
		for class_id in classes:
			var count := float(classes[class_id].get("count", 0))
			total += count
			weighted += count * float(classes[class_id].get("unrest", 0))
		var ratio := clampf(weighted / total, 0.0, 1.0) if total > 0.0 else 0.0
		var color := Color(0.20, 0.55, 0.20).lerp(Color(0.75, 0.15, 0.10), ratio)
		color.a = 1.0 if not classes.is_empty() else 0.0
		colors[index - 1] = color
	terrain.set_province_colors(colors)


## Marteau sur les provinces avec une construction en cours (`get_province_city`).
func _refresh_construction_markers() -> void:
	if not _city_available():
		return
	var ids := PackedStringArray()
	for index in range(1, map_data.province_count + 1):
		ids.append(str(map_data.get_province(index).get("id", "")))
	construction_markers.refresh(ids, _is_under_construction, _construction_marker_position)


func _is_under_construction(province_id: String) -> bool:
	if sim == null or not _city_available():
		return false
	var city: Dictionary = sim.call("get_province_city", province_id)
	return not (city.get("construction", {}) as Dictionary).is_empty()


func _construction_marker_position(province_id: String) -> Vector3:
	var centroid := map_data.centroid_of_id(province_id)
	return Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y) + 6.0, centroid.y)


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
	if flow != null and not flow.before_end_turn():  # F3 : confirmation (réglage)
		return
	_close_battle_dialog()  # M7 : les batailles laissées en attente sont auto-résolues
	var events: Array = sim.call("end_turn")
	ui.add_events(events, str(sim.call("get_date_label")))
	var audio := get_node_or_null("/root/AudioDirector")  # M10 assets
	if audio != null:
		audio.on_turn_events(events)
	refresh_all()
	if diplomacy != null:
		diplomacy.after_end_turn()
	if victory != null:
		victory.after_end_turn()
	if chronicle != null:  # M10
		chronicle.after_end_turn()
	for event in events:
		if str(event.get("kind", "")) == "battle":
			ui.show_toast(str(event.get("text_fr", "Bataille")))
			break
	if flow != null:  # F3 : sauvegarde auto, alertes, rapport de saison
		flow.after_end_turn(events)
	if hud != null:  # F10b : alertes de la cloche
		hud.after_end_turn(events)
	_offer_pending_battles()  # M7


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
	ui.journal_player_faction = player_faction
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
	var distance := camera_rig.distance
	var fine_distance := zoom_tiers.fine_terrain_distance if zoom_tiers != null else 0.0
	var t0 := Time.get_ticks_usec()
	terrain.update_lod(camera.global_position, distance, camera_rig.focus, fine_distance)
	cities.update_visibility(distance)
	var t1 := Time.get_ticks_usec()
	if zoom_tiers != null:  # C6 : paliers de zoom
		cities.set_tier_alpha(zoom_tiers.far_weight(distance))
		settlement_layer.update_view(distance)
		roads.update_view(zoom_tiers.medium_weight(distance), zoom_tiers.near_weight(distance))
	if _fps_probe_frames > 0:
		_fps_probe_map_us += Vector2(t1 - t0, Time.get_ticks_usec() - t1)
	_update_fps_probe()
	rivers.update_visibility(camera_rig.distance)
	armies.update_scale(camera_rig.distance)
	if _screenshot_countdown > 0:
		# C6 : la capture attend le relief fin et les rubans / hameaux des tuiles proches.
		if _screenshot_countdown == 3 and not terrain.fine_ready():
			terrain.wait_fine_jobs()
			return
		if _screenshot_countdown == 2 and settlement_layer != null:
			settlement_layer.flush()
			roads.flush(zoom_tiers.near_weight(distance))
		_screenshot_countdown -= 1
		if _screenshot_countdown == 0:
			_take_screenshot(_screenshot_path, true)


## `--fps-probe` : FPS moyen sur 240 images une fois le relief fin prêt (mesure de perf C6).
func _update_fps_probe() -> void:
	if _fps_probe_frames < 0:
		return
	if _fps_probe_frames == 0:
		if not terrain.fine_ready() or Engine.get_process_frames() < 120:
			return
		_fps_probe_start = Time.get_ticks_usec()
		_fps_probe_gpu_ms = 0.0
		_fps_probe_cpu_ms = 0.0
	_fps_probe_frames += 1
	# Temps de rendu mesurés (indépendants de la synchronisation verticale).
	var viewport_rid := get_viewport().get_viewport_rid()
	_fps_probe_gpu_ms += RenderingServer.viewport_get_measured_render_time_gpu(viewport_rid)
	_fps_probe_cpu_ms += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid) + RenderingServer.get_frame_setup_time_cpu()
	if _fps_probe_frames == 241:
		var seconds := (Time.get_ticks_usec() - _fps_probe_start) / 1000000.0
		print("CampaignMap: fps_probe %s" % JSON.stringify({
			"fps": snappedf(240.0 / seconds, 0.1),
			"gpu_ms": snappedf(_fps_probe_gpu_ms / 240.0, 0.01),
			"render_cpu_ms": snappedf(_fps_probe_cpu_ms / 240.0, 0.01),
			"process_ms": snappedf(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0, 0.01),
			"terrain_lod_ms": snappedf(_fps_probe_map_us.x / 240000.0, 0.01),
			"c6_layers_ms": snappedf(_fps_probe_map_us.y / 240000.0, 0.01),
			"distance": snappedf(camera_rig.distance, 0.1),
			"fine_chunks": terrain.fine_chunk_count(),
			"near_chunks": terrain.near_chunk_count(),
			"ribbons": roads.ribbon_count() if roads != null else 0,
			"hamlets": settlement_layer.hamlet_instance_count() if settlement_layer != null else 0,
			"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
			"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		}))
		_fps_probe_frames = -1
		if _screenshot_path == "":
			get_tree().quit()


func _unhandled_input(event: InputEvent) -> void:
	if diplomacy != null and diplomacy.handle_input(event):
		return
	if victory != null and victory.handle_input(event):
		return
	if help != null and help.handle_input(event):
		return
	if event.is_action_pressed("map_screenshot"):
		var folder := "user://" if OS.has_feature("template") else MapPaths.project_root().path_join("docs/img")
		var path := folder.path_join("godot-map-%d.png" % Time.get_unix_time_from_system())
		_take_screenshot(path, false)
	elif event.is_action_pressed("map_toggle_edge_pan"):
		camera_rig.edge_pan_enabled = not camera_rig.edge_pan_enabled
	elif event.is_action_pressed("map_toggle_unrest"):
		_toggle_unrest_mode()
	elif event.is_action_pressed("map_toggle_court"):
		if ui.court_panel_visible():
			_court_open = false
			ui.hide_court()
		else:
			_on_court_panel_requested()
	elif event.is_action_pressed("map_toggle_tech"):  # M6
		if ui.tech_panel_visible():
			_tech_open = false
			ui.hide_tech()
		else:
			_on_tech_panel_requested()
	elif event.is_action_pressed("ui_cancel") and selected_army != "":
		deselect_army()


func _parse_cmdline() -> void:
	var args := OS.get_cmdline_user_args()
	for arg in args:
		if arg.begins_with("--stage="):
			_screenshot_stage = arg.trim_prefix("--stage=")
	for arg in args:
		if arg == "--fps-probe":
			_fps_probe_frames = 0
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			Engine.max_fps = 0
			RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
		elif arg == "--no-fine-terrain":
			terrain.fine_enabled = false
		elif arg.begins_with("--fine-step="):
			terrain.fine_step = int(arg.trim_prefix("--fine-step="))
			terrain.fine_enabled = terrain.fine_step > 0
		elif arg.begins_with("--select-settlement=") and settlement_layer != null:
			settlement_layer.select(arg.trim_prefix("--select-settlement="))
	for arg in args:
		if arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			_screenshot_countdown = SCREENSHOT_DELAY_FRAMES
			camera_rig.edge_pan_enabled = false
			match _screenshot_stage:
				"province":
					_stage_screenshot_province()
				"city":
					_stage_screenshot_city()
				"faction":
					_stage_screenshot_faction()
				"budget":  # U3 : budget et courbe du trésor après quelques saisons
					_stage_screenshot_budget()
				"court":
					_stage_screenshot_court()
				"skills":
					_stage_screenshot_skills()
				"siege":
					_stage_screenshot_siege()  # M8
				"map":
					pass  # V2 : carte seule, sans sélection ni panneau (captures du terrain)
				"help":
					help.toggle()  # M10
				"objectives":
					_focus_capital()
					victory.open_panel()  # M10
				"diplomacy":
					_focus_capital()
					diplomacy.open_panel("fac_england")
				"diplomacy_map":
					_focus_capital()
					diplomacy._toggle_mode(DiplomacyController.MapMode.DIPLOMACY)
				"tech":
					_stage_screenshot_tech()  # M6
				"chronicle":  # M10
					_focus_capital()
					chronicle.stage_screenshot()
				"tech_civil":
					_stage_screenshot_tech()  # M6
					ui.tech_panel.select_branch("civil")
				"battle":
					_stage_screenshot_battle()
				"tooltips":  # F2
					_stage_screenshot_tooltips()
				"tutorial", "encyclopedia":  # F8
					tutorial.stage_screenshot(_screenshot_stage)
				"settlement", "settlement_orders":  # C5
					settlements_ctl.stage_screenshot(_screenshot_stage)
				"agents", "agents_registry":  # C6 agents
					agents_ctl.stage_screenshot(_screenshot_stage == "agents_registry")
				_:
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


## Mise en scène « province » : capitale du joueur sélectionnée, panneau de recrutement ouvert.
func _stage_screenshot_province() -> void:
	var capital: String = str(SimFacade.faction_info(player_faction).get("capital", ""))
	var index := map_data.index_of_id(capital)
	if index == 0:
		var ids := player_army_ids()
		if not ids.is_empty():
			var first: Dictionary = sim.call("get_army", ids[0])
			index = map_data.index_of_id(str(first.get("location_province", first.get("location", ""))))
	if index == 0:
		index = mini(3, map_data.province_count)
	var centroid: Vector2 = map_data.get_province(index).get("centroid", Vector2.ZERO)
	camera_rig.look_at_point(Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y), maxf(map_data.size.x, map_data.size.y) * 0.09)
	camera_rig.snap()
	picker.select_index(index)
	ui.province_panel.recruit_panel.show()


## Remplace la simulation par le mock si la sim active n'expose pas encore `get_province_city`
## (§ 2, en attendant `core/`) : sert uniquement aux captures `--stage=city`/`--stage=faction`.
func _ensure_city_capable_sim() -> void:
	if _city_available():
		return
	var mock := CampaignSimMock.new()
	if not mock.new_campaign(MapPaths.data_dir, player_faction, SimFacade.pending_seed):
		return
	SimFacade.sim = mock
	SimFacade.is_real = false
	sim = mock
	refresh_all()


## Idem pour `get_character` (§ 3, en attendant le pont M4) : sert aux captures
## `--stage=court`/`--stage=skills` et au smoke test.
func _ensure_characters_capable_sim() -> void:
	if _characters_available():
		return
	var mock := CampaignSimMock.new()
	if not mock.new_campaign(MapPaths.data_dir, player_faction, SimFacade.pending_seed):
		return
	SimFacade.sim = mock
	SimFacade.is_real = false
	sim = mock
	refresh_all()


## Mise en scène « cour » : panneau de la cour du joueur ouvert.
func _stage_screenshot_court() -> void:
	_ensure_characters_capable_sim()
	_focus_capital()
	_on_court_panel_requested()


## Mise en scène « compétences » : fiche du dirigeant ouverte sur l'arbre de compétences.
func _stage_screenshot_skills() -> void:
	_ensure_characters_capable_sim()
	_focus_capital()
	var info := _faction_info(player_faction)
	var ruler: String = str(info.get("ruler", ""))
	if ruler == "":
		var ids: Array = sim.call("get_faction_characters", player_faction)
		if not ids.is_empty():
			ruler = str(ids[0])
	if ruler != "":
		sim.call("submit_order", {"type": "debug_grant_xp", "character": ruler, "amount": 400})
		# Une compétence apprise pour montrer les trois états (appris/disponible/verrouillé).
		var learnable: Array = sim.call("get_learnable", ruler)
		if not learnable.is_empty():
			sim.call("submit_order", {"type": "learn_skill", "character": ruler, "skill": str(learnable[0])})
		_on_character_selected(ruler)


func _faction_info(faction_id: String) -> Dictionary:
	if SimFacade.store_loaded():
		return SimFacade.store.call("get_faction", faction_id)
	return {}


func _focus_capital() -> void:
	var capital: String = str(SimFacade.faction_info(player_faction).get("capital", ""))
	var index := map_data.index_of_id(capital)
	if index == 0:
		index = mini(3, map_data.province_count)
	var centroid: Vector2 = map_data.get_province(index).get("centroid", Vector2.ZERO)
	camera_rig.look_at_point(Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y), maxf(map_data.size.x, map_data.size.y) * 0.09)
	camera_rig.snap()
	picker.select_index(index)


## Mise en scène « ville » : capitale du joueur, panneau de province sur l'onglet Ville.
func _stage_screenshot_city() -> void:
	_ensure_city_capable_sim()
	_focus_capital()
	ui.province_panel.show_ville_tab()


## F2 : panneau de recrutement de la capitale et infobulles riches figées à l'écran (une
## unité recrutable, un bâtiment constructible), une capture ne montrant pas le survol.
func _stage_screenshot_tooltips() -> void:
	_stage_screenshot_province()
	var column := VBoxContainer.new()
	column.position = Vector2(16, 60)
	column.add_theme_constant_override("separation", 10)
	var recruitable: Array = sim.call("get_recruitable", str(SimFacade.faction_info(player_faction).get("capital", "")))
	if not recruitable.is_empty():
		column.add_child(RichTooltip.make_panel(RichTooltip.unit(str(recruitable[0].get("unit_type", "")), recruitable[0])))
	column.add_child(RichTooltip.make_panel(RichTooltip.building("bld_castle")))
	column.add_child(RichTooltip.make_panel(RichTooltip.gauge("unrest", 12)))
	ui.add_child(column)


## Mise en scène « faction » : panneau de faction ouvert sur la capitale du joueur.
func _stage_screenshot_faction() -> void:
	_ensure_city_capable_sim()
	_focus_capital()
	ui.hide_province()
	_show_faction_panel(player_faction)


## Mise en scène « budget » (lot U3) : six saisons jouées, puis le panneau de faction (tableau
## du budget avec la saison passée et l'écart, courbe du trésor).
func _stage_screenshot_budget() -> void:
	_ensure_city_capable_sim()
	_focus_capital()
	ui.hide_province()
	selected_index = 0
	for _i in 6:
		sim.call("end_turn")
	refresh_all()
	_show_faction_panel(player_faction)


func _take_screenshot(path: String, quit_after: bool) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("CampaignMap: screenshot %s (%s)" % [path, error_string(err)])
	if quit_after:
		get_tree().quit(0 if err == OK else 1)


## Mise en scène « siège » (M8) : la première armée du joueur marche sur la province ennemie la
## plus proche en posture de siège ; quelques tours passent jusqu'au siège, puis elle est sélectionnée.
func _stage_screenshot_siege() -> void:
	var ids := player_army_ids()
	if ids.is_empty():
		return
	var army_id := str(ids[0])
	# Le siège vit sur la colonie (lot C) : on attend que `get_assault_odds` le voie ; la cible
	# n'est choisie qu'une fois, sans quoi l'armée change de route à chaque tour (audit A3 C12).
	for _turn in 14:
		var army: Dictionary = sim.call("get_army", army_id)
		if army.is_empty():
			return
		if bool((sim.call("get_assault_odds", army_id) as Dictionary).get("available", false)):
			break
		if not Array(army.get("path", [])).is_empty():
			sim.call("end_turn")
			continue
		var target := ""
		var best := 1 << 30
		for id in SimFacade.store.call("get_province_ids"):
			var st: Dictionary = sim.call("get_province_state", id)
			var summary: Dictionary = sim.call("get_faction_summary", player_faction)
			if str(st.get("controller", "")) in summary.get("at_war_with", PackedStringArray()):
				var path: PackedStringArray = sim.call("find_path", army_id, id)
				if not path.is_empty() and path.size() < best:
					best = path.size()
					target = str(id)
		if target != "":
			sim.call("submit_order", {"type": "set_stance", "army": army_id, "stance": "siege"})
			sim.call("submit_order", {"type": "move_army", "army": army_id, "path": Array(sim.call("find_path", army_id, target))})
		sim.call("end_turn")
	refresh_all()
	select_army(army_id)
	var army_now: Dictionary = sim.call("get_army", army_id)
	var centroid := map_data.centroid_of_id(str(army_now.get("location_province", army_now.get("location", ""))))
	camera_rig.look_at_point(Vector3(centroid.x, 0.0, centroid.y), 260.0)
# --- Batailles (M7) --------------------------------------------------------------------
# Dialogue d'avant-bataille en fin de tour, lancement de la scène 3D (la carte est mise en
# sommeil, pas détruite : la simulation reste la même) et retour avec le résultat appliqué.

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"
const PRE_BATTLE_DIALOG := "res://scenes/battle/pre_battle_dialog.tscn"

var _battle_dialog: PreBattleDialog = null


func _battles_available() -> bool:
	return sim != null and sim.has_method("get_pending_battles")


## Ouvre le dialogue sur la première bataille en attente (s'il y en a une).
func _offer_pending_battles() -> void:
	if not _battles_available():
		return
	var pending: Array = sim.call("get_pending_battles")
	if pending.is_empty():
		_close_battle_dialog()
		return
	if _battle_dialog == null:
		_battle_dialog = load(PRE_BATTLE_DIALOG).instantiate()
		ui.add_child(_battle_dialog)
		_battle_dialog.fight_requested.connect(_on_battle_fight)
		_battle_dialog.auto_requested.connect(_on_battle_auto)
	_battle_dialog.show_battle(sim, pending[0])


func _close_battle_dialog() -> void:
	if _battle_dialog != null:
		_battle_dialog.visible = false


func _on_battle_auto(index: int) -> void:
	var events: Array = sim.call("auto_resolve_battle", index)
	ui.add_events(events, "%s (résolution automatique)" % sim.call("get_date_label"))
	if flow != null:  # P2 : le résultat rejoint le rapport de saison déjà affiché
		flow.report_late_events(events)
	refresh_all()
	_offer_pending_battles()


func _on_battle_fight(index: int, seed: int) -> void:
	var battle: Node = load(BATTLE_SCENE).instantiate()
	battle.configure(sim, index, seed)
	battle.returned.connect(_on_battle_returned.bind(battle))
	_set_campaign_active(false)
	get_tree().root.add_child(battle)


func _on_battle_returned(result: Dictionary, battle: Node) -> void:
	battle.queue_free()
	_set_campaign_active(true)
	if result.get("ok", false):
		var events: Array = result.get("events", [])
		ui.add_events(events, "%s (bataille)" % sim.call("get_date_label"))
		if flow != null:  # P2 : le résultat rejoint le rapport de saison déjà affiché
			flow.report_late_events(events)
	else:
		ui.show_toast("Résultat de bataille refusé : %s" % result.get("error", "?"), true)
	refresh_all()
	_offer_pending_battles()


func _set_campaign_active(active: bool) -> void:
	visible = active
	ui.visible = active
	process_mode = Node.PROCESS_MODE_INHERIT if active else Node.PROCESS_MODE_DISABLED
	if active:
		camera.make_current()


## `--stage=battle` : bataille France–Angleterre mise en scène, dialogue ouvert.
func _stage_screenshot_battle() -> void:
	if not _battles_available() or not sim.has_method("debug_stage_battle"):
		return
	var enemy := "fac_england" if player_faction != "fac_england" else "fac_france"
	var armies := BattleScene.main_armies(sim, player_faction, enemy)
	if armies.is_empty():
		return
	sim.call("debug_stage_battle", armies[0], armies[1])
	refresh_all()
	_offer_pending_battles()
