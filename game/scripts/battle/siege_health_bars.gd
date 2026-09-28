class_name SiegeHealthBars
extends CanvasLayer

## SB (TW2, ADR 0107) : barres de vie flottantes des ouvrages et engins de siège, façon Total
## War. Une petite barre enluminée (fond de vélin, filet d'encre et d'or, remplissage à la
## couleur du camp propriétaire) au-dessus de chaque pan de mur ou porte endommagé ou visé
## (`under_attack` exposé par le cœur), et au-dessus des béliers et beffrois entamés ; masquée
## quand la pièce est intacte et non visée, ou tombée (les gravats parlent d'eux-mêmes).
## Taille constante à l'écran (contrôles 2D placés par projection de la caméra). Étiquette
## « Porte : 320/540 » affichée au-dessus des pièces visées et au survol de la souris (sans
## capter les clics : le champ de bataille reste cliquable sous les barres).
## Aucune règle ici : PV, maxima et « visée » viennent de `get_siege()`.

const BAR_SIZE := Vector2(92, 11)
const CAPTION_SIZE := 13
## Hauteur au-dessus du chemin de ronde (m) des barres de murs et de porte.
const WALL_LIFT := 4.0
## Hauteur au-dessus du sol (m) des barres des engins.
const RAM_LIFT := 5.0
const TOWER_LIFT := 4.0
const DEFAULT_COLORS := {"attacker": Color(0.20, 0.33, 0.62), "defender": Color(0.62, 0.13, 0.08)}
const NAMES := {"gate": "Porte", "wall": "Muraille", "ram": "Bélier", "tower": "Beffroi"}

## Couleur des camps (`battle_scene.side_colors`), clés « attacker » / « defender ».
var side_colors: Dictionary = DEFAULT_COLORS.duplicate()
## Hauteur du chemin de ronde (m), pour placer les barres des murs.
var wall_height: float = 8.0
var height_at: Callable
## Tests : position de souris imposée (celle du headless est arbitraire).
var mouse_override: Variant = null

var _root: Control
var _bars: Dictionary = {}  # clé -> Bar
var _state: Dictionary = {}  # clé -> {shown, ratio, text, world: Vector3, side, attacked}


## Une barre : cadre d'encre et d'or sur fond de vélin, remplissage au ratio, étiquette.
class Bar:
	extends Control

	var ratio := 1.0
	var fill := Color(0.6, 0.1, 0.1)
	var caption: Label

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_IGNORE
		custom_minimum_size = SiegeHealthBars.BAR_SIZE
		size = SiegeHealthBars.BAR_SIZE
		caption = BattleUiKit.label("", SiegeHealthBars.CAPTION_SIZE, BattleUiKit.INK, false, true)
		caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
		caption.add_theme_color_override("font_outline_color", BattleUiKit.PARCHMENT_LIGHT)
		caption.add_theme_constant_override("outline_size", 5)
		caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		caption.position = Vector2(-40, -22)
		caption.size = Vector2(SiegeHealthBars.BAR_SIZE.x + 80, 20)
		caption.visible = false
		add_child(caption)

	func _draw() -> void:
		var rect := Rect2(Vector2.ZERO, size)
		# Ombre portée légère : lisible sur le ciel comme sur la pierre.
		draw_rect(Rect2(rect.position + Vector2(1, 2), rect.size), Color(0, 0, 0, 0.35))
		draw_rect(rect, BattleUiKit.PARCHMENT_DARK)
		var inner := rect.grow(-2.0)
		draw_rect(Rect2(inner.position, Vector2(inner.size.x * clampf(ratio, 0.0, 1.0), inner.size.y)), fill)
		# Quatre graduations (quarts), comme une règle de copiste.
		for q in [0.25, 0.5, 0.75]:
			var x: float = inner.position.x + inner.size.x * q
			draw_line(Vector2(x, inner.position.y), Vector2(x, inner.end.y), Color(BattleUiKit.INK, 0.35), 1.0)
		draw_rect(rect, BattleUiKit.INK, false, 1.5)
		draw_rect(rect.grow(-1.5), BattleUiKit.GOLD, false, 1.0)


func _init() -> void:
	name = "SiegeHealthBars"
	layer = 2
	_root = Control.new()
	_root.name = "Bars"
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_root)


## États des barres tirés du dictionnaire `get_siege()` (fonction pure, testée) :
## clé « piece:i » / « engine:id » -> {shown, ratio, text, world: Vector3, side, attacked}.
## `ground` : hauteur du sol en (x, z) ; `wall_h` : hauteur du chemin de ronde.
static func states(siege: Dictionary, ground: Callable, wall_h: float) -> Dictionary:
	var out := {}
	for piece in siege.get("pieces", []):
		var hp := float(piece.get("hp", 0.0))
		var max_hp := maxf(float(piece.get("max_hp", 1.0)), 1.0)
		var intact := bool(piece.get("intact", hp > 0.0))
		var attacked := bool(piece.get("under_attack", false))
		var a: Vector2 = piece.get("a", Vector2.ZERO)
		var b: Vector2 = piece.get("b", Vector2.ZERO)
		var mid := (a + b) * 0.5
		var kind := str(piece.get("kind", "wall"))
		out["piece:%d" % int(piece.get("index", 0))] = {
			"shown": intact and (attacked or hp < max_hp - 0.5),
			"ratio": clampf(hp / max_hp, 0.0, 1.0),
			"text": "%s : %d/%d" % [NAMES.get(kind, kind), roundi(hp), roundi(max_hp)],
			"world": Vector3(mid.x, _ground_of(ground, mid) + wall_h + WALL_LIFT, mid.y),
			"side": "defender",
			"attacked": attacked,
		}
	for engine in siege.get("engines", []):
		var hp := float(engine.get("hp", 0.0))
		var max_hp := maxf(float(engine.get("max_hp", 1.0)), 1.0)
		var kind := str(engine.get("kind", "ram"))
		var p := Vector2(float(engine.get("x", 0.0)), float(engine.get("z", 0.0)))
		var lift := (wall_h + TOWER_LIFT) if kind == "tower" else RAM_LIFT
		out["engine:%d" % int(engine.get("unit", 0))] = {
			"shown": hp > 0.0 and hp < max_hp - 0.01,
			"ratio": clampf(hp / max_hp, 0.0, 1.0),
			"text": "%s : %d/%d" % [NAMES.get(kind, kind), roundi(hp), roundi(max_hp)],
			"world": Vector3(p.x, _ground_of(ground, p) + lift, p.y),
			"side": str(engine.get("side", "attacker")),
			"attacked": false,
		}
	return out


static func _ground_of(ground: Callable, p: Vector2) -> float:
	return float(ground.call(p.x, p.y)) if ground.is_valid() else 0.0


## Met les barres à jour depuis `get_siege()` (appelé par `BattleSiege.update`).
func sync(siege: Dictionary) -> void:
	_state = states(siege, height_at, wall_height)
	for key in _state:
		var s: Dictionary = _state[key]
		var bar: Bar = _bars.get(key)
		if bar == null:
			if not s["shown"]:
				continue
			bar = Bar.new()
			bar.name = str(key).replace(":", "_")
			_root.add_child(bar)
			_bars[key] = bar
		var fill: Color = side_colors.get(s["side"], DEFAULT_COLORS.get(s["side"], Color.GRAY))
		if absf(bar.ratio - float(s["ratio"])) > 0.001 or bar.fill != fill:
			bar.ratio = float(s["ratio"])
			bar.fill = fill
			bar.queue_redraw()
		bar.caption.text = s["text"]
	for key in _bars:
		if not _state.has(key):
			(_bars[key] as Bar).visible = false
	_place()


## État d'une barre (tests) : {} si inconnue.
func state(key: String) -> Dictionary:
	return _state.get(key, {})


## Barre affichée à l'écran pour `key` (null si jamais créée).
func bar(key: String) -> Control:
	return _bars.get(key)


func _process(_delta: float) -> void:
	_place()


## Place chaque barre au-dessus de sa pièce (projection caméra, taille fixe à l'écran) ;
## étiquette visible pour les pièces visées et sous la souris.
func _place() -> void:
	var viewport := get_viewport()
	var camera: Camera3D = viewport.get_camera_3d() if viewport != null else null
	var screen := viewport.get_visible_rect() if viewport != null else Rect2()
	var mouse := viewport.get_mouse_position() if viewport != null else Vector2(-1e6, -1e6)
	if mouse_override != null:
		mouse = mouse_override
	for key in _bars:
		var bar_node: Bar = _bars[key]
		var s: Dictionary = _state.get(key, {})
		if s.is_empty() or not s["shown"] or camera == null:
			bar_node.visible = false
			continue
		var world: Vector3 = s["world"]
		if camera.is_position_behind(world):
			bar_node.visible = false
			continue
		var point := camera.unproject_position(world)
		if not screen.grow(BAR_SIZE.x).has_point(point):
			bar_node.visible = false
			continue
		bar_node.visible = true
		bar_node.position = (point - BAR_SIZE * 0.5).round()
		var hovered := Rect2(bar_node.position - Vector2(6, 24), BAR_SIZE + Vector2(12, 30)).has_point(mouse)
		bar_node.caption.visible = hovered or bool(s["attacked"])
