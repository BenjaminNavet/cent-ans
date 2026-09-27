class_name BattleAbilityIcons
extends RefCounted

## CB4 : capacités actives des régiments sur les cartes d'unité (tir tendu, pavois, ralliement à la
## bannière, rangs serrés, piques plantées). Pictogrammes dessinés en code (glyphes provisoires :
## la session principale générera les icônes DA5 ensuite, comme pour les modes CB2), cadran de
## recharge, infobulle chiffrée par RuleValues (`<id>_cooldown`, `<id>_<effet>_percent`…).
## Aucune règle : l'état de chaque capacité vient de `BattleSim.get_units()` (`abilities`), les
## textes de `BattleSim.get_ability_catalog()`, l'emploi passe par `{type: "use_ability"}`.

const INK := Color(0.22, 0.14, 0.07)
const GOLD := Color(0.95, 0.75, 0.15)
const PAPER := Color(0.95, 0.9, 0.78, 0.95)

## Effets cités par l'infobulle : suffixe de la valeur RuleValues, libellé, unité (« % » : écart
## signé en pour cent ; « » : nombre brut).
const EFFECTS := [
	["range_percent", "portée", "%"],
	["accuracy_percent", "précision", "%"],
	["vs_mounted_percent", "pertes infligées aux cavaliers", "%"],
	["reload_percent", "temps de rechargement", "%"],
	["missile_taken_percent", "pertes par le trait", "%"],
	["speed_percent", "vitesse", "%"],
	["melee_taken_front_percent", "pertes en mêlée de face", "%"],
	["melee_taken_flank_percent", "pertes de flanc et de dos", "%"],
	["charge_taken_percent", "hommes renversés par une charge", "%"],
	["horse_taken_front_percent", "pertes face aux cavaliers de front", "%"],
	["vs_horse_front_percent", "pertes infligées aux cavaliers de front", "%"],
	["melee_fatigue_percent", "fatigue en mêlée", "%"],
	["morale_bonus", "moral regagné d'un coup", ""],
	["morale_recovery", "moral regagné par seconde hors contact", ""],
]


## Numéro (1…4) de l'emplacement de la capacité `ability_id` dans la liste `abilities` d'une unité
## (0 si absente) : Alt+n.
static func slot_of(abilities: Array, ability_id: String) -> int:
	for i in abilities.size():
		if str((abilities[i] as Dictionary).get("id", "")) == ability_id:
			return i + 1
	return 0


## Infobulle d'une capacité : nom, raccourci, texte historique, recharge et durée, effets chiffrés
## (RuleValues), état (active, recharge, raison d'indisponibilité ou d'arrêt).
static func tip(entry: Dictionary, state: Dictionary, slot: int) -> String:
	var id := str(entry.get("id", state.get("id", "")))
	var title := "[b]%s[/b]" % str(entry.get("name", id))
	if slot >= 1 and slot <= 4:
		title += " (Alt+%d)" % slot
	var lines: PackedStringArray = [title, str(entry.get("description", ""))]
	var timing := "Recharge : %s s" % RuleValues.text(id + "_cooldown")
	var duration := RuleValues.value(id + "_duration", 0.0)
	timing += " · durée : %s s" % RuleValues.text(id + "_duration") if duration > 0.0 else " · tant que l'unité s'y tient"
	if RuleValues.value(id + "_setup_time", 0.0) > 0.0:
		timing += " · mise en place : %s s" % RuleValues.text(id + "_setup_time")
	lines.append(timing)
	var effects: PackedStringArray = []
	for effect in EFFECTS:
		var key := id + "_" + str(effect[0])
		if not RuleValues.has(key):
			continue
		var amount := RuleValues.value(key)
		if is_zero_approx(amount):
			continue
		if str(effect[2]) == "%":
			effects.append("%s %s%s %%" % [str(effect[1]), "+" if amount > 0.0 else "−", RuleValues.number(absf(amount))])
		else:
			effects.append("%s +%s" % [str(effect[1]), RuleValues.number(amount)])
	if not effects.is_empty():
		lines.append("Effets : " + " · ".join(effects) + ".")
	lines.append(status_text(state))
	return "\n".join(lines)


## État lisible d'une capacité (« Active », « Recharge : 7 s », « Indisponible : au contact… »).
static func status_text(state: Dictionary) -> String:
	if bool(state.get("active", false)):
		var text := "[b]Active[/b]"
		if float(state.get("setup_remaining", 0.0)) > 0.0:
			text += " (mise en place : %d s)" % ceili(float(state["setup_remaining"]))
		elif float(state.get("remaining", 0.0)) > 0.0:
			text += " (encore %d s)" % ceili(float(state["remaining"]))
		return text + " — cliquer pour la lever."
	if bool(state.get("available", false)):
		var ended := str(state.get("ended_reason", ""))
		return "Prête." + (" (Dernier arrêt : %s.)" % ended if ended != "" else "")
	var reason := str(state.get("reason", ""))
	var out := "Indisponible : %s." % reason if reason != "" else "Indisponible."
	var ended := str(state.get("ended_reason", ""))
	if ended != "":
		out += " (Arrêtée : %s.)" % ended
	return out


## Part de la recharge qui reste (0 : prête, 1 : vient de commencer).
static func cooldown_fraction(state: Dictionary) -> float:
	var total := float(state.get("cooldown", 0.0))
	if total <= 0.0:
		return 0.0
	return clampf(float(state.get("cooldown_remaining", 0.0)) / total, 0.0, 1.0)


## Bouton de capacité dans `rect` : fond de parchemin, glyphe, anneau doré si active, cadran de
## recharge (secteur sombre qui se vide dans le sens horaire), voile gris si indisponible.
static func draw_button(canvas: CanvasItem, rect: Rect2, kind: String, state: Dictionary, hovered: bool) -> void:
	var c := rect.get_center()
	var r := minf(rect.size.x, rect.size.y) * 0.5
	var active := bool(state.get("active", false))
	var available := bool(state.get("available", false))
	canvas.draw_circle(c, r, PAPER if not active else Color(1.0, 0.9, 0.55, 0.98))
	var ink := INK if available or active else Color(0.45, 0.4, 0.35)
	if hovered and available:
		ink = GOLD.darkened(0.35)
	draw_ability(canvas, kind, c, r / 8.0, ink)
	var fraction := cooldown_fraction(state)
	if fraction > 0.0 and not active:
		var points := PackedVector2Array([c])
		var steps := maxi(3, int(24 * fraction))
		for i in steps + 1:
			var angle := -PI * 0.5 + TAU * (1.0 - fraction) + TAU * fraction * float(i) / float(steps)
			points.append(c + Vector2(cos(angle), sin(angle)) * r)
		canvas.draw_colored_polygon(points, Color(0.08, 0.05, 0.02, 0.55))
	elif not available and not active:
		canvas.draw_circle(c, r, Color(0.3, 0.28, 0.26, 0.5))
	if active:
		var setup := float(state.get("setup_remaining", 0.0))
		canvas.draw_arc(c, r - 0.8, 0, TAU, 20, GOLD if setup <= 0.0 else Color(GOLD, 0.5), 1.8)
	canvas.draw_arc(c, r, 0, TAU, 20, INK, 1.0)


## Pictogramme de la capacité `kind` centré en `c`, dans un carré d'environ 16 px × `k`.
static func draw_ability(canvas: CanvasItem, kind: String, c: Vector2, k: float, ink: Color = INK) -> void:
	match kind:
		"aimed_shot":  # flèche à plat vers une cible
			canvas.draw_line(c + Vector2(-7, 0) * k, c + Vector2(3, 0) * k, ink, 1.4 * k)
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(5, 0) * k, c + Vector2(1.5, -2.5) * k, c + Vector2(1.5, 2.5) * k]), ink)
			canvas.draw_line(c + Vector2(-7, 0) * k, c + Vector2(-5, -2.2) * k, ink, 1.0 * k)
			canvas.draw_line(c + Vector2(-7, 0) * k, c + Vector2(-5, 2.2) * k, ink, 1.0 * k)
			canvas.draw_arc(c + Vector2(5.5, 0) * k, 2.2 * k, 0, TAU, 10, ink, 1.0 * k)
		"pavise":  # grand pavois à nervure, planté
			var shield := PackedVector2Array([c + Vector2(-4.5, -4) * k, c + Vector2(-2.5, -6.5) * k, c + Vector2(2.5, -6.5) * k, c + Vector2(4.5, -4) * k, c + Vector2(4.5, 6) * k, c + Vector2(-4.5, 6) * k])
			canvas.draw_colored_polygon(shield, ink)
			canvas.draw_line(c + Vector2(0, -5.5) * k, c + Vector2(0, 5) * k, Color(1, 0.85, 0.35), 1.2 * k)
			canvas.draw_line(c + Vector2(-6.5, 7) * k, c + Vector2(6.5, 7) * k, ink, 1.0 * k)
		"banner_rally":  # bannière sur sa hampe
			canvas.draw_line(c + Vector2(-4, -7) * k, c + Vector2(-4, 7) * k, ink, 1.4 * k)
			canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(-3.5, -6.5) * k, c + Vector2(6, -5) * k, c + Vector2(3.5, -2.5) * k, c + Vector2(6, 0) * k, c + Vector2(-3.5, 0.5) * k]), ink)
			canvas.draw_circle(c + Vector2(-4, -7.2) * k, 1.1 * k, ink)
		"close_ranks":  # rangs serrés : trois hommes coude à coude derrière une ligne
			for i in 3:
				var x := -4.5 + i * 4.5
				canvas.draw_circle(c + Vector2(x, -3.5) * k, 1.7 * k, ink)
				canvas.draw_rect(Rect2(c + Vector2(x - 1.8, -1.6) * k, Vector2(3.6, 5.5) * k), ink)
			canvas.draw_line(c + Vector2(-7, 5.8) * k, c + Vector2(7, 5.8) * k, ink, 1.4 * k)
		"planted_pikes":  # piques calées au sol, pointes vers l'avant
			canvas.draw_line(c + Vector2(-7, 6.5) * k, c + Vector2(7, 6.5) * k, ink, 1.2 * k)
			for i in 3:
				var base := c + Vector2(-5.5 + i * 3.5, 6.5) * k
				var tip_point := base + Vector2(7, -11) * k
				canvas.draw_line(base, tip_point, ink, 1.2 * k)
				canvas.draw_colored_polygon(PackedVector2Array([tip_point + Vector2(1.2, -1.9) * k, tip_point + Vector2(-1.2, 0.2) * k, tip_point + Vector2(0.6, 0.9) * k]), ink)
		_:
			canvas.draw_circle(c, 3.0 * k, ink)
