class_name ArmyMarkers
extends Node3D

## Marqueurs d'armées (`army_marker.tscn`) posés au centroïde de leur province, décalés
## quand plusieurs armées partagent une province. `refresh(sim)` reconstruit l'ensemble
## (appelé après chaque fin de tour ou ordre) ; `pick_screen` renvoie l'armée sous le
## curseur (priorité sur la province).
##
## Chaque armée a une plaque d'effectif 2D (cartouche parchemin : écu de la faction,
## nombre d'hommes, état « en marche » / « siège » / « à bord »), de taille constante à l'écran,
## posée au-dessus de l'étendard (couche `PLATE_LAYER`, sous l'interface).

const MARKER_SCENE := preload("res://scenes/map/army_marker.tscn")
## SA : écart à la ville d'une armée en garnison, côté sud-est (vers la caméra par défaut), et
## marge au-delà du rayon de la ville (unités monde).
const STANDOFF_DIRECTION := Vector2(0.86, 0.51)
const STANDOFF_MARGIN := 0.4
## Distance minimale (pixels de carte) entre une armée et le modèle de ville de la province.
const CITY_CLEARANCE_PX := 26.0

## Couche des plaques : au-dessus du monde 3D, sous l'interface (`CanvasLayer` 1).
const PLATE_LAYER := 0

var map_data: MapData
var camera: Camera3D
var selected_army: String = ""
## Brouillard : provinces hors de vue (id → true) ; les armées étrangères qui s'y
## trouvent ne reçoivent pas de marqueur. Rempli par `MinimapController.refresh_fog`.
var hidden_provinces: Dictionary = {}
## Armées montrées au joueur (id → true, vue par case de la simulation). Quand
## `army_filter_active`, ce filtre remplace `hidden_provinces` pour les armées : une armée
## étrangère n'a de marqueur que si son point est vu.
var visible_armies: Dictionary = {}
var army_filter_active: bool = false
## Q2 : zones des grandes villes détaillées L1-L3 (`SettlementLayer.landmark_zones()`,
## (x, z, rayon) en pixels de carte) ; une armée stationnée dans l'une d'elles se tient devant
## ses murs plutôt qu'au milieu de la maquette (lisible et cliquable séparément).
var landmark_zones: PackedVector3Array = PackedVector3Array()
## C4/C6 : position monde d'une colonie (`SettlementLayer.world_position_of`), posée par
## la carte ; à défaut, `MapData.settlement_px`.
var settlement_position: Callable = Callable()
## SA (ADR 0160) : rayon de l'emprise d'une colonie (`SettlementLayer.model_radius_of`), posé
## par la carte ; sert à tenir l'armée en garnison à côté de la ville.
var settlement_radius: Callable = Callable()
## SA : armée sous le curseur ("" = aucune).
var hovered_army: String = ""
## Noms de ville à éviter, `func(camera: Camera3D) -> Array[Rect2]` (rectangles
## écran), posé par la carte ; les plaques s'en écartent (`LabelPlacer`).
var label_obstacles: Callable = Callable()
## Surface affichée (`TerrainBuilder.surface_height_at`) quand le relief streamé est
## actif ; à défaut, heightmap 4096 (`MapData.surface_world_at`).
var ground_height: Callable = Callable()

var _markers: Dictionary = {}  # army_id → ArmyMarker
var _homes: Dictionary = {}  # army_id → position de base issue de la simulation (M4)
var _plates: Dictionary = {}  # army_id → PanelContainer
var _plate_layer: CanvasLayer
var _current_scale: float = 1.0
## Paliers de zoom (fondu des figurines au palier « loin ») et dernière distance.
var _zoom_tiers: ZoomTiers
var _camera_distance: float = -1.0
var _layout := ArmyPlateLayout.new()
## Placement des plaques hors des noms de ville et des autres plaques.


func setup(data: MapData, view_camera: Camera3D) -> void:
	map_data = data
	camera = view_camera
	if _plate_layer == null:
		_plate_layer = CanvasLayer.new()
		_plate_layer.name = "Plates"
		_plate_layer.layer = PLATE_LAYER
		add_child(_plate_layer)


## Reconstruit les marqueurs depuis la simulation. `color_of(faction_id) -> Color`.
func refresh(sim: Object, color_of: Callable, player_faction: String) -> void:
	# PB1 : `refresh_all` suit chaque ordre du joueur et chaque fin de tour ; reconstruire toutes
	# les figurines animées coûtait 30 à 60 ms. Un marqueur est gardé tel quel si l'armée n'a pas
	# changé (signature ci-dessous, position exclue : replacée plus bas à chaque fois).
	var previous := _markers
	_markers = {}
	_homes.clear()
	# Plaques gardées avec leur marqueur inchangé (contenu lu du marqueur), sinon reconstruites.
	var previous_plates := _plates
	_plates = {}
	if sim == null or map_data == null:
		for marker in previous.values():
			marker.queue_free()
		for plate in previous_plates.values():
			plate.queue_free()
		return
	var per_province: Dictionary = {}
	var stances := StanceCues.stances(sim, player_faction)  # EN : relation de chaque faction
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if army.is_empty():
			continue
		# C4 : l'armée se tient sur une colonie ; sa position est celle de la colonie
		# (couche C6, sinon `settlements_px.json`), à défaut le centroïde de sa province.
		var location: String = str(army.get("location_province", army.get("location", "")))
		if str(army.get("faction", "")) != player_faction:
			if army_filter_active:
				if not visible_armies.has(str(army_id)):
					continue
			elif hidden_provinces.has(location):
				continue
		# Une armée en campagne se tient à sa position libre ; seules les armées
		# stationnées dans une colonie s'empilent sur celle-ci.
		var in_field := army.has("position") and str(army.get("settlement", "")) == ""
		var centroid := Vector2(-1.0, -1.0)
		var stack_key := location
		var standoff := -1.0  # SA : < 0 = en campagne, pas d'écart à une ville
		if in_field:
			centroid = army["position"]
			stack_key = "field:%d:%d" % [int(centroid.x), int(centroid.y)]
		else:
			centroid = _settlement_px(str(army.get("settlement", army.get("location", ""))))
			if centroid.x < 0.0:
				centroid = map_data.centroid_of_id(location)
			if centroid.x < 0.0:
				continue
			if not army.has("position"):
				centroid = _clear_of_city(location, centroid)
			stack_key = "settlement:" + str(army.get("settlement", army.get("location", "")))
			standoff = _standoff_base(str(army.get("settlement", army.get("location", ""))), centroid)
		var faction: String = str(army.get("faction", ""))
		var color: Color = color_of.call(faction)
		var signature := _signature(army, color, faction == player_faction)
		var marker: ArmyMarker = previous.get(army_id)
		var reused: bool = marker != null and marker.get_meta("pb1_signature", "") == signature
		if reused:
			previous.erase(army_id)
		else:
			if marker != null:
				previous.erase(army_id)
				marker.queue_free()
			marker = MARKER_SCENE.instantiate()
			add_child(marker)
			marker.setup(army_id, army, color, faction == player_faction)
			marker.set_meta("pb1_signature", signature)
		# EN : hors signature, une déclaration de guerre ne reconstruit pas les figurines.
		var cue := StanceCues.category_of(faction, player_faction, stances)
		var cue_changed := marker.cue != cue
		marker.set_cue(cue)
		marker.base_position = Vector3(centroid.x, _ground(centroid), centroid.y)
		# SA (ADR 0160) : en garnison, l'ost se tient à côté de la ville, à tout zoom.
		marker.standoff_dir = STANDOFF_DIRECTION.normalized() if standoff >= 0.0 else Vector2.ZERO
		marker.standoff_base = maxf(standoff, 0.0)
		var stack: int = per_province.get(stack_key, 0)
		per_province[stack_key] = stack + 1
		marker.offset_dir = Vector2.ZERO if stack == 0 else Vector2.RIGHT.rotated(stack * TAU / 6.0)
		marker.face(_heading(army, centroid))
		marker.set_selected(army_id == selected_army)
		marker.set_hovered(army_id == hovered_army)
		marker.apply_scale(_current_scale)
		if _camera_distance >= 0.0:
			marker.set_view(_camera_distance, figure_weight(_camera_distance))
		_markers[army_id] = marker
		_homes[army_id] = marker.base_position
		if _plate_layer != null:
			var plate: PanelContainer = previous_plates.get(army_id)
			previous_plates.erase(army_id)
			if reused and not cue_changed and plate != null and is_instance_valid(plate):
				ArmyPlate.apply_style(plate, marker, marker.army_id == selected_army, marker.army_id == hovered_army)
			else:
				if plate != null and is_instance_valid(plate):
					plate.queue_free()
				plate = ArmyPlate.build(marker, marker.army_id == selected_army)
				_plate_layer.add_child(plate)
			_plates[army_id] = plate
	for marker in previous.values():
		marker.queue_free()
	for plate in previous_plates.values():
		plate.queue_free()
	if not _markers.has(selected_army):
		selected_army = ""
	if not _markers.has(hovered_army):
		hovered_army = ""
	_layout.mark_dirty()
	_update_plates()


## Tout ce que `ArmyMarker.setup` lit de l'armée, sauf sa position (replacée à chaque refresh).
static func _signature(army: Dictionary, color: Color, player: bool) -> String:
	var copy := army.duplicate()
	copy.erase("position")
	return "%s|%s|%s" % [var_to_str(copy), color.to_html(), player]


## Direction d'avance (coordonnées carte) : vers la prochaine étape du chemin, sinon un
## trois-quarts vers le sud-est (lisible avec la caméra par défaut).
func _heading(army: Dictionary, from: Vector2) -> Vector2:
	# Prochain coin du trajet libre en cours.
	var planned: PackedVector2Array = army.get("planned_path", PackedVector2Array())
	if not planned.is_empty() and planned[0].distance_to(from) > 0.5:
		return (planned[0] - from).normalized()
	var path: Array = army.get("path", [])
	if not path.is_empty():
		var next := _settlement_px(str(path[0]))
		if next.x < 0.0:
			next = map_data.centroid_of_id(str(path[0]))
		if next.x >= 0.0 and next.distance_to(from) > 0.5:
			return (next - from).normalized()
	return Vector2(1.0, 0.45).normalized()


## Position carte d'une colonie ; Vector2(-1, -1) si inconnue.
func _settlement_px(id: String) -> Vector2:
	if settlement_position.is_valid():
		var world: Vector3 = settlement_position.call(id)
		if world != Vector3.ZERO:
			return Vector2(world.x, world.z)
	return map_data.settlement_px(id)


## Écarte l'armée du modèle de ville quand le centroïde tombe sur la capitale de la province.
func _clear_of_city(location: String, centroid: Vector2) -> Vector2:
	var province := map_data.get_province(map_data.index_of_id(location))
	if not province.has("capital_px"):
		return centroid
	var capital: Vector2 = province["capital_px"]
	var away := centroid - capital
	if away.length() >= CITY_CLEARANCE_PX:
		return centroid
	var direction := away.normalized() if away.length() > 0.5 else Vector2(1.0, 0.6).normalized()
	return capital + direction * CITY_CLEARANCE_PX


## SA (ADR 0160) : part fixe de l'écart entre une armée en garnison et le centre de sa ville
## (unités monde) : rayon de la zone L1-L3 qui contient le point (Q2), sinon rayon de l'emprise
## de la colonie, plus une marge. La demi-emprise de l'ost s'y ajoute (`ArmyMarker`).
func _standoff_base(settlement_id: String, point: Vector2) -> float:
	var radius := float(settlement_radius.call(settlement_id)) if settlement_radius.is_valid() else 0.0
	for zone in landmark_zones:
		if point.distance_to(Vector2(zone.x, zone.y)) < zone.z:
			radius = maxf(radius, zone.z)
	return radius + STANDOFF_MARGIN


func set_selected(army_id: String) -> void:
	selected_army = army_id
	for id in _markers:
		_markers[id].set_selected(id == army_id)
	for id in _plates:
		ArmyPlate.apply_style(_plates[id], _markers[id], id == army_id, id == hovered_army)
	_layout.mark_dirty()


## SA (ADR 0160) : armée sous le curseur ("" = aucune) : socle, figurines et plaque éclaircis.
func set_hovered(army_id: String) -> void:
	if not _markers.has(army_id):
		army_id = ""
	if army_id == hovered_army:
		return
	var previous := hovered_army
	hovered_army = army_id
	for id in [previous, army_id]:
		var marker: ArmyMarker = _markers.get(id)
		if marker == null:
			continue
		marker.set_hovered(id == army_id)
		var plate: PanelContainer = _plates.get(id)
		if plate != null and is_instance_valid(plate):
			ArmyPlate.apply_style(plate, marker, id == selected_army, id == army_id)


func has_army(army_id: String) -> bool:
	return _markers.has(army_id)


func army_count() -> int:
	return _markers.size()


## Texte de la plaque d'une armée ("" si absente) : effectif et état.
func plate_text(army_id: String) -> String:
	var plate: PanelContainer = _plates.get(army_id)
	if plate == null:
		return ""
	var parts := PackedStringArray()
	for label in plate.find_children("*", "Label", true, false):
		parts.append((label as Label).text)
	return " ".join(parts)


## Armée la plus proche du point écran (`ArmyPicker.PICK_NEAR_PX`) (étendard, figurines ou plaque),
## sinon "".
func pick_screen(screen_position: Vector2) -> String:
	return str(pick_screen_scored(screen_position).get("id", ""))


## Q2 / SA (ADR 0160) : voir `ArmyPicker.pick_scored` (`score`, `direct`, `exclude`).
func pick_screen_scored(screen_position: Vector2, exclude: String = "") -> Dictionary:
	return ArmyPicker.pick_scored(camera, _markers, _plates, screen_position, exclude)


## Pose le marqueur de `army_id` au point carte `point`, tourné vers
## `heading` ; `point.x < 0` le remet à sa position issue de la simulation.
func place_marker(army_id: String, point: Vector2, heading: Vector2 = Vector2.ZERO) -> void:
	var marker: ArmyMarker = _markers.get(army_id)
	if marker == null:
		return
	# Les figurines marchent pendant l'animation du déplacement.
	marker.set_walking(point.x >= 0.0)
	# SA : l'écart à la ville est suspendu pendant la marche (trajet réel), repris à l'arrivée.
	marker.set_standoff_enabled(point.x < 0.0)
	if point.x < 0.0:
		marker.base_position = _homes.get(army_id, marker.base_position)
	else:
		marker.base_position = Vector3(point.x, _ground(point), point.y)
	marker.apply_scale(_current_scale)
	if heading.length() > 0.01:
		marker.face(heading.normalized())


## Point du monde au sol d'un point carte (pixels, convention de `position` des
## armées) — sites de rencontre posés comme les armées.
func world_at_pixel(p: Vector2) -> Vector3:
	return to_global(Vector3(p.x, _ground(p), p.y))


## Hauteur du sol au point carte `p` (surface affichée si disponible).
func _ground(p: Vector2) -> float:
	if ground_height.is_valid():
		return float(ground_height.call(p.x, p.y))
	return map_data.surface_world_at(p.x, p.y)


## Repose tous les marqueurs sur le sol (échelle verticale changée, pages de relief
## arrivées) ; quelques dizaines d'appels à `surface_height_at`, bon marché.
func reground() -> void:
	for army_id: String in _markers:
		var marker: ArmyMarker = _markers[army_id]
		var home: Vector3 = _homes.get(army_id, marker.base_position)
		_homes[army_id] = Vector3(home.x, _ground(Vector2(home.x, home.z)), home.z)
		marker.base_position.y = _ground(Vector2(marker.base_position.x, marker.base_position.z))
		marker.apply_scale(_current_scale)


## TB4 : armées affichées et leur marqueur (`WarScars` y pose les engins de siège du camp).
func marker_ids() -> Array:
	return _markers.keys()


func marker_of(army_id: String) -> ArmyMarker:
	return _markers.get(army_id)


## Position monde d'une armée (pour cadrer la caméra), Vector3.ZERO si absente.
func world_position_of(army_id: String) -> Vector3:
	var marker: ArmyMarker = _markers.get(army_id)
	return marker.base_position if marker != null else Vector3.ZERO


func update_scale(camera_distance: float) -> void:
	if absf(camera_distance - _camera_distance) > minf(0.25, absf(_camera_distance) * 0.05):
		_camera_distance = camera_distance
		var weight := figure_weight(camera_distance)
		for marker in _markers.values():
			marker.set_view(camera_distance, weight)
	var new_scale := ArmyScale.scale_for_distance(camera_distance, 1.0 - figure_weight(camera_distance), ArmyScale.scale_exponent())
	if absf(new_scale - _current_scale) < minf(0.005, new_scale * 0.02):
		return
	_current_scale = new_scale
	for marker in _markers.values():
		marker.apply_scale(_current_scale)


## Présence des figurines (1 en vue normale, 0 sur le parchemin, fondu sur la
## bande de `ZoomTiers.strategic_weight`) ; au loin restent l'étendard et la plaque.
func figure_weight(camera_distance: float) -> float:
	if _zoom_tiers == null:
		_zoom_tiers = ZoomTiers.load_default()
	return 1.0 - _zoom_tiers.strategic_weight(camera_distance)


func _process(delta: float) -> void:
	_layout.tick(delta)
	_update_plates()


# --- Plaques d'effectif (construction : `ArmyPlate`, placement : `ArmyPlateLayout`) ----


func _update_plates() -> void:
	if camera == null or _plates.is_empty():
		return
	_layout.update(camera, get_viewport().get_visible_rect(), _plates, _markers, selected_army, _camera_distance, label_obstacles)


## CV3-0 (#7) : rectangles écran des plaques d'armée actuellement affichées (position finale,
## après placement). Utilisé par `SettlementLayer` pour que ses marqueurs et noms de colonies
## s'écartent des plaques. Même forme que `label_obstacles` (`func(camera) -> Array[Rect2]`),
## `camera` ignoré.
func screen_label_rects(_camera: Camera3D = null) -> Array:
	return ArmyPlateLayout.visible_rects(_plates)
