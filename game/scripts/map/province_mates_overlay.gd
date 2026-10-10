class_name ProvinceMatesOverlay
extends Node3D

## CO-B : à la sélection d'une colonie, toutes les colonies de sa province apparaissent en
## pastilles (sceau parchemin, glyphe du type) reliées à la cité de province par un trait
## d'encre pointillé posé sur le relief ; la colonie choisie est mise en avant, la cité porte
## un anneau rubrique. Cartouche « Province de X — n colonies ». Rendu seulement, aucune règle.
## Vue 3D proche/normale uniquement : le parchemin lointain (ADR 0124) ne montre pas ce calque,
## seul le cartouche (2D) reste lisible dans les deux vues.

const PASTILLE_PX := 56
const PIXEL_SIZE := 0.0009  # Taille écran constante (Sprite3D.fixed_size)
const LIFT := 6.0  # Hauteur des pastilles au-dessus du sol (m de carte)
const DOT_STEP := 3.0  # Pas du pointillé (m de carte)
const DOT_LEN := 1.2

var layer: SettlementLayer = null
var terrain: Node = null
var name_of_province: Callable = Callable()  # province id → nom affiché
var mates: Array[Dictionary] = []  # colonies affichées (vide = calque masqué)
var _items: Node3D = null
var _ink_line: MeshInstance3D = null
var _cartouche: PanelContainer = null
var _cartouche_label: Label = null
static var _textures: Dictionary = {}


func setup(settlement_layer: SettlementLayer, terrain_builder: Node, ui_parent: Node, province_namer: Callable) -> void:
	layer = settlement_layer
	terrain = terrain_builder
	name_of_province = province_namer
	name = "ProvinceMates"
	_items = Node3D.new()
	add_child(_items)
	_ink_line = MeshInstance3D.new()
	_ink_line.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_ink_line)
	var canvas := CanvasLayer.new()
	canvas.layer = 5
	ui_parent.add_child(canvas)
	_cartouche = PanelContainer.new()
	_cartouche.add_theme_stylebox_override("panel", HudStyle.note_box(8))
	_cartouche.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_cartouche.set_anchors_preset(Control.PRESET_CENTER_TOP)
	_cartouche.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_cartouche.position.y = 56
	_cartouche_label = HudStyle.label("", HudStyle.FONT_BODY)
	_cartouche.add_child(_cartouche_label)
	_cartouche.hide()
	canvas.add_child(_cartouche)
	layer.settlement_selected.connect(show_for)
	layer.selection_cleared.connect(clear)


## Colonies de la province de `settlement_id` (cité d'abord) — pure donnée, testable.
static func mates_of(data: SettlementData, settlement_id: String) -> Array[Dictionary]:
	return data.province_mates(settlement_id)


## Libellé du cartouche.
static func cartouche_text(province_name: String, count: int) -> String:
	return "Province de %s — %d colonie%s" % [province_name, count, "s" if count > 1 else ""]


func show_for(settlement_id: String) -> void:
	clear()
	if layer == null or layer.data == null:
		return
	mates = mates_of(layer.data, settlement_id)
	if mates.size() < 2:
		mates = []
		return
	var city: Dictionary = mates[0]
	for entry in mates:
		if str(entry["kind"]) == "city":
			city = entry
			break
	var hub := _ground(city["px"], 0.0)
	var dots := ImmediateMesh.new()
	dots.surface_begin(Mesh.PRIMITIVE_LINES)
	for entry in mates:
		var is_city := entry == city
		var is_selected := str(entry["id"]) == settlement_id
		var sprite := Sprite3D.new()
		sprite.texture = _texture(str(entry["kind"]), is_city, is_selected)
		sprite.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		sprite.fixed_size = true
		sprite.pixel_size = PIXEL_SIZE * (1.45 if is_selected else (1.2 if is_city else 1.0))
		sprite.no_depth_test = true
		sprite.shaded = false
		sprite.render_priority = 5
		sprite.position = _ground(entry["px"], LIFT)
		_items.add_child(sprite)
		if not is_city:
			_dotted(dots, hub, _ground(entry["px"], 0.0))
	dots.surface_end()
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = HudStyle.INK
	material.no_depth_test = true
	material.render_priority = 4
	if dots.get_surface_count() > 0:
		dots.surface_set_material(0, material)
	_ink_line.mesh = dots
	var province := str(city["province"])
	var title: String = str(name_of_province.call(province)) if name_of_province.is_valid() else province
	_cartouche_label.text = cartouche_text(title, mates.size())
	_cartouche.show()


func clear() -> void:
	mates = []
	if _items != null:
		for child in _items.get_children():
			child.queue_free()
	if _ink_line != null:
		_ink_line.mesh = null
	if _cartouche != null:
		_cartouche.hide()


func _ground(px: Vector2, lift: float) -> Vector3:
	var y := float(terrain.surface_height_at(px.x, px.y)) if terrain != null else 0.0
	return Vector3(px.x, y + lift, px.y)


func _dotted(mesh: ImmediateMesh, from: Vector3, to: Vector3) -> void:
	var length := Vector2(from.x - to.x, from.z - to.z).length()
	var steps := maxi(int(length / DOT_STEP), 1)
	for i in steps:
		var a := float(i) / steps
		var b := minf(a + DOT_LEN / maxf(length, 0.001), 1.0)
		for t in [a, b]:
			var p: Vector3 = from.lerp(to, t)
			p.y = _ground(Vector2(p.x, p.z), 0.4).y
			mesh.surface_add_vertex(p)


## Sceau parchemin : disque, liseré d'encre (rubrique pour la cité, or pour la sélection) et
## glyphe simple du type de colonie. Mémorisé par combinaison.
static func _texture(kind: String, is_city: bool, is_selected: bool) -> Texture2D:
	var key := "%s|%s|%s" % [kind, is_city, is_selected]
	if _textures.has(key):
		return _textures[key]
	var n := PASTILLE_PX
	var image := Image.create(n, n, false, Image.FORMAT_RGBA8)
	var ring: Color = HudStyle.GOLD if is_selected else (HudStyle.RUBRIC if is_city else HudStyle.INK_SOFT)
	var c := Vector2(n, n) * 0.5 - Vector2(0.5, 0.5)
	var radius := n * 0.5 - 1.0
	var ring_w := 5.0 if (is_selected or is_city) else 3.0
	for y in n:
		for x in n:
			var d := Vector2(x, y).distance_to(c)
			var color := Color(0, 0, 0, 0)
			if d <= radius - ring_w:
				color = HudStyle.PARCHMENT_LIGHT
			elif d <= radius:
				color = ring
			image.set_pixel(x, y, color)
	var ink: Color = HudStyle.INK
	match kind:
		"city":
			_rect(image, 0.30, 0.45, 0.70, 0.72, ink)
			_rect(image, 0.30, 0.32, 0.38, 0.45, ink)
			_rect(image, 0.46, 0.32, 0.54, 0.45, ink)
			_rect(image, 0.62, 0.32, 0.70, 0.45, ink)
		"town":
			_rect(image, 0.32, 0.50, 0.68, 0.70, ink)
			_rect(image, 0.42, 0.34, 0.58, 0.50, ink)
		"castle":
			_rect(image, 0.36, 0.36, 0.64, 0.72, ink)
			_rect(image, 0.30, 0.28, 0.40, 0.40, ink)
			_rect(image, 0.60, 0.28, 0.70, 0.40, ink)
		"abbey":
			_rect(image, 0.46, 0.28, 0.54, 0.72, ink)
			_rect(image, 0.34, 0.40, 0.66, 0.48, ink)
		_:
			_rect(image, 0.38, 0.50, 0.62, 0.68, ink)
			_rect(image, 0.44, 0.38, 0.56, 0.50, ink)
	var texture := ImageTexture.create_from_image(image)
	_textures[key] = texture
	return texture


static func _rect(image: Image, x0: float, y0: float, x1: float, y1: float, color: Color) -> void:
	var n := float(image.get_width())
	image.fill_rect(Rect2i(int(x0 * n), int(y0 * n), int((x1 - x0) * n), int((y1 - y0) * n)), color)
