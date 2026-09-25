class_name MinimapController
extends Node

## Minicarte et brouillard de guerre de la carte de campagne (lot C1).
##
## - Minicarte (`CampaignMinimap`) posée dans `MapUI` (placement : `MapUI.layout_hud`, en haut à
##   droite sous la barre, lettres scellées dessous) ; clic ou glisser = recentre la caméra par son
##   API publique (`CampaignCamera.look_at_point`) ; cadre de la vue suivi à chaque image.
## - Brouillard : la portée de vue est une règle de jeu, calculée par la simulation
##   (`CampaignSim.get_visible_provinces`, `data/rules/vision.json`) ; ici on ne fait qu'afficher :
##   provinces hors de vue voilées (terrain et minicarte), armées étrangères qui s'y trouvent
##   masquées (marqueurs et minicarte). Réglage `map/fog_of_war` (menu Réglages, onglet Carte).
##   Sans getter (simulation de repli), pas de brouillard.
##
## `campaign_map.gd` n'appelle que `setup`, `refresh_fog` (avant les marqueurs d'armée),
## `refresh` et `set_province_colors`.

const FOG_SETTING := "map/fog_of_war"

var map: Node = null  # CampaignMap
var minimap: CampaignMinimap = null
## Provinces vues par le joueur (id → true) ; vide quand le brouillard est inactif.
var visible_provinces: Dictionary = {}
var fog_active: bool = false


func setup(campaign_map: Node) -> void:
	map = campaign_map
	var ui: MapUI = map.get("ui")
	minimap = CampaignMinimap.new()
	ui.add_child(minimap)
	# A3 C1 : la minicarte se dessine sous tous les panneaux (technologies, encyclopédie,
	# diplomatie…), juste après la barre du haut ; elle ne masque plus leur bouton ×.
	var top_bar := ui.get_node_or_null("TopBar")
	ui.move_child(minimap, top_bar.get_index() + 1 if top_bar != null else 0)
	ui.set("minimap", minimap)
	minimap.setup(map.get("map_data"))
	minimap.clicked.connect(center_camera_on)
	minimap.mode_changed.connect(_on_mode_changed)
	# Lot C5 : le bouton « Commerce » quitte la barre du haut pour la rangée des modes.
	var trade_button: Button = ui.get("trade_button")
	if trade_button != null:
		minimap.add_layer_button(trade_button)
	var settings := get_node_or_null("/root/Settings")
	if settings != null:
		settings.connect("changed", func(key: String) -> void:
			if key == FOG_SETTING:
				map.call("refresh_all"))
	ui.call("queue_layout")


## Lot DP2 : le bouton « Diplomatie » de la minicarte bascule aussi la carte 3D dans le mode
## diplomatique (touche N), et en sort quand on choisit un autre mode.
func _on_mode_changed(new_mode: String) -> void:
	var diplomacy: Object = map.get("diplomacy")
	if diplomacy == null or not diplomacy.has_method("set_diplomacy_mode"):
		return
	if new_mode == CampaignMinimap.MODE_DIPLOMACY and not DiplomaticStances.available(_sim()):
		minimap.set_mode(CampaignMinimap.MODE_POLITICAL)
		return
	diplomacy.call("set_diplomacy_mode", new_mode == CampaignMinimap.MODE_DIPLOMACY)


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
	fog_active = fog_wanted()
	var ids := PackedStringArray()
	if fog_active:
		ids = _sim().call("get_visible_provinces", str(map.get("player_faction")))
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
	(map.get("terrain") as TerrainBuilder).set_fog(fog_active, indices)
	(map.get("armies") as ArmyMarkers).hidden_provinces = hidden
	if minimap != null:
		minimap.set_fog(fog_active, ids)


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
		var location := str(army.get("location_province", army.get("location", "")))
		if faction != player and not is_province_visible(location):
			continue
		var centroid := map_data.centroid_of_id(location)
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
