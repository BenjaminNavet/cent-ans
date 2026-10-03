class_name UiZones
extends Node

## Chantier PO (ADR 0097, bible DA § 12.1) : autoload `UiLayout` (classe `UiZones`), zones d'écran fixes. Chaque
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
## Écart PO1 : la barre du haut mesure 64 px (hauteur logique 800 px à 1280×720, échelle
## d'interface bornée à 0,9) ; `TOP_BAR` passe de 0,05 à 0,08 et `SIDE_PANEL` / `TOASTS`
## commencent à 0,09 au lieu de 0,06 / 0,07 (mêmes bords bas).
const ZONE_RECTS := {
	Zone.TOP_BAR: Rect2(0.0, 0.0, 1.0, 0.08),
	Zone.BOTTOM_SELECTION: Rect2(0.01, 0.80, 0.80, 0.20),
	Zone.MINIMAP: Rect2(0.82, 0.72, 0.18, 0.28),
	Zone.SIDE_PANEL: Rect2(0.67, 0.09, 0.33, 0.61),  # A6-U11 : 422 px à 1280 (0,70 / 0,30 avant)
	Zone.TOASTS: Rect2(0.01, 0.09, 0.26, 0.48),
	Zone.MODAL: Rect2(0.2, 0.12, 0.6, 0.76),
}
const TOAST_SECONDS := 6.0
## NT6b : lignes visibles au plus d'un avis (le reste : points de suspension).
const TOAST_MAX_LINES := 3
const MAX_TOASTS := 3
## Voile des fenêtres modales (bible § 12.1 : noir 45 %).
const MODAL_DIM := Color(0.0, 0.0, 0.0, 0.45)
## Étage de la couche par défaut (au-dessus des HUD de scène, sous les écrans de chargement).
const DEFAULT_LAYER := 60
const ZONE_META := &"ui_layout_zone"
const _TOAST_META := &"ui_layout_toast"
const _WRAPPER_META := &"ui_layout_wrapper"

## Hôte courant : `{node, zones: {Zone: Control}, dim: ColorRect, stack: VBoxContainer,
## toasts: VBoxContainer, spacer: Control, more: Label}`.
var _host: Dictionary = {}
var _default_host: Dictionary = {}
var _default_layer: CanvasLayer = null
## Occupants par zone (`Zone` → `Array[Control]`), hôte courant (`_host["occupants"]`).
var _occupants: Dictionary = {}
## Hôtes posés, du plus ancien au plus récent (la bataille se pose sur la carte, qui reste
## chargée) : à la sortie d'un hôte, le précédent redevient courant.
var _hosts: Array = []
## Hôte (dictionnaire) de chaque occupant.
var _owner: Dictionary = {}
## Contrôles suivant une zone sans reparentage (`anchor_to`).
var _anchored: Dictionary = {}


## L'autoload `UiLayout` (les scripts à `class_name` sont compilés avant les autoloads : ils
## passent par `UiZones.layout()`, `UiZones.put(…)`, `UiZones.rect(…)` et `UiZones.Zone`).
static func layout() -> UiZones:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("UiLayout") as UiZones if tree != null else null


## Raccourci de `layout().claim(zone, control, at_end)` (sans effet sans l'autoload).
static func put(zone: Zone, control: Control, at_end: bool = false) -> void:
	var node := layout()
	if node != null:
		node.claim(zone, control, at_end)


## Raccourci de `layout().zone_rect(zone)` ; sans l'autoload, proportions de la fenêtre.
static func rect(zone: Zone) -> Rect2:
	var node := layout()
	if node != null:
		return node.zone_rect(zone)
	var tree := Engine.get_main_loop() as SceneTree
	var view := tree.root.get_visible_rect().size if tree != null else Vector2(1440, 900)
	var part: Rect2 = ZONE_RECTS[zone]
	return Rect2(part.position * view, part.size * view)


## Donne à `host` (un `CanvasLayer` ou un `Control` plein écran) les six zones ; il devient l'hôte
## courant jusqu'à sa sortie de l'arbre ou au prochain `attach_host`. Rappeler avec le même hôte
## ne fait rien.
func attach_host(host: Node) -> void:
	if host == null:
		return
	if not _host.is_empty() and _host.get("node") == host:
		return
	for entry in _hosts:
		if entry.get("node") == host:
			_hosts.erase(entry)
			_hosts.append(entry)
			_use(entry)
			return
	var built := _build_host(host)
	_hosts.append(built)
	_use(built)
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
	if zone == Zone.SIDE_PANEL:
		parent = _side_wrapper(control, parent)
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
		_owner[control] = _host
		control.visibility_changed.connect(_on_occupant_visibility.bind(control))
		control.tree_exiting.connect(_forget.bind(control), CONNECT_ONE_SHOT)
	if zone == Zone.SIDE_PANEL:
		# La zone fixe la largeur : une largeur minimale propre au panneau (380-420 px) dépasse la
		# zone quand l'écran logique est étroit (341 px à 1280×720) et le texte est coupé au bord.
		control.custom_minimum_size.x = 0.0
		control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		control.size_flags_vertical = Control.SIZE_EXPAND_FILL
		(parent as CanvasItem).visible = control.visible
		if control.visible:
			_close_other_side_panels(control)
			side_panel_changed.emit(control)
	if zone == Zone.MODAL:
		_center(control)
		_update_dim()


## Retire `control` de sa zone (sans le libérer) : il sort de l'arbre si son parent est le
## conteneur de la zone (ou l'enveloppe défilante du panneau latéral, alors libérée).
func release(control: Control) -> void:
	if control == null:
		return
	_forget(control)
	if control.tree_exiting.is_connected(_forget.bind(control)):
		control.tree_exiting.disconnect(_forget.bind(control))
	var parent := control.get_parent()
	if parent != null and parent.has_meta(_WRAPPER_META):
		parent.remove_child(control)
		parent.queue_free()
	elif parent != null and _is_zone_container(parent):
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
	# Q8 : un même avis répété (« Carte politique. » à chaque sortie de mode) remplace le
	# précédent au lieu de s'empiler.
	for previous in toasts():
		var previous_label := previous.find_child("Text", true, false) as Label
		if previous_label != null and previous_label.text == text:
			_dismiss_toast(previous)
	var entry := PanelContainer.new()
	entry.name = "Toast"
	entry.set_meta(_TOAST_META, true)
	entry.mouse_filter = Control.MOUSE_FILTER_STOP
	entry.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	RichTooltip.attach_plain(entry, "click_to_close")
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
	label.max_lines_visible = TOAST_MAX_LINES  # NT6b : un avis très long reste dans sa zone
	label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	label.add_theme_color_override("font_color", HudStyle.INK)
	UiType.apply(label, UiType.BODY)
	row.add_child(label)
	entry.clip_contents = true
	entry.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_dismiss_toast(entry))
	box.add_child(entry)
	box.move_child(entry, 0)
	UiMotion.fade_in(entry)
	if seconds > 0.0:
		# Q7 : identifiant d'instance, l'avis a pu être fermé avant (clic, changement d'écran).
		var entry_id := entry.get_instance_id()
		get_tree().create_timer(seconds).timeout.connect(func() -> void: _dismiss_toast(instance_from_id(entry_id) as Control))
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
	for i in range(_hosts.size() - 1, -1, -1):
		if is_instance_valid(_hosts[i].get("node")) and (_hosts[i]["node"] as Node).is_inside_tree():
			_use(_hosts[i])
			return
	if _default_host.is_empty() or not is_instance_valid(_default_host.get("node")):
		_default_layer = CanvasLayer.new()
		_default_layer.name = "UiLayoutLayer"
		_default_layer.layer = DEFAULT_LAYER
		add_child(_default_layer)
		_default_host = _build_host(_default_layer)
	_use(_default_host)


## Rend `entry` (dictionnaire d'hôte) courant.
func _use(entry: Dictionary) -> void:
	_host = entry
	_occupants = entry.get("occupants", {}) if not entry.is_empty() else {}


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
		node.clip_contents = zone in [Zone.MINIMAP, Zone.SIDE_PANEL, Zone.TOASTS]
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
	# Ordre d'affichage : zones de HUD et avis, panneau latéral, voile puis modale. Dans l'interface de
	# campagne, `PanelStack.restack` respecte ces étages. Q6 : les avis et le journal (`TOASTS`)
	# sont au niveau du HUD, sous toute fenêtre ouverte par le joueur (elle en recevait les clics) ;
	# la carte ne les monte à l'étage `BANNER` que le temps du bandeau de fin de tour.
	PanelStack.set_tier(zones[Zone.TOP_BAR], PanelStack.Tier.HUD)
	PanelStack.set_tier(zones[Zone.BOTTOM_SELECTION], PanelStack.Tier.HUD)
	PanelStack.set_tier(zones[Zone.MINIMAP], PanelStack.Tier.HUD)
	PanelStack.set_tier(zones[Zone.SIDE_PANEL], PanelStack.Tier.PANEL)
	PanelStack.set_tier(zones[Zone.TOASTS], PanelStack.Tier.HUD)
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
		"spacer": spacer, "more": more, "occupants": {}}


func _on_host_exiting(host: Node) -> void:
	for entry in _hosts.duplicate():
		if entry.get("node") == host:
			_hosts.erase(entry)
	if _host.get("node") == host:
		_use({})


func _parent_for(zone: Zone, _at_end: bool) -> Control:
	if zone != Zone.TOASTS:
		return (_host["zones"] as Dictionary)[zone]
	# Pile verticale : occupants permanents avant l'espaceur, `at_end` après (voir `claim`).
	return _host["stack"]


func _is_zone_container(node: Node) -> bool:
	if _host.is_empty():
		return false
	return (_host["zones"] as Dictionary).values().has(node) or node == _host.get("stack")


## Panneau latéral : chaque occupant est posé dans une enveloppe défilante qui remplit la zone
## (un panneau plus haut que la zone défile au lieu de déborder sur la minicarte).
func _side_wrapper(control: Control, zone_node: Control) -> Control:
	var current := control.get_parent()
	if current != null and current.has_meta(_WRAPPER_META) and current.get_parent() == zone_node:
		return current
	var wrapper := ScrollContainer.new()
	wrapper.name = str(control.name) + "Scroll"
	wrapper.set_meta(_WRAPPER_META, true)
	wrapper.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	wrapper.follow_focus = true
	wrapper.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	zone_node.add_child(wrapper)
	return wrapper


## Fenêtre modale centrée dans sa zone, à sa taille (la zone, elle, ne bouge pas).
func _center(control: Control) -> void:
	control.set_anchors_and_offsets_preset(Control.PRESET_CENTER, Control.PRESET_MODE_MINSIZE)
	control.grow_horizontal = Control.GROW_DIRECTION_BOTH
	control.grow_vertical = Control.GROW_DIRECTION_BOTH


func _forget(control: Control) -> void:
	var entry: Dictionary = _owner.get(control, _host)
	_owner.erase(control)
	var lists: Dictionary = entry.get("occupants", {})
	for zone in lists.keys():
		(lists[zone] as Array).erase(control)
	if is_instance_valid(control):
		if control.visibility_changed.is_connected(_on_occupant_visibility.bind(control)):
			control.visibility_changed.disconnect(_on_occupant_visibility.bind(control))
		if control.has_meta(ZONE_META):
			control.remove_meta(ZONE_META)
	_anchored.erase(control)
	if is_instance_valid(control):
		var parent := control.get_parent()
		if parent != null and parent.has_meta(_WRAPPER_META) and not parent.is_queued_for_deletion():
			# Par identifiant : l'enveloppe peut être libérée avant l'appel différé (un argument
			# typé Object refuse alors l'instance morte).
			_free_wrapper.call_deferred(parent.get_instance_id())
	_update_dim.call_deferred()


## Enveloppe défilante devenue vide (occupant sorti) ; rien si elle est partie avec son hôte.
func _free_wrapper(wrapper_id: int) -> void:
	var wrapper := instance_from_id(wrapper_id) as Node
	if wrapper != null and not wrapper.is_queued_for_deletion() and wrapper.get_child_count() == 0:
		wrapper.queue_free()


func _on_occupant_visibility(control: Control) -> void:
	if not is_instance_valid(control) or not control.has_meta(ZONE_META):
		return
	# Règles appliquées dans l'hôte de l'occupant (la carte, sous une bataille, reste un hôte).
	var current := _host
	_use(_owner.get(control, _host))
	_apply_visibility(control)
	_use(current)


func _apply_visibility(control: Control) -> void:
	var zone: int = control.get_meta(ZONE_META)
	if zone == Zone.SIDE_PANEL:
		var wrapper := control.get_parent()
		if wrapper != null and wrapper.has_meta(_WRAPPER_META):
			(wrapper as CanvasItem).visible = control.visible
		if control.visible:
			# Un panneau refermé par `fade_out` garde son alpha nul : on le rétablit.
			control.modulate.a = 1.0
			_close_other_side_panels(control)
			side_panel_changed.emit(control)
		elif visible_occupants(Zone.SIDE_PANEL).is_empty():
			side_panel_changed.emit(null)
	if zone == Zone.MODAL:
		if control.visible:
			control.modulate.a = 1.0
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
