extends Node

## Chantier PO (ADR 0097, bible DA § 12.1) : autoload `UiLayout`, zones d'écran fixes. Chaque
## panneau réclame une zone au lieu de se placer en coordonnées absolues ; les rectangles sont
## fixés par des ancres en proportion de l'écran et ne dépendent jamais de la taille minimale des
## enfants (boucles de mise en page vues en UI1).
## - `SIDE_PANEL` n'a qu'un occupant : en réclamer un ferme le précédent (`side_panel_changed`).
## - `MODAL` assombrit le fond et bloque les entrées derrière.
## - `TOASTS` empile 3 avis au plus (conseiller, annonces, avis de résultat), effacés après `seconds`.
##
## Lot PO1 — fonctionnement :
## - Un « hôte » porte les zones : chaque zone est un `Control` nu (taille minimale nulle), ancré
##   en proportion de l'écran et qui coupe ce qui dépasse (`clip_contents`) ; un occupant plus grand
##   que sa zone est coupé ou défile (à lui de porter un `ScrollContainer`), la zone ne grandit
##   jamais. Hôte par défaut : un `CanvasLayer` propre à l'autoload. Un écran (carte de campagne,
##   bataille) appelle `attach_host(sa_couche)` : ses zones vivent alors dans sa couche (thème,
##   ordre d'affichage, durée de vie de la scène) et disparaissent avec elle.
## - `claim` reparente l'occupant sous le `Control` de la zone. `SIDE_PANEL` et `MODAL` : l'occupant
##   remplit la zone. `TOASTS` : l'occupant rejoint la pile verticale (sous les avis éphémères, ou
##   en bas de la zone avec `at_end`). Autres zones : l'occupant garde sa disposition, en
##   coordonnées de la zone.
## - `SIDE_PANEL` : dès qu'un occupant devient visible, les autres se ferment (`UiMotion.fade_out`)
##   et `side_panel_changed` est émis — l'exclusivité tient même quand un panneau s'ouvre par
##   `show()` sans repasser par `claim`.
## - `MODAL` : un voile noir à 45 % couvre l'écran et arrête la souris tant qu'un occupant est
##   visible.
## - Un contrôle qui vit ailleurs (couche d'un autoload, comme le conseiller) peut suivre une zone
##   sans être reparenté : `anchor_to(control, zone)`.

signal side_panel_changed(control: Control)

enum Zone { TOP_BAR, BOTTOM_SELECTION, MINIMAP, SIDE_PANEL, TOASTS, MODAL }

## Rectangles des zones en part de l'écran (x, y, largeur, hauteur), bible DA § 12.1.
const ZONE_RECTS := {
	Zone.TOP_BAR: Rect2(0.0, 0.0, 1.0, 0.05),
	Zone.BOTTOM_SELECTION: Rect2(0.01, 0.80, 0.80, 0.20),
	Zone.MINIMAP: Rect2(0.82, 0.72, 0.18, 0.28),
	Zone.SIDE_PANEL: Rect2(0.70, 0.06, 0.30, 0.64),
	Zone.TOASTS: Rect2(0.01, 0.07, 0.26, 0.50),
	Zone.MODAL: Rect2(0.2, 0.12, 0.6, 0.76),
}
const TOAST_SECONDS := 6.0
const MAX_TOASTS := 3
## Voile des fenêtres modales (bible § 12.1 : noir 45 %).
const MODAL_DIM := Color(0.0, 0.0, 0.0, 0.45)
## Étage de la couche par défaut (au-dessus des HUD de scène, sous les écrans de chargement).
const DEFAULT_LAYER := 60
const ZONE_META := &"ui_layout_zone"
const _TOAST_META := &"ui_layout_toast"

## Hôte courant : `{node, zones: {Zone: Control}, dim: ColorRect, stack: VBoxContainer,
## toasts: VBoxContainer, spacer: Control, more: Label}`.
var _host: Dictionary = {}
var _default_host: Dictionary = {}
var _default_layer: CanvasLayer = null
## Occupants par zone (`Zone` → `Array[Control]`), hôte courant.
var _occupants: Dictionary = {}
## Contrôles suivant une zone sans reparentage (`anchor_to`).
var _anchored: Dictionary = {}


## Donne à `host` (un `CanvasLayer` ou un `Control` plein écran) les six zones ; il devient l'hôte
## courant jusqu'à sa sortie de l'arbre ou au prochain `attach_host`. Rappeler avec le même hôte
## ne fait rien.
func attach_host(host: Node) -> void:
	if host == null:
		return
	if not _host.is_empty() and _host.get("node") == host:
		return
	_host = _build_host(host)
	_occupants = {}
	if not host.tree_exiting.is_connected(_on_host_exiting.bind(host)):
		host.tree_exiting.connect(_on_host_exiting.bind(host), CONNECT_ONE_SHOT)


## Hôte courant (l'hôte par défaut si aucun écran n'en a posé).
func host() -> Node:
	_ensure_host()
	return _host.get("node")


## `Control` de `zone` dans l'hôte courant (parent des occupants).
func zone_node(zone: Zone) -> Control:
	_ensure_host()
	return (_host["zones"] as Dictionary)[zone]


## Place `control` dans `zone` (reparenté sous le conteneur de la zone). `at_end` (zone `TOASTS`
## seulement) : en bas de la zone plutôt que sous les avis (journal).
func claim(zone: Zone, control: Control, at_end: bool = false) -> void:
	if control == null:
		return
	_ensure_host()
	var parent := _parent_for(zone, at_end)
	if control.get_parent() != parent:
		if control.get_parent() == null:
			parent.add_child(control)
		else:
			control.reparent(parent, false)
	if zone == Zone.TOASTS:
		if at_end:
			parent.move_child(control, -1)
		else:
			var spacer: Control = _host["spacer"]
			var before := 1 if control.get_index() < spacer.get_index() else 0
			parent.move_child(control, spacer.get_index() - before)
	control.set_meta(ZONE_META, zone)
	var list: Array = _occupants.get(zone, [])
	if not list.has(control):
		list.append(control)
		_occupants[zone] = list
		control.visibility_changed.connect(_on_occupant_visibility.bind(control))
		control.tree_exiting.connect(_forget.bind(control), CONNECT_ONE_SHOT)
	if zone == Zone.SIDE_PANEL or zone == Zone.MODAL:
		_fill(control)
	if zone == Zone.SIDE_PANEL and control.visible:
		_close_other_side_panels(control)
		side_panel_changed.emit(control)
	if zone == Zone.MODAL:
		_update_dim()


## Retire `control` de sa zone (sans le libérer) : il sort de l'arbre si son parent est le
## conteneur de la zone.
func release(control: Control) -> void:
	if control == null:
		return
	_forget(control)
	if control.tree_exiting.is_connected(_forget.bind(control)):
		control.tree_exiting.disconnect(_forget.bind(control))
	var parent := control.get_parent()
	if parent != null and _is_zone_container(parent):
		parent.remove_child(control)
	_update_dim()


## Rectangle de `zone` en pixels de la fenêtre courante (coordonnées du canevas).
func zone_rect(zone: Zone) -> Rect2:
	var view := _view_size()
	var part: Rect2 = ZONE_RECTS[zone]
	return Rect2(part.position * view, part.size * view)


## Occupants de `zone` (hôte courant), visibles ou non.
func occupants(zone: Zone) -> Array[Control]:
	var out: Array[Control] = []
	for control in _occupants.get(zone, []):
		if is_instance_valid(control):
			out.append(control)
	return out


## Occupants visibles de `zone`.
func visible_occupants(zone: Zone) -> Array[Control]:
	var out: Array[Control] = []
	for control in occupants(zone):
		if control.is_visible_in_tree():
			out.append(control)
	return out


## Vrai si une fenêtre modale occupe l'écran.
func modal_open() -> bool:
	return not visible_occupants(Zone.MODAL).is_empty()


## Avis éphémère dans la zone `TOASTS` : pile de `MAX_TOASTS` au plus (les plus anciens se
## replient en une ligne « + N »), effacé après `seconds` ou au clic. Renvoie le contrôle de l'avis.
func toast(text: String, icon: String = "", seconds: float = TOAST_SECONDS) -> Control:
	_ensure_host()
	var box: VBoxContainer = _host["toasts"]
	var entry := PanelContainer.new()
	entry.name = "Toast"
	entry.set_meta(_TOAST_META, true)
	entry.mouse_filter = Control.MOUSE_FILTER_STOP
	entry.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	entry.tooltip_text = "Cliquer pour fermer"
	entry.add_theme_stylebox_override("panel", HudStyle.note_box(8))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	entry.add_child(row)
	if icon != "":
		var texture := HudStyle.icon(icon)
		if texture != null:
			var picture := TextureRect.new()
			picture.texture = texture
			picture.custom_minimum_size = Vector2(20, 20)
			picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			picture.mouse_filter = Control.MOUSE_FILTER_IGNORE
			row.add_child(picture)
	var label := Label.new()
	label.name = "Text"
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	label.add_theme_color_override("font_color", HudStyle.INK)
	UiType.apply(label, UiType.BODY)
	row.add_child(label)
	entry.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_dismiss_toast(entry))
	box.add_child(entry)
	box.move_child(entry, 0)
	UiMotion.fade_in(entry)
	if seconds > 0.0:
		get_tree().create_timer(seconds).timeout.connect(func() -> void: _dismiss_toast(entry))
	_refresh_toasts()
	return entry


## Avis éphémères présents (visibles ou repliés dans « + N »), du plus récent au plus ancien.
func toasts() -> Array[Control]:
	var out: Array[Control] = []
	if _host.is_empty():
		return out
	for child in (_host["toasts"] as Node).get_children():
		if child.has_meta(_TOAST_META) and not child.is_queued_for_deletion():
			out.append(child)
	return out


## Nombre d'avis repliés dans la ligne « + N ».
func folded_toasts() -> int:
	return maxi(0, toasts().size() - MAX_TOASTS)


## Ferme tous les avis éphémères (changement d'écran, tests).
func clear_toasts() -> void:
	for entry in toasts():
		entry.queue_free()
		(_host["toasts"] as Node).remove_child(entry)
	_refresh_toasts()


## Fait suivre à `control` le rectangle de `zone` par ses ancres, sans le reparenter (contrôle
## d'une autre couche, comme la bulle du conseiller). `inset` : marge intérieure en pixels.
func anchor_to(control: Control, zone: Zone, inset: float = 0.0) -> void:
	if control == null:
		return
	var part: Rect2 = ZONE_RECTS[zone]
	control.anchor_left = part.position.x
	control.anchor_top = part.position.y
	control.anchor_right = part.end.x
	control.anchor_bottom = part.end.y
	control.offset_left = inset
	control.offset_top = inset
	control.offset_right = -inset
	control.offset_bottom = -inset
	control.clip_contents = true
	control.set_meta(ZONE_META, zone)
	_anchored[control] = zone


# --- Interne -------------------------------------------------------------------------


func _ensure_host() -> void:
	if not _host.is_empty() and is_instance_valid(_host.get("node")):
		return
	if _default_host.is_empty() or not is_instance_valid(_default_host.get("node")):
		_default_layer = CanvasLayer.new()
		_default_layer.name = "UiLayoutLayer"
		_default_layer.layer = DEFAULT_LAYER
		add_child(_default_layer)
		_default_host = _build_host(_default_layer)
	_host = _default_host
	_occupants = {}


func _build_host(host: Node) -> Dictionary:
	var zones := {}
	var dim := ColorRect.new()
	dim.name = "UiModalDim"
	dim.color = MODAL_DIM
	dim.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	dim.mouse_filter = Control.MOUSE_FILTER_STOP
	dim.hide()
	for zone in Zone.values():
		var node := Control.new()
		node.name = "UiZone_" + str(Zone.keys()[zone])
		node.mouse_filter = Control.MOUSE_FILTER_IGNORE
		node.clip_contents = zone != Zone.MODAL
		var part: Rect2 = ZONE_RECTS[zone]
		node.anchor_left = part.position.x
		node.anchor_top = part.position.y
		node.anchor_right = part.end.x
		node.anchor_bottom = part.end.y
		node.offset_left = 0.0
		node.offset_top = 0.0
		node.offset_right = 0.0
		node.offset_bottom = 0.0
		zones[zone] = node
	# Ordre d'affichage : zones de HUD, panneau latéral, avis, voile puis modale. Dans l'interface de
	# campagne, `PanelStack.restack` respecte ces étages.
	PanelStack.set_tier(zones[Zone.TOP_BAR], PanelStack.Tier.HUD)
	PanelStack.set_tier(zones[Zone.BOTTOM_SELECTION], PanelStack.Tier.HUD)
	PanelStack.set_tier(zones[Zone.MINIMAP], PanelStack.Tier.HUD)
	PanelStack.set_tier(zones[Zone.SIDE_PANEL], PanelStack.Tier.PANEL)
	PanelStack.set_tier(zones[Zone.TOASTS], PanelStack.Tier.BANNER)
	PanelStack.set_tier(dim, PanelStack.Tier.MODAL)
	PanelStack.set_tier(zones[Zone.MODAL], PanelStack.Tier.MODAL)
	for zone in [Zone.TOP_BAR, Zone.BOTTOM_SELECTION, Zone.MINIMAP, Zone.SIDE_PANEL, Zone.TOASTS]:
		host.add_child(zones[zone])
	host.add_child(dim)
	host.add_child(zones[Zone.MODAL])
	# Pile des avis : avis éphémères en haut, occupants ensuite, `at_end` en bas.
	var stack := VBoxContainer.new()
	stack.name = "Stack"
	stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
	stack.add_theme_constant_override("separation", 8)
	stack.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	(zones[Zone.TOASTS] as Control).add_child(stack)
	var toasts_box := VBoxContainer.new()
	toasts_box.name = "Toasts"
	toasts_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	toasts_box.add_theme_constant_override("separation", 4)
	stack.add_child(toasts_box)
	var more := Label.new()
	more.name = "More"
	more.mouse_filter = Control.MOUSE_FILTER_IGNORE
	more.add_theme_color_override("font_color", HudStyle.INK)
	more.add_theme_stylebox_override("normal", HudStyle.note_box(4))
	UiType.apply(more, UiType.CAPTION)
	more.hide()
	toasts_box.add_child(more)
	var spacer := Control.new()
	spacer.name = "Spacer"
	spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	stack.add_child(spacer)
	return {"node": host, "zones": zones, "dim": dim, "stack": stack, "toasts": toasts_box,
		"spacer": spacer, "more": more}


func _on_host_exiting(host: Node) -> void:
	if _host.get("node") == host:
		_host = {}
		_occupants = {}


func _parent_for(zone: Zone, _at_end: bool) -> Control:
	if zone != Zone.TOASTS:
		return (_host["zones"] as Dictionary)[zone]
	# Pile verticale : occupants permanents avant l'espaceur, `at_end` après (voir `claim`).
	return _host["stack"]


func _is_zone_container(node: Node) -> bool:
	if _host.is_empty():
		return false
	return (_host["zones"] as Dictionary).values().has(node) or node == _host.get("stack")


func _fill(control: Control) -> void:
	control.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	control.grow_horizontal = Control.GROW_DIRECTION_BOTH
	control.grow_vertical = Control.GROW_DIRECTION_END


func _forget(control: Control) -> void:
	for zone in _occupants.keys():
		(_occupants[zone] as Array).erase(control)
	if is_instance_valid(control):
		if control.visibility_changed.is_connected(_on_occupant_visibility.bind(control)):
			control.visibility_changed.disconnect(_on_occupant_visibility.bind(control))
		if control.has_meta(ZONE_META):
			control.remove_meta(ZONE_META)
	_anchored.erase(control)
	_update_dim.call_deferred()


func _on_occupant_visibility(control: Control) -> void:
	if not is_instance_valid(control) or not control.has_meta(ZONE_META):
		return
	var zone: int = control.get_meta(ZONE_META)
	if zone == Zone.SIDE_PANEL and control.visible:
		# Un panneau refermé par `fade_out` garde son alpha nul et son glissement : on les
		# rétablit à la réouverture.
		control.modulate.a = 1.0
		_fill(control)
		_close_other_side_panels(control)
		side_panel_changed.emit(control)
	elif zone == Zone.SIDE_PANEL and visible_occupants(Zone.SIDE_PANEL).is_empty():
		side_panel_changed.emit(null)
	if zone == Zone.MODAL:
		if control.visible:
			control.modulate.a = 1.0
			_fill(control)
		_update_dim()
	if zone == Zone.TOASTS and control.visible:
		control.modulate.a = 1.0


func _close_other_side_panels(keep: Control) -> void:
	for other in occupants(Zone.SIDE_PANEL):
		if other != keep and other.visible:
			UiMotion.fade_out(other)


func _update_dim() -> void:
	if _host.is_empty() or not is_instance_valid(_host.get("dim")):
		return
	var dim: ColorRect = _host["dim"]
	dim.visible = modal_open()


func _dismiss_toast(entry: Control) -> void:
	if not is_instance_valid(entry) or entry.is_queued_for_deletion():
		return
	var parent := entry.get_parent()
	if parent != null:
		parent.remove_child(entry)
	entry.queue_free()
	_refresh_toasts()


## Les `MAX_TOASTS` plus récents visibles ; les autres repliés dans « + N ».
func _refresh_toasts() -> void:
	if _host.is_empty():
		return
	var list := toasts()
	for i in list.size():
		list[i].visible = i < MAX_TOASTS
	var more: Label = _host["more"]
	var folded := maxi(0, list.size() - MAX_TOASTS)
	more.text = "+ %d" % folded
	more.visible = folded > 0
	(_host["toasts"] as Node).move_child(more, -1)


func _view_size() -> Vector2:
	var node: Node = _host.get("node") if not _host.is_empty() else null
	if node != null and is_instance_valid(node) and node.is_inside_tree():
		if node is CanvasItem:
			return (node as CanvasItem).get_viewport_rect().size
		return node.get_viewport().get_visible_rect().size
	return get_viewport().get_visible_rect().size
