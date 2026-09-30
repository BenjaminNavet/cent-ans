class_name RiverLabels
extends Node3D

## Noms des cours d'eau de la carte de campagne (chantier RC, ADR 0117), rendu seulement.
##
## Petites étiquettes en italique à l'encre bleue, couchées sur le relief dans le sens du courant
## (lecture cartographique), posées à intervalles réguliers le long des tronçons nommés de
## `RiversRenderer.rivers`. Noms français de `river_names.json` (nom source → nom affiché ; valeur
## vide = pas d'étiquette). Chaque étiquette disparaît au-delà d'une distance caméra qui croît avec
## l'importance du cours d'eau (`visibility_range_end`, sans coût par image) : au dézoom seuls les
## grands fleuves sont nommés, les affluents apparaissent en se rapprochant.
## Réglages : `river_display.json` → `labels`.

const FONT_PATH := "res://assets/third_party/fonts/eb_garamond/EBGaramond-Italic-VariableFont_wght.ttf"
## Demi-fenêtre (px carte) pour lisser la direction du texte le long du cours.
const TANGENT_WINDOW := 6.0
## Deux étiquettes du même nom plus proches que cette part de l'espacement sont fusionnées.
const SAME_NAME_CLEARANCE := 0.6

var _labels: Array[Label3D] = []


func count() -> int:
	return _labels.size()


## Étiquettes posées : [{text, px: Vector2, importance}] (tests).
func placed() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for label in _labels:
		out.append({"text": label.text, "px": Vector2(label.position.x, label.position.z), "importance": int(label.get_meta("importance", 0))})
	return out


func build(renderer: RiversRenderer, cfg: Dictionary, names: Dictionary) -> void:
	for label in _labels:
		label.queue_free()
	_labels.clear()
	var map_data := renderer.map_data
	var extent := maxf(map_data.size.x, map_data.size.y)
	var spacing: Array = cfg.get("spacing_px", [])
	var max_dist: Array = cfg.get("max_distance_by_importance", [])
	var min_importance := int(cfg.get("min_importance", 0))
	var min_len := float(cfg.get("min_segment_px", 40.0))
	var clear_px := float(cfg.get("clear_px_from_settlements", 4.0))
	var max_labels := int(cfg.get("max_labels", 900))
	var font := _font(int(cfg.get("weight", 500)), int(cfg.get("letter_spacing", 1)))
	# Les tronçons les plus importants et les plus longs d'abord : ils gagnent la place.
	var order: Array = range(renderer.rivers.size())
	var lengths := {}
	for ri in order:
		lengths[ri] = _length(renderer.rivers[ri]["points"])
	order.sort_custom(func(a: int, b: int) -> bool:
		var ia := int(renderer.rivers[a]["importance"])
		var ib := int(renderer.rivers[b]["importance"])
		return ia > ib if ia != ib else float(lengths[a]) > float(lengths[b]))
	var taken := {}  # nom affiché → Array[Vector2]
	for ri in order:
		if _labels.size() >= max_labels:
			break
		var river: Dictionary = renderer.rivers[ri]
		var importance := clampi(int(river["importance"]), 0, 6)
		var source := str(river["name"])
		var text := str(names.get(source, source)).strip_edges()
		if text == "" or importance < min_importance or float(lengths[ri]) < min_len:
			continue
		var step := float(spacing[importance]) if importance < spacing.size() else 0.0
		var points: PackedVector2Array = river["points"]
		var total := float(lengths[ri])
		# Au moins une étiquette au milieu ; sinon une tous les `step` px, centrées sur le tronçon.
		var n := 1 if step <= 0.0 else maxi(1, int(floor(total / step)))
		var gap := total / float(n)
		var visible_to := (float(max_dist[importance]) if importance < max_dist.size() else 0.1) * extent
		var clearance := maxf(step, gap) * SAME_NAME_CLEARANCE
		for k in n:
			var at := gap * (float(k) + 0.5)
			var sample := _sample(points, at)
			var px: Vector2 = sample[0]
			if renderer.in_custom_zone(px, clear_px) or _near_cover(renderer, px, clear_px):
				continue
			var previous: Array = taken.get(text, [])
			var crowded := false
			for q: Vector2 in previous:
				if q.distance_to(px) < clearance:
					crowded = true
					break
			if crowded:
				continue
			previous.append(px)
			taken[text] = previous
			var dir := _tangent(points, at, total)
			_labels.append(_make_label(map_data, cfg, font, text, px, dir, importance, visible_to))
			if _labels.size() >= max_labels:
				break


func _make_label(map_data: MapData, cfg: Dictionary, font: Font, text: String, px: Vector2, dir: Vector2, importance: int, visible_to: float) -> Label3D:
	var label := Label3D.new()
	label.name = "River_%d" % _labels.size()
	label.text = text
	label.set_meta("importance", importance)
	if font != null:
		label.font = font
	label.font_size = int(cfg.get("font_size", 13))
	label.outline_size = int(cfg.get("outline_px", 4))
	label.modulate = _color(cfg.get("color", []), Color(0.12, 0.27, 0.42), 1.0)
	label.outline_modulate = _color(cfg.get("outline_color", []), Color(0.93, 0.89, 0.78), float(cfg.get("outline_alpha", 0.55)))
	label.fixed_size = true
	label.pixel_size = float(cfg.get("pixel_size", 0.0011))
	label.no_depth_test = true
	label.double_sided = true
	label.render_priority = 3  # sous les noms de colonies (4)
	label.outline_render_priority = 2
	label.visibility_range_end = visible_to
	label.visibility_range_end_margin = visible_to * 0.15
	label.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
	label.position = Vector3(px.x, map_data.surface_world_at(px.x, px.y), px.y)
	if bool(cfg.get("flat", true)):
		label.billboard = BaseMaterial3D.BILLBOARD_DISABLED
		# Texte couché sur le sol, lu de gauche à droite (jamais à l'envers, carte nord en haut).
		if dir.x < 0.0:
			dir = -dir
		label.rotation = Vector3(-PI / 2.0, -atan2(dir.y, dir.x), 0.0)
		label.rotation_order = EULER_ORDER_YXZ
	else:
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	add_child(label)
	return label


static func _near_cover(renderer: RiversRenderer, px: Vector2, margin: float) -> bool:
	for cover in renderer.covers:
		if px.distance_to(Vector2(cover.x, cover.y)) < cover.z + margin:
			return true
	return false


static func _length(points: PackedVector2Array) -> float:
	var total := 0.0
	for i in range(1, points.size()):
		total += points[i].distance_to(points[i - 1])
	return total


## [point, index du segment] à l'abscisse curviligne `at`.
static func _sample(points: PackedVector2Array, at: float) -> Array:
	var run := 0.0
	for i in range(1, points.size()):
		var seg := points[i].distance_to(points[i - 1])
		if run + seg >= at and seg > 0.0:
			return [points[i - 1].lerp(points[i], (at - run) / seg), i]
		run += seg
	return [points[points.size() - 1], points.size() - 1]


## Direction lissée du cours autour de `at` (corde sur ± TANGENT_WINDOW px).
static func _tangent(points: PackedVector2Array, at: float, total: float) -> Vector2:
	var a: Vector2 = _sample(points, maxf(at - TANGENT_WINDOW, 0.0))[0]
	var b: Vector2 = _sample(points, minf(at + TANGENT_WINDOW, total))[0]
	var dir := (b - a).normalized()
	return dir if dir != Vector2.ZERO else Vector2.RIGHT


static func _color(rgb: Array, fallback: Color, alpha: float) -> Color:
	var c := fallback if rgb.size() != 3 else Color(float(rgb[0]), float(rgb[1]), float(rgb[2]))
	c.a = alpha
	return c


static func _font(weight: int, spacing: int) -> Font:
	if not ResourceLoader.exists(FONT_PATH):
		return null
	var variation := FontVariation.new()
	variation.base_font = load(FONT_PATH) as Font
	variation.variation_opentype = {TextServerManager.get_primary_interface().name_to_tag("wght"): weight}
	variation.spacing_glyph = spacing
	return variation
