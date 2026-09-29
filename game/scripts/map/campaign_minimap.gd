class_name CampaignMinimap
extends PanelContainer

## Minicarte de campagne (lot C1), dans un cadre de parchemin : terres et mer,
## couleurs de faction par province, frontières, provinces voilées par le brouillard, armées
## visibles en points, cadre de la vue caméra. Clic ou glisser = `clicked(map_pos)` (la carte
## recentre la caméra). Deux modes : politique (aplats de faction) et relief.
##
## Le fond (relief et index de province réduits) est calculé une fois dans `setup` ; les
## couleurs de faction et le masque de brouillard sont des textures 1D (une entrée par
## province) remplacées à chaque rafraîchissement, sans reparcourir l'image.
## Pure présentation : `MinimapController` lui passe couleurs, visibilité, armées et vue.

signal clicked(map_pos: Vector2)
signal mode_changed(mode: String)
## UX1 : bouton « Légende » basculé (le contrôleur ouvre ou ferme `MapLegend`).
signal legend_toggled(pressed: bool)

const SHADER := preload("res://shaders/campaign_minimap.gdshader")
const MODE_POLITICAL := "political"
const MODE_RELIEF := "relief"
## Largeur de la carte dans le cadre (le cadre fait la largeur des lettres scellées).
const MAP_WIDTH := 280.0
const MAX_MAP_HEIGHT := 230.0
## Résolution du fond pré-calculé par rapport à l'affichage.
const TEXTURE_SCALE := 1.5
## Marge autour de l'emprise des provinces (fraction de sa taille).
const CROP_MARGIN := 0.03
const SEA := Color(0.46, 0.55, 0.56)
const SEA_DEEP := Color(0.33, 0.42, 0.45)
const LOWLAND := Color(0.86, 0.80, 0.62)
const HIGHLAND := Color(0.58, 0.47, 0.32)
const FRAME_COLOR := Color(1.0, 0.96, 0.82)
const PLAYER_RING := Color(0.95, 0.80, 0.30)

var map_data: MapData
var mode: String = MODE_POLITICAL
var fog_enabled: bool = false
## Emprise affichée, en coordonnées carte (pixels de `province_ids.png`).
var crop := Rect2(0, 0, 1, 1)

var _view: TextureRect
var _overlay: Control
var _material: ShaderMaterial
var _mode_buttons: Dictionary = {}  # mode → Button
var _modes_row: HBoxContainer
var legend_button: Button
var _armies: Array = []  # [{pos: Vector2 carte, color: Color, player: bool}]
var _frame := PackedVector2Array()  # quadrilatère de la vue caméra (coordonnées carte)
var _visible_count: int = -1


func _init() -> void:
	name = "CampaignMinimap"
	mouse_filter = Control.MOUSE_FILTER_STOP
	# UX1 : boutons des modes au thème parchemin, comme le reste du HUD.
	theme = load("res://scenes/ui/parchment_theme.tres")
	add_theme_stylebox_override("panel", HudStyle.panel_box(6))
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	add_child(box)
	_view = TextureRect.new()
	_view.name = "MapView"
	_view.stretch_mode = TextureRect.STRETCH_SCALE
	_view.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_view.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_view.custom_minimum_size = Vector2(MAP_WIDTH, MAP_WIDTH * 0.75)
	box.add_child(_view)
	_overlay = Control.new()
	_overlay.name = "Overlay"
	_overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_overlay.mouse_filter = Control.MOUSE_FILTER_STOP
	_overlay.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	RichTooltip.attach_plain(_overlay, "minimap_click_camera")
	_overlay.clip_contents = true
	_overlay.draw.connect(_draw_overlay)
	_overlay.gui_input.connect(_on_overlay_input)
	_view.add_child(_overlay)
	var modes := HBoxContainer.new()
	modes.add_theme_constant_override("separation", 4)
	modes.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(modes)
	_modes_row = modes
	for entry in [[MODE_POLITICAL, "Politique", "Couleurs des royaumes"], [MODE_RELIEF, "Relief", "Terres, montagnes et mers"]]:
		var button := Button.new()
		button.name = "Mode_%s" % entry[0]
		button.text = entry[1]
		button.tooltip_text = entry[2]
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
		var mode_id: String = entry[0]
		_decorate_ink(button, "lens_" + mode_id, 14)  # DA5
		button.pressed.connect(func() -> void: set_mode(mode_id))
		modes.add_child(button)
		_mode_buttons[mode_id] = button
	# UX1 : bouton de la légende de la carte, au bout de la rangée des modes.
	legend_button = Button.new()
	legend_button.name = "LegendButton"
	legend_button.text = "Légende"
	RichTooltip.attach_plain(legend_button, "map_legend_open")
	legend_button.toggle_mode = true
	legend_button.focus_mode = Control.FOCUS_NONE
	legend_button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	# DA5 : rose des vents à l'encre (repli : livre du Codex, puis « ? »).
	var book := HudStyle.icon("map_legend")
	if book != null:
		_decorate_ink(legend_button, "map_legend", 14)
	else:
		book = HudStyle.icon("hud_codex")
	if book != null and legend_button.icon == null:
		legend_button.icon = book
		legend_button.add_theme_constant_override("icon_max_width", 14)
	elif book == null:
		legend_button.text = "? Légende"
	legend_button.toggled.connect(func(pressed: bool) -> void: legend_toggled.emit(pressed))
	modes.add_child(legend_button)
	_material = ShaderMaterial.new()
	_material.shader = SHADER
	_view.material = _material
	_sync_mode_buttons()


## Lot C5 : range un bouton de couche de la carte (routes commerciales) dans la rangée des modes,
## en tête ; la barre du haut n'a plus la place de le porter en 1280 px.
func add_layer_button(button: Button) -> void:
	if _modes_row == null or button == null:
		return
	if button.get_parent() != null:
		button.get_parent().remove_child(button)
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	_modes_row.add_child(button)
	_modes_row.move_child(button, 0)


## Pré-calcule le fond : emprise des provinces, index de province et relief réduits.
func setup(data: MapData) -> void:
	map_data = data
	var cached := _textures_for(data)
	crop = cached["crop"]
	_view.custom_minimum_size = cached["view_size"]
	_natural_view = cached["view_size"]
	_material.set_shader_parameter("province_ids", cached["ids"])
	_material.set_shader_parameter("relief", cached["relief"])
	_view.texture = cached["relief"]
	set_province_colors(PackedColorArray())
	set_fog(false, PackedStringArray())


## PB1 : textures d'identifiants et de relief calculées une fois par carte (pixel par pixel en
## GDScript, ~0,5 s) puis partagées par toutes les mini-cartes (carte de campagne, panneau de
## diplomatie) : elles ne dépendent que de `MapData` et des constantes d'affichage.
static var _texture_cache: Dictionary = {}


static func _textures_for(data: MapData) -> Dictionary:
	var key := data.get_instance_id()
	if _texture_cache.has(key):
		return _texture_cache[key]
	_texture_cache.clear()  # une seule carte vivante à la fois
	var crop_rect := _province_extent(data)
	var aspect := crop_rect.size.y / maxf(crop_rect.size.x, 1.0)
	var view_size := Vector2(MAP_WIDTH, MAP_WIDTH * aspect)
	if view_size.y > MAX_MAP_HEIGHT:
		view_size = Vector2(MAX_MAP_HEIGHT / aspect, MAX_MAP_HEIGHT)
	# Texture à 1,5 fois l'affichage : frontières nettes pour un coût de calcul modéré.
	var tex_size := Vector2i((view_size * TEXTURE_SCALE).round())
	var id_bytes := PackedByteArray()
	id_bytes.resize(tex_size.x * tex_size.y * 2)
	var rgb_bytes := PackedByteArray()
	rgb_bytes.resize(tex_size.x * tex_size.y * 3)
	var step := crop_rect.size / Vector2(tex_size)
	var slope_step := maxi(int(step.x), 1)
	var offset := 0
	for ty in tex_size.y:
		var py := int(crop_rect.position.y + (ty + 0.5) * step.y)
		for tx in tex_size.x:
			var px := int(crop_rect.position.x + (tx + 0.5) * step.x)
			var index := data.province_index_at(px, py)
			id_bytes[offset * 2] = index & 0xFF
			id_bytes[offset * 2 + 1] = (index >> 8) & 0xFF
			var color := _relief_color(data, px, py, slope_step)
			rgb_bytes[offset * 3] = color.r8
			rgb_bytes[offset * 3 + 1] = color.g8
			rgb_bytes[offset * 3 + 2] = color.b8
			offset += 1
	var ids := Image.create_from_data(tex_size.x, tex_size.y, false, Image.FORMAT_RG8, id_bytes)
	var shaded := Image.create_from_data(tex_size.x, tex_size.y, false, Image.FORMAT_RGB8, rgb_bytes)
	var result := {
		"crop": crop_rect,
		"view_size": view_size.round(),
		"ids": ImageTexture.create_from_image(ids),
		"relief": ImageTexture.create_from_image(shaded),
	}
	_texture_cache[key] = result
	return result


## Emprise des provinces (anneaux), élargie de `CROP_MARGIN`, dans les bornes de la carte.
static func _province_extent(data: MapData) -> Rect2:
	var bounds := Rect2()
	var first := true
	for index in data.provinces:
		for ring in data.provinces[index].get("rings", []):
			for point in ring:
				if first:
					bounds = Rect2(point, Vector2.ZERO)
					first = false
				else:
					bounds = bounds.expand(point)
	var full := Rect2(Vector2.ZERO, Vector2(data.size))
	if first or bounds.size.x < 1.0 or bounds.size.y < 1.0:
		return full
	bounds = bounds.grow_individual(bounds.size.x * CROP_MARGIN, bounds.size.y * CROP_MARGIN, bounds.size.x * CROP_MARGIN, bounds.size.y * CROP_MARGIN)
	return bounds.intersection(full)


## Couleur de fond : terres ombrées selon l'altitude (ombrage nord-ouest), mer selon la profondeur.
static func _relief_color(data: MapData, px: int, py: int, sample_step: int) -> Color:
	var span := data.height_max_m - data.height_min_m
	var height := data.height_min_m + data.height01_px(px, py) * span
	if not data.is_land_px(px, py):
		return SEA.lerp(SEA_DEEP, clampf(-height / 2000.0, 0.0, 1.0))
	var t := clampf(pow(maxf(height, 0.0) / 2500.0, 0.6), 0.0, 1.0)
	var color := LOWLAND.lerp(HIGHLAND, t)
	var north_west := data.height_min_m + data.height01_px(px - sample_step, py - sample_step) * span
	var slope := (north_west - height) / maxf(sample_step * data.meters_per_px, 1.0)
	return color.darkened(clampf(slope * 6.0, -0.15, 0.3))


## Couleur de chaque province (`colors[index - 1]`, alpha 0 = neutre), comme le terrain.
func set_province_colors(colors: PackedColorArray) -> void:
	var count := map_data.province_count if map_data != null else 0
	var image := Image.create(maxi(count + 1, 1), 1, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	for i in mini(colors.size(), count):
		image.set_pixel(i + 1, 0, colors[i])
	_material.set_shader_parameter("faction_colors", ImageTexture.create_from_image(image))


## Brouillard : `visible_ids` = provinces vues (ids) ; `enabled` faux = tout est vu.
func set_fog(enabled: bool, visible_ids: PackedStringArray) -> void:
	fog_enabled = enabled
	var count := map_data.province_count if map_data != null else 0
	var image := Image.create(maxi(count + 1, 1), 1, false, Image.FORMAT_R8)
	image.fill(Color(0, 0, 0))
	_visible_count = 0
	for id in visible_ids:
		var index := map_data.index_of_id(id) if map_data != null else 0
		if index > 0 and index <= count:
			image.set_pixel(index, 0, Color(1, 0, 0))
			_visible_count += 1
	_material.set_shader_parameter("fog_mask", ImageTexture.create_from_image(image))
	_material.set_shader_parameter("fog_enabled", enabled)
	_material.set_shader_parameter("fog_by_cell", false)


## Brouillard par case (lot M5a) : `cells` = texture de vue de la simulation couvrant `size_px`
## pixels carte ; `visible_count` = nombre de provinces visibles (statistique, tests).
func set_fog_cells(enabled: bool, cells: Texture2D, size_px: Vector2, visible_count: int) -> void:
	fog_enabled = enabled
	_visible_count = visible_count if enabled else 0
	var size := Vector2(maxf(size_px.x, 1.0), maxf(size_px.y, 1.0))
	_material.set_shader_parameter("fog_cells", cells)
	_material.set_shader_parameter("fog_crop", Vector4(crop.position.x / size.x, crop.position.y / size.y, crop.size.x / size.x, crop.size.y / size.y))
	_material.set_shader_parameter("fog_by_cell", enabled and cells != null)
	_material.set_shader_parameter("fog_enabled", enabled)


## Armées affichées : `[{pos: Vector2 (carte), color: Color, player: bool}]` (déjà filtrées).
func set_armies(armies: Array) -> void:
	_armies = armies
	_overlay.queue_redraw()


## Quadrilatère de la vue caméra au sol (coordonnées carte) ; vide = pas de cadre.
func set_view_frame(points: PackedVector2Array) -> void:
	if points == _frame:
		return
	_frame = points
	_overlay.queue_redraw()


func set_mode(new_mode: String) -> void:
	if new_mode != MODE_POLITICAL and new_mode != MODE_RELIEF:
		return
	var changed := new_mode != mode
	mode = new_mode
	_material.set_shader_parameter("political", mode == MODE_POLITICAL)
	_sync_mode_buttons()
	if changed:
		mode_changed.emit(mode)


## DA5 : icône d'action à l'encre sur un bouton de la rangée (or au survol) ; rien si absente.
static func _decorate_ink(target: Button, icon_id: String, size: int) -> void:
	var library := HudStyle.icon_library()
	if library == null or not bool(library.call("has_icon", icon_id)):
		return
	library.call("decorate_button", target, icon_id, size)


func _sync_mode_buttons() -> void:
	_material.set_shader_parameter("political", mode == MODE_POLITICAL)
	for key in _mode_buttons:
		(_mode_buttons[key] as Button).set_pressed_no_signal(key == mode)


func army_dot_count() -> int:
	return _armies.size()


func visible_province_count() -> int:
	return _visible_count


## PO1 : taille de la carte réduite à l'affichage d'origine (`setup`), avant `fit_to`.
var _natural_view := Vector2.ZERO


## PO1 (bible DA § 12.1) : ajuste la minicarte à la zone `MINIMAP` (`room`, pixels) — la carte
## réduite garde ses proportions et rétrécit si besoin ; la rangée des modes passe en icônes
## seules (infobulles gardées) quand leurs libellés ne tiennent pas. Jamais plus grande que
## l'affichage d'origine.
func fit_to(room: Vector2) -> void:
	if _natural_view == Vector2.ZERO:
		_natural_view = _view.custom_minimum_size
	if _natural_view.x <= 0.0 or _natural_view.y <= 0.0:
		return
	var frame := get_theme_stylebox("panel").get_minimum_size() if has_theme_stylebox("panel") else Vector2.ZERO
	_set_mode_labels(true)
	if _modes_row.get_combined_minimum_size().x + frame.x > room.x:
		_set_mode_labels(false)
	var row_height := _modes_row.get_combined_minimum_size().y + 4.0
	var free := Vector2(room.x - frame.x, room.y - frame.y - row_height)
	var ratio := minf(1.0, minf(free.x / _natural_view.x, free.y / _natural_view.y))
	var target := (_natural_view * maxf(ratio, 0.2)).floor()
	if not _view.custom_minimum_size.is_equal_approx(target):
		_view.custom_minimum_size = target
		_overlay.queue_redraw()


## Libellés des boutons de la rangée des modes (`false` : icônes seules, s'ils en ont une).
func _set_mode_labels(labelled: bool) -> void:
	for child in _modes_row.get_children():
		var button := child as Button
		if button == null or button.icon == null:
			continue
		if not button.has_meta(&"po1_label"):
			button.set_meta(&"po1_label", button.text)
		button.text = str(button.get_meta(&"po1_label")) if labelled else ""


## Rectangle de la carte réduite dans le repère de la minicarte (tests, placement).
func map_rect() -> Rect2:
	return Rect2(_view.position + _overlay.position, _overlay.size)


## Coordonnées carte → pixels de la carte réduite (repère de `Overlay`).
func map_to_view(map_pos: Vector2) -> Vector2:
	return (map_pos - crop.position) / crop.size * _overlay.size


## Pixels de la carte réduite (repère de `Overlay`) → coordonnées carte (bornées à l'emprise).
func view_to_map(local: Vector2) -> Vector2:
	var t := (local / _overlay.size.max(Vector2.ONE)).clamp(Vector2.ZERO, Vector2.ONE)
	return crop.position + t * crop.size


## Simule un clic à la position `local` (repère de `Overlay`) : tests headless.
func click_at(local: Vector2) -> void:
	clicked.emit(view_to_map(local))


func _draw_overlay() -> void:
	for army in _armies:
		var center := map_to_view(army["pos"])
		var color: Color = army["color"]
		if bool(army.get("player", false)):
			_overlay.draw_circle(center, 4.5, PLAYER_RING)
			_overlay.draw_circle(center, 3.0, color)
		else:
			var diamond := PackedVector2Array([center + Vector2(0, -4), center + Vector2(4, 0), center + Vector2(0, 4), center + Vector2(-4, 0)])
			_overlay.draw_colored_polygon(diamond, HudStyle.INK)
			var inner := PackedVector2Array([center + Vector2(0, -2.5), center + Vector2(2.5, 0), center + Vector2(0, 2.5), center + Vector2(-2.5, 0)])
			_overlay.draw_colored_polygon(inner, color)
	if _frame.size() >= 3:
		var outline := PackedVector2Array()
		for point in _frame:
			outline.append(map_to_view(point))
		outline.append(outline[0])
		_overlay.draw_polyline(outline, HudStyle.INK, 3.0)
		_overlay.draw_polyline(outline, FRAME_COLOR, 1.5)
	_overlay.draw_rect(Rect2(Vector2.ZERO, _overlay.size), HudStyle.INK, false, 1.5)


func _on_overlay_input(event: InputEvent) -> void:
	var pressed: bool = event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT
	var dragged: bool = event is InputEventMouseMotion and (event.button_mask & MOUSE_BUTTON_MASK_LEFT) != 0
	if pressed or dragged:
		clicked.emit(view_to_map(event.position))
		_overlay.accept_event()
