class_name DiplomacyMapView
extends DiplomacyView

## Colonne « Carte des relations » : minicarte aux provinces teintées selon la position
## diplomatique (DP2), vue du joueur ou de la faction choisie (DZ) ; un clic désigne le seigneur
## de la province. La carte occupe la place restante du `host` (le panneau), qu'elle ramène à
## `target_size` quand le contenu l'a fait grandir (Q6).

signal faction_clicked(faction_id: String)

## Teintes de la carte diplomatique (plus saturées que la carte 3D : fond clair de la minicarte).
const MAP_COLORS := {
	"self": Color(0.86, 0.68, 0.18), "war": Color(0.78, 0.10, 0.08), "truce": Color(0.95, 0.55, 0.15),
	"alliance": Color(0.18, 0.40, 0.85), "vassal": Color(0.55, 0.25, 0.72), "suzerain": Color(0.55, 0.25, 0.72),
}
const FRIENDLY := Color(0.30, 0.62, 0.30)
const HOSTILE := Color(0.72, 0.36, 0.22)
const NEUTRAL := Color(0.62, 0.60, 0.55)
## Taille plancher de la carte des relations (elle grandit ensuite jusqu'à remplir sa colonne).
const MAP_MIN_WIDTH := 200.0
const MAP_MIN_HEIGHT := 150.0

var map_data: MapData = null
## Panneau hôte et taille visée (voir `_fit_minimap`).
var host: Control = null
var target_size := Vector2.ZERO
var their_view := true
var entries: Array = []
var _minimap: CampaignMinimap
var _caption: Label
var _view_toggle: CheckButton
var _fit_queued := false
var _fitting := false


func _init() -> void:
	super("MapColumn", 6)
	add_child(_section("Carte des relations"))
	var view_row := HBoxContainer.new()  # DZ
	view_row.add_theme_constant_override("separation", 8)
	_caption = _label("", UiType.BODY, HudStyle.INK)
	_caption.name = "MapViewCaption"
	_caption.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	view_row.add_child(_caption)
	_view_toggle = CheckButton.new()
	_view_toggle.name = "MapViewToggle"
	_view_toggle.text = "Vue de la faction choisie"
	_view_toggle.button_pressed = their_view
	_view_toggle.focus_mode = Control.FOCUS_NONE
	TooltipHost.attach_plain(_view_toggle, "diplomacy_map_view")
	_view_toggle.toggled.connect(func(on: bool) -> void:
		their_view = on
		_render())
	view_row.add_child(_view_toggle)
	add_child(view_row)
	var holder := CenterContainer.new()
	holder.name = "MapHolder"
	holder.size_flags_vertical = Control.SIZE_EXPAND_FILL
	holder.resized.connect(_fit_minimap)
	add_child(holder)
	add_child(DiplomaticStances.legend(UiType.CAPTION))  # DP2
	var hint := _label("Cliquez une province pour traiter avec son seigneur. Les terres voilées sont hors de vue de vos agents et de vos armées.", UiType.CAPTION, HudStyle.INK_FADED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(hint)


func _ensure_minimap() -> void:
	if _minimap != null or map_data == null:
		return
	var holder: Control = find_child("MapHolder", true, false)
	if holder == null:
		return
	_minimap = CampaignMinimap.new()
	_minimap.name = "DiplomacyMap"
	_minimap.setup(map_data)
	# Pas de boutons de mode : la carte des relations n'a qu'un rendu.
	for child in _minimap.get_child(0).get_children():
		if child is HBoxContainer:
			child.hide()
	_minimap.clicked.connect(_on_map_clicked)
	holder.add_child(_minimap)
	_fit_minimap()
	refit_later()


## Q6 : ajustement différé (une fois par image) quand le panneau change de taille ; appelé
## directement depuis `resized`, il se relançait lui-même en rendant sa taille au panneau.
func queue_fit() -> void:
	if _fit_queued or _fitting:
		return
	_fit_queued = true
	(func() -> void:
		_fit_queued = false
		if is_inside_tree():
			_fit_minimap()).call_deferred()


func _fit_minimap() -> void:
	if _minimap == null or _fitting:
		return
	var holder := _minimap.get_parent() as Control
	var view := _minimap.find_child("MapView", true, false) as Control
	if holder == null or view == null or _minimap.crop.size.x <= 0.0:
		return
	var aspect := _minimap.crop.size.y / _minimap.crop.size.x
	# Habillage réel de la minicarte (cadre, boutons) autour de la vue, plus un jeu de 4 px :
	# une marge fixe plus petite que le cadre ferait grandir le conteneur à chaque `resized`.
	var chrome := _minimap.get_combined_minimum_size() - view.get_combined_minimum_size() + Vector2(4.0, 4.0)
	# Q6 : place prise au-delà de la taille visée (le panneau a grandi avec son contenu).
	var excess := Vector2.ZERO
	if host != null and target_size != Vector2.ZERO:
		excess = (host.size - target_size).max(Vector2.ZERO)
	var room := holder.size - chrome - excess
	var width := maxf(room.x, MAP_MIN_WIDTH)
	var height := width * aspect
	if height > room.y:
		height = maxf(room.y, MAP_MIN_HEIGHT)
		width = height / aspect
	var fitted := Vector2(width, height).floor()
	if fitted != view.custom_minimum_size:
		view.custom_minimum_size = fitted
	if excess != Vector2.ZERO:
		_fitting = true
		host.size = target_size  # rendu à la taille visée (bornée par la nouvelle taille minimale)
		_fitting = false
	_minimap.tooltip_text = ""


func _render() -> void:
	_ensure_minimap()
	if _minimap == null or sim == null:
		return
	var ids := PackedStringArray()
	for index in range(1, map_data.province_count + 1):
		ids.append(str(map_data.get_province(index).get("id", "")))
	# RS-E : instantané groupé (mêmes index que `ids`, même construction) au lieu d'un
	# `get_province_state` par province.
	var snapshot := ProvinceSnapshot.of(sim, map_data)
	# DP2 : mêmes couleurs que le mode « Diplomatie » de la carte et de la minicarte.
	if DiplomaticStances.available(sim):
		# DZ : vue de la faction choisie (ses ennemis en rouge, ses terres en blanc).
		var viewer := map_viewer()
		_update_caption(viewer)
		var stances := DiplomaticStances.stances(sim, ids, viewer)
		var stance_colors := PackedColorArray()
		for index in ids.size():
			var key := stances[index] if index < stances.size() else ""
			var stance_color := DiplomaticStances.color_of(key)
			if viewer == "" and key != "" and key != "self" and faction_id != "" and _controller_of(snapshot, index) == faction_id:
				stance_color = stance_color.lightened(0.3)
			stance_colors.append(stance_color)
		_minimap.set_province_colors(stance_colors)
		return
	var relations: PackedStringArray = sim.call("get_province_relations", ids)
	var attitude_of := {}
	for faction_entry in entries:
		attitude_of[str(faction_entry["id"])] = int(faction_entry["attitude"])
	var colors := PackedColorArray()
	for index in ids.size():
		var relation := relations[index] if index < relations.size() else ""
		var color := Color(0, 0, 0, 0)
		# Contrôleur lu dans l'instantané groupé (et seulement s'il sert).
		var controller: String = _controller_of(snapshot, index) if relation != "" and relation != "self" else ""
		if MAP_COLORS.has(relation):
			color = MAP_COLORS[relation]
		elif relation == "peace":
			var attitude := int(attitude_of.get(controller, 0))
			color = NEUTRAL.lerp(FRIENDLY if attitude >= 0 else HOSTILE, clampf(absf(attitude) / 60.0, 0.0, 1.0))
		if relation != "" and relation != "self" and faction_id != "" and controller == faction_id:
			color = color.lightened(0.22)
		colors.append(color)
	_minimap.set_province_colors(colors)


## DZ : faction dont la carte montre les relations ("" : le joueur).
func map_viewer() -> String:
	if not their_view or faction_id == "" or faction_id == player_faction:
		return ""
	if sim == null or not sim.has_method("get_province_stances_for"):
		return ""
	return faction_id


func _update_caption(viewer: String) -> void:
	if _caption == null:
		return
	var facade := get_node_or_null("/root/SimFacade")  # autoload : pas d'identifiant (compilé avant)
	_caption.text = "Relations de %s" % facade.call("faction_short_name", viewer) if viewer != "" and facade != null else "Vos relations"
	if _view_toggle != null:
		_view_toggle.visible = sim != null and sim.has_method("get_province_stances_for")


## Contrôleur de la province d'index `index` dans l'instantané groupé (RS-E : `ProvinceSnapshot`,
## au lieu d'une lecture `get_province_state` par province).
func _controller_of(snapshot: ProvinceSnapshot, index: int) -> String:
	if index < 0 or index >= snapshot.controller.size():
		return ""
	return snapshot.controller[index]


func _on_map_clicked(map_pos: Vector2) -> void:
	if map_data == null or sim == null:
		return
	var index := map_data.province_index_at(map_pos.x, map_pos.y)
	if index <= 0:
		return
	var id := str(map_data.get_province(index).get("id", ""))
	var state: Dictionary = sim.call("get_province_state", id)
	var owner := str(state.get("controller", state.get("owner", "")))
	if owner != "" and owner != player_faction:
		faction_clicked.emit(owner)


## P2g : la carte repart de sa taille plancher, puis se réajuste une image plus tard, une fois les
## conteneurs recalculés (lue dans la même image, la taille du cadre serait encore l'ancienne).
func refit_later() -> void:
	if _minimap == null or not is_inside_tree():
		return
	var map_view := _minimap.find_child("MapView", true, false) as Control
	if map_view != null:
		map_view.custom_minimum_size = Vector2(MAP_MIN_WIDTH, MAP_MIN_HEIGHT)
	if not get_tree().process_frame.is_connected(_fit_minimap):
		get_tree().process_frame.connect(_fit_minimap, CONNECT_ONE_SHOT)
