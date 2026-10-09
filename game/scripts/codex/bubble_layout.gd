class_name BubbleLayout
extends RefCounted

## Placement des bulles du Codex (IB4) : racine près de son ancre, fille à côté de sa parente sans
## chevaucher ses ancêtres (les plus anciennes se réduisent faute de place), alignement sur la ligne
## du mot-lien. Présentation pure ; lit la pile de `CodexBubbles` (`host`) mais n'y change rien
## d'autre que la réduction des ancêtres (`set_collapsed`).

const MARGIN := 8.0
const MOUSE_OFFSET := Vector2(14, 18)
## Écart entre une fille et sa parente.
const GAP := 6.0

## Zone imposée (tests) ; vide = zone visible du viewport.
var area_override := Rect2()
var host: CodexBubbles


func _init(owner_host: CodexBubbles) -> void:
	host = owner_host


## Zone de placement des bulles (`area_override` en test, sinon la zone visible).
func zone() -> Rect2:
	return area_override if area_override.has_area() else host.get_viewport().get_visible_rect()


## Place `bubble` : racine près de son ancre (`_clamp`) ; fille à côté de sa parente, sans
## chevaucher ses ancêtres — faute de place, les ancêtres les plus anciennes se réduisent.
func place(bubble: PanelContainer) -> void:
	if not is_instance_valid(bubble) or not host.bubbles.has(bubble) or bool(bubble.get_meta("collapsed", false)):
		return  # une ancêtre réduite reste à sa place
	var parent := host.parent_of(bubble)
	if parent == null:
		clamp_to_area(bubble)
		return
	var area := zone().grow(-MARGIN)
	var anchor_y := parent.position.y + float(bubble.get_meta("anchor_dy", 0.0))
	var pos := place_beside(bubble.size, parent.get_rect(), anchor_y, area, ancestor_rects(bubble))
	while is_nan(pos.x):
		var victim := oldest_open_ancestor(bubble)
		if victim == null:
			break
		host.set_collapsed(victim, true)
		if not is_instance_valid(bubble) or not host.bubbles.has(bubble):
			return
		pos = place_beside(bubble.size, parent.get_rect(), anchor_y, area, ancestor_rects(bubble))
	if is_nan(pos.x):
		# Aucune place libre même réduites : à droite de la parente, gardée à l'écran.
		pos = Vector2(parent.get_rect().end.x + GAP, anchor_y)
		pos.x = clampf(pos.x, area.position.x, maxf(area.position.x, area.end.x - bubble.size.x))
		pos.y = clampf(pos.y, area.position.y, maxf(area.position.y, area.end.y - bubble.size.y))
	bubble.position = pos.floor()


## Garde la bulle à l'écran : à gauche de l'ancre si elle déborde à droite, remontée sinon.
func clamp_to_area(bubble: PanelContainer) -> void:
	if not is_instance_valid(bubble):
		return
	var rect := zone()
	var low := rect.position + Vector2(MARGIN, MARGIN)
	var high := rect.end - Vector2(MARGIN, MARGIN)
	var at: Vector2 = bubble.get_meta("anchor", bubble.position)
	var pos := at
	if pos.x + bubble.size.x > high.x:
		pos.x = at.x - bubble.size.x - MOUSE_OFFSET.x * 2.0
	pos.x = clampf(pos.x, low.x, maxf(low.x, high.x - bubble.size.x))
	pos.y = clampf(pos.y, low.y, maxf(low.y, high.y - bubble.size.y))
	bubble.position = pos.floor()


## Position d'une bulle de taille `size` à côté de `parent` : à droite, sinon à gauche (haut
## aligné sur `anchor_y`, glissée vers le bas sous une ancêtre gênante), sinon dessous ; dans
## `area` et hors des rectangles `avoid` (ancêtres). (NAN, NAN) si aucune place.
static func place_beside(size: Vector2, parent: Rect2, anchor_y: float, area: Rect2, avoid: Array, gap: float = GAP) -> Vector2:
	var top := clampf(anchor_y, area.position.y, maxf(area.position.y, area.end.y - size.y))
	for x: float in [parent.end.x + gap, parent.position.x - gap - size.x]:
		if x < area.position.x or x + size.x > area.end.x:
			continue
		var y := free_y(x, top, size, area, avoid, gap)
		if not is_nan(y):
			return Vector2(x, y)
	var below_x := clampf(parent.position.x, area.position.x, maxf(area.position.x, area.end.x - size.x))
	var below_y := free_y(below_x, parent.end.y + gap, size, area, avoid, gap)
	if not is_nan(below_y):
		return Vector2(below_x, below_y)
	return Vector2(NAN, NAN)


## Première ordonnée ≥ `y` où le rectangle (`x`, `size`) tient dans `area` sans toucher `avoid`.
static func free_y(x: float, y: float, size: Vector2, area: Rect2, avoid: Array, gap: float) -> float:
	for _attempt in avoid.size() + 1:
		if y + size.y > area.end.y:
			return NAN
		var rect := Rect2(Vector2(x, y), size)
		var blocker: Variant = null
		for other: Rect2 in avoid:
			if rect.intersects(other):
				blocker = other
				break
		if blocker == null:
			return y
		y = (blocker as Rect2).end.y + gap
	return NAN


func ancestor_rects(bubble: PanelContainer) -> Array:
	var rects: Array = []
	for ancestor in host.ancestors_of(bubble):
		rects.append(ancestor.get_rect())
	return rects


## Ancêtre non réduite la plus ancienne de `bubble`, hors sa parente directe (null si aucune).
func oldest_open_ancestor(bubble: PanelContainer) -> PanelContainer:
	var parent := host.parent_of(bubble)
	for ancestor in host.ancestors_of(bubble):
		if ancestor != parent and not bool(ancestor.get_meta("collapsed", false)):
			return ancestor
	return null


## Décalage vertical (depuis le haut de `parent`) de la ligne du mot-lien `meta` dans `source` ;
## repli : la souris si elle est sur la parente, sinon 0 (haut de la parente).
func keyword_offset(source: Control, meta: String, parent: PanelContainer) -> float:
	var label := source as RichTextLabel
	if label == null or not is_instance_valid(label) or not parent.is_ancestor_of(label):
		return 0.0
	var y := keyword_y(label, meta)
	if is_nan(y):
		var mouse := host.get_viewport().get_mouse_position()
		if not parent.get_global_rect().has_point(mouse):
			return 0.0
		y = mouse.y - float(UiType.size(UiType.BODY))
	return maxf(0.0, y - parent.global_position.y)


## Ordonnée globale du haut de la ligne du mot-lien `meta` dans `label`, NAN si introuvable.
static func keyword_y(label: RichTextLabel, meta: String) -> float:
	var bbcode := str(label.get_meta("base_text", label.text))
	var open_tag := "[url=%s]" % meta
	var start := bbcode.find(open_tag)
	if start < 0:
		return NAN
	var inner := start + open_tag.length()
	var end := bbcode.find("[/url]", inner)
	if end < 0:
		return NAN
	var word := RegEx.create_from_string("\\[[^\\]]*\\]").sub(bbcode.substr(inner, end - inner), "", true)
	var index := label.get_parsed_text().find(word) if word != "" else -1
	if index < 0:
		return NAN
	var line := label.get_character_line(index)
	if line < 0:
		return NAN
	return label.get_global_rect().position.y + label.get_line_offset(line)
