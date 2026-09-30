class_name BattleAlertsColumn
extends PanelContainer

## Lot CB5 (ADR 0095) : colonne d'alertes de bataille, en haut à gauche (le journal est en haut
## à droite, `BattleHud._build_log`). Cinq alertes au plus, glyphe dessiné en code (comme
## `BattleHud._draw_command_icon`, pas d'icône DA5), texte court en français, effacement après
## `cb5_alert_duration_s` (RuleValues) ; deux alertes du même type dans la même zone à moins de
## `cb5_alert_merge_window_s` l'une de l'autre se regroupent (« ×2 »). Un clic sur une ligne émet
## `pinged(x, z)` ; la scène s'en sert pour recentrer la caméra et faire pulser la minicarte.
## Pure présentation : aucune règle, tout vient de `BattleSim.get_alerts()`.

signal pinged(x: float, z: float)

const INK := Color(0.22, 0.14, 0.07)
const GOLD := Color(0.62, 0.47, 0.16)

const LABELS := {
	"rout": "Unité en déroute",
	"general_down": "Général tombé ou capturé",
	"flanked": "Flanc ou dos attaqué",
	"reinforcements": "Renforts en approche",
	"ammo_out": "Munitions épuisées",
	"wall_breached": "Muraille rompue",
	"gate_destroyed": "Porte détruite",
	"square_threatened": "La place est menacée",
}
## Cris courts (spec CB5) : seuls la déroute et la chute du général en ont un. `_detect_events`
## de `BattleAudio` les joue déjà sur la transition d'état ; celui-ci s'appuie sur le
## cooldown/les instances max de l'événement pour ne pas doubler le son.
const CRIES := {"rout": "rout_cry", "general_down": "general_death"}
const CRY_FALLBACK := "ui_alert"

## `{name: value}` par défaut si `RuleValues` n'a pas encore de données (maquette, tests).
const FALLBACK_DURATION_S := 8.0
const FALLBACK_MERGE_WINDOW_S := 5.0
const FALLBACK_MERGE_RADIUS_M := 60.0
const FALLBACK_MAX_SHOWN := 5

## Régie audio optionnelle (`BattleAudio`) ; `null` en test ou en maquette : pas de cri.
var battle_audio: Object = null

var _entries: Array[Dictionary] = []  # {kind, x, z, side, unit, last_time, remaining, count}
var _box: VBoxContainer


func _ready() -> void:
	name = "BattleAlertsColumn"
	mouse_filter = Control.MOUSE_FILTER_PASS
	anchor_left = 0.0
	anchor_right = 0.0
	anchor_top = 0.0
	anchor_bottom = 0.0
	offset_left = 8
	offset_top = 8
	custom_minimum_size = Vector2(240, 0)
	_box = VBoxContainer.new()
	_box.add_theme_constant_override("separation", 2)
	add_child(_box)
	visible = false
	set_process(false)


func _rule(name: String, fallback: float) -> float:
	return RuleValues.value(name, fallback) if RuleValues.has(name) else fallback


func duration_s() -> float:
	return _rule("cb5_alert_duration_s", FALLBACK_DURATION_S)


func merge_window_s() -> float:
	return _rule("cb5_alert_merge_window_s", FALLBACK_MERGE_WINDOW_S)


func merge_radius_m() -> float:
	return _rule("cb5_alert_merge_radius_m", FALLBACK_MERGE_RADIUS_M)


func max_shown() -> int:
	return int(_rule("cb5_alert_max_shown", float(FALLBACK_MAX_SHOWN)))


## Alertes `[{kind, time, x, z, side, unit}]` de `BattleSim.get_alerts()`, ajoutées ou fusionnées
## avec une entrée récente de même type dans la même zone.
func push_alerts(alerts: Array) -> void:
	if alerts.is_empty():
		return
	for raw in alerts:
		var alert: Dictionary = raw
		_push_one(alert)
	_rebuild()
	set_process(true)


func _push_one(alert: Dictionary) -> void:
	var kind := str(alert.get("kind", ""))
	var x := float(alert.get("x", 0.0))
	var z := float(alert.get("z", 0.0))
	var radius := merge_radius_m()
	var window := merge_window_s()
	var time := float(alert.get("time", 0.0))
	for entry in _entries:
		if str(entry["kind"]) != kind:
			continue
		var d := Vector2(x, z).distance_to(Vector2(float(entry["x"]), float(entry["z"])))
		if d <= radius and time - float(entry["last_time"]) <= window:
			entry["last_time"] = time
			entry["count"] = int(entry["count"]) + 1
			entry["remaining"] = duration_s()
			return
	_entries.append({
		"kind": kind,
		"x": x,
		"z": z,
		"side": str(alert.get("side", "")),
		"unit": int(alert.get("unit", -1)),
		"last_time": time,
		"remaining": duration_s(),
		"count": 1,
	})
	_cry(kind, x, z)


func _cry(kind: String, x: float, z: float) -> void:
	if battle_audio == null or not battle_audio.has_method("play_event"):
		return
	var event: String = str(CRIES.get(kind, ""))
	if event == "":
		return
	var pos := Vector3(x, 0.0, z)
	if not battle_audio.call("play_event", event, pos):
		battle_audio.call("play_event", CRY_FALLBACK, pos)


func _process(delta: float) -> void:
	var changed := false
	for i in range(_entries.size() - 1, -1, -1):
		_entries[i]["remaining"] = float(_entries[i]["remaining"]) - delta
		if float(_entries[i]["remaining"]) <= 0.0:
			_entries.remove_at(i)
			changed = true
	if changed:
		_rebuild()
	if _entries.is_empty():
		set_process(false)


## Alertes affichées, les plus importantes d'abord (`data/rules/battle_alerts.json`), au plus
## `max_shown()`.
func visible_entries() -> Array[Dictionary]:
	var sorted := _entries.duplicate()
	sorted.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var ia := _importance(str(a["kind"]))
		var ib := _importance(str(b["kind"]))
		if ia != ib:
			return ia > ib
		return float(a["last_time"]) > float(b["last_time"]))
	var out: Array[Dictionary] = []
	for entry in sorted:
		out.append(entry)
		if out.size() >= max_shown():
			break
	return out


## Ordre d'affichage : `cb5_alert_importance_<type>` (`RuleValues`, un par `AlertKind::key()`,
## depuis `data/rules/battle_alerts.json` § `importance`) ; repli sur les poids du fichier
## d'origine si les données ne sont pas encore chargées (maquette, tests).
const IMPORTANCE_FALLBACK := {
	"general_down": 100, "square_threatened": 95, "gate_destroyed": 90, "wall_breached": 80,
	"rout": 70, "flanked": 60, "reinforcements": 50, "ammo_out": 40,
}


func _importance(kind: String) -> int:
	var name := "cb5_alert_importance_" + kind
	if RuleValues.has(name):
		return int(RuleValues.value(name))
	return int(IMPORTANCE_FALLBACK.get(kind, 0))


func _rebuild() -> void:
	for child in _box.get_children():
		child.queue_free()
	var shown := visible_entries()
	for entry in shown:
		_box.add_child(_row(entry))
	# VN4 : sans alerte, pas de bandeau de parchemin vide en haut à gauche.
	visible = not shown.is_empty()


func _row(entry: Dictionary) -> Control:
	var row := Button.new()
	row.flat = true
	row.focus_mode = Control.FOCUS_NONE
	row.alignment = HORIZONTAL_ALIGNMENT_LEFT
	row.custom_minimum_size = Vector2(0, 28)
	var kind := str(entry["kind"])
	var text: String = str(LABELS.get(kind, kind))
	if int(entry["count"]) > 1:
		text += " (×%d)" % int(entry["count"])
	row.text = "   " + text
	RichTooltip.attach_plain(row, "click_camera_focus")
	var glyph := Control.new()
	glyph.custom_minimum_size = Vector2(22, 22)
	glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
	glyph.set_meta("kind", kind)
	glyph.draw.connect(_draw_glyph.bind(glyph))
	row.add_child(glyph)
	glyph.set_anchors_preset(Control.PRESET_CENTER_LEFT)
	glyph.position = Vector2(4, 3)
	var x := float(entry["x"])
	var z := float(entry["z"])
	row.pressed.connect(func() -> void: pinged.emit(x, z))
	return row


## Icône à l'encre DA5 par type d'alerte (lot CB, clé `battle_alert_<kind>`) ; glyphe vectoriel
## en repli si le PNG manque, même esprit que `BattleHud._draw_command_icon`.
func _draw_glyph(glyph: Control) -> void:
	var kind := str(glyph.get_meta("kind", ""))
	var c := glyph.size * 0.5
	if BattleModeIcons.draw_ink_icon(glyph, "battle_alert_" + kind, c, minf(glyph.size.x, glyph.size.y) - 2.0, INK):
		return
	match kind:
		"rout":  # flèche fuyante
			glyph.draw_line(c + Vector2(-8, 0), c + Vector2(6, 0), INK, 2.0)
			glyph.draw_colored_polygon(PackedVector2Array([c + Vector2(8, 0), c + Vector2(3, -4), c + Vector2(3, 4)]), INK)
		"general_down":  # couronne barrée
			glyph.draw_rect(Rect2(c + Vector2(-7, 2), Vector2(14, 4)), INK)
			for i in 3:
				glyph.draw_colored_polygon(PackedVector2Array([
					c + Vector2(-7 + i * 7, 2), c + Vector2(-4 + i * 7, -6), c + Vector2(-1 + i * 7, 2)]), INK)
			glyph.draw_line(c + Vector2(-9, 8), c + Vector2(9, -8), Color(0.55, 0.1, 0.08), 2.0)
		"flanked":  # deux flèches convergentes
			glyph.draw_line(c + Vector2(-9, -6), c + Vector2(-1, 0), INK, 2.0)
			glyph.draw_line(c + Vector2(9, -6), c + Vector2(1, 0), INK, 2.0)
			glyph.draw_circle(c, 2.5, INK)
		"reinforcements":  # croix montante
			glyph.draw_line(c + Vector2(0, 8), c + Vector2(0, -8), INK, 2.5)
			glyph.draw_line(c + Vector2(-6, 1), c + Vector2(6, 1), INK, 2.5)
			glyph.draw_colored_polygon(PackedVector2Array([c + Vector2(0, -10), c + Vector2(-4, -5), c + Vector2(4, -5)]), GOLD)
		"ammo_out":  # flèche brisée
			glyph.draw_line(c + Vector2(-8, -6), c + Vector2(-1, 1), INK, 2.0)
			glyph.draw_line(c + Vector2(1, -1), c + Vector2(8, 6), INK, 2.0)
			glyph.draw_circle(c, 1.5, Color(0.55, 0.1, 0.08))
		"wall_breached":  # muraille avec une brèche
			glyph.draw_rect(Rect2(c + Vector2(-9, -2), Vector2(6, 8)), INK)
			glyph.draw_rect(Rect2(c + Vector2(3, -2), Vector2(6, 8)), INK)
		"square_threatened":  # drapeau planté sur la place
			glyph.draw_line(c + Vector2(-5, 9), c + Vector2(-5, -9), INK, 2.0)
			glyph.draw_colored_polygon(PackedVector2Array([
				c + Vector2(-4, -9), c + Vector2(8, -6), c + Vector2(-4, -2)]), Color(0.55, 0.1, 0.08))
			glyph.draw_line(c + Vector2(-9, 9), c + Vector2(9, 9), GOLD, 2.0)
		"gate_destroyed":  # porte effondrée
			glyph.draw_rect(Rect2(c + Vector2(-9, -8), Vector2(4, 16)), INK)
			glyph.draw_rect(Rect2(c + Vector2(5, -8), Vector2(4, 16)), INK)
			glyph.draw_line(c + Vector2(-4, 4), c + Vector2(4, -3), INK, 2.5)
		_:
			glyph.draw_circle(c, 4.0, INK)
