class_name RegionLabels
extends Control

## Lot TB2 : noms de région discrets en vue moyenne (ils n'existaient qu'en vue stratégique). Même
## source que le parchemin (`ParchmentOverlay.province_names`, ADR 0124) : nom court et centroïde de
## chaque province, les plus vastes d'abord. Italique espacée, pâle, sans écu ni cadre ; un nom ne
## s'affiche que s'il tient entier dans l'écran et ne touche ni un nom de ville ni un autre nom de
## région. S'efface de près (les noms de lieux suffisent) et quand le parchemin prend le relais.
## Réglages : bloc `region_labels` de `data/ui/campaign_map.json`. Purement visuel.

const FONT := preload("res://assets/third_party/fonts/im_fell_english/IMFeENit28P.ttf")
## Redessin au plus à ce rythme quand la caméra ne bouge pas (les noms de ville changent).
const REFRESH_INTERVAL_MS := 200

var camera: Camera3D
## Source des noms : la couche du parchemin.
var source: ParchmentOverlay
## Rectangles écran à éviter (noms de ville affichés) : `func(camera) -> Array[Rect2]`.
var obstacles: Callable = Callable()
## Bord haut occupé par le HUD (px) : `func() -> float`.
var top_inset: Callable = Callable()
## Opacité courante [0, 1] (distance caméra × fondu du parchemin), exposée pour les tests.
var weight: float = 0.0
## Rectangles et noms dessinés à la dernière passe (tests et mesures).
var placed_rects: Array[Rect2] = []
var placed_names: PackedStringArray = PackedStringArray()

var _font: FontVariation
var _last_view: Array = []
var _last_draw_ms := -REFRESH_INTERVAL_MS


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_font = FontVariation.new()
	_font.base_font = FONT
	_font.spacing_glyph = int(MapReadability.number("region_labels", "letter_spacing_px", 2.0))


## Opacité des noms de région à la distance caméra `distance` (0 hors de la vue moyenne).
static func weight_at(distance: float) -> float:
	var fade := maxf(MapReadability.number("region_labels", "fade", 60.0), 1.0)
	var near := MapReadability.number("region_labels", "min_distance", 180.0)
	var far := MapReadability.number("region_labels", "max_distance", 1250.0)
	return smoothstep(near, near + fade, distance) * (1.0 - smoothstep(far - fade, far, distance))


## Chaque image (`StrategicView.update_view`) : distance caméra et poids du parchemin.
func set_view(distance: float, parchment: float) -> void:
	weight = weight_at(distance) * (1.0 - smoothstep(0.3, 0.7, parchment))
	visible = weight > 0.01
	if not visible:
		placed_rects.clear()
		placed_names.clear()


func _process(_delta: float) -> void:
	if not visible or camera == null:
		return
	var view := [camera.global_transform, camera.fov, get_viewport_rect().size, snappedf(weight, 0.02)]
	var now := Time.get_ticks_msec()
	if view != _last_view or now - _last_draw_ms >= REFRESH_INTERVAL_MS:
		_last_view = view
		_last_draw_ms = now
		queue_redraw()


## Rectangle de l'écran où un nom peut s'écrire en entier (marge de bord, HUD du haut exclu).
func safe_rect() -> Rect2:
	var margin := MapReadability.number("region_labels", "edge_margin_px", 12.0)
	var top := float(top_inset.call()) if top_inset.is_valid() else 0.0
	return get_viewport_rect().grow_individual(-margin, -margin - top, -margin, -margin)


## Noms retenus pour l'écran courant : [{name, rect}], sans chevauchement, entiers dans l'écran.
func layout() -> Array[Dictionary]:
	var result: Array[Dictionary] = []
	if camera == null or source == null:
		return result
	var size := int(MapReadability.number("region_labels", "font_size_px", 17.0))
	var limit := int(MapReadability.number("region_labels", "max_on_screen", 14.0))
	var safe := safe_rect()
	var taken: Array[Rect2] = []
	if obstacles.is_valid():
		for rect: Rect2 in obstacles.call(camera):
			taken.append(rect.grow(4.0))
	for province: Dictionary in source.province_names():
		if result.size() >= limit:
			break
		var world: Vector3 = province["world"]
		if camera.is_position_behind(world):
			continue
		var p := camera.unproject_position(world)
		if not safe.has_point(p):
			continue
		var text: String = province["name"]
		var extent := _font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size)
		var rect := Rect2(p - extent * 0.5, extent)
		if not safe.encloses(rect):
			continue  # jamais de nom coupé en bord d'écran
		var free := true
		for other in taken:
			if other.intersects(rect):
				free = false
				break
		if not free:
			continue
		taken.append(rect.grow(6.0))
		result.append({"name": text, "rect": rect})
	return result


func _draw() -> void:
	placed_rects.clear()
	placed_names.clear()
	if weight <= 0.01:
		return
	var size := int(MapReadability.number("region_labels", "font_size_px", 17.0))
	var ink := MapReadability.color("region_labels", "color", Color(0.18, 0.13, 0.08))
	var halo := MapReadability.color("region_labels", "halo_color", Color(0.94, 0.89, 0.77))
	ink.a = MapReadability.number("region_labels", "alpha", 0.5) * weight
	halo.a = MapReadability.number("region_labels", "halo_alpha", 0.3) * weight
	for entry in layout():
		var rect: Rect2 = entry["rect"]
		var origin := rect.position + Vector2(0.0, _font.get_ascent(size))
		draw_string_outline(_font, origin, entry["name"], HORIZONTAL_ALIGNMENT_LEFT, -1, size, 3, halo)
		draw_string(_font, origin, entry["name"], HORIZONTAL_ALIGNMENT_LEFT, -1, size, ink)
		placed_rects.append(rect)
		placed_names.append(str(entry["name"]))
