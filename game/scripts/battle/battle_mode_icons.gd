class_name BattleModeIcons
extends RefCounted

## CB2 : pictogrammes des modes d'unité (course, garde, escarmouche, mêlée, battre en brèche) et
## des nouveaux états (hésite, sous le feu) : icônes à l'encre DA5 (lot CB, clés `battle_mode_<mode>`
## et `battle_state_<état>` via `HudStyle.icon`), glyphes dessinés en code en repli si le PNG manque.
## Aucune règle : les modes et états viennent de `BattleSim.get_units()`.

## Modes dans l'ordre des boutons (clé de `set_mode`, champ de `get_units`, libellé, infobulle).
const MODES := [
	{"mode": "run", "field": "mode_run", "label": "Course", "tip": "Tous les déplacements au pas de course (le double clic droit fait courir un seul ordre)."},
	{"mode": "guard", "field": "guard", "label": "Garde", "tip": "Tenir sa position : pas de poursuite d'un ennemi en fuite ni d'un adversaire qui rompt le contact."},
	{"mode": "skirmish", "field": "skirmish", "label": "Escarmouche", "tip": "Les tireurs reculent de {rule.unit_modes_skirmish_retreat_m} m devant une troupe de mêlée qui approche à moins de {rule.unit_modes_skirmish_trigger_m} m."},
	{"mode": "melee", "field": "melee_mode", "label": "Mêlée", "tip": "Les tireurs rangent leurs armes de trait et engagent au corps à corps."},
	{"mode": "breach", "field": "breach", "label": "Battre en brèche", "tip": "Engins de siège : tir concentré sur les murs et les portes (+{rule.unit_modes_breach_damage_percent} % de dégâts, mangonneau +{rule.unit_modes_breach_mangonel_percent} %, recharge +{rule.unit_modes_breach_reload_percent} %), jamais sur les hommes."},
]

const INK := Color(0.22, 0.14, 0.07)
## Côté (px) d'une icône DA5 dans une pastille d'état de 13 px.
const BADGE_ICON_SIDE := 10.0


## Champ de `get_units` qui dit si `mode` est actif.
static func field_of(mode: String) -> String:
	for entry in MODES:
		if str(entry["mode"]) == mode:
			return str(entry["field"])
	return ""


## Libellé français d'un mode.
static func label_of(mode: String) -> String:
	for entry in MODES:
		if str(entry["mode"]) == mode:
			return str(entry["label"])
	return mode


## Infobulle d'un mode, chiffres lus dans les données par RuleValues.
static func tip_of(mode: String) -> String:
	for entry in MODES:
		if str(entry["mode"]) == mode:
			return RuleValues.format(str(entry["tip"]))
	return ""


## Modes actifs d'une unité (clés, dans l'ordre de `MODES`).
static func active_modes(unit: Dictionary) -> Array[String]:
	var out: Array[String] = []
	for entry in MODES:
		if bool(unit.get(str(entry["field"]), false)):
			out.append(str(entry["mode"]))
	return out


## Icône à l'encre DA5 (lot CB) de la clé `key` (`battle_mode_run`, `battle_state_rout`…) posée
## dans un carré de côté `side` centré en `c`, l'encre changée en `ink` ; faux si l'icône manque
## (le glyphe dessiné en code sert alors de repli : le jeu tourne sans les PNG).
static func draw_ink_icon(canvas: CanvasItem, key: String, c: Vector2, side: float, ink: Color = INK) -> bool:
	var texture := HudStyle.icon(key)
	if texture == null:
		return false
	var modulate := Color(ink.r / INK.r, ink.g / INK.g, ink.b / INK.b, ink.a)
	canvas.draw_texture_rect(texture, Rect2(c - Vector2(side, side) * 0.5, Vector2(side, side)), false, modulate)
	return true


## Pictogramme du mode `mode` centré en `c`, dans un carré d'environ 16 px × `k` : icône DA5
## `battle_mode_<mode>` si elle existe, sinon glyphe dessiné.
static func draw_mode(canvas: CanvasItem, mode: String, c: Vector2, k: float, ink: Color = INK) -> void:
	if draw_ink_icon(canvas, "battle_mode_" + mode, c, 16.0 * k, ink):
		return
	match mode:
		"run":  # flèche en avant et traits de vitesse
			canvas.draw_line(c + Vector2(-2, 0) * k, c + Vector2(6, 0) * k, ink, 1.8 * k)
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(7.5, 0) * k, c + Vector2(3, -4) * k, c + Vector2(3, 4) * k]), ink)
			for i in 3:
				var y := -4.0 + i * 4.0
				var start := -8.0 if i == 1 else -6.5
				canvas.draw_line(c + Vector2(start, y) * k, c + Vector2(-3.5, y) * k, ink, 1.2 * k)
		"guard":  # écu
			var shield := PackedVector2Array([c + Vector2(-6, -6.5) * k, c + Vector2(6, -6.5) * k, c + Vector2(6, 0) * k, c + Vector2(0, 7) * k, c + Vector2(-6, 0) * k])
			canvas.draw_colored_polygon(shield, ink)
			canvas.draw_line(c + Vector2(0, -5) * k, c + Vector2(0, 4.5) * k, Color(1, 0.85, 0.35), 1.2 * k)
			canvas.draw_line(c + Vector2(-4.5, -2) * k, c + Vector2(4.5, -2) * k, Color(1, 0.85, 0.35), 1.2 * k)
		"skirmish":  # double flèche (reculer, revenir)
			canvas.draw_line(c + Vector2(-5, 0) * k, c + Vector2(5, 0) * k, ink, 1.6 * k)
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(-7.5, 0) * k, c + Vector2(-3.5, -3.5) * k, c + Vector2(-3.5, 3.5) * k]), ink)
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(7.5, 0) * k, c + Vector2(3.5, -3.5) * k, c + Vector2(3.5, 3.5) * k]), ink)
			canvas.draw_arc(c + Vector2(0, -2) * k, 4.5 * k, PI * 1.15, PI * 1.85, 8, ink, 1.0 * k)
		"melee":  # épée droite, pointe en haut
			canvas.draw_line(c + Vector2(0, -7) * k, c + Vector2(0, 3) * k, ink, 2.0 * k)
			canvas.draw_line(c + Vector2(-4, 3) * k, c + Vector2(4, 3) * k, ink, 1.8 * k)
			canvas.draw_line(c + Vector2(0, 3) * k, c + Vector2(0, 6) * k, ink, 1.4 * k)
			canvas.draw_circle(c + Vector2(0, 7) * k, 1.3 * k, ink)
		"breach":  # pan de mur fendu
			canvas.draw_rect(Rect2(c + Vector2(-7, -5) * k, Vector2(14, 11) * k), ink)
			var mortar := Color(0.95, 0.9, 0.78)
			canvas.draw_line(c + Vector2(-7, 0.5) * k, c + Vector2(7, 0.5) * k, mortar, 0.8 * k)
			canvas.draw_line(c + Vector2(-2, -5) * k, c + Vector2(-2, 0.5) * k, mortar, 0.8 * k)
			canvas.draw_line(c + Vector2(3, 0.5) * k, c + Vector2(3, 6) * k, mortar, 0.8 * k)
			canvas.draw_polyline(PackedVector2Array([c + Vector2(1, -6) * k, c + Vector2(-1, -2) * k, c + Vector2(2, 1) * k, c + Vector2(-0.5, 4) * k, c + Vector2(1.5, 7) * k]), Color(0.85, 0.2, 0.12), 1.6 * k)


## Pictogramme d'un état (pastille) : `wavering` (hésite) ou `under_fire` (sous le feu).
static func draw_state(canvas: CanvasItem, kind: String, c: Vector2, ink: Color) -> void:
	if draw_ink_icon(canvas, "battle_state_" + kind, c, BADGE_ICON_SIDE, ink):
		return
	match kind:
		"wavering":  # point d'exclamation
			canvas.draw_rect(Rect2(c + Vector2(-1, -4.5), Vector2(2, 6)), ink)
			canvas.draw_circle(c + Vector2(0, 3.5), 1.1, ink)
		"under_fire":  # trois traits qui tombent
			for dx in [-3.0, 0.0, 3.0]:
				canvas.draw_line(c + Vector2(dx - 1.5, -4), c + Vector2(dx + 1.0, 2.5), ink, 1.1)
				canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(dx + 1.6, 4), c + Vector2(dx - 0.3, 2.0), c + Vector2(dx + 2.1, 1.4)]), ink)
