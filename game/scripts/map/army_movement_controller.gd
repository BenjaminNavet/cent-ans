class_name ArmyMovementController
extends Node

## Lot M4 : mouvement libre des armées sur la carte de campagne (spec
## `docs/design/2026-09-24-mouvement-libre.md` § 6). Rendu, UI et entrées seulement ; toute
## règle vient de `CampaignSim` :
## - bulle des cases atteignables ce tour (`get_reachable_area`, `ArmyMovementBubble`) ;
## - chemin prévu en deux couleurs, ce tour puis les tours suivants (`find_path_points`,
##   `ArmyMovementPath`), qui suit le curseur ;
## - clic droit : sur le sol → `move_army_to` ; sur une armée ennemie → `attack_army` ; sur
##   une colonie → `move_army_to_settlement` (marche, siège ou stationnement) ;
## - attaque (lot AT1) : curseur « épées croisées » (`AttackCursor`) sur une armée ou une place
##   attaquable ; sur une place ennemie, l'arrivée ce tour enchaîne l'assaut (dialogue
##   d'avant-bataille de siège : assaut, résolution automatique ou maintien du siège) ; contre
##   une faction en paix, une confirmation (`WarDeclarationDialog`) déclare d'abord la guerre ;
## - animation de l'armée le long du trajet parcouru (`walked`) ;
## - cercle de zone de contrôle au survol d'une armée ennemie.
## Remplace les anneaux et l'aperçu sur le graphe des colonies de C5 (le panneau de colonie
## et le bouton « Garnison » restent). Inactif si la simulation n'expose pas ces getters.

signal army_moved(army_id: String, report: Dictionary)

## Vitesse de l'animation (pixels carte par seconde) et durées bornées.
const ANIM_SPEED_PX := 240.0
const ANIM_MIN_SECONDS := 0.35
const ANIM_MAX_SECONDS := 1.8
## Aperçu de chemin recalculé au plus toutes les HOVER_INTERVAL_MS.
const HOVER_INTERVAL_MS := 40
const STOP_MESSAGES := {
	"stationed": "%s : l'armée y stationne.",
	"siege_started": "Siège de %s commencé.",
	"settlement_taken": "%s est prise.",
	"out_of_movement": "Plus de mouvement : la marche reprendra au prochain tour.",
	"enemy_zone_of_control": "Arrêt : zone de contrôle ennemie.",
	"enemy_settlement": "Arrêt devant %s, place ennemie.",
	"blocked": "Chemin bloqué.",
}

var map: Node = null  # CampaignMap
var bubble: ArmyMovementBubble = null
var path_line: ArmyMovementPath = null
var zoc_ring: Decal = null
## Dernière cible survolée : {kind: "ground"|"army"|"settlement", id, point: Vector2}.
var hover_target: Dictionary = {}
## Dernier aperçu de chemin (`find_path_points`), vide si aucun.
var preview: Dictionary = {}
var rules: Dictionary = {}
## Animations en cours : army_id → {points: PackedVector2Array, length, elapsed, duration}.
var _animations: Dictionary = {}
var _selected_is_player := false
var _mouse_dirty := false
var _last_hover_ms := 0
var _last_hover_key := ""
var _last_distance := -1.0
var _zoc_army := ""
## Lot AT1 : confirmation de la déclaration de guerre, attaque en attente de cette réponse
## ({army, target}) et relations du joueur (faction → {status, name}, `get_diplomacy`).
var war_dialog: WarDeclarationDialog = null
var _pending_attack: Dictionary = {}
var _relations: Dictionary = {}


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "ArmyMovementController"
	bubble = ArmyMovementBubble.new()
	map.add_child(bubble)
	bubble.setup(map.map_data)
	path_line = ArmyMovementPath.new()
	map.add_child(path_line)
	path_line.setup(map.map_data)
	zoc_ring = Decal.new()
	zoc_ring.name = "ZoneOfControlRing"
	zoc_ring.texture_albedo = ArmyMarker.ring_texture(false)
	zoc_ring.texture_emission = ArmyMarker.ring_texture(true)
	zoc_ring.modulate = Color(0.85, 0.18, 0.12, 0.9)
	zoc_ring.emission_energy = 0.6
	zoc_ring.upper_fade = 0.2
	zoc_ring.lower_fade = 0.2
	zoc_ring.visible = false
	map.add_child(zoc_ring)
	war_dialog = WarDeclarationDialog.new()
	war_dialog.confirmed.connect(_on_war_confirmed)
	war_dialog.cancelled.connect(func() -> void: _pending_attack = {})
	UiZones.put(UiZones.Zone.MODAL, war_dialog)  # PO1 : fond assombri, entrées bloquées
	if map.picker != null:
		# Prioritaire sur l'intercepteur C5 (colonies), qu'il remplace quand il est actif.
		var previous: Callable = map.picker.right_click_interceptor
		map.picker.right_click_interceptor = func(screen_position: Vector2) -> bool:
			if try_right_click(screen_position):
				return true
			return previous.is_valid() and previous.call(screen_position)


## Vrai si la simulation expose l'API du mouvement libre (vraie `CampaignSim`).
func available() -> bool:
	var sim: Object = map.sim if map != null else null
	return sim != null and sim.has_method("get_reachable_area") and sim.has_method("find_path_points")


## Vrai quand une armée du joueur est sélectionnée et que ce contrôleur gère ses ordres.
func active() -> bool:
	return available() and map.selected_army != "" and _selected_is_player


func _rules() -> Dictionary:
	if rules.is_empty() and available() and map.sim.has_method("get_movement_rules"):
		rules = map.sim.call("get_movement_rules")
	return rules


# --- Sélection ----------------------------------------------------------------------------


## Armée sélectionnée : bulle et trajet en cours. Vrai si le contrôleur a pris la main (la
## carte ne dessine alors ni masque de provinces ni aperçu C5).
func on_army_selected(army_id: String, army: Dictionary, is_player: bool) -> bool:
	_selected_is_player = is_player
	hover_target = {}
	preview = {}
	_last_hover_key = ""
	_relations.clear()
	AttackCursor.show_attack(false)
	if not available():
		return false
	map.terrain.set_reachable(PackedInt32Array(), PackedInt32Array())
	if not is_player:
		bubble.hide_bubble()
		path_line.hide_path()
		return true
	refresh_bubble(army_id)
	show_current_path(army)
	return true


func on_army_deselected() -> void:
	_selected_is_player = false
	hover_target = {}
	preview = {}
	_last_hover_key = ""
	AttackCursor.show_attack(false)
	if bubble != null:
		bubble.hide_bubble()
	if path_line != null:
		path_line.hide_path()


func refresh_bubble(army_id: String) -> void:
	var area: Dictionary = map.sim.call("get_reachable_area", army_id)
	if area.is_empty():
		bubble.hide_bubble()
	else:
		bubble.show_area(area)


## Trajet déjà ordonné (reste d'une marche sur plusieurs tours) : tout en « tours suivants ».
func show_current_path(army: Dictionary) -> void:
	var planned: PackedVector2Array = army.get("planned_path", PackedVector2Array())
	if planned.is_empty():
		path_line.hide_path()
		return
	var points := PackedVector2Array([army.get("position", Vector2.ZERO)])
	points.append_array(planned)
	var destination: Vector2 = army.get("destination_point", Vector2(-1, -1))
	if destination.x >= 0.0 and points[points.size() - 1].distance_to(destination) > 0.01:
		points.append(destination)
	path_line.show_plan(points, 0, PackedInt32Array([points.size() - 1]), map.camera_rig.distance)


# --- Cibles sous le curseur ---------------------------------------------------------------


## Cible sous un point écran : armée (autre que la sélection), colonie, ou sol.
func pick_target(screen_position: Vector2) -> Dictionary:
	# SA (ADR 0160) : même visée que le survol et le clic gauche (`CampaignMap.pick_target`),
	# l'armée sélectionnée exclue : la cible éclairée est celle de l'ordre.
	var target: Dictionary = map.pick_target(screen_position, map.selected_army)
	if str(target.get("kind", "")) == "army":
		var army_id := str(target["id"])
		var army: Dictionary = map.sim.call("get_army", army_id)
		if not army.is_empty():
			return {"kind": "army", "id": army_id, "point": army.get("position", Vector2.ZERO), "faction": str(army.get("faction", ""))}
	var settlement_id := str(target.get("settlement", ""))
	if settlement_id != "":
		var world: Vector3 = map.settlement_layer.world_position_of(settlement_id)
		return {"kind": "settlement", "id": settlement_id, "point": Vector2(world.x, world.z)}
	var hit: Dictionary = map.picker.pick_ray_screen(screen_position)
	if hit.is_empty():
		return {}
	return {"kind": "ground", "id": "", "point": Vector2(hit["x"], hit["z"])}


func is_enemy_faction(faction: String) -> bool:
	if faction == "" or faction == map.player_faction:
		return false
	var summary: Dictionary = map.sim.call("get_faction_summary", map.player_faction)
	return (summary.get("at_war_with", PackedStringArray()) as PackedStringArray).has(faction)


## Faction qui tient la cible : celle de l'armée, ou le contrôleur de la place ("" pour le sol).
func target_faction(target: Dictionary) -> String:
	match str(target.get("kind", "")):
		"army":
			return str(target.get("faction", ""))
		"settlement":
			if map.settlement_data != null:
				return str(map.settlement_data.get_settlement(str(target["id"])).get("controller", ""))
	return ""


## Lot AT1 : relation du joueur avec `faction` pour un ordre d'attaque : "war" (attaque
## directe), "peace" (paix, trêve, vassal ou suzerain : attaque après déclaration de guerre,
## qui rompt le lien vassalique côté simulation), "friend" (soi, allié, faction inconnue :
## pas d'attaque).
func relation_to(faction: String) -> String:
	if faction == "" or faction == map.player_faction:
		return "friend"
	if is_enemy_faction(faction):
		return "war"
	var status := str(_relation_entry(faction).get("status", ""))
	return "peace" if status in ["peace", "truce", "vassal", "suzerain"] else "friend"


func _relation_entry(faction: String) -> Dictionary:
	if _relations.is_empty() and map.sim.has_method("get_diplomacy"):
		for entry in map.sim.call("get_diplomacy", map.player_faction):
			_relations[str(entry.get("id", ""))] = {"status": str(entry.get("status", "")), "name": str(entry.get("name", ""))}
	return _relations.get(faction, {})


## Vrai si un clic droit sur `target` serait une attaque (avec ou sans déclaration de guerre).
func is_attack_target(target: Dictionary) -> bool:
	var kind := str(target.get("kind", ""))
	return (kind == "army" or kind == "settlement") and relation_to(target_faction(target)) in ["war", "peace"]


# --- Ordres ---------------------------------------------------------------------------------


## Intercepteur du clic droit : vrai si l'ordre a été traité (accepté ou refusé).
func try_right_click(screen_position: Vector2) -> bool:
	if not active():
		return false
	var target := pick_target(screen_position)
	if target.is_empty():
		return false
	if is_attack_target(target) and relation_to(target_faction(target)) == "peace":
		ask_war(map.selected_army, target)
		return true
	_execute(map.selected_army, target)
	return true


func _execute(army_id: String, target: Dictionary) -> void:
	var report := order_target(army_id, target)
	UiSounds.play_order_result(report)  # UB1 / U13
	if not report.get("ok", false):
		map.ui.show_toast(str(report.get("error", "Ordre refusé")), true)
	else:
		_ripple_at(target)


## PO5 : onde d'encre au point visé par un ordre accepté (rendu seulement).
func _ripple_at(target: Dictionary) -> void:
	var point: Variant = target.get("point")
	if not point is Vector2 or map.map_data == null:
		return
	var at := point as Vector2
	OrderRipple.spawn(map.ui, map.camera, Vector3(at.x, map.map_data.surface_world_at(at.x, at.y), at.y))


## Lot AT1 : attaque d'une cible en paix : confirmation, puis déclaration de guerre et attaque.
func ask_war(army_id: String, target: Dictionary) -> void:
	var faction := target_faction(target)
	_pending_attack = {"army": army_id, "target": target, "faction": faction}
	var entry := _relation_entry(faction)
	var faction_name := str(entry.get("name", faction))
	war_dialog.ask(map.sim, faction, faction_name, _target_label(target), str(entry.get("status", "")))


func _on_war_confirmed() -> void:
	var pending := _pending_attack
	_pending_attack = {}
	if pending.is_empty():
		return
	var result: Dictionary = map.sim.call("submit_order", {"type": "declare_war", "target": pending["faction"]})
	_relations.clear()
	if not result.get("ok", false):
		map.ui.show_toast(str(result.get("error", "Déclaration de guerre impossible")), true)
		return
	map.refresh_all()
	# ADR 0146 : une guerre aussitôt éteinte (paix imposée) ne doit pas laisser l'armée entrer
	# dans la place comme en temps de paix.
	if not is_enemy_faction(str(pending["faction"])):
		map.ui.show_toast("La guerre n'a pas pu commencer : voir le journal.", true)
		return
	map.ui.show_toast("La guerre est déclarée.")
	_execute(str(pending["army"]), pending["target"])


## Ordre adapté à la cible : attaque d'une armée ennemie, marche vers une colonie ou un point.
func order_target(army_id: String, target: Dictionary) -> Dictionary:
	match str(target.get("kind", "")):
		"army":
			if is_enemy_faction(str(target.get("faction", ""))):
				return order_attack(army_id, str(target["id"]))
			return order_move_point(army_id, target["point"])
		"settlement":
			# SL1 / EM (ADR 0167) : port atteint par la mer depuis le port où se tient l'armée
			# (une ou plusieurs traversées) : on embarque, plutôt que de faire le tour par les
			# terres. Un refus du cœur (saison entamée) est montré tel quel.
			if prefers_sea(army_id, target):
				return order_embark(army_id, str(target["id"]))
			if relation_to(target_faction(target)) == "war":
				return order_attack_settlement(army_id, str(target["id"]))
			return order_move_settlement(army_id, str(target["id"]))
		"ground":
			# EM : clic sur la mer depuis un port : traversée vers le port le plus proche du point.
			var port := sea_port_at(army_id, target["point"])
			if port != "":
				return order_embark(army_id, port)
			if is_water(target["point"]):
				return {"ok": false, "error": _water_refusal(army_id)}
			return order_move_point(army_id, target["point"])
	return {"ok": false, "error": "Aucune cible."}


func order_move_point(army_id: String, point: Vector2) -> Dictionary:
	return _run(army_id, map.sim.call("move_army_to", army_id, point.x, point.y))


func order_move_settlement(army_id: String, settlement_id: String) -> Dictionary:
	return _run(army_id, map.sim.call("move_army_to_settlement", army_id, settlement_id))


func order_attack(army_id: String, target_army: String) -> Dictionary:
	return _run(army_id, map.sim.call("attack_army", army_id, target_army))


## Lot AT1 : attaque d'une place ennemie. L'armée qui l'assiège déjà donne l'assaut ; sinon
## elle marche, et si elle met le siège dès ce tour, l'assaut suit aussitôt (le dialogue
## d'avant-bataille permet encore de « Maintenir le siège »). Une armée ennemie postée dans
## la place et qui arrête la marche aux portes est attaquée.
func order_attack_settlement(army_id: String, settlement_id: String) -> Dictionary:
	var army: Dictionary = map.sim.call("get_army", army_id)
	if str(army.get("settlement", "")) == settlement_id:
		return order_assault(army_id)
	var report := order_move_settlement(army_id, settlement_id)
	if not report.get("ok", false):
		return report
	match str(report.get("stop", "")):
		"siege_started":
			if str(report.get("settlement", "")) == settlement_id:
				var assault := order_assault(army_id)
				return assault if assault.get("ok", false) else report
		"enemy_zone_of_control":
			var defender := str(report.get("stop_army", ""))
			if defender != "" and str(map.sim.call("get_army", defender).get("settlement", "")) == settlement_id:
				var attack := order_attack(army_id, defender)
				return attack if attack.get("ok", false) else report
	return report


## Assaut de la place assiégée par `army_id` (comme « Donner l'assaut », `SiegeController`).
func order_assault(army_id: String) -> Dictionary:
	var result: Dictionary = map.sim.call("submit_order", {"type": "assault", "army": army_id})
	if not result.get("ok", false):
		return result
	map.refresh_all()
	var pending: Array = map.sim.call("get_pending_events")
	map.ui.show_toast(str(pending[-1].get("text_fr", "L'assaut est donné.")) if not pending.is_empty() else "L'assaut est donné.")
	if map.has_method("_offer_pending_battles"):
		map.call("_offer_pending_battles")
	return result


## EM (ADR 0167) : escales de la traversée de `army_id`, depuis le port où elle se tient,
## vers `port` cette saison (destination en dernier) ; vide sans voyage possible.
func sea_voyage(army_id: String, port: String) -> PackedStringArray:
	if army_id == "" or port == "" or not map.sim.has_method("sea_voyage"):
		return PackedStringArray()
	return map.sim.call("sea_voyage", army_id, port)


## EM : la colonie `target` se rejoint par la mer : un voyage l'atteint et la marche n'y
## arrive pas ce tour (un port voisin le long de la côte reste une marche).
func prefers_sea(army_id: String, target: Dictionary) -> bool:
	if sea_voyage(army_id, str(target.get("id", ""))).is_empty():
		return false
	var point: Vector2 = target["point"]
	var walk: Dictionary = map.sim.call("find_path_points", army_id, point.x, point.y)
	return not (walk.get("ok", false) and bool(walk.get("reachable_this_turn", false)))


## EM : port de destination d'un clic sur la mer en `point` ("" si le point est à terre ou
## si l'armée ne se tient pas dans un port d'où une traversée l'atteint).
func sea_port_at(army_id: String, point: Vector2) -> String:
	if not is_water(point) or not map.sim.has_method("sea_port_near"):
		return ""
	return str(map.sim.call("sea_port_near", army_id, point.x, point.y))


## Point sur l'eau (masque terre/mer du rendu : sert à lire le clic, pas à juger l'ordre).
func is_water(point: Vector2) -> bool:
	return map.map_data != null and not map.map_data.is_land_px(int(point.x), int(point.y))


func _water_refusal(army_id: String) -> String:
	var army: Dictionary = map.sim.call("get_army", army_id)
	if str(army.get("settlement", "")) == "":
		return "Pour prendre la mer, l'armée doit d'abord entrer dans un port."
	return "Aucune route maritime depuis ce port."


func order_embark(army_id: String, port: String) -> Dictionary:
	var army: Dictionary = map.sim.call("get_army", army_id)
	var from := str(army.get("settlement", ""))
	var stops := sea_voyage(army_id, port)
	var report := _run(army_id, map.sim.call("embark_army", army_id, port))
	if report.get("ok", false):
		var route := sea_route_points(from, stops)
		if route.size() >= 2:
			animate(army_id, route)
	return report


## EM : tracé d'un voyage (routes maritimes de `SeaLaneLayer`, trait droit pour les courts
## passages), du port `from` jusqu'à la dernière escale de `stops`.
func sea_route_points(from: String, stops: PackedStringArray) -> PackedVector2Array:
	var points := PackedVector2Array()
	if map.map_data == null:
		return points
	var lanes: SeaLaneLayer = map.get("sea_lanes_layer")
	var leg_from := from
	for leg_to in stops:
		var leg := lanes.lane_points(leg_from, leg_to) if lanes != null else PackedVector2Array()
		if leg.size() < 2:
			leg = PackedVector2Array([map.map_data.settlement_px(leg_from), map.map_data.settlement_px(leg_to)])
		var start := 1 if not points.is_empty() else 0
		for i in range(start, leg.size()):
			points.append(leg[i])
		leg_from = leg_to
	return points


func _run(army_id: String, report: Dictionary) -> Dictionary:
	if not report.get("ok", false):
		return report
	hover_target = {}
	preview = {}
	_last_hover_key = ""
	map.refresh_all()
	var walked: PackedVector2Array = report.get("walked", PackedVector2Array())
	if bool(report.get("alive", true)) and walked.size() >= 2:
		animate(army_id, walked)
	_announce(report)
	army_moved.emit(army_id, report)
	if map.has_method("_offer_pending_battles"):
		map.call("_offer_pending_battles")
	return report


func _announce(report: Dictionary) -> void:
	var events: Array = report.get("events", [])
	for event in events:
		if str(event.get("kind", "")) == "battle":
			map.ui.show_toast(str(event.get("text_fr", "Bataille")))
			return
	var stop := str(report.get("stop", ""))
	if not STOP_MESSAGES.has(stop):
		return
	var message: String = STOP_MESSAGES[stop]
	if message.contains("%s"):
		message = message % _settlement_name(str(report.get("stop_settlement", "")))
	map.ui.show_toast(message)


func _settlement_name(id: String) -> String:
	if map.settlement_data == null or id == "":
		return id
	return str(map.settlement_data.get_settlement(id).get("name", id))


# --- Animation ----------------------------------------------------------------------------


## Anime le marqueur de `army_id` le long de `points` (pixels carte), arrivée comprise.
func animate(army_id: String, points: PackedVector2Array) -> void:
	var length := 0.0
	for i in range(1, points.size()):
		length += points[i - 1].distance_to(points[i])
	if length < 0.5:
		return
	_animations[army_id] = {
		"points": points,
		"length": length,
		"elapsed": 0.0,
		"duration": clampf(length / ANIM_SPEED_PX, ANIM_MIN_SECONDS, ANIM_MAX_SECONDS),
	}
	_step_animation(army_id, 0.0)


func is_animating(army_id: String = "") -> bool:
	return _animations.has(army_id) if army_id != "" else not _animations.is_empty()


## Termine toutes les animations (tests, captures).
func finish_animations() -> void:
	for army_id in _animations.keys():
		_step_animation(army_id, INF)


func _step_animation(army_id: String, delta: float) -> void:
	var anim: Dictionary = _animations[army_id]
	anim["elapsed"] = minf(float(anim["elapsed"]) + delta, float(anim["duration"]))
	var t := float(anim["elapsed"]) / maxf(float(anim["duration"]), 0.001)
	t = t * t * (3.0 - 2.0 * t)
	var points: PackedVector2Array = anim["points"]
	var target := t * float(anim["length"])
	var point := points[points.size() - 1]
	var heading := Vector2.ZERO
	for i in range(1, points.size()):
		var segment := points[i - 1].distance_to(points[i])
		if target <= segment or i == points.size() - 1:
			var local := clampf(target / maxf(segment, 0.001), 0.0, 1.0)
			point = points[i - 1].lerp(points[i], local)
			heading = points[i] - points[i - 1]
			break
		target -= segment
	map.armies.place_marker(army_id, point, heading)
	if float(anim["elapsed"]) >= float(anim["duration"]):
		_animations.erase(army_id)
		map.armies.place_marker(army_id, Vector2(-1, -1), heading)


# --- Survol -------------------------------------------------------------------------------


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_mouse_dirty = true


func _process(delta: float) -> void:
	if map == null:
		return
	for army_id in _animations.keys():
		_step_animation(army_id, delta)
	# Lot CV3-5 : pas de zone atteignable pendant la fin de tour ni le rejeu des marches IA.
	if bubble != null:
		var replay: Variant = map.get("ai_replay")
		bubble.set_suspended(map.get("end_turn_running") == true or (replay != null and replay.get("playing") == true))
	# Lot AT1 : pas d'épées sur l'interface, ni hors de la carte (bataille, menus).
	if AttackCursor.is_shown() and (not map.visible or not active() or get_viewport().gui_get_hovered_control() != null):
		AttackCursor.show_attack(false)
	var distance: float = map.camera_rig.distance
	if not is_equal_approx(distance, _last_distance):
		_last_distance = distance
		path_line.update_scale(distance)
		_update_zoc_ring()
	if not _mouse_dirty or not available():
		return
	var now := Time.get_ticks_msec()
	if now - _last_hover_ms < HOVER_INTERVAL_MS:
		return
	_mouse_dirty = false
	_last_hover_ms = now
	hover(get_viewport().get_mouse_position())


## Survol d'un point écran : cercle de zone de contrôle sur une armée ennemie, aperçu du
## chemin de l'armée sélectionnée vers la cible.
func hover(screen_position: Vector2) -> void:
	var target := pick_target(screen_position)
	var zoc_army := ""
	if str(target.get("kind", "")) == "army" and is_enemy_faction(str(target.get("faction", ""))):
		zoc_army = str(target["id"])
	show_zoc(zoc_army)
	if not active():
		AttackCursor.show_attack(false)
		return
	AttackCursor.show_attack(is_attack_target(target) and get_viewport().gui_get_hovered_control() == null)
	preview_target(target)


## Aperçu du chemin de l'armée sélectionnée vers `target` (voir `pick_target`).
func preview_target(target: Dictionary) -> void:
	hover_target = target
	if target.is_empty():
		_clear_preview()
		return
	var point: Vector2 = target["point"]
	var key := "%s:%s:%d:%d" % [target["kind"], target["id"], int(point.x / 2.0), int(point.y / 2.0)]
	if key == _last_hover_key:
		return
	_last_hover_key = key
	if _preview_voyage(target):
		return
	preview = map.sim.call("find_path_points", map.selected_army, point.x, point.y)
	if not preview.get("ok", false):
		path_line.hide_path()
		_set_hover_text("%s — aucun chemin" % _target_label(target))
		return
	# Lot DP2 : avertissement avant l'ordre quand la marche entre sans droit de passage.
	var trespass: Dictionary = {}
	if map.sim.has_method("find_path_trespass"):
		trespass = map.sim.call("find_path_trespass", map.selected_army, point.x, point.y)
	var warning := str(trespass.get("warning", ""))
	path_line.show_plan(preview["points"], int(preview["stop_index"]), preview["turn_ends"], map.camera_rig.distance, warning != "")
	var turns := int(preview.get("turns", 1))
	var when := "ce tour" if bool(preview.get("reachable_this_turn", false)) else "%d tours" % turns
	var action := attack_hint(target)
	var text := "→ %s : %s, coût %d — %s" % [_target_label(target), when, int(preview.get("cost", 0)), action]
	if warning != "":
		text += "\n⚠ %s." % warning
	_set_hover_text(text)


## EM (ADR 0167) : aperçu d'une traversée (port atteint par la mer, ou mer survolée depuis un
## port) ; faux si la cible ne relève pas de la mer.
func _preview_voyage(target: Dictionary) -> bool:
	var army_id: String = map.selected_army
	var port := ""
	match str(target.get("kind", "")):
		"settlement":
			if not prefers_sea(army_id, target):
				return false
			port = str(target["id"])
		"ground":
			port = sea_port_at(army_id, target["point"])
			if port == "" and is_water(target["point"]):
				preview = {}
				path_line.hide_path()
				_set_hover_text(_water_refusal(army_id))
				return true
	var stops := sea_voyage(army_id, port)
	if stops.is_empty():
		return false
	preview = {}
	var army: Dictionary = map.sim.call("get_army", army_id)
	var route := sea_route_points(str(army.get("settlement", "")), stops)
	path_line.show_plan(route, route.size() - 1, PackedInt32Array(), map.camera_rig.distance, false)
	var calls := PackedStringArray()
	for i in range(stops.size() - 1):
		calls.append(_settlement_name(stops[i]))
	var text := "⚓ → %s : traversée, toute la saison" % _settlement_name(port)
	if not calls.is_empty():
		text += " (escales : %s)" % ", ".join(calls)
	_set_hover_text(text + " — clic droit pour embarquer")
	return true


## Consigne du survol : attaque, assaut (avec déclaration de guerre s'il le faut) ou marche.
func attack_hint(target: Dictionary) -> String:
	if not is_attack_target(target):
		return "clic droit pour partir"
	var hint := "clic droit pour donner l'assaut" if str(target["kind"]) == "settlement" else "clic droit pour attaquer"
	if relation_to(target_faction(target)) == "peace":
		hint += " (déclare la guerre)"
	return hint


func _clear_preview() -> void:
	preview = {}
	_last_hover_key = ""
	var army: Dictionary = map.sim.call("get_army", map.selected_army) if map.selected_army != "" else {}
	show_current_path(army)


func _target_label(target: Dictionary) -> String:
	match str(target.get("kind", "")):
		"settlement":
			return _settlement_name(str(target["id"]))
		"army":
			var army: Dictionary = map.sim.call("get_army", str(target["id"]))
			var general := str(army.get("general_name", ""))
			return "armée de %s" % general if general != "" else "armée"
	return "ce point"


func _set_hover_text(text: String) -> void:
	var label: Label = map.ui.get("hover_label")
	if label == null:
		return
	label.text = text
	label.visible = true
	if map.ui.has_method("_fit_hover_label"):
		map.ui.call("_fit_hover_label")


## Cercle de zone de contrôle autour de `army_id` ("" = masqué).
func show_zoc(army_id: String) -> void:
	_zoc_army = army_id
	_update_zoc_ring()


func zoc_visible() -> bool:
	return zoc_ring != null and zoc_ring.visible


func _update_zoc_ring() -> void:
	if zoc_ring == null:
		return
	if _zoc_army == "" or not available():
		zoc_ring.visible = false
		return
	var army: Dictionary = map.sim.call("get_army", _zoc_army)
	var radius := float(_rules().get("zoc_radius_px", 0.0))
	if army.is_empty() or radius <= 0.0:
		zoc_ring.visible = false
		return
	var point: Vector2 = army.get("position", Vector2.ZERO)
	var y: float = map.map_data.surface_world_at(point.x, point.y)
	zoc_ring.size = Vector3(radius * 2.0, 80.0, radius * 2.0)
	zoc_ring.position = Vector3(point.x, y, point.y)
	zoc_ring.visible = true


func _exit_tree() -> void:
	AttackCursor.show_attack(false)


# --- Capture (`--stage=movement`) -----------------------------------------------------------


## Première armée du joueur sélectionnée, bulle, chemin sur deux tours vers un point au-delà
## de la bulle, caméra cadrée sur l'ensemble.
## Lot DP2 : capture de l'avertissement d'intrusion. Choisit l'armée du joueur et la province
## en paix la plus proche dont la marche déclenche l'avertissement (chemin rouge, infobulle).
func stage_trespass_screenshot() -> bool:
	if not available() or not map.sim.has_method("find_path_trespass"):
		return false
	var best := {}
	var best_distance := INF
	for army_id in map.player_army_ids():
		var army: Dictionary = map.sim.call("get_army", army_id)
		var start: Vector2 = army.get("position", Vector2.ZERO)
		for index in range(1, map.map_data.province_count + 1):
			var id := str(map.map_data.get_province(index).get("id", ""))
			var centroid: Vector2 = map.map_data.centroid_of_id(id)
			var distance := start.distance_to(centroid)
			if centroid.x < 0.0 or distance >= best_distance or distance > 900.0:
				continue
			var stance: PackedStringArray = map.sim.call("get_province_stances", PackedStringArray([id]))
			if stance.is_empty() or not (stance[0] in ["neutral", "agreement", "tension"]):
				continue
			var passage: Dictionary = map.sim.call("find_path_trespass", army_id, centroid.x, centroid.y)
			if str(passage.get("warning", "")) == "":
				continue
			best = {"army": army_id, "point": centroid, "start": start}
			best_distance = distance
	if best.is_empty():
		return false
	map.select_army(best["army"])
	preview_target({"kind": "ground", "id": "", "point": best["point"]})
	var focus: Vector2 = (best["start"] as Vector2).lerp(best["point"], 0.5)
	map.camera_rig.look_at_point(Vector3(focus.x, map.map_data.surface_world_at(focus.x, focus.y), focus.y), maxf(best_distance * 1.6, 140.0))
	map.camera_rig.snap()
	return true


func stage_screenshot(close_up: bool = false) -> void:
	if not available():
		return
	var ids: PackedStringArray = map.player_army_ids()
	if ids.is_empty():
		return
	map.select_army(ids[0])
	var army: Dictionary = map.sim.call("get_army", ids[0])
	var start: Vector2 = army.get("position", Vector2.ZERO)
	var rect := bubble.covered_rect()
	var radius := maxf(rect.size.x, rect.size.y) * 0.5
	var chosen := Vector2(-1, -1)
	for i in 16:
		var direction := Vector2.RIGHT.rotated(TAU * float(i) / 16.0 + 0.4)
		var point := start + direction * radius * 1.45
		var plan: Dictionary = map.sim.call("find_path_points", ids[0], point.x, point.y)
		if plan.get("ok", false) and int(plan.get("turns", 0)) >= 2:
			chosen = point
			break
	var focus := start if chosen.x < 0.0 else start.lerp(chosen, 0.4)
	var distance := maxf(radius * 2.3, 120.0)
	if chosen.x >= 0.0:
		preview_target({"kind": "ground", "id": "", "point": chosen})
	if close_up and not preview.is_empty():
		# Gros plan sur la fin de l'étape de ce tour, au bord de la bulle.
		var points: PackedVector2Array = preview["points"]
		focus = points[int(preview["stop_index"])]
		distance = maxf(radius * 0.7, 80.0)
	map.camera_rig.look_at_point(Vector3(focus.x, map.map_data.surface_world_at(focus.x, focus.y), focus.y), distance)
	map.camera_rig.snap()
