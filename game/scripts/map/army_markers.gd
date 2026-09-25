class_name ArmyMarkers
extends Node3D

## Marqueurs d'armées (`army_marker.tscn`) posés au centroïde de leur province, décalés
## quand plusieurs armées partagent une province. `refresh(sim)` reconstruit l'ensemble
## (appelé après chaque fin de tour ou ordre) ; `pick_screen` renvoie l'armée sous le
## curseur (priorité sur la province).
##
## Lot V3 : chaque armée a une plaque d'effectif 2D (cartouche parchemin : écu de la faction,
## nombre d'hommes, état « en marche » / « siège » / « à bord »), de taille constante à l'écran,
## posée au-dessus de l'étendard (couche `PLATE_LAYER`, sous l'interface).

const MARKER_SCENE := preload("res://scenes/map/army_marker.tscn")
const PICK_RADIUS_PX := 26.0
## Échelle du marqueur = distance caméra × facteur, bornée : taille constante à l'écran aux
## paliers moyen et loin (lisible comme un pion), taille monde fixe au plus près.
const SCALE_PER_DISTANCE := 0.014
## Q2 : plafond de taille monde au palier « près » (comme Total War : la ville domine l'armée).
## À cette échelle l'étendard royal fait ~3 unités et l'escorte ~2,5 de large, soit environ un
## quart du diamètre de Paris (L1, `core_radius_px` 6) et moins qu'une ville L2/L3 (4,5-7) ;
## les figurines ont la hauteur des maisons. Atteint vers la distance 20 ; 0,8 auparavant, qui
## faisait recouvrir Paris par l'ost au plus près.
const MIN_SCALE := 0.27
const MAX_SCALE := 14.0

## Distance minimale (pixels de carte) entre une armée et le modèle de ville de la province.
const CITY_CLEARANCE_PX := 26.0
## Q2 : marge hors de la zone d'une grande ville L1-L3 pour une armée qui y stationne.
const LANDMARK_MARGIN_PX := 1.5

## Couche des plaques : au-dessus du monde 3D, sous l'interface (`CanvasLayer` 1).
const PLATE_LAYER := 0
const PLATE_FONT_SIZE := 15
const INK := Color(0.16, 0.10, 0.05)
const PARCHMENT := Color(0.94, 0.89, 0.76, 0.94)
const PLAYER_BORDER := Color(0.85, 0.66, 0.2)
const OTHER_BORDER := Color(0.30, 0.20, 0.12)
const STATUS_TEXT := {"moving": "»", "siege": "siège", "embarked": "à bord"}
## Lot UX1 : période de recalcul du placement des plaques quand la caméra est immobile (les
## armées animées et les noms de ville qui apparaissent sont repris à ce rythme).
const PLACEMENT_INTERVAL := 0.25

var map_data: MapData
var camera: Camera3D
var selected_army: String = ""
## Brouillard (lot C1) : provinces hors de vue (id → true) ; les armées étrangères qui s'y
## trouvent ne reçoivent pas de marqueur. Rempli par `MinimapController.refresh_fog`.
var hidden_provinces: Dictionary = {}
## Q2 : zones des grandes villes détaillées L1-L3 (`SettlementLayer.landmark_zones()`,
## (x, z, rayon) en pixels de carte) ; une armée stationnée dans l'une d'elles se tient devant
## ses murs plutôt qu'au milieu de la maquette (lisible et cliquable séparément).
var landmark_zones: PackedVector3Array = PackedVector3Array()
## C4/C6 : position monde d'une colonie (`SettlementLayer.world_position_of`), posée par
## la carte ; à défaut, `MapData.settlement_px`.
var settlement_position: Callable = Callable()
## Lot UX1 (A3 C8) : noms de ville à éviter, `func(camera: Camera3D) -> Array[Rect2]` (rectangles
## écran), posé par la carte ; les plaques s'en écartent (`LabelPlacer`).
var label_obstacles: Callable = Callable()

var _markers: Dictionary = {}  # army_id → ArmyMarker
var _homes: Dictionary = {}  # army_id → position de base issue de la simulation (M4)
var _plates: Dictionary = {}  # army_id → PanelContainer
var _plate_layer: CanvasLayer
var _current_scale: float = 1.0
## Lot CV2 : paliers de zoom (fondu des figurines au palier « loin ») et dernière distance.
var _zoom_tiers: ZoomTiers
var _camera_distance: float = -1.0
## Lot UX1 : placement des plaques hors des noms de ville et des autres plaques.
var _placer := LabelPlacer.new()
var _plate_offsets: Dictionary = {}  # army_id → décalage écran
var _placement_camera := Transform3D()
var _placement_view := Vector2.ZERO
var _placement_timer := 0.0
var _placement_dirty := true


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
	for marker in _markers.values():
		marker.queue_free()
	_markers.clear()
	_homes.clear()
	for plate in _plates.values():
		plate.queue_free()
	_plates.clear()
	if sim == null or map_data == null:
		return
	var per_province: Dictionary = {}
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		if army.is_empty():
			continue
		# C4 : l'armée se tient sur une colonie ; sa position est celle de la colonie
		# (couche C6, sinon `settlements_px.json`), à défaut le centroïde de sa province.
		var location: String = str(army.get("location_province", army.get("location", "")))
		if hidden_provinces.has(location) and str(army.get("faction", "")) != player_faction:
			continue
		# Lot M4 : une armée en campagne se tient à sa position libre ; seules les armées
		# stationnées dans une colonie s'empilent sur celle-ci.
		var in_field := army.has("position") and str(army.get("settlement", "")) == ""
		var centroid := Vector2(-1.0, -1.0)
		var stack_key := location
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
			centroid = _clear_of_landmark(centroid)
			stack_key = "settlement:" + str(army.get("settlement", army.get("location", "")))
		var marker: ArmyMarker = MARKER_SCENE.instantiate()
		add_child(marker)
		var faction: String = str(army.get("faction", ""))
		marker.setup(army_id, army, color_of.call(faction), faction == player_faction)
		marker.base_position = Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y)
		var stack: int = per_province.get(stack_key, 0)
		per_province[stack_key] = stack + 1
		marker.offset_dir = Vector2.ZERO if stack == 0 else Vector2.RIGHT.rotated(stack * TAU / 6.0)
		marker.face(_heading(army, centroid))
		marker.set_selected(army_id == selected_army)
		marker.apply_scale(_current_scale)
		if _camera_distance >= 0.0:
			marker.set_view(_camera_distance, figure_weight(_camera_distance))
		_markers[army_id] = marker
		_homes[army_id] = marker.base_position
		if _plate_layer != null:
			var plate := build_plate(marker, marker.army_id == selected_army)
			_plate_layer.add_child(plate)
			_plates[army_id] = plate
	if not _markers.has(selected_army):
		selected_army = ""
	_placement_dirty = true
	_update_plates()


## Direction d'avance (coordonnées carte) : vers la prochaine étape du chemin, sinon un
## trois-quarts vers le sud-est (lisible avec la caméra par défaut).
func _heading(army: Dictionary, from: Vector2) -> Vector2:
	# Lot M4 : prochain coin du trajet libre en cours.
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


## Q2 : pousse le point hors de la zone d'une grande ville détaillée (L1-L3), côté sud-est.
func _clear_of_landmark(point: Vector2) -> Vector2:
	for zone in landmark_zones:
		var center := Vector2(zone.x, zone.y)
		var away := point - center
		if away.length() >= zone.z + LANDMARK_MARGIN_PX:
			continue
		var direction := away.normalized() if away.length() > 0.5 else Vector2(1.0, 0.6).normalized()
		return center + direction * (zone.z + LANDMARK_MARGIN_PX)
	return point


func set_selected(army_id: String) -> void:
	selected_army = army_id
	for id in _markers:
		_markers[id].set_selected(id == army_id)
	for id in _plates:
		_style_plate(_plates[id], _markers[id], id == army_id)
	_placement_dirty = true


func has_army(army_id: String) -> bool:
	return _markers.has(army_id)


func army_count() -> int:
	return _markers.size()


func plate_count() -> int:
	return _plates.size()


## Texte de la plaque d'une armée ("" si absente) : effectif et état.
func plate_text(army_id: String) -> String:
	var plate: PanelContainer = _plates.get(army_id)
	if plate == null:
		return ""
	var parts := PackedStringArray()
	for label in plate.find_children("*", "Label", true, false):
		parts.append((label as Label).text)
	return " ".join(parts)


## Armée la plus proche du point écran dans `PICK_RADIUS_PX` (étendard, figurines ou plaque),
## sinon "".
func pick_screen(screen_position: Vector2) -> String:
	return str(pick_screen_scored(screen_position).get("id", ""))


## Q2 : comme `pick_screen`, avec `score` (distance / `PICK_RADIUS_PX` ; 0 sur la plaque) pour
## départager une armée et une colonie sous le même clic ; {} si rien.
func pick_screen_scored(screen_position: Vector2) -> Dictionary:
	if camera == null:
		return {}
	var best := ""
	var best_distance := PICK_RADIUS_PX
	for id in _markers:
		var marker: ArmyMarker = _markers[id]
		for world in marker.pick_positions():
			if camera.is_position_behind(world):
				continue
			var distance := camera.unproject_position(world).distance_to(screen_position)
			if distance < best_distance:
				best_distance = distance
				best = id
		var plate: PanelContainer = _plates.get(id)
		if plate != null and plate.visible and plate.get_global_rect().has_point(screen_position):
			return {"id": id, "score": 0.0}
	return {"id": best, "score": best_distance / PICK_RADIUS_PX} if best != "" else {}


## Lot M4 (animation) : pose le marqueur de `army_id` au point carte `point`, tourné vers
## `heading` ; `point.x < 0` le remet à sa position issue de la simulation.
func place_marker(army_id: String, point: Vector2, heading: Vector2 = Vector2.ZERO) -> void:
	var marker: ArmyMarker = _markers.get(army_id)
	if marker == null:
		return
	# Lot CV2 : les figurines marchent pendant l'animation du déplacement.
	marker.set_walking(point.x >= 0.0)
	if point.x < 0.0:
		marker.base_position = _homes.get(army_id, marker.base_position)
	else:
		marker.base_position = Vector3(point.x, map_data.surface_world_at(point.x, point.y), point.y)
	marker.apply_scale(_current_scale)
	if heading.length() > 0.01:
		marker.face(heading.normalized())


## Position monde d'une armée (pour cadrer la caméra), Vector3.ZERO si absente.
func world_position_of(army_id: String) -> Vector3:
	var marker: ArmyMarker = _markers.get(army_id)
	return marker.base_position if marker != null else Vector3.ZERO


func update_scale(camera_distance: float) -> void:
	if absf(camera_distance - _camera_distance) > 0.25:
		_camera_distance = camera_distance
		var weight := figure_weight(camera_distance)
		for marker in _markers.values():
			marker.set_view(camera_distance, weight)
	var new_scale := scale_for_distance(camera_distance)
	if absf(new_scale - _current_scale) < 0.005:
		return
	_current_scale = new_scale
	for marker in _markers.values():
		marker.apply_scale(_current_scale)


## Échelle des marqueurs d'armée pour une distance caméra (voir `MIN_SCALE`).
static func scale_for_distance(camera_distance: float) -> float:
	return clampf(camera_distance * SCALE_PER_DISTANCE, MIN_SCALE, MAX_SCALE)


## Lot CV2 : présence des figurines (1 aux paliers près et moyen, 0 au palier loin, fondu
## sur la bande de transition de `ZoomTiers`) ; au loin restent l'étendard et la plaque.
func figure_weight(camera_distance: float) -> float:
	if _zoom_tiers == null:
		_zoom_tiers = ZoomTiers.load_default()
	return 1.0 - _zoom_tiers.far_weight(camera_distance)


func _process(delta: float) -> void:
	_placement_timer -= delta
	_update_plates()


# --- Plaques d'effectif ------------------------------------------------------------


static func build_plate(marker: ArmyMarker, selected: bool = false) -> PanelContainer:
	var plate := PanelContainer.new()
	plate.name = "Plate_" + marker.army_id
	plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", 4)
	plate.add_child(row)
	var heraldry := ArmyMarker._heraldry(marker.faction_id)
	if heraldry != null:
		var icon := TextureRect.new()
		icon.texture = heraldry
		icon.custom_minimum_size = Vector2(18, 18)
		icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(icon)
	else:
		var swatch := ColorRect.new()
		swatch.color = marker.faction_color
		swatch.custom_minimum_size = Vector2(12, 16)
		swatch.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(swatch)
	var count := Label.new()
	count.text = format_men(marker.men)
	count.tooltip_text = FrText.count(marker.unit_count, "unité")
	count.add_theme_font_size_override("font_size", PLATE_FONT_SIZE)
	count.add_theme_color_override("font_color", INK)
	count.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(count)
	var status_text: String = STATUS_TEXT.get(marker.status, "")
	if status_text != "":
		var status := Label.new()
		status.text = status_text
		status.add_theme_font_size_override("font_size", PLATE_FONT_SIZE - 3)
		status.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08) if marker.status == "siege" else INK.lightened(0.25))
		status.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(status)
	_style_plate(plate, marker, selected)
	return plate


static func _style_plate(plate: PanelContainer, marker: ArmyMarker, selected: bool) -> void:
	var style := StyleBoxFlat.new()
	style.bg_color = PARCHMENT if not selected else PARCHMENT.lightened(0.35)
	var border := PLAYER_BORDER if marker.is_player else OTHER_BORDER
	style.border_color = border if not selected else Color(1.0, 0.82, 0.3)
	style.set_border_width_all(2 if selected or marker.is_player else 1)
	style.set_corner_radius_all(3)
	style.content_margin_left = 6
	style.content_margin_right = 6
	style.content_margin_top = 1
	style.content_margin_bottom = 1
	style.shadow_color = Color(0, 0, 0, 0.35)
	style.shadow_size = 3
	style.shadow_offset = Vector2(1, 2)
	plate.add_theme_stylebox_override("panel", style)
	plate.z_index = 1 if selected else 0


static func format_men(men: int) -> String:
	var text := str(men)
	if men >= 10000:
		text = "%s %03d" % [men / 1000, men % 1000]
	elif men >= 1000:
		text = "%d %03d" % [men / 1000, men % 1000]
	return text


func _update_plates() -> void:
	if camera == null or _plates.is_empty():
		return
	var viewport_rect := get_viewport().get_visible_rect()
	var bases: Array = []  # [{id, rect, marker}] des plaques visibles
	for id in _plates:
		var plate: PanelContainer = _plates[id]
		var marker: ArmyMarker = _markers.get(id)
		if marker == null:
			plate.visible = false
			continue
		var anchor := marker.plate_anchor()
		if camera.is_position_behind(anchor):
			plate.visible = false
			continue
		var screen := camera.unproject_position(anchor)
		var size := plate.get_combined_minimum_size()
		plate.size = size
		var lift := -2.0 if marker.plate_below() else size.y + 2.0
		var base := (screen - Vector2(size.x * 0.5, lift)).round()
		plate.visible = viewport_rect.grow(40.0).has_point(screen)
		if plate.visible:
			bases.append({"id": id, "rect": Rect2(base, size), "marker": marker})
		plate.position = base + _plate_offsets.get(id, Vector2.ZERO)
	if _placement_due(viewport_rect.size):
		_place_plates(bases)


## Lot UX1 : recalcul quand la caméra ou la vue change, quand les armées changent, sinon à
## `PLACEMENT_INTERVAL` ; entre deux, les plaques gardent leur décalage (pas de clignotement).
func _placement_due(view: Vector2) -> bool:
	var moved := not camera.global_transform.is_equal_approx(_placement_camera) or view != _placement_view
	if not (moved or _placement_dirty or _placement_timer <= 0.0):
		return false
	_placement_camera = camera.global_transform
	_placement_view = view
	_placement_dirty = false
	_placement_timer = PLACEMENT_INTERVAL
	return true


func _place_plates(bases: Array) -> void:
	# Priorité : armée sélectionnée, armées du joueur, gros effectifs (gardent leur place).
	bases.sort_custom(_plate_before)
	var obstacles: Array = label_obstacles.call(camera) if label_obstacles.is_valid() else []
	_plate_offsets = _placer.place(bases, obstacles)
	for entry in bases:
		var plate: PanelContainer = _plates[entry["id"]]
		plate.position = (entry["rect"] as Rect2).position + _plate_offsets.get(entry["id"], Vector2.ZERO)


func _plate_before(a: Dictionary, b: Dictionary) -> bool:
	var ma: ArmyMarker = a["marker"]
	var mb: ArmyMarker = b["marker"]
	var sa: bool = a["id"] == selected_army
	var sb: bool = b["id"] == selected_army
	if sa != sb:
		return sa
	if ma.is_player != mb.is_player:
		return ma.is_player
	if ma.men != mb.men:
		return ma.men > mb.men
	return str(a["id"]) < str(b["id"])


## Lot UX1 : décalage écran appliqué à la plaque d'une armée (Vector2.ZERO = à sa place).
func plate_offset(army_id: String) -> Vector2:
	return _plate_offsets.get(army_id, Vector2.ZERO)
