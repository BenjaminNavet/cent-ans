class_name IncidentMarkers
extends Node

## Lot FK5a (`docs/design/2026-09-29-carte-vivante-folk.md` § 3.4) : incidents posés sur la carte.
## Chaque décision en attente du joueur de présentation `map` (`CampaignSim.get_pending_decisions`,
## champs `province`, `presentation`, `expires_in`) reçoit un sceau de cire au-dessus de sa province,
## visible à tous les zooms (contrôle d'écran, comme les sites de rencontre CV3), avec le nombre
## de tours restants ; clic → la fenêtre de décision de la chronique, sur cette décision, la caméra
## glissant vers la province. Rendu seulement : décisions, provinces et échéances viennent du cœur.
## La scène de l'incident (FK4, `folk_scenes.gd`) est posée à part via `get_map_scenes` ; LR-02 :
## un clic sur cette scène (disque invisible sur son emprise) ouvre l'incident comme le sceau.
## Branché par `CampaignLife` (`--folk-off=incidents` le coupe : la chronique rouvre alors ces
## décisions en fenêtre de début de tour, voir `ChronicleController.modal_pending`).

const MARKER_SIZE := 34.0
## Décalage vertical au-dessus du centre de la province (px d'écran), comme les rencontres.
const LIFT := 0.9

## Rayon écran (px) de la zone cliquable d'une scène : bornes pour qu'elle reste atteignable de loin
## et ne recouvre pas la carte de près.
const SCENE_HIT_MIN := 22.0
const SCENE_HIT_MAX := 130.0

var map: Node = null  # CampaignMap
## `FolkScenes` de `CampaignLife` : fournit emprise et centre de la scène de la province.
var folk_scenes: FolkScenes = null
var _layer: CanvasLayer
var _markers: Dictionary = {}  # decision id → IncidentSeal
var _scene_hits: Dictionary = {}  # decision id → IncidentSceneHit


## Zone cliquable (invisible) posée sur la scène d'un incident : même geste que le sceau.
class IncidentSceneHit:
	extends Control

	var decision_id := -1
	var world := Vector3.ZERO
	var radius := 40.0
	var controller: IncidentMarkers

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND

	func _make_custom_tooltip(for_text: String) -> Object:
		return TooltipHost.bubble(for_text, self)

	## Disque, pas le carré englobant.
	func _has_point(point: Vector2) -> bool:
		return point.distance_to(size * 0.5) <= radius

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click == null or not click.pressed or click.button_index != MOUSE_BUTTON_LEFT:
			return
		accept_event()
		controller.decision_clicked(decision_id)


## Sceau d'un incident : galette de cire, pictogramme « décision » en relief, pastille parchemin
## des tours restants.
class IncidentSeal:
	extends Control

	var decision: Dictionary = {}
	var world := Vector3.ZERO
	var controller: IncidentMarkers
	var _hover := false

	func _ready() -> void:
		custom_minimum_size = Vector2.ONE * IncidentMarkers.MARKER_SIZE
		size = custom_minimum_size
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		mouse_entered.connect(func() -> void:
			_hover = true
			queue_redraw())
		mouse_exited.connect(func() -> void:
			_hover = false
			queue_redraw())

	func _make_custom_tooltip(for_text: String) -> Object:
		return TooltipHost.bubble(for_text, self)

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click == null or not click.pressed or click.button_index != MOUSE_BUTTON_LEFT:
			return
		accept_event()
		controller.decision_clicked(int(decision.get("id", -1)))

	func turns_left() -> int:
		return maxi(1, int(decision.get("expires_in", 1)))

	func _draw() -> void:
		var center := size * 0.5
		var radius := minf(size.x, size.y) * 0.5 - 2.0
		var wax := HudStyle.WAX_LIGHT if _hover else HudStyle.WAX
		var inner := HudStyle.draw_wax_seal(self, center, radius, wax, int(decision.get("id", 7)) % 31)
		var texture := HudStyle.icon("hud_chronicle_decision", "hud")
		if texture != null:
			HudStyle.draw_texture_fit(self, texture, center, inner * 1.5, HudStyle.PARCHMENT_LIGHT)
		else:
			HudStyle.draw_glyph(self, "chronicle_decision", center, inner * 1.5, HudStyle.PARCHMENT_LIGHT, wax)
		# Pastille des tours restants (rubrique quand il ne reste que ce tour).
		var badge_center := center + Vector2(radius * 0.72, radius * 0.72)
		var badge_radius := radius * 0.42
		draw_circle(badge_center + Vector2(0.5, 1.0), badge_radius, HudStyle.SHADOW)
		draw_circle(badge_center, badge_radius, HudStyle.PARCHMENT_LIGHT)
		var urgent := turns_left() <= 1
		draw_arc(badge_center, badge_radius - 0.5, 0.0, TAU, 24, HudStyle.RUBRIC if urgent else HudStyle.GOLD, 1.5, true)
		var font := get_theme_default_font()
		var font_size := int(badge_radius * 1.35)
		var text := str(turns_left())
		var text_size := font.get_string_size(text, HORIZONTAL_ALIGNMENT_CENTER, -1, font_size)
		var baseline := badge_center + Vector2(-text_size.x * 0.5, font.get_ascent(font_size) * 0.5 - 1.0)
		draw_string(font, baseline, text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, HudStyle.RUBRIC if urgent else HudStyle.INK)


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "IncidentMarkers"
	_layer = CanvasLayer.new()
	_layer.name = "IncidentSeals"
	_layer.layer = 0
	add_child(_layer)


## Vrai pour une décision posée sur la carte (présentation `map`, province connue).
static func is_map_decision(decision: Dictionary) -> bool:
	return str(decision.get("presentation", "")) == "map" and str(decision.get("province", "")) != ""


func marker_count() -> int:
	return _markers.size()


func marker(decision_id: int) -> Control:
	return _markers.get(decision_id)


## Zone cliquable de la scène d'un incident (null sans scène dans la province).
func scene_hit(decision_id: int) -> Control:
	return _scene_hits.get(decision_id)


## Scène posée dans la province d'un incident (`FolkScenes.staged`), vide si aucune.
func _scene_of(province_id: String) -> Dictionary:
	if folk_scenes == null:
		return {}
	for scene: Dictionary in folk_scenes.staged:
		if str(scene.get("province", "")) == province_id:
			return scene
	return {}


## Après tout changement d'état (décision prise, fin de tour) : un sceau par incident en attente.
func refresh(sim: Object) -> void:
	if map == null or sim == null or not sim.has_method("get_pending_decisions"):
		return
	var seen := {}
	for entry in sim.call("get_pending_decisions"):
		var decision: Dictionary = entry
		if not is_map_decision(decision):
			continue
		var id := int(decision.get("id", -1))
		var world: Variant = _world_of_province(str(decision["province"]))
		if world == null:
			continue
		seen[id] = true
		var node: IncidentSeal = _markers.get(id)
		if node == null:
			node = IncidentSeal.new()
			node.name = "Incident_%d" % id
			node.controller = self
			_layer.add_child(node)
			_markers[id] = node
		node.decision = decision
		node.world = world
		node.tooltip_text = seal_tooltip(decision)
		node.queue_redraw()
		_refresh_scene_hit(id, decision)
	for id in _markers.keys():
		if not seen.has(id):
			_markers[id].queue_free()
			_markers.erase(id)
	for id in _scene_hits.keys():
		if not seen.has(id) or _scene_of(str(_markers[id].decision["province"])).is_empty():
			_scene_hits[id].queue_free()
			_scene_hits.erase(id)
	_place_markers()


func _refresh_scene_hit(id: int, decision: Dictionary) -> void:
	var scene := _scene_of(str(decision["province"]))
	if scene.is_empty():
		return
	var hit: IncidentSceneHit = _scene_hits.get(id)
	if hit == null:
		hit = IncidentSceneHit.new()
		hit.name = "IncidentScene_%d" % id
		hit.controller = self
		hit.decision_id = id
		_layer.add_child(hit)
		_scene_hits[id] = hit
	var center: Vector2 = scene["center"]
	var map_data: MapData = map.get("map_data")
	hit.world = Vector3(center.x, map_data.surface_world_at(center.x, center.y), center.y)
	hit.set_meta("footprint", folk_scenes._footprint(int(scene["index"])))
	hit.tooltip_text = seal_tooltip(decision)


## Crochet de `CampaignLife.update_view` (le placement suit la caméra à chaque image).
func update_view(_camera_distance: float) -> void:
	pass


## Bulle IB d'un incident : rubrique « Incident », titre, province, échéance, geste.
static func seal_tooltip(decision: Dictionary) -> String:
	var lines := PackedStringArray(["[b]%s[/b]" % str(decision.get("title", ""))])
	var province := str(decision.get("province_name", ""))
	if province != "":
		lines.append(province)
	var expires := int(decision.get("expires_in", 0))
	if expires <= 1:
		lines.append("[color=#9e2114]À décider ce tour-ci, sinon le conseil tranche.[/color]")
	else:
		lines.append("Encore %s pour décider, sinon le conseil tranche." % FrText.count(expires, "tour"))
	lines.append("[i]Clic : ouvrir la décision.[/i]")
	return RichTooltip.hud("hud_incident", "\n".join(lines))


## Clic sur un sceau : la fenêtre de la chronique sur cette décision, ancrée sur la province.
func decision_clicked(decision_id: int) -> void:
	var node: IncidentSeal = _markers.get(decision_id)
	if node == null or map == null:
		return
	var chronicle: Node = map.get("chronicle")
	if chronicle == null or not chronicle.has_method("open_decision"):
		return
	var rig := map.get("camera_rig") as CampaignCamera
	if rig != null:
		rig.look_at_point(node.world)
	chronicle.call("open_decision", decision_id)


func _world_of_province(province_id: String) -> Variant:
	var map_data: MapData = map.get("map_data")
	if map_data == null or map_data.index_of_id(province_id) <= 0:
		return null
	var centroid := map_data.centroid_of_id(province_id)
	return Vector3(centroid.x, map_data.surface_world_at(centroid.x, centroid.y), centroid.y)


func _place_markers() -> void:
	var camera: Camera3D = map.get("camera") if map != null else null
	if camera == null:
		return
	var mode := MapReadability.map_mode_of(map)
	for node: IncidentSeal in _markers.values():
		# TB2 : sceau réservé à la couche « Signes » et aux modes de carte, sauf dernier tour.
		if camera.is_position_behind(node.world) or not MapReadability.sign_shown("incident", mode, node.turns_left() <= 1):
			node.visible = false
			continue
		node.visible = true
		node.position = camera.unproject_position(node.world) - node.size * 0.5 - Vector2(0.0, MARKER_SIZE * LIFT)
	_place_scene_hits(camera)


## Place chaque zone de scène : disque centré sur la scène, rayon = emprise projetée, borné.
func _place_scene_hits(camera: Camera3D) -> void:
	for id in _scene_hits.keys():
		var hit: IncidentSceneHit = _scene_hits[id]
		var seal: IncidentSeal = _markers.get(id)
		var shown := seal != null and seal.visible and not camera.is_position_behind(hit.world)
		hit.visible = shown
		if not shown:
			continue
		var edge := hit.world + Vector3(float(hit.get_meta("footprint", 1.0)), 0.0, 0.0)
		var center := camera.unproject_position(hit.world)
		hit.radius = clampf(center.distance_to(camera.unproject_position(edge)), SCENE_HIT_MIN, SCENE_HIT_MAX)
		hit.size = Vector2.ONE * hit.radius * 2.0
		hit.position = center - hit.size * 0.5


func _process(_delta: float) -> void:
	if not _markers.is_empty():
		_place_markers()
