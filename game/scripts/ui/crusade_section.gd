class_name CrusadeSection
extends PanelSection

## Section « Ferveur » du panneau de faction (faction croisée seulement), construite en
## code : jauge 0-100 avec ses seuils, aumônes, état (élan / moral / débandade), bouton « Prêcher
## le passage » et contingents attendus. Aucune règle ici : tout vient de
## `CampaignSim.get_crusade` / `submit_order` (spec JR § 4 et 5) ; les seuils dessinés sur la
## jauge sont lus dans `data/rules/crusade.json` (affichage seul, la vue ne les porte pas encore).

signal passage_preached

const RULES_FILE := "rules/crusade.json"
const GLYPH := "✠"
const MIN_WIDTH := 300.0

var crusade: Dictionary = {}

var header_label: Label
var value_label: Label
var gauge: FervorGauge
var marks_label: Label
var alms_label: Label
var status_label: Label
var target_label: Label
var preach_button: Button
var blocker_label: Label
var pending_box: VBoxContainer
## Posé par la carte de campagne : `submit(order) -> Dictionary` (son, toast, rafraîchissement,
## refus pendant la fin de tour) ; sans lui, l'ordre va droit à la simulation (tests, maquettes).
var submit: Callable = Callable()

static var _rules_cache: Dictionary = {}
static var _rules_dir: String = ""


func _init() -> void:
	super("CrusadeSection", 3)
	var head := HBoxContainer.new()
	header_label = UiBuild.label("Ferveur")
	header_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	UiType.apply(header_label, UiType.BODY)
	head.add_child(header_label)
	value_label = Label.new()
	UiType.apply(value_label, UiType.BODY)
	head.add_child(value_label)
	add_child(head)
	gauge = FervorGauge.new()
	add_child(gauge)
	marks_label = _label("", HudStyle.INK_SOFT)
	add_child(marks_label)
	alms_label = _label("", RichTooltip.INK)
	alms_label.mouse_filter = Control.MOUSE_FILTER_PASS
	add_child(alms_label)
	status_label = _label("", RichTooltip.INK)
	add_child(status_label)
	target_label = _label("", HudStyle.INK_SOFT)
	add_child(target_label)
	preach_button = RichButton.new()
	preach_button.name = "PreachButton"
	preach_button.text = "Prêcher le passage"
	preach_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	preach_button.pressed.connect(func() -> void: request_preach())
	add_child(preach_button)
	blocker_label = _label("", Money.LOSS_COLOR)
	add_child(blocker_label)
	pending_box = UiBuild.vbox(1)
	pending_box.name = "PendingBox"
	add_child(pending_box)
	_add_error_label(MIN_WIDTH, Money.LOSS_COLOR)
	hide()


## Remplit la section ; masquée si le joueur n'est pas la faction croisée (`get_crusade` vide)
## ou si le panneau montre une autre faction que la sienne.
func show_for(player_owned: bool = true, sim: Object = null) -> void:
	read_only = not player_owned
	_sim = _resolve_sim(sim)
	crusade = {}
	if _sim != null and _sim.has_method("get_crusade") and not read_only:
		crusade = _sim.call("get_crusade")
	if crusade.is_empty():
		hide()
		return
	show()
	error_label.hide()
	var fervor := int(crusade.get("fervor", 0))
	var marks := thresholds(crusade)
	value_label.text = "%d / 100" % fervor
	gauge.set_view(fervor, int(crusade.get("floor", 0)), marks)
	gauge.tooltip_text = tooltip(crusade)
	marks_label.text = marks_text(crusade, marks)
	marks_label.visible = marks_label.text != ""
	alms_label.text = "Aumônes : %s par tour" % Money.amount(int(crusade.get("alms", 0)))
	alms_label.tooltip_text = RichTooltip.plain("Aumônes", "\n".join(PackedStringArray([
		"Dons de la chrétienté, versés au trésor chaque tour : plus la ferveur est haute, plus ils sont larges.",
		"Tour passé : %s" % Money.amount(int(crusade.get("alms_last_turn", 0)))])))
	status_label.text = status_text(crusade)
	var deserting := int(crusade.get("desertion_percent", 0)) > 0
	var morale := int(crusade.get("zeal_morale", 0))
	status_label.add_theme_color_override("font_color",
		Money.LOSS_COLOR if deserting or morale < 0 else (Money.GAIN_COLOR if morale > 0 else HudStyle.INK_SOFT))
	var target := str(crusade.get("target_name", ""))
	if bool(crusade.get("target_taken", false)):
		target_label.text = "%s est tenue : la ferveur ne descend plus sous %d." % [target, int(crusade.get("floor", 0))]
	else:
		target_label.text = "But du vœu : %s." % target
	target_label.visible = target != ""
	_show_passage()
	_show_pending(crusade.get("pending", []))


func _show_passage() -> void:
	var blocker := str(crusade.get("passage_blocker", ""))
	var available := bool(crusade.get("passage_available", blocker == ""))
	preach_button.text = "Prêcher le passage (%s)" % Money.amount(int(crusade.get("passage_cost", 0)))
	preach_button.disabled = not available
	var body := promise_text(crusade) if available else "Impossible : %s" % blocker
	preach_button.tooltip_text = RichTooltip.plain("Prêcher le passage", body)
	blocker_label.text = "Impossible : %s" % blocker
	blocker_label.visible = not available and blocker != ""


func _show_pending(pending: Array) -> void:
	UiBuild.clear_children(pending_box)
	pending_box.visible = not pending.is_empty()
	if pending.is_empty():
		return
	pending_box.add_child(_label("Contingents attendus :", HudStyle.INK_SOFT))
	for passage in pending:
		pending_box.add_child(_label("• %s" % pending_text(passage), RichTooltip.INK))


## Ordre `preach_passage` ; en cas de refus, message de la simulation affiché en rouge.
func request_preach() -> Dictionary:
	if _sim == null:
		return {}
	if _submit({"type": "preach_passage"}, func() -> void: show_for(true, _sim), submit):
		passage_preached.emit()
	return last_result


# --- Textes (partagés avec le repère du HUD) ------------------------------------------------


## « Élan de la Croix : moral +10 », « moral −10 », « débandade 5 % par tour » selon la vue.
static func status_text(view: Dictionary) -> String:
	var parts := PackedStringArray()
	var morale := int(view.get("zeal_morale", 0))
	if morale > 0:
		parts.append("Élan de la Croix : moral %s" % signed(morale))
	elif morale < 0:
		parts.append("Ferveur chancelante : moral %s" % signed(morale))
	var desertion := int(view.get("desertion_percent", 0))
	if desertion > 0:
		parts.append("Débandade : %d %% des hommes rentrent chez eux chaque tour" % desertion)
	if parts.is_empty():
		return "Ost résolu : ni élan ni découragement."
	return " · ".join(parts) + "."


## « 3 unités de volontaires débarqueront dans 2 tours. »
static func promise_text(view: Dictionary) -> String:
	return "%s de volontaires débarqueront dans %s dans un port que vous tenez." % [
		count(int(view.get("passage_units", 0)), "unité", "unités"), count(int(view.get("passage_delay", 0)), "tour", "tours")]


## « Limassol : 3 unités, dans 2 tours » (contingent attendu).
static func pending_text(passage: Dictionary) -> String:
	var turns := int(passage.get("turns_left", 0))
	var when := "dans %s" % count(turns, "tour", "tours") if turns > 0 else "ce tour"
	return "%s : %s, %s" % [str(passage.get("port_name", passage.get("port", "?"))),
		count(int(passage.get("units", 0)), "unité", "unités"), when]


static func count(value: int, singular: String, plural: String) -> String:
	return "%d %s" % [value, singular if value <= 1 else plural]


static func signed(value: int) -> String:
	return ("+%d" % value) if value > 0 else ("%s%d" % [Money.MINUS, absi(value)] if value < 0 else "0")


## Seuils de la jauge `{desertion, low, high, high_morale, low_morale, desertion_percent}` : ceux
## de la vue si le cœur les porte, sinon ceux de `data/rules/crusade.json` ; `{}` sans données.
static func thresholds(view: Dictionary = {}) -> Dictionary:
	var rules := _rules()
	var zeal: Dictionary = rules.get("zeal", {})
	var desertion: Dictionary = rules.get("desertion", {})
	var result := {}
	for spec in [["high", "zeal_high_threshold", zeal, "high_threshold"], ["low", "zeal_low_threshold", zeal, "low_threshold"],
			["desertion", "desertion_threshold", desertion, "threshold"], ["high_morale", "zeal_high_morale", zeal, "high_morale"],
			["low_morale", "zeal_low_morale", zeal, "low_morale"], ["desertion_percent", "desertion_men_percent", desertion, "men_percent_per_turn"]]:
		if view.has(spec[1]):
			result[spec[0]] = int(view[spec[1]])
		elif (spec[2] as Dictionary).has(spec[3]):
			result[spec[0]] = int(spec[2][spec[3]])
	return result


## Légende des repères sous la jauge : « Débandade sous 20 · élan à partir de 70 · plancher 50 ».
static func marks_text(view: Dictionary, marks: Dictionary) -> String:
	var parts := PackedStringArray()
	if marks.has("desertion"):
		parts.append("Débandade sous %d" % int(marks["desertion"]))
	if marks.has("high"):
		parts.append("élan à partir de %d" % int(marks["high"]))
	if int(view.get("floor", 0)) > 0:
		parts.append("plancher %d" % int(view.get("floor", 0)))
	var text := " · ".join(parts)
	return text.substr(0, 1).to_upper() + text.substr(1) if text != "" else ""


## Infobulle de la jauge et du repère du HUD : valeur, causes du tour (`changes`), seuils.
static func tooltip(view: Dictionary) -> String:
	var lines := PackedStringArray(["[b]Ferveur : %d / 100[/b]" % int(view.get("fervor", 0)),
		"Elle paie l'ost par les aumônes, le renforce par le passage et le disperse quand elle s'éteint.",
		"", "[b]Ce tour[/b]"])
	var changes: Array = view.get("changes", [])
	var listed := 0
	for change in changes:
		var delta := int(change.get("delta", 0))
		if delta == 0:
			continue
		lines.append("• %s : [color=#%s]%s[/color]" % [str(change.get("cause", "?")), Money.color_of(delta).to_html(false), signed(delta)])
		listed += 1
	if listed == 0:
		lines.append("Aucun mouvement.")
	lines.append("")
	lines.append("[b]État[/b]")
	lines.append(status_text(view))
	lines.append("Aumônes : %s par tour." % Money.amount(int(view.get("alms", 0))))
	var marks := thresholds(view)
	var scale := PackedStringArray()
	if marks.has("high"):
		scale.append("• %d et plus : élan de la Croix%s" % [int(marks["high"]), _morale_note(marks, "high_morale")])
	if marks.has("low"):
		scale.append("• Sous %d : ferveur chancelante%s" % [int(marks["low"]), _morale_note(marks, "low_morale")])
	if marks.has("desertion"):
		var share := " (%d %% des hommes par tour, le double à 0)" % int(marks["desertion_percent"]) if marks.has("desertion_percent") else ""
		scale.append("• Sous %d : débandade%s" % [int(marks["desertion"]), share])
	if int(view.get("floor", 0)) > 0:
		scale.append("• Plancher : %d tant que %s est tenue" % [int(view.get("floor", 0)), str(view.get("target_name", "la cité du vœu"))])
	if not scale.is_empty():
		lines.append("")
		lines.append("[b]Seuils[/b]")
		lines.append_array(scale)
	return "\n".join(lines)


static func _morale_note(marks: Dictionary, key: String) -> String:
	return " (moral %s)" % signed(int(marks[key])) if marks.has(key) else ""


static func _rules() -> Dictionary:
	var dir := DataFile.data_dir()
	if dir != _rules_dir:
		_rules_dir = dir
		_rules_cache = {}
		var path := dir.path_join(RULES_FILE)
		var parsed: Variant = DataFile.parse_file(path) if FileAccess.file_exists(path) else null
		if parsed is Dictionary:
			_rules_cache = parsed
	return _rules_cache


func _label(text: String, color: Color) -> Label:
	var label := RichLabel.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(MIN_WIDTH, 0)
	UiType.apply(label, UiType.CAPTION)
	label.add_theme_color_override("font_color", color)
	return label


## Jauge 0-100 : remplissage teinté selon l'état (débandade, chancelante, résolue, élan), traits
## aux seuils, losange doré au plancher. Infobulle BBCode (`CrusadeSection.tooltip`).
class FervorGauge:
	extends Control

	const BAR_HEIGHT := 14.0
	const TICK_OVERSHOOT := 3.0

	var fervor := 0
	var floor_value := 0
	var marks: Dictionary = {}

	func _init() -> void:
		name = "FervorGauge"
		custom_minimum_size = Vector2(CrusadeSection.MIN_WIDTH, BAR_HEIGHT + TICK_OVERSHOOT * 2.0)
		size_flags_horizontal = Control.SIZE_EXPAND_FILL
		mouse_filter = Control.MOUSE_FILTER_PASS  # infobulle sans bloquer la molette du panneau

	func set_view(value: int, floor_mark: int, thresholds: Dictionary) -> void:
		fervor = clampi(value, 0, 100)
		floor_value = clampi(floor_mark, 0, 100)
		marks = thresholds
		queue_redraw()

	## Encre du remplissage : palette des jauges du HUD (`HudStyle.POOR` / `FAIR` / `GOOD` / `GOLD`).
	func fill_color() -> Color:
		if marks.has("desertion") and fervor < int(marks["desertion"]):
			return HudStyle.POOR
		if marks.has("low") and fervor < int(marks["low"]):
			return HudStyle.FAIR
		if marks.has("high") and fervor >= int(marks["high"]):
			return HudStyle.GOLD
		return HudStyle.GOOD

	func _draw() -> void:
		var bar := Rect2(0.0, TICK_OVERSHOOT, size.x, BAR_HEIGHT)
		draw_rect(bar, HudStyle.PARCHMENT_DARK)
		draw_rect(Rect2(bar.position, Vector2(bar.size.x * fervor / 100.0, bar.size.y)), fill_color())
		draw_rect(bar.grow(-0.5), HudStyle.INK_SOFT, false, 1.0)
		for key in ["desertion", "low", "high"]:
			if not marks.has(key):
				continue
			var x := roundf(bar.size.x * int(marks[key]) / 100.0)
			draw_line(Vector2(x, 0.0), Vector2(x, size.y), HudStyle.INK, 1.0)
		if floor_value > 0:
			var center := Vector2(roundf(bar.size.x * floor_value / 100.0), bar.get_center().y)
			var half := BAR_HEIGHT * 0.5
			var diamond := PackedVector2Array([center + Vector2(0, -half), center + Vector2(half * 0.7, 0),
				center + Vector2(0, half), center + Vector2(-half * 0.7, 0)])
			draw_colored_polygon(diamond, HudStyle.GOLD_PALE)
			draw_polyline(diamond + PackedVector2Array([diamond[0]]), HudStyle.INK, 1.0, true)

	func _make_custom_tooltip(for_text: String) -> Object:
		return TooltipHost.bubble(for_text, self)
