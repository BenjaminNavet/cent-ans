class_name MinimapController
extends Node

## Minicarte et brouillard de guerre de la carte de campagne (lot C1).
##
## - Minicarte (`CampaignMinimap`) posée dans `MapUI` (placement : `MapUI.layout_hud`, en haut à
##   droite sous la barre, lettres scellées dessous) ; clic ou glisser = recentre la caméra par son
##   API publique (`CampaignCamera.look_at_point`) ; cadre de la vue suivi à chaque image.
## - Brouillard : la portée de vue est une règle de jeu, calculée par la simulation ; ici on ne
##   fait qu'afficher. Lot M5a : vue par case (`CampaignSim.get_vision` : texture R8 512², bords
##   doux, provinces visibles, armées vues ; rayons dans `data/movement/rules.json`) ; le terrain et
##   la minicarte voilent ce qui n'est pas vu, les armées étrangères hors de vue n'ont ni marqueur
##   ni point. Repli sans `get_vision` : masque par province (`get_visible_provinces`, lot C1).
##   Réglage `map/fog_of_war` (menu Réglages, onglet Carte). Sans getter, pas de brouillard.
##
## `campaign_map.gd` n'appelle que `setup`, `refresh_fog` (avant les marqueurs d'armée),
## `refresh` et `set_province_colors`.

const FOG_SETTING := "map/fog_of_war"

var map: Node = null  # CampaignMap
var minimap: CampaignMinimap = null
## Provinces vues par le joueur (id → true) ; vide quand le brouillard est inactif.
var visible_provinces: Dictionary = {}
## Lot M5a : armées montrées au joueur (id → true) ; vide quand le brouillard est inactif.
var visible_armies: Dictionary = {}
## Lot M5a : vrai quand la vue vient de la texture par case (`get_vision`).
var fog_by_cell: bool = false
## Lot M5a : dernière texture de vue (tests, captures) et part de la carte vue.
var fog_texture: ImageTexture = null
var seen_share: float = 0.0
var fog_active: bool = false
## Lot UX1 : légende de la carte (créée à la première ouverture).
var legend: MapLegend = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	var ui: MapUI = map.get("ui")
	minimap = CampaignMinimap.new()
	# PO1 (bible DA § 12.1) : zone `MINIMAP`, en bas à droite (étage HUD : sous tous les panneaux,
	# elle ne masque jamais leur bouton ×) ; `MapUI.layout_hud` l'ajuste à la zone.
	UiLayout.claim(UiLayout.Zone.MINIMAP, minimap)
	ui.set("minimap", minimap)
	minimap.setup(map.get("map_data"))
	minimap.clicked.connect(center_camera_on)
	# Lot C5 : le bouton « Commerce » quitte la barre du haut pour la rangée des modes.
	var trade_button: Button = ui.get("trade_button")
	if trade_button != null:
		minimap.add_layer_button(trade_button)
	var settings := get_node_or_null("/root/Settings")
	if settings != null:
		settings.connect("changed", func(key: String) -> void:
			if key == FOG_SETTING:
				map.call("refresh_all"))
	minimap.legend_toggled.connect(func(pressed: bool) -> void: set_legend_open(pressed))
	ui.call("queue_layout")


# --- Légende de la carte (lot UX1) ------------------------------------------------------


## Ouvre ou ferme la légende (panneau posé à gauche de la minicarte).
func set_legend_open(open: bool) -> void:
	if open and legend == null:
		legend = MapLegend.new()
		var ui: Node = map.get("ui")
		ui.add_child(legend)
		ui.move_child(legend, minimap.get_index() + 1)
		legend.closed.connect(func() -> void: minimap.legend_button.set_pressed_no_signal(false))
	if legend == null:
		return
	if open:
		legend.build(current_map_mode(), legend_context())
		legend.show()
		_place_legend()
	else:
		legend.hide()
	minimap.legend_button.set_pressed_no_signal(open)


func legend_open() -> bool:
	return legend != null and legend.visible


## Mode de carte affiché : « diplomacy », « religion », « unrest » ou « political ».
func current_map_mode() -> String:
	# MF1 : un seul filtre actif, porté par `MapModeController`.
	var modes: Object = map.get("map_modes")
	var current := str(modes.get("mode")) if modes != null else "political"
	return current if current in MapLegend.MODES else "political"


## Couleurs du joueur et des royaumes d'exemple (`factions` de la légende).
func legend_context() -> Dictionary:
	var facade: Node = get_node_or_null("/root/SimFacade")
	var player := str(map.get("player_faction"))
	var factions: Array = []
	for id in MapLegend.data().get("factions", []):
		if str(id) == player:
			continue
		var color: Color = facade.call("faction_color", id) if facade != null else Color.GRAY
		var label: String = str(facade.call("faction_short_name", id)) if facade != null else str(id)
		factions.append([str(id), color, label])
	var player_color: Color = facade.call("faction_color", player) if facade != null else Color(0.25, 0.35, 0.7)
	return {"player_faction": player, "player_color": player_color, "factions": factions}


## PO1 : la légende s'ouvre dans la colonne du panneau latéral, bas calé au-dessus de la
## minicarte (elle se referme d'elle-même quand un panneau s'y ouvre, voir `setup`).
func _place_legend() -> void:
	var side := UiLayout.zone_rect(UiLayout.Zone.SIDE_PANEL)
	var mini := UiLayout.zone_rect(UiLayout.Zone.MINIMAP)
	legend.set_max_height(side.size.y)
	legend.size = legend.get_combined_minimum_size()
	legend.position = Vector2(mini.end.x - legend.size.x, maxf(side.position.y, mini.position.y - legend.size.y - 8.0))


func _sim() -> Object:
	return map.get("sim")


## Vrai si le réglage est actif et que la simulation calcule la vue.
func fog_wanted() -> bool:
	var settings := get_node_or_null("/root/Settings")
	var enabled := true if settings == null else bool(settings.call("get_value", FOG_SETTING))
	var sim := _sim()
	return enabled and sim != null and sim.has_method("get_visible_provinces")


func is_province_visible(province_id: String) -> bool:
	return not fog_active or visible_provinces.has(province_id)


## Recalcule la vue du joueur et l'applique au terrain et aux marqueurs d'armée
## (à appeler avant `ArmyMarkers.refresh`).
func refresh_fog() -> void:
	visible_provinces.clear()
	visible_armies.clear()
	fog_active = fog_wanted()
	var sim := _sim()
	fog_by_cell = fog_active and sim.has_method("get_vision")
	var vision: Dictionary = {}
	var ids := PackedStringArray()
	if fog_by_cell:
		vision = sim.call("get_vision", str(map.get("player_faction")))
		fog_by_cell = vision.has("image")
	if fog_by_cell:
		ids = vision.get("provinces", PackedStringArray())
		for army_id in vision.get("armies", PackedStringArray()):
			visible_armies[army_id] = true
	elif fog_active:
		ids = sim.call("get_visible_provinces", str(map.get("player_faction")))
	for id in ids:
		visible_provinces[id] = true
	var map_data: MapData = map.get("map_data")
	var indices := PackedInt32Array()
	var hidden: Dictionary = {}
	if fog_active:
		for index in range(1, map_data.province_count + 1):
			var id := str(map_data.get_province(index).get("id", ""))
			if visible_provinces.has(id):
				indices.append(index)
			elif id != "":
				hidden[id] = true
	var terrain: TerrainBuilder = map.get("terrain")
	var armies: ArmyMarkers = map.get("armies")
	armies.hidden_provinces = hidden
	armies.visible_armies = visible_armies
	armies.army_filter_active = fog_by_cell
	if fog_by_cell:
		fog_texture = ImageTexture.create_from_image(vision["image"])
		seen_share = float(vision.get("seen_share", 0.0))
		var size: Vector2 = vision.get("size", Vector2(map_data.size))
		terrain.set_fog_cells(true, fog_texture, size)
		if minimap != null:
			minimap.set_fog_cells(true, fog_texture, size, ids.size())
	else:
		fog_texture = null
		seen_share = 0.0 if fog_active else 1.0
		terrain.set_fog(fog_active, indices)
		if minimap != null:
			minimap.set_fog(fog_active, ids)


## Vrai si l'armée `army` (dictionnaire de `get_army`) est montrée au joueur.
func is_army_visible(army_id: String, army: Dictionary) -> bool:
	if not fog_active or str(army.get("faction", "")) == str(map.get("player_faction")):
		return true
	if fog_by_cell:
		return visible_armies.has(army_id)
	return is_province_visible(str(army.get("location_province", army.get("location", ""))))


## Couleurs de faction par province (mêmes que le terrain), à la fin de chaque tour.
func set_province_colors(colors: PackedColorArray) -> void:
	if minimap != null:
		minimap.set_province_colors(colors)


## Armées de la minicarte (après chaque changement d'état).
func refresh() -> void:
	var sim := _sim()
	if minimap == null or sim == null:
		return
	var map_data: MapData = map.get("map_data")
	var player := str(map.get("player_faction"))
	var dots: Array = []
	# Autoload résolu à l'exécution : le smoke test compile ce script avant les autoloads.
	var facade: Node = get_node_or_null("/root/SimFacade")
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		var faction := str(army.get("faction", ""))
		if not is_army_visible(str(army_id), army):
			continue
		# Lot M5a : le point suit la position libre de l'armée (M2), à défaut sa province.
		var centroid: Vector2 = army.get("position", Vector2(-1.0, -1.0))
		if centroid.x < 0.0:
			centroid = map_data.centroid_of_id(str(army.get("location_province", army.get("location", ""))))
		if centroid.x < 0.0:
			continue
		dots.append({"pos": centroid, "color": facade.call("faction_color", faction) if facade != null else Color.WHITE, "player": faction == player})
	minimap.set_armies(dots)


## Recentre la caméra sur un point de la carte (coordonnées carte).
func center_camera_on(map_pos: Vector2) -> void:
	var map_data: MapData = map.get("map_data")
	var rig: CampaignCamera = map.get("camera_rig")
	rig.look_at_point(Vector3(map_pos.x, map_data.surface_world_at(map_pos.x, map_pos.y), map_pos.y))


func _process(_delta: float) -> void:
	if legend_open():  # UX1 : suit le mode de carte et la place de la minicarte
		legend.set_mode(current_map_mode())
		_place_legend()
	if minimap == null or not minimap.is_visible_in_tree():
		return
	minimap.set_view_frame(view_frame())


## Quadrilatère vu par la caméra sur le plan du sol (coordonnées carte) : coins de l'écran
## projetés ; un rayon qui ne touche pas le sol est borné à quelques distances de caméra.
func view_frame() -> PackedVector2Array:
	var camera: Camera3D = map.get("camera")
	var rig: CampaignCamera = map.get("camera_rig")
	var frame := PackedVector2Array()
	if camera == null or not camera.is_inside_tree():
		return frame
	var view := camera.get_viewport().get_visible_rect().size
	var reach := rig.distance * 4.0
	for corner in [Vector2(0, 0), Vector2(view.x, 0), view, Vector2(0, view.y)]:
		var origin := camera.project_ray_origin(corner)
		var normal := camera.project_ray_normal(corner)
		var hit: Vector3
		if normal.y < -0.02:
			var t := minf(-origin.y / normal.y, reach)
			hit = origin + normal * t
		else:
			hit = origin + Vector3(normal.x, 0.0, normal.z).normalized() * reach
		frame.append(Vector2(hit.x, hit.z))
	return frame
