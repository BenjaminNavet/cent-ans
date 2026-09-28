class_name BattleFormationPicker
extends PanelContainer

## CB6 : formations de groupe (plan `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`,
## section CB6). Sélecteur de préréglage (Attaque / Défense / Marche, infobulle `description_fr`,
## Alt+Maj+1…6) posé en bas à droite, au-dessus du bandeau des cartes et sous le journal.
## Aucune règle ici : les places viennent du cœur (`BattleSim.formation_slots`, données
## `data/rules/group_formations.json`) ; ce script les montre en fantômes puis les traduit en
## ordres ordinaires :
## - en bataille, préréglage actif + clic droit (ou glisser-droit pour l'orientation) : un
##   `move` par régiment (largeur CB1, `match_speed`, `group_tag`), et le groupe est verrouillé
##   dans cette forme (CB1) ;
## - en déploiement, « Placer en formation » propose les places (fantômes) de la sélection, ou
##   de toute l'armée sans sélection ; un clic droit déplace la proposition, un second appui
##   sur le bouton la valide (`deploy_unit` avec largeur).

const STANCES := ["attack", "defense", "march"]
const STANCE_LABELS := {"attack": "Attaque", "defense": "Défense", "march": "Marche"}
const INK := Color(0.22, 0.14, 0.07)
## Recul (m) de la ligne de front proposée par rapport au bord avant de la zone de déploiement.
const ZONE_SETBACK := 30.0

## Préréglages du cœur : [{id, name_fr, description_fr, stance}] dans l'ordre des données.
var presets: Array = []
## Préréglage actif ("" : aucun ; les clics droits gardent leur sens ordinaire).
var active_id := ""
## Déploiement : proposition en attente [{id, x, z, facing, order_width, width, depth}].
var pending: Array = []
var place_button: Button
var cancel_button: Button

var scene: Node = null  # BattleScene
var _buttons: Dictionary = {}  # id -> Button
var _anchor: Dictionary = {}  # dernière proposition : {ids, point, facing}


## Construit le sélecteur ; invisible sans préréglages (pont ancien, bataille sans armée).
func setup(p_scene: Node) -> void:
	scene = p_scene
	name = "FormationPicker"
	if scene.battle != null and scene.battle.has_method("formation_presets"):
		presets = scene.battle.call("formation_presets")
	_build()
	visible = not presets.is_empty()


func _build() -> void:
	anchor_left = 1.0
	anchor_right = 1.0
	anchor_top = 1.0
	anchor_bottom = 1.0
	grow_horizontal = Control.GROW_DIRECTION_BEGIN
	grow_vertical = Control.GROW_DIRECTION_BEGIN
	offset_right = -12
	offset_bottom = -BattleHud.BAND_HEIGHT - 16
	mouse_filter = Control.MOUSE_FILTER_STOP
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	add_child(box)
	var title := Label.new()
	title.text = "Formations de groupe"
	title.add_theme_font_size_override("font_size", 14)
	title.add_theme_color_override("font_color", INK)
	box.add_child(title)
	for stance in STANCES:
		var row := HBoxContainer.new()
		row.name = "Row_%s" % stance
		row.add_theme_constant_override("separation", 4)
		var label := Label.new()
		label.text = str(STANCE_LABELS[stance])
		label.custom_minimum_size = Vector2(62, 0)
		label.add_theme_font_size_override("font_size", 12)
		label.add_theme_color_override("font_color", INK)
		row.add_child(label)
		for k in presets.size():
			var preset: Dictionary = presets[k]
			if str(preset.get("stance", "")) != stance:
				continue
			var button := Button.new()
			var id := str(preset["id"])
			button.name = "Preset_%s" % id
			button.text = str(preset["name_fr"])
			button.toggle_mode = true
			button.focus_mode = Control.FOCUS_NONE
			button.add_theme_font_size_override("font_size", 12)
			button.tooltip_text = tooltip_for(preset, k + 1)
			button.pressed.connect(toggle.bind(id))
			row.add_child(button)
			_buttons[id] = button
		if row.get_child_count() > 1:
			box.add_child(row)
		else:
			row.free()
	var actions := HBoxContainer.new()
	actions.add_theme_constant_override("separation", 4)
	box.add_child(actions)
	place_button = Button.new()
	place_button.name = "PlaceButton"
	place_button.text = "Placer en formation"
	place_button.focus_mode = Control.FOCUS_NONE
	place_button.tooltip_text = "Propose une place à chaque régiment de la sélection (sans sélection : toute l'armée) selon la formation choisie ; clic droit : déplacer la proposition ; second appui : valider."
	place_button.pressed.connect(on_place_pressed)
	actions.add_child(place_button)
	cancel_button = Button.new()
	cancel_button.name = "CancelButton"
	cancel_button.text = "Annuler"
	cancel_button.focus_mode = Control.FOCUS_NONE
	cancel_button.pressed.connect(cancel)
	actions.add_child(cancel_button)
	_update_buttons()


## Infobulle d'un préréglage : nom, description historique, raccourci.
static func tooltip_for(preset: Dictionary, index: int) -> String:
	return "%s\n%s\nRaccourci : Alt+Maj+%d" % [str(preset.get("name_fr", "")), str(preset.get("description_fr", "")), index]


## Numéro (1…9) de préréglage d'un appui Alt+Maj+chiffre (touche physique : AZERTY compris) ;
## 0 pour toute autre touche.
static func shortcut_index(event: InputEventKey) -> int:
	if not event.pressed or event.echo or not event.alt_pressed or not event.shift_pressed:
		return 0
	if event.physical_keycode >= KEY_1 and event.physical_keycode <= KEY_9:
		return int(event.physical_keycode - KEY_0)
	return 0


func is_active() -> bool:
	return active_id != ""


## Nom français du préréglage `id` ("" s'il est inconnu).
func name_of(id: String) -> String:
	for preset in presets:
		if str(preset["id"]) == id:
			return str(preset["name_fr"])
	return ""


## Active le préréglage `id`, ou le désactive s'il l'était déjà.
func toggle(id: String) -> void:
	active_id = "" if active_id == id else id
	if active_id == "":
		cancel()
	elif not pending.is_empty():
		_repropose()
	_update_buttons()
	if scene != null and scene.hud != null:
		if active_id == "":
			scene.hud.show_toast("Formation de groupe désactivée.", false)
		elif _deploying():
			scene.hud.show_toast("« %s » : « Placer en formation » ou clic droit pour proposer les places." % name_of(active_id), false)
		else:
			scene.hud.show_toast("« %s » : clic droit pour placer la sélection (glisser : orientation)." % name_of(active_id), false)


## Raccourci Alt+Maj+`index` (1 = premier préréglage des données).
func select_index(index: int) -> bool:
	if index < 1 or index > presets.size():
		return false
	toggle(str(presets[index - 1]["id"]))
	return true


func _update_buttons() -> void:
	for id in _buttons:
		(_buttons[id] as Button).set_pressed_no_signal(id == active_id)
	if place_button != null:
		var deploying := _deploying()
		place_button.visible = deploying
		place_button.disabled = active_id == ""
		place_button.text = "Valider la formation" if not pending.is_empty() else "Placer en formation"
		cancel_button.visible = deploying and not pending.is_empty()


func _deploying() -> bool:
	return scene != null and scene.deployment != null and scene.deployment.active


func _process(_delta: float) -> void:
	if scene == null:
		return
	if place_button.visible != _deploying():
		if not _deploying():
			pending.clear()
		_update_buttons()
	# Le relâché du clic droit efface l'aperçu en direct : la proposition reste affichée.
	if not pending.is_empty() and scene.path_preview != null and not scene.path_preview.live_active:
		scene.path_preview.show_ghosts(pending, scene.units)


# ----- places --------------------------------------------------------------------------------

## Places du préréglage actif pour `ids`, front centré en `point` (x, z) tourné vers `facing` :
## [{id, x, z, facing, order_width, width, depth}] (`order_width` : largeur à ordonner, 0 : la
## formation reste ; `width`/`depth` : taille d'arrivée, pour les fantômes).
func slots_for(ids: Array, point: Vector2, facing: float) -> Array:
	var out: Array = []
	if active_id == "" or ids.is_empty() or scene.battle == null or not scene.battle.has_method("formation_slots"):
		return out
	var raw: Array = scene.battle.call("formation_slots", active_id, PackedInt32Array(ids), point.x, point.y, facing)
	for slot in raw:
		var id := int(slot["id"])
		var order_width := float(slot["width"])
		var size := Vector2.ZERO
		if order_width > 0.0 and scene.battle.has_method("formation_extent"):
			size = scene.battle.call("formation_extent", id, order_width)
		else:
			var unit := _unit(id)
			size = Vector2(float(unit.get("width", 0.0)), float(unit.get("depth", 0.0)))
		out.append({"id": id, "x": float(slot["x"]), "z": float(slot["z"]), "facing": float(slot["facing"]), "order_width": order_width, "width": size.x, "depth": size.y})
	return out


## Orientation d'un clic simple : du centre des régiments `ids` vers `point` ; trop près,
## leur orientation moyenne.
func facing_for(ids: Array, point: Vector2) -> float:
	var center := Vector2.ZERO
	var facings: Array = []
	var n := 0
	for unit in scene.units:
		if ids.has(int(unit["id"])):
			center += Vector2(float(unit["x"]), float(unit["z"]))
			facings.append(float(unit["facing"]))
			n += 1
	if n == 0:
		return 0.0
	center /= n
	var dir := point - center
	if dir.length() < 5.0:
		return FormationDrag.mean_facing(facings)
	return atan2(dir.x, dir.y)


## Point et orientation d'un clic droit (`p0 == p1`) ou d'un glisser-droit (ligne p0 → p1, front
## à l'opposé de la caméra `cam`) pour `ids` : {point: Vector2, facing}.
func aim(ids: Array, p0: Vector3, p1: Vector3, cam: Vector3, dragged: bool) -> Dictionary:
	if dragged:
		var line := FormationDrag.drag_line(p0, p1, cam)
		if bool(line["ok"]):
			var mid: Vector3 = line["mid"]
			return {"point": Vector2(mid.x, mid.z), "facing": float(line["facing"])}
	var point := Vector2(p1.x, p1.z)
	return {"point": point, "facing": facing_for(ids, point)}


## Ordres d'une formation de groupe : un `move` par place, largeur à ordonner, allure du plus
## lent et étiquette de groupe communes (`tag` 0 : sans étiquette) ; fonction pure, testée.
static func slot_orders(places: Array, tag: int, run: bool, queued: bool) -> Array:
	var out: Array = []
	for place in places:
		var command := {"type": "move", "units": [int(place["id"])], "x": float(place["x"]), "z": float(place["z"]), "run": run, "facing": float(place["facing"]), "match_speed": true}
		if float(place.get("order_width", 0.0)) > 0.0:
			command["width"] = float(place["order_width"])
		if tag != 0:
			command["group_tag"] = tag
		if queued:
			command["queue"] = true
		out.append(command)
	return out


# ----- bataille ------------------------------------------------------------------------------

## Aperçu (fantômes et trajets) des places pendant le clic droit maintenu.
func preview(ids: Array, p0: Vector3, p1: Vector3, cam: Vector3, dragged: bool, queued: bool, final: bool) -> void:
	var preview_node: BattlePathPreview = scene.path_preview
	if preview_node == null:
		return
	var target := aim(ids, p0, p1, cam, dragged)
	var point: Vector2 = target["point"]
	var at := Vector3(point.x, 0.0, point.y)
	var facing := float(target["facing"])
	var now := Time.get_ticks_msec() / 1000.0
	if final or preview_node.should_recompute(at, facing, now, queued):
		preview_node.compute_places(slots_for(ids, point, facing), scene.units, now, queued, at, facing)


## Ordres du clic droit (ou glisser-droit) avec le préréglage actif : places, contrôle des
## chemins (rien d'envoyé si aucun régiment n'en a), puis verrouillage du groupe dans cette
## forme. Renvoie les ordres à envoyer ([] : refus, message déjà affiché).
func battle_orders(ids: Array, p0: Vector3, p1: Vector3, cam: Vector3, dragged: bool, run: bool, queued: bool) -> Array:
	var target := aim(ids, p0, p1, cam, dragged)
	var places := slots_for(ids, target["point"], float(target["facing"]))
	if places.is_empty():
		return []
	var preview_node: BattlePathPreview = scene.path_preview
	if preview_node != null:
		var point: Vector2 = target["point"]
		preview_node.compute_places(places, scene.units, Time.get_ticks_msec() / 1000.0, queued, Vector3(point.x, 0.0, point.y), float(target["facing"]))
		if not preview_node.reachable():
			scene.hud.show_toast("Ordre impossible : %s." % preview_node.refusal().to_lower())
			return []
	var tag := 0
	if scene.hud != null and scene.hud.groups != null:
		tag = scene.hud.groups.lock_as(places)
		scene.hud.update_cards(scene.units, scene.player_side, scene.selected)
	return slot_orders(places, tag, run, queued)


# ----- déploiement ---------------------------------------------------------------------------

## Régiments visés en déploiement : la sélection, sinon toute l'armée présente du joueur.
func deploy_ids() -> Array:
	var ids: Array = []
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]) and (scene.selected.is_empty() or scene.selected.has(int(unit["id"]))):
			ids.append(int(unit["id"]))
	return ids


## Front proposé par défaut : au milieu de la zone, un peu en retrait de son bord avant, face à
## l'ennemi : {point: Vector2, facing}.
func default_anchor() -> Dictionary:
	var zone: Dictionary = scene.deployment.zone
	var cx := (float(zone["x0"]) + float(zone["x1"])) * 0.5
	if scene.player_side == "attacker":
		return {"point": Vector2(cx, float(zone["z1"]) - ZONE_SETBACK), "facing": 0.0}
	return {"point": Vector2(cx, float(zone["z0"]) + ZONE_SETBACK), "facing": PI}


## Propose (fantômes) les places des régiments `ids` au front `point`, tourné vers `facing`.
func propose(ids: Array, point: Vector2, facing: float) -> int:
	_anchor = {"ids": ids.duplicate(), "point": point, "facing": facing}
	pending = slots_for(ids, point, facing)
	if scene.path_preview != null:
		if pending.is_empty():
			scene.path_preview.clear_live()
		else:
			scene.path_preview.show_ghosts(pending, scene.units)
	_update_buttons()
	return pending.size()


## Clic droit (ou glisser-droit) en déploiement avec le préréglage actif : la proposition se
## déplace (la sélection, sinon toute l'armée).
func deploy_click(p0: Vector3, p1: Vector3, cam: Vector3, dragged: bool) -> void:
	var ids := deploy_ids()
	var target := aim(ids, p0, p1, cam, dragged)
	var facing := float(target["facing"]) if dragged else float(default_anchor()["facing"])
	propose(ids, target["point"], facing)


## Fantômes en direct pendant le clic droit maintenu en déploiement.
func deploy_preview(p0: Vector3, p1: Vector3, cam: Vector3, dragged: bool) -> void:
	deploy_click(p0, p1, cam, dragged)


## Bouton : sans proposition, propose au front par défaut ; sinon valide.
func on_place_pressed() -> void:
	if active_id == "" or not _deploying():
		return
	if pending.is_empty():
		var anchor := default_anchor()
		propose(deploy_ids(), anchor["point"], float(anchor["facing"]))
	else:
		apply()


## Valide la proposition : `deploy_unit` avec largeur pour chaque place. Renvoie le nombre de
## régiments posés (les refus s'affichent).
func apply() -> int:
	var placed := 0
	for slot in pending:
		var result: Dictionary = scene.battle.call("deploy_unit", int(slot["id"]), float(slot["x"]), float(slot["z"]), float(slot["facing"]), float(slot["order_width"]))
		if bool(result.get("ok", false)):
			placed += 1
		else:
			scene.hud.show_toast(str(result.get("error", "?")))
	pending.clear()
	if scene.path_preview != null:
		scene.path_preview.clear_live()
	_update_buttons()
	if scene.has_method("_refresh_view"):
		scene._refresh_view(true)
	return placed


## Abandonne la proposition en attente.
func cancel() -> void:
	if pending.is_empty():
		_update_buttons()
		return
	pending.clear()
	if scene != null and scene.path_preview != null:
		scene.path_preview.clear_live()
	_update_buttons()


## Recalcule la proposition en attente avec le nouveau préréglage (même front, mêmes régiments).
func _repropose() -> void:
	propose(_anchor["ids"], _anchor["point"], float(_anchor["facing"]))


func _unit(id: int) -> Dictionary:
	for unit in scene.units:
		if int(unit["id"]) == id:
			return unit
	return {}
