class_name EncounterController
extends Node

## Lot CV3-4 : rencontres sur la carte de campagne (spec campagne vivante § 2 et § 4).
## - Marqueurs des sites que le joueur voit (`CampaignSim.get_encounter_sites`, positions en
##   pixels de carte comme les armées) : médaillon parchemin à l'icône à l'encre, bulle au
##   survol (titre, province, expiration) ; clic (gauche ou droit) avec une armée du joueur
##   sélectionnée = ordre de déplacement normal vers le site.
## - Fenêtre de choix `EncounterWindow` (gabarit de la chronique) ouverte quand
##   `get_pending_encounters` n'est pas vide, comme le dialogue des batailles en attente ;
##   le choix part par `choose_encounter_option`.
## Aucune règle ici : sites, options, disponibilités et raisons viennent du cœur.

const MARKER_SIZE := 30.0

var map: Node = null  # CampaignMap
var window: EncounterWindow
var _layer: CanvasLayer
var _markers: Dictionary = {}  # site id → SiteMarker
## Rencontres fermées par « Plus tard » (clé "armée:site") : pas de réouverture automatique.
var _dismissed: Dictionary = {}
var _watched_report: Control = null


## Marqueur d'un site : disque parchemin cerclé d'or, icône « rencontre » à l'encre.
class SiteMarker:
	extends Control

	var site: Dictionary = {}
	var world := Vector3.ZERO
	var controller: EncounterController
	var _hover := false

	func _ready() -> void:
		custom_minimum_size = Vector2.ONE * EncounterController.MARKER_SIZE
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
		return RichTooltip.make_panel(for_text)

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click == null or not click.pressed:
			return
		if click.button_index == MOUSE_BUTTON_LEFT or click.button_index == MOUSE_BUTTON_RIGHT:
			accept_event()
			controller.site_clicked(int(site.get("id", -1)))

	func _draw() -> void:
		var center := size * 0.5
		var radius := minf(size.x, size.y) * 0.5 - 1.0
		draw_circle(center + Vector2(1, 2), radius, HudStyle.SHADOW)
		draw_circle(center, radius, HudStyle.PARCHMENT_LIGHT)
		var ring := HudStyle.GOLD_PALE if _hover else HudStyle.GOLD
		draw_arc(center, radius - 1.0, 0.0, TAU, 32, ring, 2.5 if _hover else 2.0, true)
		if bool(site.get("claimed", false)):
			draw_arc(center, radius + 2.0, 0.0, TAU, 32, HudStyle.RUBRIC, 2.0, true)
		var texture := HudStyle.icon("hud_encounter", "hud")
		if texture != null:
			HudStyle.draw_texture_fit(self, texture, center, radius * 1.45)
		else:
			HudStyle.draw_glyph(self, "chronicle", center, radius * 1.4, HudStyle.INK)


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "EncounterController"
	_layer = CanvasLayer.new()
	_layer.name = "EncounterSites"
	_layer.layer = 0
	add_child(_layer)
	window = EncounterWindow.new()
	window.hide()
	# PO1 (bible DA § 12.1) : choix bloquant → zone `MODAL` de `UiLayout` (fond assombri, entrées
	# bloquées derrière), puis inscrit dans la pile (Échap) : après le reparentage, qui désinscrit.
	UiZones.put(UiZones.Zone.MODAL, window)
	map.ui.register_panel(window, PanelStack.Kind.CENTRAL)
	window.encounter_option_chosen.connect(_on_option_chosen)
	window.closed.connect(_on_window_closed)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_encounter_sites")


func sites() -> Array:
	return map.sim.call("get_encounter_sites") if available() else []


func pending() -> Array:
	return map.sim.call("get_pending_encounters") if available() else []


func marker_count() -> int:
	return _markers.size()


func marker(site_id: int) -> Control:
	return _markers.get(site_id)


## Après tout changement d'état : marqueurs, puis fenêtre si une rencontre attend.
func refresh() -> void:
	_refresh_markers()
	offer_pending()


func _refresh_markers() -> void:
	var seen := {}
	for entry in sites():
		var site: Dictionary = entry
		var id := int(site.get("id", -1))
		seen[id] = true
		var node: SiteMarker = _markers.get(id)
		if node == null:
			node = SiteMarker.new()
			node.name = "Site_%d" % id
			node.controller = self
			_layer.add_child(node)
			_markers[id] = node
		node.site = site
		node.world = _world_of(site.get("position", Vector2.ZERO))
		node.tooltip_text = site_tooltip(site)
		node.queue_redraw()
	for id in _markers.keys():
		if not seen.has(id):
			_markers[id].queue_free()
			_markers.erase(id)
	_place_markers()


## Bulle d'un site : rubrique « Rencontre », titre, province, expiration.
static func site_tooltip(site: Dictionary) -> String:
	var lines := PackedStringArray(["[b]%s[/b]" % str(site.get("title", ""))])
	var province := str(site.get("province_name", ""))
	if province != "":
		lines.append(province)
	var expires := int(site.get("expires_in", 0))
	if bool(site.get("claimed", false)):
		lines.append("[color=#9e2114]Une de vos armées l'a atteinte : décision en attente.[/color]")
	elif expires <= 1:
		lines.append("Disparaît à la fin de la saison.")
	else:
		lines.append("Disparaît dans %s." % FrText.count(expires, "saison"))
	lines.append("[i]Armée sélectionnée : clic pour y marcher.[/i]")
	return RichTooltip.hud("hud_encounter", "\n".join(lines))


func _world_of(point: Variant) -> Vector3:
	var pixel: Vector2 = point if point is Vector2 else Vector2.ZERO
	if map.armies != null and map.armies.has_method("world_at_pixel"):
		return map.armies.world_at_pixel(pixel)
	return Vector3(pixel.x, 0.0, pixel.y)


func _place_markers() -> void:
	var camera: Camera3D = map.camera if map != null else null
	if camera == null:
		return
	var mode := MapReadability.map_mode_of(map)
	var army_selected := str(map.get("selected_army")) != ""
	for node: SiteMarker in _markers.values():
		# TB2 : site réservé à la couche « Signes », sauf décision en attente, dernier tour ou
		# armée sélectionnée (le site est une destination).
		var urgent := bool(node.site.get("claimed", false)) or int(node.site.get("expires_in", 0)) <= 1
		if camera.is_position_behind(node.world) or not MapReadability.sign_shown("encounter", mode, urgent, army_selected):
			node.visible = false
			continue
		node.visible = true
		node.position = camera.unproject_position(node.world) - node.size * 0.5 - Vector2(0.0, MARKER_SIZE * 0.4)


func _process(_delta: float) -> void:
	if not _markers.is_empty():
		_place_markers()


## Clic sur un site : l'armée du joueur sélectionnée y marche (ordre de déplacement normal).
func site_clicked(site_id: int) -> void:
	var node: SiteMarker = _markers.get(site_id)
	if node == null:
		return
	var army_id := str(map.selected_army)
	var army: Dictionary = map.sim.call("get_army", army_id) if army_id != "" else {}
	if army.is_empty() or str(army.get("faction", "")) != str(map.player_faction):
		map.ui.show_toast("Sélectionnez d'abord une de vos armées pour marcher vers « %s »." % str(node.site.get("title", "")))
		return
	var point: Vector2 = node.site.get("position", Vector2.ZERO)
	var report: Dictionary = map.movement_ctl.order_move_point(army_id, point) if map.movement_ctl != null \
		else map.sim.call("move_army_to", army_id, point.x, point.y)
	if not report.get("ok", false):
		map.ui.show_toast(str(report.get("error", "Marche impossible.")), true)


# --- Fenêtre de choix ---------------------------------------------------------------------


static func key_of(encounter: Dictionary) -> String:
	return "%s:%d" % [str(encounter.get("army", "")), int(encounter.get("site", -1))]


## Ouvre la fenêtre sur la première rencontre en attente non écartée ; sauf pendant la fin
## de tour, le dialogue d'une bataille ou le rapport de saison (elle suit leur fermeture).
func offer_pending() -> bool:
	if not available() or window == null:
		return false
	var queue := pending()
	if window.visible:
		var current := key_of(window.current_encounter())
		for encounter in queue:
			if key_of(encounter) == current:
				return true
		window.hide()
	var open: Array = queue.filter(func(e: Dictionary) -> bool: return not _dismissed.has(key_of(e)))
	if open.is_empty() or bool(map.get("end_turn_running")):
		return false
	var dialog: Control = map.get("_battle_dialog")
	if dialog != null and dialog.visible:
		return false
	if _report_open():
		return false
	window.show_encounter(open[0], open.size())
	return true


## Ouvre la fenêtre sur la rencontre en attente, même écartée (bulle, tests).
func open_window() -> bool:
	_dismissed.clear()
	return offer_pending()


func _report_open() -> bool:
	var flow: Node = map.get("flow")
	var report: Control = flow.get("season_report") if flow != null else null
	if report == null or not report.visible:
		return false
	if _watched_report != report:
		_watched_report = report
		report.visibility_changed.connect(_on_report_visibility)
	return true


func _on_report_visibility() -> void:
	if _watched_report == null or _watched_report.visible:
		return
	_watched_report.visibility_changed.disconnect(_on_report_visibility)
	_watched_report = null
	offer_pending()


func _on_window_closed() -> void:
	var current: Dictionary = window._encounter
	if not current.is_empty():
		_dismissed[key_of(current)] = true


func _on_option_chosen(army_id: String, site: int, option: int) -> void:
	var result: Dictionary = map.sim.call("choose_encounter_option", army_id, site, option)
	if result.get("ok", false):
		map.ui.show_toast("Décision prise.")
	else:
		map.ui.show_toast(str(result.get("error", "Choix impossible")), true)
	window.hide()
	map.refresh_all()
	if map.has_method("_offer_pending_battles"):  # une option « bataille » ouvre son dialogue
		map.call("_offer_pending_battles")
