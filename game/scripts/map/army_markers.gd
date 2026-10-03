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
## SA (ADR 0160) : tolérance autour de la silhouette de visée (clic juste à côté, sans rien
## d'autre dessous).
const PICK_NEAR_PX := 10.0
## Échelle du pion sur le parchemin = distance caméra × facteur, bornée (taille constante à
## l'écran) ; en vue normale la loi est sous-linéaire (`scale_for_distance`).
const SCALE_PER_DISTANCE := 0.014
## SA (ADR 0160) : exposant par défaut de la loi d'échelle en vue normale (1 = taille écran
## constante, 0 = taille monde fixe) ; réglage `map.army_scale_exponent`.
const DEFAULT_SCALE_EXPONENT := 0.65
## SA : écart à la ville d'une armée en garnison, côté sud-est (vers la caméra par défaut), et
## marge au-delà du rayon de la ville (unités monde).
const STANDOFF_DIRECTION := Vector2(0.86, 0.51)
const STANDOFF_MARGIN := 0.4
## Q2 : plafond de taille monde au palier « près » (la ville doit dominer l'armée).
## À cette échelle l'étendard royal fait ~3 unités et l'escorte ~2,5 de large, soit environ un
## quart du diamètre de Paris (L1, `core_radius_px` 6) et moins qu'une ville L2/L3 (4,5-7) ;
## les figurines ont la hauteur des maisons. Atteint vers la distance 20 ; 0,8 auparavant, qui
## faisait recouvrir Paris par l'ost au plus près.
const MIN_SCALE := 0.27
const MAX_SCALE := 14.0
## Lot ZG4 : distance sous laquelle l'échelle décroît de nouveau avec la distance.
const CLOSE_KNEE_DISTANCE := 12.0
## Lot ZG4 : portée des plaques sous `CLOSE_KNEE_DISTANCE`, en multiples de la distance caméra.
const CLOSE_PLATE_RANGE_FACTOR := 40.0

## Distance minimale (pixels de carte) entre une armée et le modèle de ville de la province.
const CITY_CLEARANCE_PX := 26.0

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
## Lot M5a : armées montrées au joueur (id → true, vue par case de la simulation). Quand
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
## Lot UX1 (A3 C8) : noms de ville à éviter, `func(camera: Camera3D) -> Array[Rect2]` (rectangles
## écran), posé par la carte ; les plaques s'en écartent (`LabelPlacer`).
var label_obstacles: Callable = Callable()
## Lot ZG4 : surface affichée (`TerrainBuilder.surface_height_at`) quand le relief streamé est
## actif ; à défaut, heightmap 4096 (`MapData.surface_world_at`).
var ground_height: Callable = Callable()

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
		# Lot M4 : une armée en campagne se tient à sa position libre ; seules les armées
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
				_style_plate(plate, marker, marker.army_id == selected_army, marker.army_id == hovered_army)
			else:
				if plate != null and is_instance_valid(plate):
					plate.queue_free()
				plate = build_plate(marker, marker.army_id == selected_army)
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
	_placement_dirty = true
	_update_plates()


## Tout ce que `ArmyMarker.setup` lit de l'armée, sauf sa position (replacée à chaque refresh).
static func _signature(army: Dictionary, color: Color, player: bool) -> String:
	var copy := army.duplicate()
	copy.erase("position")
	return "%s|%s|%s" % [var_to_str(copy), color.to_html(), player]


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
		_style_plate(_plates[id], _markers[id], id == army_id, id == hovered_army)
	_placement_dirty = true


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
			_style_plate(plate, marker, id == selected_army, id == army_id)


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


## Q2 / SA (ADR 0160) : comme `pick_screen`, avec `score` et `direct` pour départager une armée
## et une colonie sous le même point. `direct` : le point est dans la silhouette projetée de
## l'armée (`ArmyMarker.screen_rect`) ou sur sa plaque ; `score` vaut alors 0 sur la plaque,
## sinon la distance normalisée au centre de la silhouette (0 à 1). À moins de `PICK_NEAR_PX`
## de la silhouette : `direct` faux, `score` entre 1 et 2. {} si rien.
## `exclude` : armée ignorée (la sélection, quand on vise une cible pour elle).
func pick_screen_scored(screen_position: Vector2, exclude: String = "") -> Dictionary:
	if camera == null:
		return {}
	var best := ""
	var best_score := INF
	for id in _markers:
		var marker: ArmyMarker = _markers[id]
		if id == exclude or not marker.is_visible_in_tree():
			continue
		var plate: PanelContainer = _plates.get(id)
		if plate != null and plate.visible and plate.get_global_rect().has_point(screen_position):
			return {"id": id, "score": 0.0, "direct": true}
		var rect := marker.screen_rect(camera)
		if rect.size == Vector2.ZERO:
			continue
		var score := INF
		if rect.has_point(screen_position):
			var offset := (screen_position - rect.get_center()).abs() / (rect.size * 0.5)
			score = maxf(offset.x, offset.y)
		else:
			var outside := (screen_position - rect.get_center()).abs() - rect.size * 0.5
			var gap := Vector2(maxf(outside.x, 0.0), maxf(outside.y, 0.0)).length()
			if gap < PICK_NEAR_PX:
				score = 1.0 + gap / PICK_NEAR_PX
		if score < best_score:
			best_score = score
			best = id
	return {"id": best, "score": best_score, "direct": best_score <= 1.0} if best != "" else {}


## Lot M4 (animation) : pose le marqueur de `army_id` au point carte `point`, tourné vers
## `heading` ; `point.x < 0` le remet à sa position issue de la simulation.
func place_marker(army_id: String, point: Vector2, heading: Vector2 = Vector2.ZERO) -> void:
	var marker: ArmyMarker = _markers.get(army_id)
	if marker == null:
		return
	# Lot CV2 : les figurines marchent pendant l'animation du déplacement.
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


## Lot CV3-4 : point du monde au sol d'un point carte (pixels, convention de `position` des
## armées) — sites de rencontre posés comme les armées.
func world_at_pixel(p: Vector2) -> Vector3:
	return to_global(Vector3(p.x, _ground(p), p.y))


## Hauteur du sol au point carte `p` (surface affichée si disponible).
func _ground(p: Vector2) -> float:
	if ground_height.is_valid():
		return float(ground_height.call(p.x, p.y))
	return map_data.surface_world_at(p.x, p.y)


## Lot ZG4 : repose tous les marqueurs sur le sol (échelle verticale changée, pages de relief
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
	var new_scale := scale_for_distance(camera_distance, 1.0 - figure_weight(camera_distance), scale_exponent())
	if absf(new_scale - _current_scale) < minf(0.005, new_scale * 0.02):
		return
	_current_scale = new_scale
	for marker in _markers.values():
		marker.apply_scale(_current_scale)


## SA (ADR 0160) : exposant de la loi d'échelle (`map.army_scale_exponent` de
## `data/ui/campaign_map.json`, borné à [0 ; 1]).
static func scale_exponent() -> float:
	return clampf(float(ArmyFigures.map_settings().get("army_scale_exponent", DEFAULT_SCALE_EXPONENT)), 0.0, 1.0)


## Échelle des marqueurs d'armée pour une distance caméra.
## - Sous `CLOSE_KNEE_DISTANCE` (lot ZG4, vues vallée et site) : proportionnelle à la distance
##   (pas d'étendard de 2 km au-dessus d'un site vu à 200 m).
## - Jusqu'à `MIN_SCALE / SCALE_PER_DISTANCE` (≈ 19) : taille monde fixe `MIN_SCALE` (Q2).
## - Au-delà, vue normale (SA, ADR 0160) : `MIN_SCALE × (d / 19)^exponent`. Avec un exposant
##   inférieur à 1, l'ost rétrécit à l'écran en dézoomant, comme un objet posé sur la carte,
##   au lieu de gonfler avec la distance ; `exponent` = 1 redonne la taille écran constante.
## - Sur le parchemin (`strategic_weight` → 1) : retour à la loi linéaire bornée, l'étendard
##   devient un pion de taille écran constante.
static func scale_for_distance(camera_distance: float, strategic_weight: float = 0.0, exponent: float = DEFAULT_SCALE_EXPONENT) -> float:
	if camera_distance < CLOSE_KNEE_DISTANCE:
		return MIN_SCALE * maxf(camera_distance, 0.02) / CLOSE_KNEE_DISTANCE
	var token := clampf(camera_distance * SCALE_PER_DISTANCE, MIN_SCALE, MAX_SCALE)
	var knee := MIN_SCALE / SCALE_PER_DISTANCE
	var posed := minf(MIN_SCALE * pow(maxf(camera_distance / knee, 1.0), exponent), token)
	return lerpf(posed, token, clampf(strategic_weight, 0.0, 1.0))


## Lot CV2 / DV : présence des figurines (1 en vue normale, 0 sur le parchemin, fondu sur la
## bande de `ZoomTiers.strategic_weight`) ; au loin restent l'étendard et la plaque.
func figure_weight(camera_distance: float) -> float:
	if _zoom_tiers == null:
		_zoom_tiers = ZoomTiers.load_default()
	return 1.0 - _zoom_tiers.strategic_weight(camera_distance)


func _process(delta: float) -> void:
	_placement_timer -= delta
	_update_plates()


# --- Plaques d'effectif ------------------------------------------------------------


## A6-C3 : facteur de taille des plaques (`map.plate_scale` de `data/ui/campaign_map.json`, 0,8 = −20 %).
static func plate_scale() -> float:
	return clampf(float(ArmyFigures.map_settings().get("plate_scale", 1.0)), 0.3, 1.5)


static func _ps(value: float) -> int:
	return maxi(1, roundi(value * plate_scale()))


static func build_plate(marker: ArmyMarker, selected: bool = false) -> PanelContainer:
	var plate := PanelContainer.new()
	plate.name = "Plate_" + marker.army_id
	plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", _ps(4.0))
	plate.add_child(row)
	var heraldry := ArmyMarker._heraldry(marker.faction_id)
	if heraldry != null:
		var icon := TextureRect.new()
		icon.texture = heraldry
		icon.custom_minimum_size = Vector2(_ps(18.0), _ps(18.0))
		icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(icon)
	else:
		var swatch := ColorRect.new()
		swatch.color = marker.faction_color
		swatch.custom_minimum_size = Vector2(_ps(12.0), _ps(16.0))
		swatch.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(swatch)
	var glyph_text := StanceCues.plate_glyph(marker.cue)
	if glyph_text != "":  # EN : marque des armées ennemies, lisible sans la couleur
		var glyph := Label.new()
		glyph.name = "EnemyGlyph"
		glyph.text = glyph_text
		glyph.add_theme_font_size_override("font_size", _ps(PLATE_FONT_SIZE - 2.0))
		glyph.add_theme_color_override("font_color", StanceCues.plate_border(marker.cue, OTHER_BORDER, 1)["color"])
		glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(glyph)
	var count := Label.new()
	count.text = format_men(marker.men)
	count.tooltip_text = FrText.count(marker.unit_count, "unité")
	count.add_theme_font_size_override("font_size", _ps(float(PLATE_FONT_SIZE)))
	count.add_theme_color_override("font_color", INK)
	count.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(count)
	var status_text: String = STATUS_TEXT.get(marker.status, "")
	if status_text != "":
		var status := Label.new()
		status.text = status_text
		status.add_theme_font_size_override("font_size", _ps(PLATE_FONT_SIZE - 3.0))
		status.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08) if marker.status == "siege" else INK.lightened(0.25))
		status.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_child(status)
	_style_plate(plate, marker, selected)
	return plate


## `hovered` (SA, ADR 0160) : plaque de l'armée sous le curseur, fond clair et liseré doré épais.
static func _style_plate(plate: PanelContainer, marker: ArmyMarker, selected: bool, hovered: bool = false) -> void:
	var style := StyleBoxFlat.new()
	style.bg_color = PARCHMENT if not (selected or hovered) else PARCHMENT.lightened(0.5 if hovered else 0.35)
	# EN : bordure selon la relation avec le joueur (rouge réservé aux ennemis) ; au survol
	# (SA) elle garde sa couleur de relation et s'épaissit, le doré reste à la sélection.
	var border := StanceCues.plate_border(marker.cue, PLAYER_BORDER if marker.is_player else OTHER_BORDER, 2 if marker.is_player else 1)
	style.border_color = border["color"] if not selected else Color(1.0, 0.82, 0.3)
	style.set_border_width_all(3 if hovered else (2 if selected else int(border["width"])))
	style.set_corner_radius_all(3)
	style.content_margin_left = _ps(6.0)
	style.content_margin_right = _ps(6.0)
	style.content_margin_top = 1
	style.content_margin_bottom = 1
	style.shadow_color = Color(0, 0, 0, 0.35)
	style.shadow_size = 3
	style.shadow_offset = Vector2(1, 2)
	plate.add_theme_stylebox_override("panel", style)
	plate.z_index = 2 if hovered else (1 if selected else 0)


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
		if camera.is_position_behind(anchor) or not marker.is_visible_in_tree():
			plate.visible = false
			continue
		# ZG4 : vues vallée / site (rasantes) : pas de plaques d'armées lointaines sur l'horizon.
		if _camera_distance >= 0.0 and _camera_distance < CLOSE_KNEE_DISTANCE and id != selected_army \
				and camera.global_position.distance_to(anchor) > CLOSE_PLATE_RANGE_FACTOR * _camera_distance:
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


## CV3-0 (#7) : rectangles écran des plaques d'armée actuellement affichées (position finale,
## après placement de ce lot). Utilisé par `SettlementLayer` pour que ses marqueurs et noms de
## colonies (Saint-Denis, Paris...) s'écartent des plaques, au lieu de se chevaucher au pied de
## l'armée. Même forme que `label_obstacles` (`func(camera) -> Array[Rect2]`), `camera` ignoré.
func screen_label_rects(_camera: Camera3D = null) -> Array:
	var rects: Array = []
	for id in _plates:
		var plate: PanelContainer = _plates[id]
		if plate.visible:
			rects.append(Rect2(plate.position, plate.size))
	return rects
